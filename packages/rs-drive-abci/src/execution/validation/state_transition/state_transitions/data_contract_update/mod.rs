mod identity_contract_nonce;
mod state;

use dpp::block::block_info::BlockInfo;
use dpp::state_transition::data_contract_update_transition::DataContractUpdateTransition;
use dpp::validation::ConsensusValidationResult;

use drive::grovedb::TransactionArg;

use crate::error::execution::ExecutionError;
use crate::error::Error;

use crate::execution::types::state_transition_execution_context::StateTransitionExecutionContext;

use drive::state_transition_action::StateTransitionAction;

use crate::execution::validation::state_transition::data_contract_update::state::v0::DataContractUpdateStateTransitionStateValidationV0;
use crate::execution::validation::state_transition::transformer::StateTransitionActionTransformerV0;
use crate::execution::validation::state_transition::ValidationMode;
use crate::platform_types::platform::PlatformRef;
use crate::rpc::core::CoreRPCLike;

impl StateTransitionActionTransformerV0 for DataContractUpdateTransition {
    fn transform_into_action<C: CoreRPCLike>(
        &self,
        platform: &PlatformRef<C>,
        _block_info: &BlockInfo,
        validation_mode: ValidationMode,
        _execution_context: &mut StateTransitionExecutionContext,
        _tx: TransactionArg,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        let platform_version = platform.state.current_platform_version()?;

        match platform_version
            .drive_abci
            .validation_and_processing
            .state_transitions
            .contract_update_state_transition
            .transform_into_action
        {
            0 => self.transform_into_action_v0(validation_mode, platform_version),
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "data contract update transition: transform_into_action".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::config::{ExecutionConfig, PlatformConfig, PlatformTestConfig};
    use crate::platform_types::platform::PlatformRef;
    use crate::rpc::core::MockCoreRPCLike;
    use crate::test::helpers::setup::{TempPlatform, TestPlatformBuilder};
    use dpp::block::block_info::BlockInfo;

    use dpp::data_contract::DataContract;
    use dpp::platform_value::BinaryData;
    use dpp::state_transition::data_contract_update_transition::DataContractUpdateTransitionV0;

    use dpp::tests::fixtures::get_data_contract_fixture;
    use dpp::version::PlatformVersion;

    struct TestData<T> {
        data_contract: DataContract,
        platform: TempPlatform<T>,
    }

    fn apply_contract(
        platform: &TempPlatform<MockCoreRPCLike>,
        data_contract: &DataContract,
        block_info: BlockInfo,
    ) {
        let platform_version = PlatformVersion::latest();
        platform
            .drive
            .apply_contract(
                data_contract,
                block_info,
                true,
                None,
                None,
                platform_version,
            )
            .expect("to apply contract");
    }

    fn setup_test() -> TestData<MockCoreRPCLike> {
        let platform_version = PlatformVersion::latest();
        let data_contract = get_data_contract_fixture(None, 0, platform_version.protocol_version)
            .data_contract_owned();

        let config = PlatformConfig {
            validator_set_quorum_size: 10,
            execution: ExecutionConfig {
                verify_sum_trees: true,
                validator_set_rotation_block_count: 25,
                ..Default::default()
            },
            block_spacing_ms: 300,
            testing_configs: PlatformTestConfig::default_with_no_block_signing(),
            ..Default::default()
        };
        let platform = TestPlatformBuilder::new()
            .with_config(config)
            .build_with_mock_rpc();

        TestData {
            data_contract,
            platform: platform.set_initial_state_structure(),
        }
    }

    mod validate_state {
        use super::*;
        use serde_json::json;

        use dpp::assert_state_consensus_errors;
        use dpp::consensus::state::state_error::StateError;
        use dpp::consensus::state::state_error::StateError::DataContractIsReadonlyError;
        use dpp::errors::consensus::ConsensusError;

        use crate::execution::validation::state_transition::processor::v0::StateTransitionStateValidationV0;
        use dpp::block::block_info::BlockInfo;
        use dpp::data_contract::accessors::v0::{DataContractV0Getters, DataContractV0Setters};

        use dpp::data_contract::config::v0::DataContractConfigSettersV0;
        use dpp::data_contract::schema::DataContractSchemaMethodsV0;

        use dpp::data_contract::serialized_version::DataContractInSerializationFormat;
        use dpp::platform_value::platform_value;
        use dpp::state_transition::data_contract_update_transition::DataContractUpdateTransition;

        use crate::execution::types::state_transition_execution_context::StateTransitionExecutionContext;
        use crate::execution::validation::state_transition::ValidationMode;
        use dpp::version::TryFromPlatformVersioned;
        use platform_version::version::LATEST_PLATFORM_VERSION;
        use platform_version::{DefaultForPlatformVersion, TryIntoPlatformVersioned};

        #[test]
        pub fn should_return_error_if_trying_to_update_document_schema_in_a_readonly_contract() {
            let platform_version = PlatformVersion::latest();
            let TestData {
                mut data_contract,
                platform,
            } = setup_test();

            data_contract.config_mut().set_readonly(true);
            apply_contract(&platform, &data_contract, Default::default());

            let updated_document = platform_value!({
                "type": "object",
                "properties": {
                    "name": {
                        "type": "string",
                        "position": 0
                    },
                    "newProp": {
                        "type": "integer",
                        "minimum": 0,
                        "position": 1
                    }
                },
                "required": [
                "$createdAt"
                ],
                "additionalProperties": false
            });

            data_contract.increment_version();
            data_contract
                .set_document_schema("niceDocument", updated_document, true, platform_version)
                .expect("to be able to set document schema");

            let state_transition = DataContractUpdateTransitionV0 {
                identity_contract_nonce: 1,
                data_contract: DataContractInSerializationFormat::try_from_platform_versioned(
                    data_contract,
                    platform_version,
                )
                .expect("to be able to convert data contract to serialization format"),
                user_fee_increase: 0,
                signature: BinaryData::new(vec![0; 65]),
                signature_public_key_id: 0,
            };

            let state = platform.state.load();

            let platform_ref = PlatformRef {
                drive: &platform.drive,
                state: &state,
                config: &platform.config,
                core_rpc: &platform.core_rpc,
            };

            let mut execution_context =
                StateTransitionExecutionContext::default_for_platform_version(platform_version)
                    .expect("expected a platform version");

            let result = DataContractUpdateTransition::V0(state_transition)
                .validate_state(
                    None,
                    &platform_ref,
                    ValidationMode::Validator,
                    &mut execution_context,
                    None,
                )
                .expect("state transition to be validated");

            assert!(!result.is_valid());
            assert_state_consensus_errors!(result, DataContractIsReadonlyError, 1);
        }

        #[test]
        pub fn should_keep_history_if_contract_config_keeps_history_is_true() {
            let TestData {
                mut data_contract,
                platform,
            } = setup_test();

            let platform_version = PlatformVersion::latest();

            data_contract.config_mut().set_keeps_history(true);
            data_contract.config_mut().set_readonly(false);

            apply_contract(
                &platform,
                &data_contract,
                BlockInfo {
                    time_ms: 1000,
                    height: 100,
                    core_height: 10,
                    epoch: Default::default(),
                },
            );

            let updated_document = platform_value!({
                "type": "object",
                "properties": {
                    "name": {
                        "type": "string",
                        "position": 0
                    },
                    "newProp": {
                        "type": "integer",
                        "minimum": 0,
                        "position": 1
                    }
                },
                "required": [
                "$createdAt"
                ],
                "additionalProperties": false
            });

            data_contract.increment_version();
            data_contract
                .set_document_schema("niceDocument", updated_document, true, platform_version)
                .expect("to be able to set document schema");

            // TODO: add a data contract stop transition
            let state_transition = DataContractUpdateTransitionV0 {
                identity_contract_nonce: 1,
                data_contract: DataContractInSerializationFormat::try_from_platform_versioned(
                    data_contract.clone(),
                    platform_version,
                )
                .expect("to be able to convert data contract to serialization format"),
                user_fee_increase: 0,
                signature: BinaryData::new(vec![0; 65]),
                signature_public_key_id: 0,
            };

            let state = platform.state.load();

            let platform_ref = PlatformRef {
                drive: &platform.drive,
                state: &state,
                config: &platform.config,
                core_rpc: &platform.core_rpc,
            };

            let mut execution_context =
                StateTransitionExecutionContext::default_for_platform_version(platform_version)
                    .expect("expected a platform version");

            let result = DataContractUpdateTransition::V0(state_transition)
                .validate_state(
                    None,
                    &platform_ref,
                    ValidationMode::Validator,
                    &mut execution_context,
                    None,
                )
                .expect("state transition to be validated");

            assert!(result.is_valid());

            // This should store update and history
            apply_contract(
                &platform,
                &data_contract,
                BlockInfo {
                    time_ms: 2000,
                    height: 110,
                    core_height: 11,
                    epoch: Default::default(),
                },
            );

            // Fetch from time 0 without a limit or offset
            let contract_history = platform
                .drive
                .fetch_contract_with_history(
                    *data_contract.id().as_bytes(),
                    None,
                    0,
                    None,
                    None,
                    platform_version,
                )
                .expect("to get contract history");

            let keys = contract_history.keys().copied().collect::<Vec<u64>>();

            // Check that keys sorted from oldest to newest
            assert_eq!(contract_history.len(), 2);
            assert_eq!(keys[0], 1000);
            assert_eq!(keys[1], 2000);

            // Fetch with an offset should offset from the newest to oldest
            let contract_history = platform
                .drive
                .fetch_contract_with_history(
                    *data_contract.id().as_bytes(),
                    None,
                    0,
                    None,
                    Some(1),
                    platform_version,
                )
                .expect("to get contract history");

            let keys = contract_history.keys().copied().collect::<Vec<u64>>();

            assert_eq!(contract_history.len(), 1);
            assert_eq!(keys[0], 1000);

            // Check that when we limit ny 1 we get only the most recent contract
            let contract_history = platform
                .drive
                .fetch_contract_with_history(
                    *data_contract.id().as_bytes(),
                    None,
                    0,
                    Some(1),
                    None,
                    platform_version,
                )
                .expect("to get contract history");

            let keys = contract_history.keys().copied().collect::<Vec<u64>>();

            // Check that when we limit ny 1 we get only the most recent contract
            assert_eq!(contract_history.len(), 1);
            assert_eq!(keys[0], 2000);
        }

        #[test]
        fn should_fail_if_trying_to_update_config() {
            let TestData {
                mut data_contract,
                platform,
            } = setup_test();

            let platform_version = PlatformVersion::latest();

            data_contract.config_mut().set_keeps_history(true);
            data_contract.config_mut().set_readonly(false);

            apply_contract(
                &platform,
                &data_contract,
                BlockInfo {
                    time_ms: 1000,
                    height: 100,
                    core_height: 10,
                    epoch: Default::default(),
                },
            );

            let updated_document_type = json!({
                "type": "object",
                "properties": {
                    "name": {
                        "type": "string",
                        "position": 0,
                    },
                    "newProp": {
                        "type": "integer",
                        "minimum": 0,
                        "position": 1,
                    }
                },
                "required": [
                "$createdAt"
                ],
                "additionalProperties": false
            });

            data_contract.increment_version();
            data_contract
                .set_document_schema(
                    "niceDocument",
                    updated_document_type.into(),
                    true,
                    LATEST_PLATFORM_VERSION,
                )
                .expect("to be able to set document schema");

            // It should be not possible to modify this
            data_contract.config_mut().set_keeps_history(false);

            let state_transition: DataContractUpdateTransitionV0 = (data_contract, 1)
                .try_into_platform_versioned(LATEST_PLATFORM_VERSION)
                .expect("expected an update transition");

            let state_transition: DataContractUpdateTransition = state_transition.into();

            let state = platform.state.load();

            let platform_ref = PlatformRef {
                drive: &platform.drive,
                state: &state,
                config: &platform.config,
                core_rpc: &platform.core_rpc,
            };

            let mut execution_context =
                StateTransitionExecutionContext::default_for_platform_version(platform_version)
                    .expect("expected a platform version");

            let result = state_transition
                .validate_state(
                    None,
                    &platform_ref,
                    ValidationMode::Validator,
                    &mut execution_context,
                    None,
                )
                .expect("state transition to be validated");

            assert!(!result.is_valid());
            let errors = assert_state_consensus_errors!(
                result,
                StateError::DataContractConfigUpdateError,
                1
            );
            let error = errors.first().expect("to have an error");
            assert_eq!(
                error.additional_message(),
                "contract can not change whether it keeps history: changing from true to false"
            );
        }
    }
}
