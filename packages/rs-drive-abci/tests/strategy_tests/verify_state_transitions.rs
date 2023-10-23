use dapi_grpc::platform::v0::{
    get_proofs_request, get_proofs_response, GetProofsRequest, GetProofsResponse,
};

use dpp::document::Document;
use dpp::identity::PartialIdentity;
use dpp::state_transition::{StateTransition, StateTransitionLike};
use drive::drive::Drive;
use drive::query::SingleDocumentDriveQuery;
use drive_abci::abci::AbciApplication;
use drive_abci::platform_types::platform::PlatformRef;
use drive_abci::rpc::core::MockCoreRPCLike;

use dapi_grpc::platform::v0::get_proofs_request::{get_proofs_request_v0, GetProofsRequestV0};
use dapi_grpc::platform::v0::get_proofs_response::GetProofsResponseV0;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::version::PlatformVersion;
use drive::state_transition_action::document::documents_batch::document_transition::DocumentTransitionAction;
use drive::state_transition_action::StateTransitionAction;
use drive_abci::execution::validation::state_transition::transformer::StateTransitionActionTransformerV0;
use drive_abci::platform_types::platform_state::v0::PlatformStateV0Methods;
use prost::Message;
use tenderdash_abci::proto::abci::ExecTxResult;

use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dpp::platform_value::string_encoding::Encoding;
use dpp::state_transition::documents_batch_transition::accessors::DocumentsBatchTransitionAccessorsV0;
use dpp::state_transition::documents_batch_transition::document_base_transition::v0::v0_methods::DocumentBaseTransitionV0Methods;
use dpp::state_transition::documents_batch_transition::document_transition::action_type::TransitionActionTypeGetter;
use dpp::state_transition::documents_batch_transition::document_transition::DocumentTransitionV0Methods;
use drive::state_transition_action::document::documents_batch::document_transition::document_base_transition_action::DocumentBaseTransitionActionAccessorsV0;
use drive::state_transition_action::document::documents_batch::document_transition::document_create_transition_action::DocumentFromCreateTransition;
use drive::state_transition_action::document::documents_batch::document_transition::document_replace_transition_action::DocumentFromReplaceTransition;

