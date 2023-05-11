use dapi_grpc::platform::v0::{get_proofs_request, GetProofsRequest, GetProofsResponse};
use dpp::data_contract::state_transition::data_contract_create_transition::DataContractCreateTransitionAction;
use dpp::data_contract::state_transition::data_contract_update_transition::DataContractUpdateTransitionAction;
use dpp::identity::PartialIdentity;
use dpp::state_transition::{StateTransition, StateTransitionAction, StateTransitionLike};
use drive::drive::Drive;
use drive_abci::abci::AbciApplication;
use drive_abci::platform::{Platform, PlatformRef};
use drive_abci::rpc::core::MockCoreRPCLike;
use drive_abci::validation::state_transition::StateTransitionValidation;
use mockall::Any;
use prost::Message;
use tenderdash_abci::proto::abci::RequestQuery;
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
                let response_proof = result.into_data().expect("expected queries to be valid");
                // we expect to get a data contract that matches the state transition
                //todo
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
                let response_proof = result.into_data().expect("expected queries to be valid");
                // we expect to get a data contract that matches the state transition
                //todo
            }
            StateTransitionAction::DocumentsBatchAction(_) => {}
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

                let GetProofsResponse { proof, metadata } =
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
                assert_eq!(&root_hash, expected_root_hash);
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
                let response_proof = result.into_data().expect("expected queries to be valid");
                // we expect to get an identity that matches the state transition
                //todo
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
                let response_proof = result.into_data().expect("expected queries to be valid");
                // we expect to get an identity that matches the state transition
            }
        }
    }

    true
}
