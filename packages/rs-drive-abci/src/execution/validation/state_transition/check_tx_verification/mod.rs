pub(crate) mod v0;

use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::execution::types::execution_event::ExecutionEvent;
use crate::platform_types::platform::PlatformRef;
use crate::rpc::core::CoreRPCLike;
use dpp::prelude::ConsensusValidationResult;
use dpp::state_transition::StateTransition;
use dpp::version::PlatformVersion;

use crate::execution::check_tx::CheckTxLevel;

/// === CHECK TX: NEW ====
/// Full validation for identity create and identity top up
/// Otherwise only validate:
/// * identity has enough balance for fee
/// * identity signature on tx is valid
/// * ST structure is valid
///
/// === CHECK TX: RECHECK ===
/// For identity create and identity top up, make sure asset lock has not been used up
/// For other state transitions verify that the user still has enough balance
///
pub(in crate::execution) fn state_transition_to_execution_event_for_check_tx<'a, C: CoreRPCLike>(
    platform: &'a PlatformRef<C>,
    state_transition: StateTransition,
    check_tx_level: CheckTxLevel,
    platform_version: &PlatformVersion,
) -> Result<ConsensusValidationResult<Option<ExecutionEvent<'a>>>, Error> {
    match platform_version
        .drive_abci
        .validation_and_processing
        .state_transition_to_execution_event_for_check_tx
    {
        0 => v0::state_transition_to_execution_event_for_check_tx_v0(
            platform,
            state_transition,
            check_tx_level,
            platform_version,
        ),
        version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
            method: "state_transition_to_execution_event_for_check_tx".to_string(),
            known_versions: vec![0],
            received: version,
        })),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::execution::check_tx::CheckTxLevel;
    use crate::execution::validation::state_transition::state_transitions::tests::setup_identity;
    use crate::platform_types::platform::PlatformRef;
    use crate::platform_types::platform_state::PlatformStateV0Methods;
    use crate::test::helpers::setup::TestPlatformBuilder;
    use dpp::dash_to_credits;
    use dpp::data_contract::accessors::v0::DataContractV0Setters;
    use dpp::data_contract::config::DataContractConfig;
    use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
    use dpp::state_transition::data_contract_create_transition::methods::DataContractCreateTransitionMethodsV0;
    use dpp::state_transition::data_contract_create_transition::DataContractCreateTransition;
    use dpp::state_transition::StateTransition;
    use dpp::tests::json_document::json_document_to_contract_with_ids;
    use dpp::version::{DefaultForPlatformVersion, PlatformVersion};

    mod version_dispatch {
        use super::*;

        #[test]
        fn should_dispatch_to_v0_for_known_version() {
            let platform_version = PlatformVersion::latest();
            let mut platform = TestPlatformBuilder::new()
                .build_with_mock_rpc()
                .set_genesis_state();

            let (identity, signer, key) = setup_identity(&mut platform, 958, dash_to_credits!(2.0));

            let mut data_contract = json_document_to_contract_with_ids(
                "tests/supporting_files/contract/dpns/dpns-contract-contested-unique-index.json",
                None,
                None,
                false,
                platform_version,
            )
            .expect("expected to get contract");

            // Upgrade config to V1 (required since protocol version 12)
            data_contract
                .set_config(DataContractConfig::default_for_version(platform_version).unwrap());

            let data_contract_create_transition =
                DataContractCreateTransition::new_from_data_contract(
                    data_contract,
                    1,
                    &identity.into_partial_identity_info(),
                    key.id(),
                    &signer,
                    platform_version,
                    None,
                )
                .expect("expected to create transition");

            let state_transition: StateTransition = data_contract_create_transition.into();

            let platform_state = platform.state.load();
            let platform_ref = PlatformRef {
                drive: &platform.drive,
                state: &platform_state,
                config: &platform.config,
                core_rpc: &platform.core_rpc,
            };

            // version 0 is the only known version -- it should succeed (valid result)
            let result = state_transition_to_execution_event_for_check_tx(
                &platform_ref,
                state_transition,
                CheckTxLevel::FirstTimeCheck,
                platform_version,
            );
            assert!(result.is_ok(), "v0 dispatch should not return an error");
            let validation_result = result.unwrap();
            assert!(
                validation_result.is_valid(),
                "validation should pass: {:?}",
                validation_result.errors
            );
        }

        #[test]
        fn should_error_for_unknown_version() {
            let platform_version = PlatformVersion::latest();
            let mut platform = TestPlatformBuilder::new()
                .build_with_mock_rpc()
                .set_genesis_state();

            // Create a modified platform version with unknown check_tx version
            let mut modified_version = platform_version.clone();
            modified_version
                .drive_abci
                .validation_and_processing
                .state_transition_to_execution_event_for_check_tx = 99;

            // Set up identity and build transition before borrowing platform state
            let (identity, signer, key) = setup_identity(&mut platform, 958, dash_to_credits!(2.0));

            let mut data_contract = json_document_to_contract_with_ids(
                "tests/supporting_files/contract/dpns/dpns-contract-contested-unique-index.json",
                None,
                None,
                false,
                platform_version,
            )
            .expect("expected to get contract");

            // Upgrade config to V1 (required since protocol version 12)
            data_contract
                .set_config(DataContractConfig::default_for_version(platform_version).unwrap());

            let data_contract_create_transition =
                DataContractCreateTransition::new_from_data_contract(
                    data_contract,
                    1,
                    &identity.into_partial_identity_info(),
                    key.id(),
                    &signer,
                    platform_version,
                    None,
                )
                .expect("expected to create transition");

            let state_transition: StateTransition = data_contract_create_transition.into();

            // Now borrow platform state after mutable borrows are done
            let platform_state = platform.state.load();
            let platform_ref = PlatformRef {
                drive: &platform.drive,
                state: &platform_state,
                config: &platform.config,
                core_rpc: &platform.core_rpc,
            };

            let result = state_transition_to_execution_event_for_check_tx(
                &platform_ref,
                state_transition,
                CheckTxLevel::FirstTimeCheck,
                &modified_version,
            );

            match result {
                Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                    method,
                    known_versions,
                    received,
                })) => {
                    assert_eq!(method, "state_transition_to_execution_event_for_check_tx");
                    assert_eq!(known_versions, vec![0]);
                    assert_eq!(received, 99);
                }
                Err(e) => panic!("expected UnknownVersionMismatch, got: {:?}", e),
                Ok(_) => panic!("expected an error for unknown version"),
            }
        }
    }
}
