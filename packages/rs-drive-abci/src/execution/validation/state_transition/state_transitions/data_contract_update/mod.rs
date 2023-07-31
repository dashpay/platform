mod identity_and_signatures;
mod state;
mod structure;

use dpp::identity::PartialIdentity;

use dpp::state_transition::data_contract_update_transition::DataContractUpdateTransition;
use dpp::validation::{ConsensusValidationResult, SimpleConsensusValidationResult};
use drive::drive::Drive;
use drive::grovedb::TransactionArg;

use crate::error::execution::ExecutionError;
use crate::error::Error;

use dpp::state_transition_action::StateTransitionAction;
use dpp::version::PlatformVersion;

use crate::platform_types::platform::PlatformRef;
use crate::rpc::core::CoreRPCLike;
use crate::execution::validation::state_transition::data_contract_update::identity_and_signatures::v0::StateTransitionIdentityAndSignaturesValidationV0;
use crate::execution::validation::state_transition::data_contract_update::state::v0::StateTransitionStateValidationV0;
use crate::execution::validation::state_transition::data_contract_update::structure::v0::StateTransitionStructureValidationV0;
use crate::execution::validation::state_transition::processor::v0::StateTransitionValidationV0;
use crate::execution::validation::state_transition::transformer::StateTransitionActionTransformerV0;
use crate::platform_types::platform_state::v0::PlatformStateV0Methods;