pub(crate) fn verify_state_transitions_were_or_were_not_executed(
    abci_app: &AbciApplication<MockCoreRPCLike>,
    expected_root_hash: &[u8; 32],
    state_transitions: &Vec<(StateTransition, ExecTxResult)>,
    platform_version: &PlatformVersion,
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
        .enumerate()
        .map(|(num, (state_transition, result))| {
            if let StateTransition::DocumentsBatch(batch) = state_transition {
                let _first = batch.transitions().first().unwrap();

                // dbg!(batch.transitions().len(), hex::encode(first.base().id()), state.height(), first.to_string());
            }

            let consensus_validation_result = state_transition
                .transform_into_action(&platform, false, None)
                .expect("expected state transitions to validate");

            if !consensus_validation_result.is_valid() {
                panic!(
                    "expected state transition {:?} to be valid, errors are {:?}",
                    num, consensus_validation_result.errors
                )
            }
            let action = consensus_validation_result.into_data().unwrap_or_else(|_| {
                panic!(
                    "expected state transitions to be valid {:?}",
                    state_transition
                )
            });
            (action, result.code == 0)
        })
        .collect::<Vec<_>>();

    for (action, was_executed) in &actions {
        let mut proofs_request = GetProofsRequestV0 {
            identities: vec![],
            contracts: vec![],
            documents: vec![],
        };

        match action {
            StateTransitionAction::DataContractCreateAction(data_contract_create) => {
                proofs_request
                    .contracts
                    .push(get_proofs_request_v0::ContractRequest {
                        contract_id: data_contract_create.data_contract_ref().id().to_vec(),
                    });
                let result = abci_app
                    .platform
                    .query("/proofs", &proofs_request.encode_to_vec(), platform_version)
                    .expect("expected to query proofs");
                let serialized_get_proofs_response =
                    result.into_data().expect("expected queries to be valid");

                let GetProofsResponse { version } =
                    GetProofsResponse::decode(serialized_get_proofs_response.as_slice())
                        .expect("expected to decode proof response");

                let get_proofs_response::Version::V0(GetProofsResponseV0 { proof, metadata: _ }) =
                    version.expect("expected a versioned response");

                let response_proof = proof.expect("proof should be present");

                // we expect to get an identity that matches the state transition
                let (root_hash, contract) = Drive::verify_contract(
                    &response_proof.grovedb_proof,
                    None,
                    false,
                    data_contract_create.data_contract_ref().id().into_buffer(),
                    platform_version,
                )
                .expect("expected to verify contract");
                assert_eq!(
                    &root_hash,
                    expected_root_hash,
                    "state last block info {:?}",
                    platform.state.last_committed_block_info()
                );

                if *was_executed {
                    assert_eq!(
                        &contract.expect("expected a contract"),
                        data_contract_create.data_contract_ref(),
                    )
                } else {
                    //there is the possibility that the state transition was not executed because it already existed,
                    // we can discount that for now in tests
                    assert!(contract.is_none(),)
                }
            }
            StateTransitionAction::DataContractUpdateAction(data_contract_update) => {
                proofs_request
                    .contracts
                    .push(get_proofs_request_v0::ContractRequest {
                        contract_id: data_contract_update.data_contract_ref().id().to_vec(),
                    });
                let result = abci_app
                    .platform
                    .query("/proofs", &proofs_request.encode_to_vec(), platform_version)
                    .expect("expected to query proofs");
                let serialized_get_proofs_response =
                    result.into_data().expect("expected queries to be valid");

                let GetProofsResponse { version } =
                    GetProofsResponse::decode(serialized_get_proofs_response.as_slice())
                        .expect("expected to decode proof response");

                let get_proofs_response::Version::V0(GetProofsResponseV0 { proof, metadata: _ }) =
                    version.expect("expected a versioned response");

                let response_proof = proof.expect("proof should be present");

                // we expect to get an identity that matches the state transition
                let (root_hash, contract) = Drive::verify_contract(
                    &response_proof.grovedb_proof,
                    None,
                    false,
                    data_contract_update.data_contract_ref().id().into_buffer(),
                    platform_version,
                )
                .expect("expected to verify full identity");
                assert_eq!(
                    &root_hash,
                    expected_root_hash,
                    "state last block info {:?}",
                    platform.state.last_committed_block_info()
                );
                if *was_executed {
                    assert_eq!(
                        &contract.expect("expected a contract"),
                        data_contract_update.data_contract_ref(),
                    );
                } else if contract.is_some() {
                    //there is the possibility that the state transition was not executed and the state is equal to the previous
                    // state, aka there would have been no change anyways, we can discount that for now
                    assert_ne!(
                        &contract.expect("expected a contract"),
                        data_contract_update.data_contract_ref(),
                    );
                }
            }
            StateTransitionAction::DocumentsBatchAction(documents_batch_transition) => {
                documents_batch_transition
                    .transitions()
                    .iter()
                    .for_each(|transition| {
                        proofs_request
                            .documents
                            .push(get_proofs_request_v0::DocumentRequest {
                                contract_id: transition.base().data_contract_id().to_vec(),
                                document_type: transition.base().document_type_name().clone(),
                                document_type_keeps_history: transition
                                    .base()
                                    .data_contract_fetch_info()
                                    .contract
                                    .document_type_for_name(
                                        transition.base().document_type_name().as_str(),
                                    )
                                    .expect("get document type")
                                    .documents_keep_history(),
                                document_id: transition.base().id().to_vec(),
                            });
                    });
                let result = abci_app
                    .platform
                    .query("/proofs", &proofs_request.encode_to_vec(), platform_version)
                    .expect("expected to query proofs");
                let serialized_get_proofs_response =
                    result.into_data().expect("expected queries to be valid");

                let GetProofsResponse { version } =
                    GetProofsResponse::decode(serialized_get_proofs_response.as_slice())
                        .expect("expected to decode proof response");

                let get_proofs_response::Version::V0(GetProofsResponseV0 { proof, metadata: _ }) =
                    version.expect("expected a versioned response");

                let response_proof = proof.expect("proof should be present");

                for document_transition_action in documents_batch_transition.transitions().iter() {
                    let contract_fetch_info =
                        document_transition_action.base().data_contract_fetch_info();

                    let document_type = contract_fetch_info
                        .contract
                        .document_type_for_name(
                            document_transition_action
                                .base()
                                .document_type_name()
                                .as_str(),
                        )
                        .expect("get document type");
                    let query = SingleDocumentDriveQuery {
                        contract_id: document_transition_action
                            .base()
                            .data_contract_id()
                            .into_buffer(),
                        document_type_name: document_transition_action
                            .base()
                            .document_type_name()
                            .clone(),
                        document_type_keeps_history: document_type.documents_keep_history(),
                        document_id: document_transition_action.base().id().into_buffer(),
                        block_time_ms: None, //None because we want latest
                    };

                    // dbg!(
                    //     platform.state.height(),
                    //     document_transition_action.action_type(),
                    //     document_transition_action
                    //         .base()
                    //         .id()
                    //         .to_string(Encoding::Base58)
                    // );

                    let (root_hash, document) = query
                        .verify_proof(
                            false,
                            &response_proof.grovedb_proof,
                            document_type,
                            platform_version,
                        )
                        .expect("expected to verify a document");

                    assert_eq!(
                        &root_hash,
                        expected_root_hash,
                        "state last block info {:?}",
                        platform.state.last_committed_block_info()
                    );

                    match document_transition_action {
                        DocumentTransitionAction::CreateAction(creation_action) => {
                            if *was_executed {
                                let document = document.expect("expected a document");
                                // dbg!(
                                //     &document,
                                //     Document::try_from_create_transition(
                                //         creation_action,
                                //         documents_batch_transition.owner_id(),
                                //         platform_version,
                                //     )
                                //     .expect("expected to get document")
                                // );
                                assert_eq!(
                                    document,
                                    Document::try_from_create_transition(
                                        creation_action,
                                        documents_batch_transition.owner_id(),
                                        platform_version,
                                    )
                                    .expect("expected to get document")
                                );
                            } else {
                                //there is the possibility that the state transition was not executed because it already existed,
                                // we can discount that for now in tests
                                assert!(document.is_none());
                            }
                        }
                        DocumentTransitionAction::ReplaceAction(replace_action) => {
                            if *was_executed {
                                // it's also possible we deleted something we replaced
                                if let Some(document) = document {
                                    assert_eq!(
                                        document,
                                        Document::try_from_replace_transition(
                                            replace_action,
                                            documents_batch_transition.owner_id(),
                                            platform_version,
                                        )
                                        .expect("expected to get document")
                                    );
                                }
                            } else {
                                //there is the possibility that the state transition was not executed and the state is equal to the previous
                                // state, aka there would have been no change anyways, we can discount that for now
                                if let Some(document) = document {
                                    assert_ne!(
                                        document,
                                        Document::try_from_replace_transition(
                                            replace_action,
                                            documents_batch_transition.owner_id(),
                                            platform_version,
                                        )
                                        .expect("expected to get document")
                                    );
                                }
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
                    .push(get_proofs_request_v0::IdentityRequest {
                        identity_id: identity_create_transition.identity_id().to_vec(),
                        request_type: get_proofs_request_v0::identity_request::Type::FullIdentity
                            .into(),
                    });
                let result = abci_app
                    .platform
                    .query("/proofs", &proofs_request.encode_to_vec(), platform_version)
                    .expect("expected to query proofs");
                let serialized_get_proofs_response =
                    result.into_data().expect("expected queries to be valid");

                let GetProofsResponse { version } =
                    GetProofsResponse::decode(serialized_get_proofs_response.as_slice())
                        .expect("expected to decode proof response");

                let get_proofs_response::Version::V0(GetProofsResponseV0 { proof, metadata: _ }) =
                    version.expect("expected a versioned response");

                let response_proof = proof.expect("proof should be present");

                // we expect to get an identity that matches the state transition
                let (root_hash, identity) = Drive::verify_full_identity_by_identity_id(
                    &response_proof.grovedb_proof,
                    false,
                    identity_create_transition.identity_id().into_buffer(),
                    platform_version,
                )
                .expect("expected to verify full identity");
                assert_eq!(
                    &root_hash,
                    expected_root_hash,
                    "state last block info {:?}",
                    platform.state.last_committed_block_info()
                );
                if *was_executed {
                    assert_eq!(
                        identity
                            .expect("expected an identity")
                            .into_partial_identity_info_no_balance(),
                        PartialIdentity {
                            id: identity_create_transition.identity_id(),
                            loaded_public_keys: identity_create_transition
                                .public_keys()
                                .iter()
                                .map(|key| (key.id(), key.clone()))
                                .collect(),
                            balance: None,
                            revision: Some(0),
                            not_found_public_keys: Default::default(),
                        }
                    )
                } else {
                    //there is the possibility that the state transition was not executed because it already existed,
                    // we can discount that for now in tests
                    assert!(identity.is_none());
                }
            }
            StateTransitionAction::IdentityTopUpAction(identity_top_up_transition) => {
                proofs_request
                    .identities
                    .push(get_proofs_request_v0::IdentityRequest {
                        identity_id: identity_top_up_transition.identity_id().to_vec(),
                        request_type: get_proofs_request_v0::identity_request::Type::Balance.into(),
                    });
                let result = abci_app
                    .platform
                    .query("/proofs", &proofs_request.encode_to_vec(), platform_version)
                    .expect("expected to query proofs");
                let serialized_get_proofs_response =
                    result.into_data().expect("expected queries to be valid");

                let GetProofsResponse { version } =
                    GetProofsResponse::decode(serialized_get_proofs_response.as_slice())
                        .expect("expected to decode proof response");

                let get_proofs_response::Version::V0(GetProofsResponseV0 { proof, metadata: _ }) =
                    version.expect("expected a versioned response");

                let response_proof = proof.expect("proof should be present");

                // we expect to get an identity that matches the state transition
                let (root_hash, balance) = Drive::verify_identity_balance_for_identity_id(
                    &response_proof.grovedb_proof,
                    identity_top_up_transition.identity_id().into_buffer(),
                    false,
                    platform_version,
                )
                .expect("expected to verify balance identity");
                let balance = balance.expect("expected a balance");
                assert_eq!(
                    &root_hash,
                    expected_root_hash,
                    "state last block info {:?}",
                    platform.state.last_committed_block_info()
                );

                if *was_executed {
                    //while this isn't 100% sure to be true (in the case of debt,
                    // for the tests we have we can use it
                    assert!(identity_top_up_transition.top_up_balance_amount() <= balance);
                }
            }
            StateTransitionAction::IdentityCreditWithdrawalAction(
                identity_credit_withdrawal_transition,
            ) => {
                proofs_request
                    .identities
                    .push(get_proofs_request_v0::IdentityRequest {
                        identity_id: identity_credit_withdrawal_transition.identity_id().to_vec(),
                        request_type: get_proofs_request_v0::identity_request::Type::Balance.into(),
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
                    .query("/proofs", &proofs_request.encode_to_vec(), platform_version)
                    .expect("expected to query proofs");

                let serialized_get_proofs_response =
                    result.into_data().expect("expected queries to be valid");

                let GetProofsResponse { version } =
                    GetProofsResponse::decode(serialized_get_proofs_response.as_slice())
                        .expect("expected to decode proof response");

                let get_proofs_response::Version::V0(GetProofsResponseV0 { proof, metadata: _ }) =
                    version.expect("expected a versioned response");

                let response_proof = proof.expect("proof should be present");

                // we expect to get an identity that matches the state transition
                let (root_hash, balance) = Drive::verify_identity_balance_for_identity_id(
                    &response_proof.grovedb_proof,
                    identity_credit_withdrawal_transition
                        .identity_id()
                        .into_buffer(),
                    false,
                    platform_version,
                )
                .expect("expected to verify balance identity");
                let _balance = balance.expect("expected a balance");
                assert_eq!(
                    &root_hash,
                    expected_root_hash,
                    "state last block info {:?}",
                    platform.state.last_committed_block_info()
                );

                //todo: we need to do more here
            }
            StateTransitionAction::IdentityUpdateAction(identity_update_transition) => {
                proofs_request
                    .identities
                    .push(get_proofs_request_v0::IdentityRequest {
                        identity_id: identity_update_transition.identity_id().to_vec(),
                        request_type: get_proofs_request_v0::identity_request::Type::Keys.into(),
                    });
                let result = abci_app
                    .platform
                    .query("/proofs", &proofs_request.encode_to_vec(), platform_version)
                    .expect("expected to query proofs");
                let serialized_get_proofs_response =
                    result.into_data().expect("expected queries to be valid");

                let GetProofsResponse { version } =
                    GetProofsResponse::decode(serialized_get_proofs_response.as_slice())
                        .expect("expected to decode proof response");

                let get_proofs_response::Version::V0(GetProofsResponseV0 { proof, metadata: _ }) =
                    version.expect("expected a versioned response");

                let response_proof = proof.expect("proof should be present");

                // we expect to get an identity that matches the state transition
                let (root_hash, identity) = Drive::verify_identity_keys_by_identity_id(
                    &response_proof.grovedb_proof,
                    false,
                    identity_update_transition.identity_id().into_buffer(),
                    platform_version,
                )
                .expect("expected to verify identity keys");
                let identity = identity.expect("expected an identity");
                assert_eq!(
                    &root_hash,
                    expected_root_hash,
                    "state last block info {:?}",
                    platform.state.last_committed_block_info()
                );
                // we need to verify that the partial identity has all keys we added
                let has_all_keys = identity_update_transition
                    .public_keys_to_add()
                    .iter()
                    .all(|added| identity.loaded_public_keys.contains_key(&added.id()));
                let has_no_removed_key = !identity_update_transition
                    .public_keys_to_disable()
                    .iter()
                    .any(|removed| identity.loaded_public_keys.contains_key(removed));
                assert!(has_all_keys);
                assert!(has_no_removed_key);
            }
            StateTransitionAction::IdentityCreditTransferAction(
                identity_credit_transfer_action,
            ) => {
                proofs_request
                    .identities
                    .push(get_proofs_request_v0::IdentityRequest {
                        identity_id: identity_credit_transfer_action.identity_id().to_vec(),
                        request_type: get_proofs_request_v0::identity_request::Type::Balance.into(),
                    });

                proofs_request
                    .identities
                    .push(get_proofs_request_v0::IdentityRequest {
                        identity_id: identity_credit_transfer_action.recipient_id().to_vec(),
                        request_type: get_proofs_request_v0::identity_request::Type::Balance.into(),
                    });

                let result = abci_app
                    .platform
                    .query("/proofs", &proofs_request.encode_to_vec(), platform_version)
                    .expect("expected to query proofs");
                let serialized_get_proofs_response =
                    result.into_data().expect("expected queries to be valid");

                let GetProofsResponse { version } =
                    GetProofsResponse::decode(serialized_get_proofs_response.as_slice())
                        .expect("expected to decode proof response");

                let get_proofs_response::Version::V0(GetProofsResponseV0 { proof, metadata: _ }) =
                    version.expect("expected a versioned response");

                let response_proof = proof.expect("proof should be present");

                // we expect to get an identity that matches the state transition
                let (root_hash_identity, _balance_identity) =
                    Drive::verify_identity_balance_for_identity_id(
                        &response_proof.grovedb_proof,
                        identity_credit_transfer_action.identity_id().into_buffer(),
                        true,
                        platform_version,
                    )
                    .expect("expected to verify balance identity");

                assert_eq!(
                    &root_hash_identity,
                    expected_root_hash,
                    "state last block info {:?}",
                    platform.state.last_committed_block_info()
                );

                let (root_hash_recipient, balance_recipient) =
                    Drive::verify_identity_balance_for_identity_id(
                        &response_proof.grovedb_proof,
                        identity_credit_transfer_action.recipient_id().into_buffer(),
                        true,
                        platform_version,
                    )
                    .expect("expected to verify balance recipient");

                assert_eq!(
                    &root_hash_recipient,
                    expected_root_hash,
                    "state last block info {:?}",
                    platform.state.last_committed_block_info()
                );

                if *was_executed {
                    let balance_recipient = balance_recipient.expect("expected a balance");

                    assert!(balance_recipient >= identity_credit_transfer_action.transfer_amount());
                }
            }
        }
    }

    true
}
