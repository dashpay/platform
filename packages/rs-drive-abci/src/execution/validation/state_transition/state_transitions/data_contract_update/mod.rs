mod state;
mod structure;

use dpp::state_transition::data_contract_update_transition::DataContractUpdateTransition;
use dpp::validation::{ConsensusValidationResult, SimpleConsensusValidationResult};

use drive::grovedb::TransactionArg;

use crate::error::execution::ExecutionError;
use crate::error::Error;

use crate::execution::types::state_transition_execution_context::StateTransitionExecutionContext;
use dpp::version::PlatformVersion;
use drive::state_transition_action::StateTransitionAction;

use crate::execution::validation::state_transition::data_contract_update::state::v0::DataContractUpdateStateTransitionStateValidationV0;
use crate::execution::validation::state_transition::data_contract_update::structure::v0::DataContractUpdateStateTransitionStructureValidationV0;
use crate::execution::validation::state_transition::processor::v0::{
    StateTransitionStateValidationV0, StateTransitionStructureValidationV0,
};
use crate::execution::validation::state_transition::transformer::StateTransitionActionTransformerV0;
use crate::platform_types::platform::{PlatformRef, PlatformStateRef};
use crate::platform_types::platform_state::v0::PlatformStateV0Methods;
use crate::rpc::core::CoreRPCLike;

impl StateTransitionActionTransformerV0 for DataContractUpdateTransition {
    fn transform_into_action<C: CoreRPCLike>(
        &self,
        platform: &PlatformRef<C>,
        _validate: bool,
        _execution_context: &mut StateTransitionExecutionContext,
        _tx: TransactionArg,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        let platform_version =
            PlatformVersion::get(platform.state.current_protocol_version_in_consensus())?;
        match platform_version
            .drive_abci
            .validation_and_processing
            .state_transitions
            .contract_update_state_transition
            .transform_into_action
        {
            0 => self.transform_into_action_v0(platform_version),
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "data contract update transition: transform_into_action".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}

impl StateTransitionStructureValidationV0 for DataContractUpdateTransition {
    fn validate_structure(
        &self,
        _platform: &PlatformStateRef,
        _action: Option<&StateTransitionAction>,
        protocol_version: u32,
    ) -> Result<SimpleConsensusValidationResult, Error> {
        let platform_version = PlatformVersion::get(protocol_version)?;
        match platform_version
            .drive_abci
            .validation_and_processing
            .state_transitions
            .contract_update_state_transition
            .structure
        {
            0 => self.validate_base_structure_v0(platform_version),
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "data contract update transition: validate_structure".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}

impl StateTransitionStateValidationV0 for DataContractUpdateTransition {
    fn validate_state<C: CoreRPCLike>(
        &self,
        _action: Option<StateTransitionAction>,
        platform: &PlatformRef<C>,
        _execution_context: &mut StateTransitionExecutionContext,
        tx: TransactionArg,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        let platform_version =
            PlatformVersion::get(platform.state.current_protocol_version_in_consensus())?;
        match platform_version
            .drive_abci
            .validation_and_processing
            .state_transitions
            .contract_update_state_transition
            .state
        {
            0 => self.validate_state_v0(platform, tx, platform_version),
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "data contract update transition: validate_state".to_string(),
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
        let data_contract = get_data_contract_fixture(None, platform_version.protocol_version)
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
                data_contract: DataContractInSerializationFormat::try_from_platform_versioned(
                    data_contract,
                    platform_version,
                )
                .expect("to be able to convert data contract to serialization format"),
                signature: BinaryData::new(vec![0; 65]),
                signature_public_key_id: 0,
            };

            let platform_ref = PlatformRef {
                drive: &platform.drive,
                state: &platform.state.read().unwrap(),
                config: &platform.config,
                core_rpc: &platform.core_rpc,
                block_info: &BlockInfo::default(),
            };

            let mut execution_context =
                StateTransitionExecutionContext::default_for_platform_version(platform_version)
                    .expect("expected a platform version");

            let result = DataContractUpdateTransition::V0(state_transition)
                .validate_state(None, &platform_ref, &mut execution_context, None)
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
                data_contract: DataContractInSerializationFormat::try_from_platform_versioned(
                    data_contract.clone(),
                    platform_version,
                )
                .expect("to be able to convert data contract to serialization format"),
                signature: BinaryData::new(vec![0; 65]),
                signature_public_key_id: 0,
            };

            let platform_ref = PlatformRef {
                drive: &platform.drive,
                state: &platform.state.read().unwrap(),
                config: &platform.config,
                core_rpc: &platform.core_rpc,
                block_info: &BlockInfo::default(),
            };

            let mut execution_context =
                StateTransitionExecutionContext::default_for_platform_version(platform_version)
                    .expect("expected a platform version");

            let result = DataContractUpdateTransition::V0(state_transition)
                .validate_state(None, &platform_ref, &mut execution_context, None)
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

            let state_transition: DataContractUpdateTransitionV0 = data_contract
                .try_into_platform_versioned(LATEST_PLATFORM_VERSION)
                .expect("expected an update transition");

            let state_transition: DataContractUpdateTransition = state_transition.into();

            let platform_ref = PlatformRef {
                drive: &platform.drive,
                state: &platform.state.read().unwrap(),
                config: &platform.config,
                core_rpc: &platform.core_rpc,
                block_info: &BlockInfo::default(),
            };

            let mut execution_context =
                StateTransitionExecutionContext::default_for_platform_version(platform_version)
                    .expect("expected a platform version");

            let result = state_transition
                .validate_state(None, &platform_ref, &mut execution_context, None)
                .expect("state transition to be validated");

            assert!(!result.is_valid());
            let errors = assert_state_consensus_errors!(
                result,
                StateError::DataContractConfigUpdateError,
                1
            );
            let error = errors.get(0).expect("to have an error");
            assert_eq!(
                error.additional_message(),
                "contract can not change whether it keeps history: changing from true to false"
            );
        }
    }
}
