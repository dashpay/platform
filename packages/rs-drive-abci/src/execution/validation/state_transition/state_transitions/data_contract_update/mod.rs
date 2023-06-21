mod identity_and_signatures;
mod state;
mod structure;

use dpp::identity::PartialIdentity;

use dpp::data_contract::state_transition::data_contract_update_transition::DataContractUpdateTransition;
use dpp::validation::{ConsensusValidationResult, SimpleConsensusValidationResult};
use drive::drive::Drive;
use drive::grovedb::TransactionArg;

use crate::error::Error;
use dpp::state_transition::StateTransitionAction;

use crate::platform_types::platform::PlatformRef;
use crate::rpc::core::CoreRPCLike;
use crate::execution::validation::state_transition::data_contract_update::identity_and_signatures::v0::StateTransitionIdentityAndSignaturesValidationV0;
use crate::execution::validation::state_transition::data_contract_update::state::v0::StateTransitionStateValidationV0;
use crate::execution::validation::state_transition::data_contract_update::structure::v0::StateTransitionStructureValidationV0;
use crate::execution::validation::state_transition::processor::v0::StateTransitionValidationV0;
use crate::execution::validation::state_transition::transformer::StateTransitionActionTransformerV0;

impl StateTransitionActionTransformerV0 for DataContractUpdateTransition {
    fn transform_into_action<C: CoreRPCLike>(
        &self,
        _platform: &PlatformRef<C>,
        _tx: TransactionArg,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        //todo: use protocol version to determine validation
        self.transform_into_action_v0()
    }
}

impl StateTransitionValidationV0 for DataContractUpdateTransition {
    fn validate_structure(
        &self,
        _drive: &Drive,
        _protocol_version: u32,
        _tx: TransactionArg,
    ) -> Result<SimpleConsensusValidationResult, Error> {
        //todo: use protocol version to determine validation
        self.validate_structure_v0()
    }

    fn validate_identity_and_signatures(
        &self,
        drive: &Drive,
        _protocol_version: u32,
        transaction: TransactionArg,
    ) -> Result<ConsensusValidationResult<Option<PartialIdentity>>, Error> {
        //todo: use protocol version to determine validation
        self.validate_identity_and_signatures_v0(drive, transaction)
    }

    fn validate_state<C: CoreRPCLike>(
        &self,
        platform: &PlatformRef<C>,
        tx: TransactionArg,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        //todo: use protocol version to determine validation
        self.validate_state_v0(platform, tx)
    }
}

#[cfg(test)]
mod tests {
    use crate::config::{PlatformConfig, PlatformTestConfig};
    use crate::platform_types::platform::PlatformRef;
    use crate::rpc::core::MockCoreRPCLike;
    use crate::test::helpers::setup::{TempPlatform, TestPlatformBuilder};
    use dpp::block::block_info::BlockInfo;
    use dpp::data_contract::state_transition::data_contract_update_transition::DataContractUpdateTransition;
    use dpp::data_contract::DataContract;
    use dpp::platform_value::{BinaryData, Value};
    use dpp::state_transition::{StateTransitionConvert, StateTransitionType};
    use dpp::tests::fixtures::get_data_contract_fixture;
    use dpp::version::LATEST_VERSION;

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
        platform
            .drive
            .apply_contract(data_contract, block_info, true, None, None)
            .expect("to apply contract");
    }

    fn setup_test() -> TestData<MockCoreRPCLike> {
        let data_contract = get_data_contract_fixture(None).data_contract;
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
        use dpp::consensus::state::state_error::StateError;
        use dpp::consensus::state::state_error::StateError::DataContractIsReadonlyError;
        use dpp::errors::consensus::ConsensusError;

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

            data_contract.config.keeps_history = true;
            data_contract.config.readonly = false;

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
                .fetch_contract_with_history(*data_contract.id.as_bytes(), None, 0, None, None)
                .expect("to get contract history");

            let keys = contract_history.keys().copied().collect::<Vec<u64>>();

            // Check that keys sorted from oldest to newest
            assert_eq!(contract_history.len(), 2);
            assert_eq!(keys[0], 1000);
            assert_eq!(keys[1], 2000);

            // Fetch with an offset should offset from the newest to oldest
            let contract_history = platform
                .drive
                .fetch_contract_with_history(*data_contract.id.as_bytes(), None, 0, None, Some(1))
                .expect("to get contract history");

            let keys = contract_history.keys().copied().collect::<Vec<u64>>();

            assert_eq!(contract_history.len(), 1);
            assert_eq!(keys[0], 1000);

            // Check that when we limit ny 1 we get only the most recent contract
            let contract_history = platform
                .drive
                .fetch_contract_with_history(*data_contract.id.as_bytes(), None, 0, Some(1), None)
                .expect("to get contract history");

            let keys = contract_history.keys().copied().collect::<Vec<u64>>();

            // Check that when we limit ny 1 we get only the most recent contract
            assert_eq!(contract_history.len(), 1);
            assert_eq!(keys[0], 2000);
        }

        #[test]
        fn should_fail_if_trying_to_update_config() {
            let TestData {
                raw_state_transition: _,
                mut data_contract,
                platform,
            } = setup_test();

            data_contract.config.keeps_history = true;
            data_contract.config.readonly = false;

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

            // It should be not possible to modify this
            data_contract.config.keeps_history = false;

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
