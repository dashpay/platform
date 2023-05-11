use dapi_grpc::platform::v0::{get_proofs_request, GetProofsRequest, GetProofsResponse};

use dpp::document::document_transition::DocumentTransitionAction;
use dpp::document::Document;
use dpp::identity::PartialIdentity;
use dpp::state_transition::{StateTransition, StateTransitionAction, StateTransitionLike};
use drive::drive::Drive;
use drive::query::SingleDocumentDriveQuery;
use drive_abci::abci::AbciApplication;
use drive_abci::platform::PlatformRef;
use drive_abci::rpc::core::MockCoreRPCLike;
use drive_abci::validation::state_transition::StateTransitionValidation;

use prost::Message;

use tenderdash_abci::Application;

pub(crate) fn verify_state_transitions_were_executed(
    abci_app: &AbciApplication<MockCoreRPCLike>,
    expected_root_hash: &[u8; 32],
    state_transitions: &Vec<StateTransition>,
) -> bool {
    let state = abci_app.platform.state.read().unwrap();
    let platform = PlatformRef {
        drive: &abci_app.platform.drive,
        state: &state,
        config: &abci_app.platform.config,
        core_rpc: &abci_app.platform.core_rpc,
    };

    //actions are easier to transform to queries
    let actions = state_transitions
        .iter()
        .map(|state_transition| {
            state_transition
                .transform_into_action(&platform, None)
                .expect("expected state transitions to validate")
                .into_data()
                .expect(
                    format!(
                        "expected state transitions to be valid {}",
                        state_transition.get_type()
                    )
                    .as_str(),
                )
        })
        .collect::<Vec<_>>();

    for action in &actions {
        let mut proofs_request = GetProofsRequest {
            identities: vec![],
            contracts: vec![],
            documents: vec![],
        };

        match action {
            StateTransitionAction::DataContractCreateAction(data_contract_create) => {
                proofs_request
                    .contracts
                    .push(get_proofs_request::ContractRequest {
                        contract_id: data_contract_create.data_contract.id.to_vec(),
                    });
                let result = abci_app
                    .platform
                    .query("/proofs", &proofs_request.encode_to_vec())
                    .expect("expected to query proofs");
                let serialized_get_proofs_response =
                    result.into_data().expect("expected queries to be valid");

                let GetProofsResponse { proof, metadata: _ } =
                    GetProofsResponse::decode(serialized_get_proofs_response.as_slice())
                        .expect("expected to decode proof response");

                let response_proof = proof.expect("proof should be present");

                // we expect to get an identity that matches the state transition
                let (root_hash, contract) = Drive::verify_contract(
                    &response_proof.grovedb_proof,
                    None,
                    false,
                    data_contract_create.data_contract.id.into_buffer(),
                )
                .expect("expected to verify full identity");
                assert_eq!(
                    &root_hash, expected_root_hash,
                    "state last block info {:?}",
                    platform.state.last_committed_block_info
                );
                assert_eq!(
                    &contract.expect("expected a contract"),
                    &data_contract_create.data_contract,
                )
            }
            StateTransitionAction::DataContractUpdateAction(data_contract_update) => {
                proofs_request
                    .contracts
                    .push(get_proofs_request::ContractRequest {
                        contract_id: data_contract_update.data_contract.id.to_vec(),
                    });
                let result = abci_app
                    .platform
                    .query("/proofs", &proofs_request.encode_to_vec())
                    .expect("expected to query proofs");
                let serialized_get_proofs_response =
                    result.into_data().expect("expected queries to be valid");

                let GetProofsResponse { proof, metadata: _ } =
                    GetProofsResponse::decode(serialized_get_proofs_response.as_slice())
                        .expect("expected to decode proof response");

                let response_proof = proof.expect("proof should be present");

                // we expect to get an identity that matches the state transition
                let (root_hash, contract) = Drive::verify_contract(
                    &response_proof.grovedb_proof,
                    None,
                    false,
                    data_contract_update.data_contract.id.into_buffer(),
                )
                .expect("expected to verify full identity");
                assert_eq!(
                    &root_hash, expected_root_hash,
                    "state last block info {:?}",
                    platform.state.last_committed_block_info
                );
                assert_eq!(
                    &contract.expect("expected a contract"),
                    &data_contract_update.data_contract,
                )
            }
            StateTransitionAction::DocumentsBatchAction(documents_batch_transition) => {
                documents_batch_transition
                    .transitions
                    .iter()
                    .for_each(|transition| {
                        proofs_request
                            .documents
                            .push(get_proofs_request::DocumentRequest {
                                contract_id: transition.base().data_contract_id.to_vec(),
                                document_type: transition.base().document_type_name.clone(),
                                document_type_keeps_history: transition
                                    .base()
                                    .data_contract
                                    .document_type_for_name(
                                        transition.base().document_type_name.as_str(),
                                    )
                                    .expect("get document type")
                                    .documents_keep_history,
                                document_id: transition.base().id.to_vec(),
                            });
                    });
                let result = abci_app
                    .platform
                    .query("/proofs", &proofs_request.encode_to_vec())
                    .expect("expected to query proofs");
                let serialized_get_proofs_response =
                    result.into_data().expect("expected queries to be valid");

                let GetProofsResponse { proof, metadata: _ } =
                    GetProofsResponse::decode(serialized_get_proofs_response.as_slice())
                        .expect("expected to decode proof response");

                let response_proof = proof.expect("proof should be present");

                for document_transition_action in documents_batch_transition.transitions.iter() {
                    let document_type = document_transition_action
                        .base()
                        .data_contract
                        .document_type_for_name(
                            document_transition_action
                                .base()
                                .document_type_name
                                .as_str(),
                        )
                        .expect("get document type");
                    let query = SingleDocumentDriveQuery {
                        contract_id: document_transition_action
                            .base()
                            .data_contract_id
                            .into_buffer(),
                        document_type_name: document_transition_action
                            .base()
                            .document_type_name
                            .clone(),
                        document_type_keeps_history: document_type.documents_keep_history,
                        document_id: document_transition_action.base().id.into_buffer(),
                        block_time_ms: None, //None because we want latest
                    };

                    let (root_hash, document) = query
                        .verify_proof(false, &response_proof.grovedb_proof, document_type)
                        .expect("expected to verify a document");

                    assert_eq!(
                        &root_hash, expected_root_hash,
                        "state last block info {:?}",
                        platform.state.last_committed_block_info
                    );

                    match document_transition_action {
                        DocumentTransitionAction::CreateAction(creation_action) => {
                            let document = document.expect("expected a document");
                            assert_eq!(
                                document,
                                Document::try_from_create_transition(
                                    creation_action,
                                    documents_batch_transition.owner_id
                                )
                                .expect("expected to get document")
                            );
                        }
                        DocumentTransitionAction::ReplaceAction(replace_action) => {
                            // it's also possible we deleted something we replaced
                            if let Some(document) = document {
                                assert_eq!(
                                    document,
                                    Document::try_from_replace_transition(
                                        replace_action,
                                        documents_batch_transition.owner_id
                                    )
                                    .expect("expected to get document")
                                );
                            }
                        }
                        DocumentTransitionAction::DeleteAction(_) => {
                            // we expect no document
                            assert!(document.is_none());
                        }
                    }
                }
            }
            StateTransitionAction::IdentityCreateAction(identity_create_transition) => {
                proofs_request
                    .identities
                    .push(get_proofs_request::IdentityRequest {
                        identity_id: identity_create_transition.identity_id.to_vec(),
                        request_type: get_proofs_request::identity_request::Type::FullIdentity
                            .into(),
                    });
                let result = abci_app
                    .platform
                    .query("/proofs", &proofs_request.encode_to_vec())
                    .expect("expected to query proofs");
                let serialized_get_proofs_response =
                    result.into_data().expect("expected queries to be valid");

                let GetProofsResponse { proof, metadata: _ } =
                    GetProofsResponse::decode(serialized_get_proofs_response.as_slice())
                        .expect("expected to decode proof response");

                let response_proof = proof.expect("proof should be present");

                // we expect to get an identity that matches the state transition
                let (root_hash, identity) = Drive::verify_full_identity_by_identity_id(
                    &response_proof.grovedb_proof,
                    false,
                    identity_create_transition.identity_id.into_buffer(),
                )
                .expect("expected to verify full identity");
                assert_eq!(
                    &root_hash, expected_root_hash,
                    "state last block info {:?}",
                    platform.state.last_committed_block_info
                );
                assert_eq!(
                    identity
                        .expect("expected an identity")
                        .into_partial_identity_info_no_balance(),
                    PartialIdentity {
                        id: identity_create_transition.identity_id,
                        loaded_public_keys: identity_create_transition
                            .public_keys
                            .iter()
                            .map(|key| (key.id, key.clone()))
                            .collect(),
                        balance: None,
                        revision: Some(0),
                        not_found_public_keys: Default::default(),
                    }
                )
            }
            StateTransitionAction::IdentityTopUpAction(identity_top_up_transition) => {
                proofs_request
                    .identities
                    .push(get_proofs_request::IdentityRequest {
                        identity_id: identity_top_up_transition.identity_id.to_vec(),
                        request_type: get_proofs_request::identity_request::Type::Balance.into(),
                    });
                let result = abci_app
                    .platform
                    .query("/proofs", &proofs_request.encode_to_vec())
                    .expect("expected to query proofs");
                let serialized_get_proofs_response =
                    result.into_data().expect("expected queries to be valid");

                let GetProofsResponse { proof, metadata: _ } =
                    GetProofsResponse::decode(serialized_get_proofs_response.as_slice())
                        .expect("expected to decode proof response");

                let response_proof = proof.expect("proof should be present");

                // we expect to get an identity that matches the state transition
                let (root_hash, balance) = Drive::verify_identity_balance_for_identity_id(
                    &response_proof.grovedb_proof,
                    identity_top_up_transition.identity_id.into_buffer(),
                )
                .expect("expected to verify balance identity");
                let balance = balance.expect("expected a balance");
                assert_eq!(
                    &root_hash, expected_root_hash,
                    "state last block info {:?}",
                    platform.state.last_committed_block_info
                );

                //while this isn't 100% sure to be true (in the case of debt,
                // for the tests we have we can use it
                assert!(identity_top_up_transition.top_up_balance_amount <= balance);
            }
            StateTransitionAction::IdentityCreditWithdrawalAction(
                identity_credit_withdrawal_transition,
            ) => {
                proofs_request
                    .identities
                    .push(get_proofs_request::IdentityRequest {
                        identity_id: identity_credit_withdrawal_transition.identity_id.to_vec(),
                        request_type: get_proofs_request::identity_request::Type::Balance.into(),
                    });
                //todo: we should also verify the document
                // proofs_request.documents.push(get_proofs_request::DocumentProofRequest {
                //     contract_id: vec![],
                //     document_type: "".to_string(),
                //     document_type_keeps_history: false,
                //     document_id: vec![],
                // } );
                // we expect to get an identity that matches the state transition

                let result = abci_app
                    .platform
                    .query("/proofs", &proofs_request.encode_to_vec())
                    .expect("expected to query proofs");

                let serialized_get_proofs_response =
                    result.into_data().expect("expected queries to be valid");

                let GetProofsResponse { proof, metadata: _ } =
                    GetProofsResponse::decode(serialized_get_proofs_response.as_slice())
                        .expect("expected to decode proof response");

                let response_proof = proof.expect("proof should be present");

                // we expect to get an identity that matches the state transition
                let (root_hash, balance) = Drive::verify_identity_balance_for_identity_id(
                    &response_proof.grovedb_proof,
                    identity_credit_withdrawal_transition
                        .identity_id
                        .into_buffer(),
                )
                .expect("expected to verify balance identity");
                let _balance = balance.expect("expected a balance");
                assert_eq!(
                    &root_hash, expected_root_hash,
                    "state last block info {:?}",
                    platform.state.last_committed_block_info
                );

                //todo: we need to do more here
            }
            StateTransitionAction::IdentityUpdateAction(identity_update_transition) => {
                proofs_request
                    .identities
                    .push(get_proofs_request::IdentityRequest {
                        identity_id: identity_update_transition.identity_id.to_vec(),
                        request_type: get_proofs_request::identity_request::Type::Keys.into(),
                    });
                let result = abci_app
                    .platform
                    .query("/proofs", &proofs_request.encode_to_vec())
                    .expect("expected to query proofs");
                let serialized_get_proofs_response =
                    result.into_data().expect("expected queries to be valid");

                let GetProofsResponse { proof, metadata: _ } =
                    GetProofsResponse::decode(serialized_get_proofs_response.as_slice())
                        .expect("expected to decode proof response");

                let response_proof = proof.expect("proof should be present");

                // we expect to get an identity that matches the state transition
                let (root_hash, identity) = Drive::verify_identity_keys_by_identity_id(
                    &response_proof.grovedb_proof,
                    false,
                    identity_update_transition.identity_id.into_buffer(),
                )
                .expect("expected to verify identity keys");
                let identity = identity.expect("expected an identity");
                assert_eq!(
                    &root_hash, expected_root_hash,
                    "state last block info {:?}",
                    platform.state.last_committed_block_info
                );
                // we need to verify that the partial identity has all keys we added
                let has_all_keys = identity_update_transition
                    .add_public_keys
                    .iter()
                    .all(|added| identity.loaded_public_keys.contains_key(&added.id));
                let has_no_removed_key = !identity_update_transition
                    .disable_public_keys
                    .iter()
                    .any(|removed| identity.loaded_public_keys.contains_key(removed));
                assert!(has_all_keys);
                assert!(has_no_removed_key);
            }
        }
    }

    true
}