impl StateTransitionActionTransformerV0 for DataContractUpdateTransition {
    fn transform_into_action<C: CoreRPCLike>(
        &self,
        platform: &PlatformRef<C>,
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
            0 => self.transform_into_action_v0(),
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "data contract update transition: transform_into_action".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}

impl StateTransitionValidationV0 for DataContractUpdateTransition {
    fn validate_structure(
        &self,
        _drive: &Drive,
        protocol_version: u32,
        _tx: TransactionArg,
    ) -> Result<SimpleConsensusValidationResult, Error> {
        let platform_version = PlatformVersion::get(protocol_version)?;
        match platform_version
            .drive_abci
            .validation_and_processing
            .state_transitions
            .contract_update_state_transition
            .structure
        {
            0 => self.validate_structure_v0(),
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "data contract update transition: validate_structure".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }

    fn validate_identity_and_signatures(
        &self,
        drive: &Drive,
        protocol_version: u32,
        transaction: TransactionArg,
    ) -> Result<ConsensusValidationResult<Option<PartialIdentity>>, Error> {
        let platform_version = PlatformVersion::get(protocol_version)?;
        match platform_version
            .drive_abci
            .validation_and_processing
            .state_transitions
            .contract_update_state_transition
            .identity_signatures
        {
            0 => self.validate_identity_and_signatures_v0(drive, transaction),
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "data contract update transition: validate_identity_and_signatures"
                    .to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }

    fn validate_state<C: CoreRPCLike>(
        &self,
        platform: &PlatformRef<C>,
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
            0 => self.validate_state_v0(platform, tx),
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
    use crate::config::{PlatformConfig, PlatformTestConfig};
    use crate::platform_types::platform::PlatformRef;
    use crate::rpc::core::MockCoreRPCLike;
    use crate::test::helpers::setup::{TempPlatform, TestPlatformBuilder};
    use dpp::block::block_info::BlockInfo;
    use dpp::data_contract::DataContract;
    use dpp::platform_value::{BinaryData, Value};
    use dpp::state_transition::{StateTransitionFieldTypes, StateTransitionType};
    use dpp::tests::fixtures::get_data_contract_fixture;
    use dpp::version::{PlatformVersion, LATEST_VERSION};

    struct TestData<T> {
        raw_state_transition: Value,
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
        let data_contract =
            get_data_contract_fixture(None, platform_version.protocol_version).data_contract;
        let mut updated_data_contract = data_contract.clone();

        updated_data_contract.increment_version();

        let state_transition = DataContractUpdateTransition {
            protocol_version: LATEST_VERSION,
            data_contract: updated_data_contract,
            signature: BinaryData::new(vec![0; 65]),
            signature_public_key_id: 0,
            transition_type: StateTransitionType::DataContractUpdate,
        };

        let raw_state_transition = state_transition.to_object(false).unwrap();

        let dc = data_contract;

        let config = PlatformConfig {
            verify_sum_trees: true,
            quorum_size: 10,
            validator_set_quorum_rotation_block_count: 25,
            block_spacing_ms: 300,
            testing_configs: PlatformTestConfig::default_with_no_block_signing(),
            ..Default::default()
        };
        let platform = TestPlatformBuilder::new()
            .with_config(config)
            .build_with_mock_rpc();

        TestData {
            raw_state_transition,
            data_contract: dc,
            platform: platform.set_initial_state_structure(),
        }
    }

    mod validate_state {
        use super::super::StateTransitionValidationV0;
        use super::*;

        use dpp::assert_state_consensus_errors;
        use dpp::consensus::state::state_error::StateError::DataContractIsReadonlyError;
        use dpp::errors::consensus::ConsensusError;

        use dpp::block::block_info::BlockInfo;
        use dpp::data_contract::base::DataContractBaseMethodsV0;
        use dpp::data_contract::document_schema::DataContractDocumentSchemaMethodsV0;
        use serde_json::json;

        #[test]
        pub fn should_return_error_if_trying_to_update_document_schema_in_a_readonly_contract() {
            let TestData {
                raw_state_transition: _,
                mut data_contract,
                platform,
            } = setup_test();

            data_contract.config.readonly = true;
            apply_contract(&platform, &data_contract, Default::default());

            let updated_document = json!({
                "type": "object",
                "properties": {
                    "name": {
                        "type": "string"
                    },
                    "newProp": {
                        "type": "integer",
                        "minimum": 0
                    }
                },
                "required": [
                "$createdAt"
                ],
                "additionalProperties": false
            });

            data_contract.increment_version();
            data_contract
                .set_document_schema("niceDocument".into(), updated_document)
                .expect("to be able to set document schema");

            let state_transition = DataContractUpdateTransition {
                protocol_version: LATEST_VERSION,
                data_contract,
                signature: BinaryData::new(vec![0; 65]),
                signature_public_key_id: 0,
                transition_type: StateTransitionType::DataContractUpdate,
            };

            let platform_ref = PlatformRef {
                drive: &platform.drive,
                state: &platform.state.read().unwrap(),
                config: &platform.config,
                core_rpc: &platform.core_rpc,
            };

            let result = state_transition
                .validate_state(&platform_ref, None)
                .expect("state transition to be validated");

            assert!(!result.is_valid());
            assert_state_consensus_errors!(result, DataContractIsReadonlyError, 1);
        }

        #[test]
        pub fn should_keep_history_if_contract_config_keeps_history_is_true() {
            let TestData {
                raw_state_transition: _,
                mut data_contract,
                platform,
            } = setup_test();

            let platform_version = PlatformVersion::latest();

            data_contract.config.keeps_history = true;
            data_contract.config.readonly = false;

            // TODO: check that keep_history actually works
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

            let updated_document = json!({
                "type": "object",
                "properties": {
                    "name": {
                        "type": "string"
                    },
                    "newProp": {
                        "type": "integer",
                        "minimum": 0
                    }
                },
                "required": [
                "$createdAt"
                ],
                "additionalProperties": false
            });

            data_contract.increment_version();
            data_contract
                .set_document_schema("niceDocument".into(), updated_document)
                .expect("to be able to set document schema");

            // TODO: add a data contract stop transition
            let state_transition = DataContractUpdateTransition {
                protocol_version: LATEST_VERSION,
                data_contract: data_contract.clone(),
                signature: BinaryData::new(vec![0; 65]),
                signature_public_key_id: 0,
                transition_type: StateTransitionType::DataContractUpdate,
            };

            let platform_ref = PlatformRef {
                drive: &platform.drive,
                state: &platform.state.read().unwrap(),
                config: &platform.config,
                core_rpc: &platform.core_rpc,
            };

            let result = state_transition
                .validate_state(&platform_ref, None)
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
                    *data_contract.id.as_bytes(),
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
                    *data_contract.id.as_bytes(),
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
                    *data_contract.id.as_bytes(),
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
    }
}
