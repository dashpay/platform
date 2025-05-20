mod basic_structure;
mod identity_contract_nonce;
mod state;

use basic_structure::v0::DataContractUpdateStateTransitionBasicStructureValidationV0;
use dpp::block::block_info::BlockInfo;
use dpp::state_transition::data_contract_update_transition::DataContractUpdateTransition;
use dpp::validation::{ConsensusValidationResult, SimpleConsensusValidationResult};

use dpp::version::PlatformVersion;
use drive::grovedb::TransactionArg;

use crate::error::execution::ExecutionError;
use crate::error::Error;

use crate::execution::types::state_transition_execution_context::StateTransitionExecutionContext;
use crate::execution::validation::state_transition::processor::v0::StateTransitionBasicStructureValidationV0;

use drive::state_transition_action::StateTransitionAction;

use crate::execution::validation::state_transition::data_contract_update::state::v0::DataContractUpdateStateTransitionStateValidationV0;
use crate::execution::validation::state_transition::transformer::StateTransitionActionTransformerV0;
use crate::execution::validation::state_transition::ValidationMode;
use crate::platform_types::platform::PlatformRef;
use crate::platform_types::platform_state::v0::PlatformStateV0Methods;
use crate::rpc::core::CoreRPCLike;

impl StateTransitionBasicStructureValidationV0 for DataContractUpdateTransition {
    fn validate_basic_structure(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error> {
        match platform_version
            .drive_abci
            .validation_and_processing
            .state_transitions
            .contract_update_state_transition
            .basic_structure
        {
            Some(0) => self.validate_basic_structure_v0(platform_version),
            Some(version) => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "data contract update transition: validate_basic_structure".to_string(),
                known_versions: vec![0],
                received: version,
            })),
            None => Err(Error::Execution(ExecutionError::VersionNotActive {
                method: "data contract update transition: validate_basic_structure".to_string(),
                known_versions: vec![0],
            })),
        }
    }
}

impl StateTransitionActionTransformerV0 for DataContractUpdateTransition {
    fn transform_into_action<C: CoreRPCLike>(
        &self,
        platform: &PlatformRef<C>,
        block_info: &BlockInfo,
        validation_mode: ValidationMode,
        execution_context: &mut StateTransitionExecutionContext,
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
            0 => self.transform_into_action_v0(
                block_info,
                validation_mode,
                execution_context,
                platform_version,
            ),
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
    use crate::config::{ExecutionConfig, PlatformConfig, PlatformTestConfig, ValidatorSetConfig};
    use crate::platform_types::platform::PlatformRef;
    use crate::rpc::core::MockCoreRPCLike;
    use crate::test::helpers::setup::{TempPlatform, TestPlatformBuilder};
    use dpp::block::block_info::BlockInfo;
    use dpp::consensus::state::state_error::StateError;
    use dpp::consensus::ConsensusError;
    use dpp::dash_to_credits;
    use dpp::data_contract::accessors::v0::{DataContractV0Getters, DataContractV0Setters};
    use rand::prelude::StdRng;
    use rand::SeedableRng;
    use std::collections::BTreeMap;

    use dpp::data_contract::DataContract;
    use dpp::fee::Credits;
    use dpp::identifier::Identifier;
    use dpp::identity::accessors::IdentityGettersV0;
    use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
    use dpp::identity::{Identity, IdentityPublicKey, IdentityV0};
    use dpp::platform_value::BinaryData;
    use dpp::serialization::PlatformSerializable;
    use dpp::state_transition::data_contract_update_transition::methods::DataContractUpdateTransitionMethodsV0;
    use dpp::state_transition::data_contract_update_transition::{
        DataContractUpdateTransition, DataContractUpdateTransitionV0,
    };

    use crate::platform_types::platform_state::v0::PlatformStateV0Methods;
    use crate::platform_types::state_transitions_processing_result::StateTransitionExecutionResult;
    use assert_matches::assert_matches;
    use dpp::consensus::basic::BasicError;
    use dpp::data_contract::accessors::v1::DataContractV1Getters;
    use dpp::data_contract::group::v0::GroupV0;
    use dpp::data_contract::group::Group;
    use dpp::tests::fixtures::get_data_contract_fixture;
    use dpp::tests::json_document::json_document_to_contract;
    use dpp::version::PlatformVersion;
    use drive::util::storage_flags::StorageFlags;
    use simple_signer::signer::SimpleSigner;

    struct TestData<T> {
        data_contract: DataContract,
        platform: TempPlatform<T>,
    }

    fn setup_identity(
        platform: &mut TempPlatform<MockCoreRPCLike>,
        seed: u64,
        credits: Credits,
    ) -> (Identity, SimpleSigner, IdentityPublicKey) {
        let platform_version = PlatformVersion::latest();
        let mut signer = SimpleSigner::default();

        let mut rng = StdRng::seed_from_u64(seed);

        let (master_key, master_private_key) =
            IdentityPublicKey::random_ecdsa_master_authentication_key_with_rng(
                0,
                &mut rng,
                platform_version,
            )
            .expect("expected to get key pair");

        signer.add_key(master_key.clone(), master_private_key);

        let (critical_public_key, private_key) =
            IdentityPublicKey::random_ecdsa_critical_level_authentication_key_with_rng(
                1,
                &mut rng,
                platform_version,
            )
            .expect("expected to get key pair");

        signer.add_key(critical_public_key.clone(), private_key);

        let identity: Identity = IdentityV0 {
            id: Identifier::random_with_rng(&mut rng),
            public_keys: BTreeMap::from([
                (0, master_key.clone()),
                (1, critical_public_key.clone()),
            ]),
            balance: credits,
            revision: 0,
        }
        .into();

        // We just add this identity to the system first

        platform
            .drive
            .add_new_identity(
                identity.clone(),
                false,
                &BlockInfo::default(),
                true,
                None,
                platform_version,
            )
            .expect("expected to add a new identity");

        (identity, signer, critical_public_key)
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
            validator_set: ValidatorSetConfig {
                quorum_size: 10,
                ..Default::default()
            },
            execution: ExecutionConfig {
                verify_sum_trees: true,

                ..Default::default()
            },
            block_spacing_ms: 300,
            testing_configs: PlatformTestConfig::default_minimal_verifications(),
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
                .set_document_schema(
                    "niceDocument",
                    updated_document,
                    true,
                    &mut vec![],
                    platform_version,
                )
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
                    &BlockInfo::default(),
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
                .set_document_schema(
                    "niceDocument",
                    updated_document,
                    true,
                    &mut vec![],
                    platform_version,
                )
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
                    &BlockInfo::default(),
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
        fn should_return_invalid_result_if_trying_to_update_config() {
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
                    &mut vec![],
                    platform_version,
                )
                .expect("to be able to set document schema");

            // It should be not possible to modify this
            data_contract.config_mut().set_keeps_history(false);

            let state_transition: DataContractUpdateTransitionV0 = (data_contract, 1)
                .try_into_platform_versioned(platform_version)
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
                    &BlockInfo::default(),
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

    #[test]
    fn test_data_contract_update_changing_various_document_type_options() {
        let mut platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_initial_state_structure();

        let (identity, signer, key) = setup_identity(&mut platform, 958, dash_to_credits!(1.0));

        let card_game_path = "tests/supporting_files/contract/crypto-card-game/crypto-card-game-direct-purchase-creation-restricted-to-owner.json";

        let platform_state = platform.state.load();
        let platform_version = platform_state
            .current_platform_version()
            .expect("expected to get current platform version");

        // let's construct the grovedb structure for the card game data contract
        let mut contract = json_document_to_contract(card_game_path, true, platform_version)
            .expect("expected to get data contract");

        contract.set_owner_id(identity.id());

        platform
            .drive
            .apply_contract(
                &contract,
                BlockInfo::default(),
                true,
                StorageFlags::optional_default_as_cow(),
                None,
                platform_version,
            )
            .expect("expected to apply contract successfully");

        let updated_card_game_path = "tests/supporting_files/contract/crypto-card-game/crypto-card-game-direct-purchase.json";

        // let's construct the grovedb structure for the card game data contract
        let mut contract_not_restricted_to_owner =
            json_document_to_contract(updated_card_game_path, true, platform_version)
                .expect("expected to get data contract");

        contract_not_restricted_to_owner.set_owner_id(identity.id());

        contract_not_restricted_to_owner.set_version(2);

        let data_contract_update_transition = DataContractUpdateTransition::new_from_data_contract(
            contract_not_restricted_to_owner,
            &identity.into_partial_identity_info(),
            key.id(),
            2,
            0,
            &signer,
            platform_version,
            None,
        )
        .expect("expect to create documents batch transition");

        let data_contract_update_serialized_transition = data_contract_update_transition
            .serialize_to_bytes()
            .expect("expected documents batch serialized state transition");

        let transaction = platform.drive.grove.start_transaction();

        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &[data_contract_update_serialized_transition.clone()],
                &platform_state,
                &BlockInfo::default(),
                &transaction,
                platform_version,
                false,
                None,
            )
            .expect("expected to process state transition");

        // There is no issue because the creator of the contract made the document

        assert_eq!(processing_result.invalid_paid_count(), 1);

        platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("expected to commit transaction");

        let result = processing_result.into_execution_results().remove(0);

        assert!(matches!(
            result,
            StateTransitionExecutionResult::PaidConsensusError(
                ConsensusError::StateError(
                    StateError::DocumentTypeUpdateError(error)
                ), _
            ) if error.data_contract_id() == &contract.id()
                && error.document_type_name() == "card"
                && error.additional_message() == "document type can not change creation restriction mode: changing from Owner Only to No Restrictions"
        ));
    }

    mod group_tests {
        use super::*;
        use crate::platform_types::state_transitions_processing_result::StateTransitionExecutionResult::UnpaidConsensusError;

        #[test]
        fn test_data_contract_update_can_not_remove_groups() {
            let mut platform = TestPlatformBuilder::new()
                .build_with_mock_rpc()
                .set_initial_state_structure();

            let (identity, signer, key) = setup_identity(&mut platform, 958, dash_to_credits!(1.0));

            let platform_state = platform.state.load();
            let platform_version = platform_state
                .current_platform_version()
                .expect("expected to get current platform version");

            // Create an initial data contract with groups
            let mut data_contract =
                get_data_contract_fixture(None, 0, platform_version.protocol_version)
                    .data_contract_owned();

            data_contract.set_owner_id(identity.id());

            {
                // Add groups to the contract
                let groups = data_contract.groups_mut().expect("expected groups");
                groups.insert(
                    0,
                    Group::V0(GroupV0 {
                        members: [(identity.id(), 1)].into(),
                        required_power: 1,
                    }),
                );
                groups.insert(
                    1,
                    Group::V0(GroupV0 {
                        members: [(identity.id(), 1)].into(),
                        required_power: 1,
                    }),
                );
            }

            platform
                .drive
                .apply_contract(
                    &data_contract,
                    BlockInfo::default(),
                    true,
                    StorageFlags::optional_default_as_cow(),
                    None,
                    platform_version,
                )
                .expect("expected to apply contract successfully");

            // Create an updated contract with one group removed
            let mut updated_data_contract = data_contract.clone();
            updated_data_contract.set_version(2);

            {
                // Remove a group from the updated contract
                let groups = updated_data_contract.groups_mut().expect("expected groups");
                groups.remove(&1).expect("expected to remove group");
            }

            let data_contract_update_transition =
                DataContractUpdateTransition::new_from_data_contract(
                    updated_data_contract,
                    &identity.into_partial_identity_info(),
                    key.id(),
                    2,
                    0,
                    &signer,
                    platform_version,
                    None,
                )
                .expect("expect to create data contract update transition");

            let data_contract_update_serialized_transition = data_contract_update_transition
                .serialize_to_bytes()
                .expect("expected serialized state transition");

            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &[data_contract_update_serialized_transition.clone()],
                    &platform_state,
                    &BlockInfo::default(),
                    &transaction,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected to process state transition");

            // Extract the error and check the message
            if let [StateTransitionExecutionResult::PaidConsensusError(
                ConsensusError::StateError(StateError::DataContractUpdateActionNotAllowedError(
                    error,
                )),
                _,
            )] = processing_result.execution_results().as_slice()
            {
                assert_eq!(
                    error.action(),
                    "remove group",
                    "expected error message to match 'remove group'"
                );
                assert_eq!(
                    error.data_contract_id(),
                    data_contract.id(),
                    "expected the error to reference the correct data contract ID"
                );
            } else {
                panic!("Expected a DataContractUpdateActionNotAllowedError");
            }

            platform
                .drive
                .grove
                .commit_transaction(transaction)
                .unwrap()
                .expect("expected to commit transaction");
        }

        #[test]
        fn test_data_contract_update_can_not_alter_group() {
            let mut platform = TestPlatformBuilder::new()
                .build_with_mock_rpc()
                .set_initial_state_structure();

            let (identity, signer, key) = setup_identity(&mut platform, 958, dash_to_credits!(1.0));

            let platform_state = platform.state.load();
            let platform_version = platform_state
                .current_platform_version()
                .expect("expected to get current platform version");

            // Create an initial data contract with groups
            let mut data_contract =
                get_data_contract_fixture(None, 0, platform_version.protocol_version)
                    .data_contract_owned();

            data_contract.set_owner_id(identity.id());

            {
                // Add groups to the contract
                let groups = data_contract.groups_mut().expect("expected groups");
                groups.insert(
                    0,
                    Group::V0(GroupV0 {
                        members: [(identity.id(), 1)].into(),
                        required_power: 1,
                    }),
                );
                groups.insert(
                    1,
                    Group::V0(GroupV0 {
                        members: [(identity.id(), 1)].into(),
                        required_power: 1,
                    }),
                );
            }

            platform
                .drive
                .apply_contract(
                    &data_contract,
                    BlockInfo::default(),
                    true,
                    StorageFlags::optional_default_as_cow(),
                    None,
                    platform_version,
                )
                .expect("expected to apply contract successfully");

            // Create an updated contract with one group removed
            let mut updated_data_contract = data_contract.clone();
            updated_data_contract.set_version(2);

            {
                // Remove a group from the updated contract
                let groups = updated_data_contract.groups_mut().expect("expected groups");
                groups.insert(
                    1,
                    Group::V0(GroupV0 {
                        members: [(identity.id(), 2)].into(),
                        required_power: 2,
                    }),
                );
            }

            let data_contract_update_transition =
                DataContractUpdateTransition::new_from_data_contract(
                    updated_data_contract,
                    &identity.into_partial_identity_info(),
                    key.id(),
                    2,
                    0,
                    &signer,
                    platform_version,
                    None,
                )
                .expect("expect to create data contract update transition");

            let data_contract_update_serialized_transition = data_contract_update_transition
                .serialize_to_bytes()
                .expect("expected serialized state transition");

            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &[data_contract_update_serialized_transition.clone()],
                    &platform_state,
                    &BlockInfo::default(),
                    &transaction,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected to process state transition");

            // Extract the error and check the message
            if let [StateTransitionExecutionResult::PaidConsensusError(
                ConsensusError::StateError(StateError::DataContractUpdateActionNotAllowedError(
                    error,
                )),
                _,
            )] = processing_result.execution_results().as_slice()
            {
                assert_eq!(
                    error.action(),
                    "change group at position 1 is not allowed",
                    "expected error message to match 'change group at position 1 is not allowed'"
                );
                assert_eq!(
                    error.data_contract_id(),
                    data_contract.id(),
                    "expected the error to reference the correct data contract ID"
                );
            } else {
                panic!("Expected a DataContractUpdateActionNotAllowedError");
            }

            platform
                .drive
                .grove
                .commit_transaction(transaction)
                .unwrap()
                .expect("expected to commit transaction");
        }

        #[test]
        fn test_data_contract_update_can_not_add_new_group_with_gap() {
            let mut platform = TestPlatformBuilder::new()
                .build_with_mock_rpc()
                .set_initial_state_structure();

            let (identity, signer, key) = setup_identity(&mut platform, 958, dash_to_credits!(1.0));

            let platform_state = platform.state.load();
            let platform_version = platform_state
                .current_platform_version()
                .expect("expected to get current platform version");

            // Create an initial data contract with groups
            let mut data_contract =
                get_data_contract_fixture(None, 0, platform_version.protocol_version)
                    .data_contract_owned();

            data_contract.set_owner_id(identity.id());

            {
                // Add groups to the contract
                let groups = data_contract.groups_mut().expect("expected groups");
                groups.insert(
                    0,
                    Group::V0(GroupV0 {
                        members: [(identity.id(), 1)].into(),
                        required_power: 1,
                    }),
                );
                groups.insert(
                    1,
                    Group::V0(GroupV0 {
                        members: [(identity.id(), 1)].into(),
                        required_power: 1,
                    }),
                );
            }

            platform
                .drive
                .apply_contract(
                    &data_contract,
                    BlockInfo::default(),
                    true,
                    StorageFlags::optional_default_as_cow(),
                    None,
                    platform_version,
                )
                .expect("expected to apply contract successfully");

            // Create an updated contract with one group removed
            let mut updated_data_contract = data_contract.clone();
            updated_data_contract.set_version(2);

            {
                // Remove a group from the updated contract
                let groups = updated_data_contract.groups_mut().expect("expected groups");
                groups.insert(
                    3,
                    Group::V0(GroupV0 {
                        members: [(identity.id(), 2)].into(),
                        required_power: 2,
                    }),
                );
            }

            let data_contract_update_transition =
                DataContractUpdateTransition::new_from_data_contract(
                    updated_data_contract,
                    &identity.into_partial_identity_info(),
                    key.id(),
                    2,
                    0,
                    &signer,
                    platform_version,
                    None,
                )
                .expect("expect to create data contract update transition");

            let data_contract_update_serialized_transition = data_contract_update_transition
                .serialize_to_bytes()
                .expect("expected serialized state transition");

            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &[data_contract_update_serialized_transition.clone()],
                    &platform_state,
                    &BlockInfo::default(),
                    &transaction,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected to process state transition");

            assert_matches!(
                processing_result.execution_results().as_slice(),
                [UnpaidConsensusError(ConsensusError::BasicError(
                    BasicError::NonContiguousContractGroupPositionsError(_)
                ))]
            );

            platform
                .drive
                .grove
                .commit_transaction(transaction)
                .unwrap()
                .expect("expected to commit transaction");
        }

        #[test]
        fn test_data_contract_update_can_add_new_group() {
            let mut platform = TestPlatformBuilder::new()
                .build_with_mock_rpc()
                .set_initial_state_structure();

            let (identity, signer, key) = setup_identity(&mut platform, 958, dash_to_credits!(1.0));

            let (identity_2, _, _) = setup_identity(&mut platform, 928, dash_to_credits!(0.1));

            let (identity_3, _, _) = setup_identity(&mut platform, 8, dash_to_credits!(0.1));

            let platform_state = platform.state.load();
            let platform_version = platform_state
                .current_platform_version()
                .expect("expected to get current platform version");

            // Create an initial data contract with groups
            let mut data_contract =
                get_data_contract_fixture(None, 0, platform_version.protocol_version)
                    .data_contract_owned();

            data_contract.set_owner_id(identity.id());

            {
                // Add groups to the contract
                let groups = data_contract.groups_mut().expect("expected groups");
                groups.insert(
                    0,
                    Group::V0(GroupV0 {
                        members: [
                            (identity.id(), 1),
                            (identity_2.id(), 1),
                            (identity_3.id(), 1),
                        ]
                        .into(),
                        required_power: 3,
                    }),
                );
                groups.insert(
                    1,
                    Group::V0(GroupV0 {
                        members: [
                            (identity.id(), 1),
                            (identity_2.id(), 2),
                            (identity_3.id(), 1),
                        ]
                        .into(),
                        required_power: 3,
                    }),
                );
            }

            platform
                .drive
                .apply_contract(
                    &data_contract,
                    BlockInfo::default(),
                    true,
                    StorageFlags::optional_default_as_cow(),
                    None,
                    platform_version,
                )
                .expect("expected to apply contract successfully");

            // Create an updated contract with one group removed
            let mut updated_data_contract = data_contract.clone();
            updated_data_contract.set_version(2);

            {
                // Remove a group from the updated contract
                let groups = updated_data_contract.groups_mut().expect("expected groups");
                groups.insert(
                    2,
                    Group::V0(GroupV0 {
                        members: [
                            (identity.id(), 1),
                            (identity_2.id(), 2),
                            (identity_3.id(), 2),
                        ]
                        .into(),
                        required_power: 3,
                    }),
                );
            }

            let data_contract_update_transition =
                DataContractUpdateTransition::new_from_data_contract(
                    updated_data_contract,
                    &identity.into_partial_identity_info(),
                    key.id(),
                    2,
                    0,
                    &signer,
                    platform_version,
                    None,
                )
                .expect("expect to create data contract update transition");

            let data_contract_update_serialized_transition = data_contract_update_transition
                .serialize_to_bytes()
                .expect("expected serialized state transition");

            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &[data_contract_update_serialized_transition.clone()],
                    &platform_state,
                    &BlockInfo::default(),
                    &transaction,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected to process state transition");

            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::SuccessfulExecution(_, _)]
            );

            platform
                .drive
                .grove
                .commit_transaction(transaction)
                .unwrap()
                .expect("expected to commit transaction");
        }
    }

    mod token_tests {
        use super::*;
        use crate::platform_types::state_transitions_processing_result::StateTransitionExecutionResult::UnpaidConsensusError;
        use dpp::data_contract::accessors::v1::DataContractV1Setters;
        use dpp::data_contract::associated_token::token_configuration::accessors::v0::{TokenConfigurationV0Getters, TokenConfigurationV0Setters};
        use dpp::data_contract::associated_token::token_configuration::v0::TokenConfigurationV0;
        use dpp::data_contract::associated_token::token_configuration::TokenConfiguration;
        use dpp::data_contract::associated_token::token_configuration_convention::accessors::v0::TokenConfigurationConventionV0Getters;
        use dpp::data_contract::associated_token::token_configuration_convention::v0::TokenConfigurationConventionV0;
        use dpp::data_contract::associated_token::token_configuration_convention::TokenConfigurationConvention;
        use dpp::data_contract::associated_token::token_configuration_localization::v0::TokenConfigurationLocalizationV0;
        use dpp::data_contract::associated_token::token_configuration_localization::TokenConfigurationLocalization;
        use dpp::data_contract::associated_token::token_distribution_rules::accessors::v0::TokenDistributionRulesV0Setters;
        use dpp::data_contract::associated_token::token_perpetual_distribution::distribution_function::DistributionFunction;
        use dpp::data_contract::associated_token::token_perpetual_distribution::distribution_recipient::TokenDistributionRecipient;
        use dpp::data_contract::associated_token::token_perpetual_distribution::reward_distribution_type::RewardDistributionType;
        use dpp::data_contract::associated_token::token_perpetual_distribution::TokenPerpetualDistribution;
        use dpp::data_contract::associated_token::token_perpetual_distribution::v0::TokenPerpetualDistributionV0;
        use dpp::data_contract::change_control_rules::authorized_action_takers::AuthorizedActionTakers;
        use dpp::data_contract::change_control_rules::ChangeControlRules;
        use dpp::data_contract::change_control_rules::v0::ChangeControlRulesV0;

        #[test]
        fn test_data_contract_update_can_add_new_token() {
            let mut platform = TestPlatformBuilder::new()
                .build_with_mock_rpc()
                .set_initial_state_structure();

            let (identity, signer, key) = setup_identity(&mut platform, 958, dash_to_credits!(1.0));

            let platform_state = platform.state.load();
            let platform_version = platform_state
                .current_platform_version()
                .expect("expected to get current platform version");

            // ── original contract (no tokens) ─────────────────────────────────
            let mut data_contract =
                get_data_contract_fixture(None, 0, platform_version.protocol_version)
                    .data_contract_owned();
            data_contract.set_owner_id(identity.id());

            platform
                .drive
                .apply_contract(
                    &data_contract,
                    BlockInfo::default(),
                    true,
                    StorageFlags::optional_default_as_cow(),
                    None,
                    platform_version,
                )
                .expect("expected to apply contract successfully");

            // ── updated contract: add a well‑formed token at position 0 ──────
            let mut updated_data_contract = data_contract.clone();
            updated_data_contract.set_version(2);

            let valid_token_cfg = {
                let mut cfg =
                    TokenConfiguration::V0(TokenConfigurationV0::default_most_restrictive());
                cfg.set_base_supply(1_000_000);

                cfg.set_conventions(TokenConfigurationConvention::V0(
                    TokenConfigurationConventionV0 {
                        localizations: BTreeMap::from([(
                            "en".to_string(),
                            TokenConfigurationLocalization::V0(TokenConfigurationLocalizationV0 {
                                should_capitalize: true,
                                singular_form: "credit".to_string(),
                                plural_form: "credits".to_string(),
                            }),
                        )]),
                        decimals: 8,
                    },
                ));
                cfg
            };

            updated_data_contract.add_token(0, valid_token_cfg);

            let data_contract_update_transition =
                DataContractUpdateTransition::new_from_data_contract(
                    updated_data_contract,
                    &identity.into_partial_identity_info(),
                    key.id(),
                    2,
                    0,
                    &signer,
                    platform_version,
                    None,
                )
                .expect("expect to create data contract update transition");

            let tx_bytes = data_contract_update_transition
                .serialize_to_bytes()
                .expect("expected serialized state transition");

            let transaction = platform.drive.grove.start_transaction();
            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &[tx_bytes],
                    &platform_state,
                    &BlockInfo::default(),
                    &transaction,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected to process state transition");

            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::SuccessfulExecution(_, _)]
            );

            platform
                .drive
                .grove
                .commit_transaction(transaction)
                .unwrap()
                .expect("expected to commit transaction");
        }

        #[test]
        fn test_data_contract_update_with_token_setting_identifier_that_does_exist() {
            let mut platform = TestPlatformBuilder::new()
                .build_with_mock_rpc()
                .set_initial_state_structure();

            let (identity, signer, key) = setup_identity(&mut platform, 958, dash_to_credits!(1.0));
            let (identity2, _signer2, _key2) =
                setup_identity(&mut platform, 93, dash_to_credits!(0.2));

            let platform_state = platform.state.load();
            let platform_version = PlatformVersion::latest();

            let mut original_contract =
                get_data_contract_fixture(None, 0, platform_version.protocol_version)
                    .data_contract_owned();
            original_contract.set_owner_id(identity.id());

            platform
                .drive
                .apply_contract(
                    &original_contract,
                    BlockInfo::default(),
                    true,
                    StorageFlags::optional_default_as_cow(),
                    None,
                    platform_version,
                )
                .expect("expected to apply contract");

            let mut updated_contract = original_contract.clone();
            updated_contract.set_version(2);

            let mut token_config =
                TokenConfiguration::V0(TokenConfigurationV0::default_most_restrictive());
            token_config.set_base_supply(100_000);
            token_config.set_manual_minting_rules(ChangeControlRules::V0(ChangeControlRulesV0 {
                authorized_to_make_change: AuthorizedActionTakers::Identity(identity2.id()),
                admin_action_takers: AuthorizedActionTakers::ContractOwner,
                changing_authorized_action_takers_to_no_one_allowed: false,
                changing_admin_action_takers_to_no_one_allowed: false,
                self_changing_admin_action_takers_allowed: false,
            }));

            token_config.set_conventions(TokenConfigurationConvention::V0(
                TokenConfigurationConventionV0 {
                    localizations: BTreeMap::from([(
                        "en".to_string(),
                        TokenConfigurationLocalization::V0(TokenConfigurationLocalizationV0 {
                            should_capitalize: true,
                            singular_form: "test".to_string(),
                            plural_form: "tests".to_string(),
                        }),
                    )]),
                    decimals: 8,
                },
            ));

            updated_contract.add_token(0, token_config);

            let transition = DataContractUpdateTransition::new_from_data_contract(
                updated_contract,
                &identity.into_partial_identity_info(),
                key.id(),
                2,
                0,
                &signer,
                platform_version,
                None,
            )
            .expect("expected update transition");

            let serialized = transition.serialize_to_bytes().expect("serialize");

            let transaction = platform.drive.grove.start_transaction();
            let result = platform
                .platform
                .process_raw_state_transitions(
                    &[serialized],
                    &platform_state,
                    &BlockInfo::default(),
                    &transaction,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected processing");

            assert_matches!(
                result.execution_results().as_slice(),
                [StateTransitionExecutionResult::SuccessfulExecution(_, _)]
            );

            platform
                .drive
                .grove
                .commit_transaction(transaction)
                .unwrap()
                .expect("commit");
        }
        #[test]
        fn test_data_contract_update_with_token_setting_identifier_that_does_not_exist() {
            let mut platform = TestPlatformBuilder::new()
                .build_with_mock_rpc()
                .set_initial_state_structure();

            let (identity, signer, key) = setup_identity(&mut platform, 958, dash_to_credits!(1.0));
            let platform_state = platform.state.load();
            let platform_version = PlatformVersion::latest();

            let mut original_contract =
                get_data_contract_fixture(None, 0, platform_version.protocol_version)
                    .data_contract_owned();
            original_contract.set_owner_id(identity.id());

            platform
                .drive
                .apply_contract(
                    &original_contract,
                    BlockInfo::default(),
                    true,
                    StorageFlags::optional_default_as_cow(),
                    None,
                    platform_version,
                )
                .expect("expected to apply contract");

            let mut updated_contract = original_contract.clone();
            updated_contract.set_version(2);

            let mut token_config =
                TokenConfiguration::V0(TokenConfigurationV0::default_most_restrictive());
            token_config.set_base_supply(1_000_000);

            token_config.set_manual_minting_rules(ChangeControlRules::V0(ChangeControlRulesV0 {
                authorized_to_make_change: AuthorizedActionTakers::Identity(Identifier::from(
                    [4; 32],
                )), // doesn't exist
                admin_action_takers: AuthorizedActionTakers::ContractOwner,
                changing_authorized_action_takers_to_no_one_allowed: false,
                changing_admin_action_takers_to_no_one_allowed: false,
                self_changing_admin_action_takers_allowed: false,
            }));

            token_config.set_conventions(TokenConfigurationConvention::V0(
                TokenConfigurationConventionV0 {
                    localizations: BTreeMap::from([(
                        "en".to_string(),
                        TokenConfigurationLocalization::V0(TokenConfigurationLocalizationV0 {
                            should_capitalize: true,
                            singular_form: "test".to_string(),
                            plural_form: "tests".to_string(),
                        }),
                    )]),
                    decimals: 8,
                },
            ));

            updated_contract.add_token(0, token_config);

            let transition = DataContractUpdateTransition::new_from_data_contract(
                updated_contract,
                &identity.into_partial_identity_info(),
                key.id(),
                2,
                0,
                &signer,
                platform_version,
                None,
            )
            .expect("expected update transition");

            let serialized = transition.serialize_to_bytes().expect("serialize");

            let transaction = platform.drive.grove.start_transaction();
            let result = platform
                .platform
                .process_raw_state_transitions(
                    &[serialized],
                    &platform_state,
                    &BlockInfo::default(),
                    &transaction,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected processing");

            assert_matches!(
                result.execution_results().as_slice(),
                [StateTransitionExecutionResult::PaidConsensusError(
                    ConsensusError::StateError(
                        StateError::IdentityInTokenConfigurationNotFoundError(_)
                    ),
                    _
                )]
            );

            platform
                .drive
                .grove
                .commit_transaction(transaction)
                .unwrap()
                .expect("commit");
        }

        #[test]
        fn test_data_contract_update_can_not_add_new_token_with_gap() {
            let mut platform = TestPlatformBuilder::new()
                .build_with_mock_rpc()
                .set_initial_state_structure();

            let (identity, signer, key) = setup_identity(&mut platform, 958, dash_to_credits!(1.0));

            let platform_state = platform.state.load();
            let platform_version = platform_state
                .current_platform_version()
                .expect("expected to get current platform version");

            // ── original contract with token at position 0 ───────────────────
            let mut data_contract =
                get_data_contract_fixture(None, 0, platform_version.protocol_version)
                    .data_contract_owned();
            data_contract.set_owner_id(identity.id());
            data_contract.add_token(
                0,
                TokenConfiguration::V0(TokenConfigurationV0::default_most_restrictive()),
            );
            data_contract
                .tokens_mut()
                .expect("expected tokens")
                .get_mut(&0)
                .expect("expected token")
                .conventions_mut()
                .localizations_mut()
                .insert(
                    "en".to_string(),
                    TokenConfigurationLocalization::V0(TokenConfigurationLocalizationV0 {
                        should_capitalize: true,
                        singular_form: "test".to_string(),
                        plural_form: "tests".to_string(),
                    }),
                );

            platform
                .drive
                .apply_contract(
                    &data_contract,
                    BlockInfo::default(),
                    true,
                    StorageFlags::optional_default_as_cow(),
                    None,
                    platform_version,
                )
                .expect("expected to apply contract successfully");

            // ── updated contract: try to add token at position 2 (gap) ───────
            let mut updated_data_contract = data_contract.clone();
            updated_data_contract.set_version(2);

            updated_data_contract.add_token(
                2, // <‑‑ non‑contiguous
                TokenConfiguration::V0(TokenConfigurationV0::default_most_restrictive()),
            );

            let data_contract_update_transition =
                DataContractUpdateTransition::new_from_data_contract(
                    updated_data_contract,
                    &identity.into_partial_identity_info(),
                    key.id(),
                    2,
                    0,
                    &signer,
                    platform_version,
                    None,
                )
                .expect("expect to create data contract update transition");

            let tx_bytes = data_contract_update_transition
                .serialize_to_bytes()
                .expect("expected serialized state transition");

            let transaction = platform.drive.grove.start_transaction();
            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &[tx_bytes],
                    &platform_state,
                    &BlockInfo::default(),
                    &transaction,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected to process state transition");

            assert_matches!(
                processing_result.execution_results().as_slice(),
                [UnpaidConsensusError(ConsensusError::BasicError(
                    BasicError::NonContiguousContractTokenPositionsError(_)
                ))]
            );

            platform
                .drive
                .grove
                .commit_transaction(transaction)
                .unwrap()
                .expect("expected to commit transaction");
        }

        #[test]
        fn test_data_contract_update_can_not_add_new_token_with_large_base_supply() {
            let mut platform = TestPlatformBuilder::new()
                .build_with_mock_rpc()
                .set_initial_state_structure();

            let (identity, signer, key) = setup_identity(&mut platform, 958, dash_to_credits!(1.0));

            let platform_state = platform.state.load();
            let platform_version = platform_state
                .current_platform_version()
                .expect("expected to get current platform version");

            // ── original contract (no tokens) ────────────────────────────────
            let mut data_contract =
                get_data_contract_fixture(None, 0, platform_version.protocol_version)
                    .data_contract_owned();
            data_contract.set_owner_id(identity.id());

            platform
                .drive
                .apply_contract(
                    &data_contract,
                    BlockInfo::default(),
                    true,
                    StorageFlags::optional_default_as_cow(),
                    None,
                    platform_version,
                )
                .expect("expected to apply contract successfully");

            // ── updated contract: token with base_supply > i64::MAX ──────────
            let mut updated_data_contract = data_contract.clone();
            updated_data_contract.set_version(2);

            let mut huge_supply_cfg =
                TokenConfiguration::V0(TokenConfigurationV0::default_most_restrictive());
            huge_supply_cfg.set_base_supply(i64::MAX as u64 + 1);

            updated_data_contract.add_token(0, huge_supply_cfg);

            let data_contract_update_transition =
                DataContractUpdateTransition::new_from_data_contract(
                    updated_data_contract,
                    &identity.into_partial_identity_info(),
                    key.id(),
                    2,
                    0,
                    &signer,
                    platform_version,
                    None,
                )
                .expect("expect to create data contract update transition");

            let tx_bytes = data_contract_update_transition
                .serialize_to_bytes()
                .expect("expected serialized state transition");

            let transaction = platform.drive.grove.start_transaction();
            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &[tx_bytes],
                    &platform_state,
                    &BlockInfo::default(),
                    &transaction,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected to process state transition");

            assert_matches!(
                processing_result.execution_results().as_slice(),
                [UnpaidConsensusError(ConsensusError::BasicError(
                    BasicError::InvalidTokenBaseSupplyError(_)
                ))]
            );

            platform
                .drive
                .grove
                .commit_transaction(transaction)
                .unwrap()
                .expect("expected to commit transaction");
        }

        #[test]
        fn test_data_contract_update_can_not_add_new_token_with_invalid_localization() {
            let mut platform = TestPlatformBuilder::new()
                .build_with_mock_rpc()
                .set_initial_state_structure();

            let (identity, signer, key) = setup_identity(&mut platform, 958, dash_to_credits!(1.0));

            let platform_state = platform.state.load();
            let platform_version = platform_state
                .current_platform_version()
                .expect("expected to get current platform version");

            // ── original contract (no tokens) ────────────────────────────────
            let mut data_contract =
                get_data_contract_fixture(None, 0, platform_version.protocol_version)
                    .data_contract_owned();
            data_contract.set_owner_id(identity.id());

            platform
                .drive
                .apply_contract(
                    &data_contract,
                    BlockInfo::default(),
                    true,
                    StorageFlags::optional_default_as_cow(),
                    None,
                    platform_version,
                )
                .expect("expected to apply contract successfully");

            // ── updated contract: token with empty localization map ──────────
            let mut updated_data_contract = data_contract.clone();
            updated_data_contract.set_version(2);

            let empty_localization_cfg = {
                let mut cfg =
                    TokenConfiguration::V0(TokenConfigurationV0::default_most_restrictive());
                cfg.set_conventions(TokenConfigurationConvention::V0(
                    TokenConfigurationConventionV0 {
                        localizations: BTreeMap::new(), // <‑‑ invalid
                        decimals: 8,
                    },
                ));
                cfg
            };

            updated_data_contract.add_token(0, empty_localization_cfg);

            let data_contract_update_transition =
                DataContractUpdateTransition::new_from_data_contract(
                    updated_data_contract,
                    &identity.into_partial_identity_info(),
                    key.id(),
                    2,
                    0,
                    &signer,
                    platform_version,
                    None,
                )
                .expect("expect to create data contract update transition");

            let tx_bytes = data_contract_update_transition
                .serialize_to_bytes()
                .expect("expected serialized state transition");

            let transaction = platform.drive.grove.start_transaction();
            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &[tx_bytes],
                    &platform_state,
                    &BlockInfo::default(),
                    &transaction,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected to process state transition");

            assert_matches!(
                processing_result.execution_results().as_slice(),
                [UnpaidConsensusError(ConsensusError::BasicError(
                    BasicError::MissingDefaultLocalizationError(_)
                ))]
            );

            platform
                .drive
                .grove
                .commit_transaction(transaction)
                .unwrap()
                .expect("expected to commit transaction");
        }

        #[test]
        fn update_token_with_missing_main_group_should_fail() {
            let mut platform = TestPlatformBuilder::new()
                .build_with_mock_rpc()
                .set_initial_state_structure();
            let (identity, signer, key) =
                setup_identity(&mut platform, 1234, dash_to_credits!(0.1));
            let platform_state = platform.state.load();
            let platform_version = PlatformVersion::latest();

            let mut contract =
                get_data_contract_fixture(None, 0, platform_version.protocol_version)
                    .data_contract_owned();
            contract.set_owner_id(identity.id());
            platform
                .drive
                .apply_contract(
                    &contract,
                    BlockInfo::default(),
                    true,
                    StorageFlags::optional_default_as_cow(),
                    None,
                    platform_version,
                )
                .unwrap();

            let mut updated_contract = contract.clone();
            updated_contract.set_version(2);

            let mut config =
                TokenConfiguration::V0(TokenConfigurationV0::default_most_restrictive());
            config.set_main_control_group(Some(1)); // Missing group
            config.set_manual_minting_rules(ChangeControlRules::V0(ChangeControlRulesV0 {
                authorized_to_make_change: AuthorizedActionTakers::MainGroup,
                admin_action_takers: AuthorizedActionTakers::MainGroup,
                changing_authorized_action_takers_to_no_one_allowed: false,
                changing_admin_action_takers_to_no_one_allowed: false,
                self_changing_admin_action_takers_allowed: false,
            }));
            config.set_conventions(TokenConfigurationConvention::V0(
                TokenConfigurationConventionV0 {
                    localizations: BTreeMap::from([(
                        "en".to_string(),
                        TokenConfigurationLocalization::V0(TokenConfigurationLocalizationV0 {
                            should_capitalize: true,
                            singular_form: "test".to_string(),
                            plural_form: "tests".to_string(),
                        }),
                    )]),
                    decimals: 8,
                },
            ));
            updated_contract.add_token(0, config);

            let transition = DataContractUpdateTransition::new_from_data_contract(
                updated_contract,
                &identity.into_partial_identity_info(),
                key.id(),
                2,
                0,
                &signer,
                platform_version,
                None,
            )
            .unwrap();
            let tx = platform.drive.grove.start_transaction();
            let result = platform
                .platform
                .process_raw_state_transitions(
                    &[transition.serialize_to_bytes().unwrap()],
                    &platform_state,
                    &BlockInfo::default(),
                    &tx,
                    platform_version,
                    false,
                    None,
                )
                .unwrap();

            assert_matches!(
                result.execution_results().as_slice(),
                [UnpaidConsensusError(ConsensusError::BasicError(
                    BasicError::GroupPositionDoesNotExistError(_)
                ))]
            );
        }

        #[test]
        fn update_token_with_invalid_distribution_function_should_fail() {
            let mut platform = TestPlatformBuilder::new()
                .build_with_mock_rpc()
                .set_initial_state_structure();
            let (identity, signer, key) =
                setup_identity(&mut platform, 1234, dash_to_credits!(0.1));
            let platform_state = platform.state.load();
            let platform_version = PlatformVersion::latest();

            let mut contract =
                get_data_contract_fixture(None, 0, platform_version.protocol_version)
                    .data_contract_owned();
            contract.set_owner_id(identity.id());
            platform
                .drive
                .apply_contract(
                    &contract,
                    BlockInfo::default(),
                    true,
                    StorageFlags::optional_default_as_cow(),
                    None,
                    platform_version,
                )
                .unwrap();

            let mut updated_contract = contract.clone();
            updated_contract.set_version(2);

            let mut config =
                TokenConfiguration::V0(TokenConfigurationV0::default_most_restrictive());
            config
                .distribution_rules_mut()
                .set_perpetual_distribution(Some(TokenPerpetualDistribution::V0(
                    TokenPerpetualDistributionV0 {
                        distribution_type: RewardDistributionType::BlockBasedDistribution {
                            interval: 100,
                            function: DistributionFunction::Exponential {
                                a: 0,
                                d: 0,
                                m: 0,
                                n: 0,
                                o: 0,
                                start_moment: None,
                                b: 0,
                                min_value: None,
                                max_value: None,
                            },
                        },
                        distribution_recipient: TokenDistributionRecipient::Identity(identity.id()),
                    },
                )));
            config.set_conventions(TokenConfigurationConvention::V0(
                TokenConfigurationConventionV0 {
                    localizations: BTreeMap::from([(
                        "en".to_string(),
                        TokenConfigurationLocalization::V0(TokenConfigurationLocalizationV0 {
                            should_capitalize: true,
                            singular_form: "test".to_string(),
                            plural_form: "tests".to_string(),
                        }),
                    )]),
                    decimals: 8,
                },
            ));
            updated_contract.add_token(0, config);

            let transition = DataContractUpdateTransition::new_from_data_contract(
                updated_contract,
                &identity.into_partial_identity_info(),
                key.id(),
                2,
                0,
                &signer,
                platform_version,
                None,
            )
            .unwrap();
            let tx = platform.drive.grove.start_transaction();
            let result = platform
                .platform
                .process_raw_state_transitions(
                    &[transition.serialize_to_bytes().unwrap()],
                    &platform_state,
                    &BlockInfo::default(),
                    &tx,
                    platform_version,
                    false,
                    None,
                )
                .unwrap();

            assert_matches!(
                result.execution_results().as_slice(),
                [UnpaidConsensusError(ConsensusError::BasicError(
                    BasicError::InvalidTokenDistributionFunctionDivideByZeroError(_)
                ))]
            );
        }

        #[test]
        fn update_token_with_random_distribution_should_fail() {
            let mut platform = TestPlatformBuilder::new()
                .build_with_mock_rpc()
                .set_initial_state_structure();
            let (identity, signer, key) =
                setup_identity(&mut platform, 1234, dash_to_credits!(0.1));
            let platform_state = platform.state.load();
            let platform_version = PlatformVersion::latest();

            let mut contract =
                get_data_contract_fixture(None, 0, platform_version.protocol_version)
                    .data_contract_owned();
            contract.set_owner_id(identity.id());
            platform
                .drive
                .apply_contract(
                    &contract,
                    BlockInfo::default(),
                    true,
                    StorageFlags::optional_default_as_cow(),
                    None,
                    platform_version,
                )
                .unwrap();

            let mut updated_contract = contract.clone();
            updated_contract.set_version(2);

            let mut config =
                TokenConfiguration::V0(TokenConfigurationV0::default_most_restrictive());
            config
                .distribution_rules_mut()
                .set_perpetual_distribution(Some(TokenPerpetualDistribution::V0(
                    TokenPerpetualDistributionV0 {
                        distribution_type: RewardDistributionType::BlockBasedDistribution {
                            interval: 100,
                            function: DistributionFunction::Random { min: 0, max: 10 },
                        },
                        distribution_recipient: TokenDistributionRecipient::Identity(identity.id()),
                    },
                )));
            config.set_conventions(TokenConfigurationConvention::V0(
                TokenConfigurationConventionV0 {
                    localizations: BTreeMap::from([(
                        "en".to_string(),
                        TokenConfigurationLocalization::V0(TokenConfigurationLocalizationV0 {
                            should_capitalize: true,
                            singular_form: "test".to_string(),
                            plural_form: "tests".to_string(),
                        }),
                    )]),
                    decimals: 8,
                },
            ));
            updated_contract.add_token(0, config);

            let transition = DataContractUpdateTransition::new_from_data_contract(
                updated_contract,
                &identity.into_partial_identity_info(),
                key.id(),
                2,
                0,
                &signer,
                platform_version,
                None,
            )
            .unwrap();
            let tx = platform.drive.grove.start_transaction();
            let result = platform
                .platform
                .process_raw_state_transitions(
                    &[transition.serialize_to_bytes().unwrap()],
                    &platform_state,
                    &BlockInfo::default(),
                    &tx,
                    platform_version,
                    false,
                    None,
                )
                .unwrap();

            assert_matches!(
                result.execution_results().as_slice(),
                [UnpaidConsensusError(ConsensusError::BasicError(
                    BasicError::UnsupportedFeatureError(_)
                ))]
            );
        }

        #[test]
        fn update_token_overwriting_existing_position_should_fail() {
            let mut platform = TestPlatformBuilder::new()
                .build_with_mock_rpc()
                .set_initial_state_structure();
            let (identity, signer, key) =
                setup_identity(&mut platform, 1234, dash_to_credits!(1.0));
            let platform_state = platform.state.load();
            let platform_version = PlatformVersion::latest();

            let mut contract =
                get_data_contract_fixture(None, 0, platform_version.protocol_version)
                    .data_contract_owned();
            contract.set_owner_id(identity.id());
            let mut config =
                TokenConfiguration::V0(TokenConfigurationV0::default_most_restrictive());
            config.set_conventions(TokenConfigurationConvention::V0(
                TokenConfigurationConventionV0 {
                    localizations: BTreeMap::from([(
                        "en".to_string(),
                        TokenConfigurationLocalization::V0(TokenConfigurationLocalizationV0 {
                            should_capitalize: true,
                            singular_form: "test".to_string(),
                            plural_form: "tests".to_string(),
                        }),
                    )]),
                    decimals: 8,
                },
            ));

            let mut config_2 =
                TokenConfiguration::V0(TokenConfigurationV0::default_most_restrictive());
            config_2.set_conventions(TokenConfigurationConvention::V0(
                TokenConfigurationConventionV0 {
                    localizations: BTreeMap::from([(
                        "en".to_string(),
                        TokenConfigurationLocalization::V0(TokenConfigurationLocalizationV0 {
                            should_capitalize: true,
                            singular_form: "test_1".to_string(),
                            plural_form: "tests_2".to_string(),
                        }),
                    )]),
                    decimals: 8,
                },
            ));
            contract.add_token(0, config);

            platform
                .drive
                .apply_contract(
                    &contract,
                    BlockInfo::default(),
                    true,
                    StorageFlags::optional_default_as_cow(),
                    None,
                    platform_version,
                )
                .unwrap();

            let mut updated_contract = contract.clone();
            updated_contract.set_version(2);
            updated_contract.add_token(0, config_2);

            let transition = DataContractUpdateTransition::new_from_data_contract(
                updated_contract,
                &identity.into_partial_identity_info(),
                key.id(),
                2,
                0,
                &signer,
                platform_version,
                None,
            )
            .unwrap();
            let tx = platform.drive.grove.start_transaction();
            let result = platform
                .platform
                .process_raw_state_transitions(
                    &[transition.serialize_to_bytes().unwrap()],
                    &platform_state,
                    &BlockInfo::default(),
                    &tx,
                    platform_version,
                    false,
                    None,
                )
                .unwrap();

            assert_matches!(
                result.execution_results().as_slice(),
                [StateTransitionExecutionResult::PaidConsensusError(
                    ConsensusError::StateError(
                        StateError::DataContractUpdateActionNotAllowedError(_)
                    ),
                    _
                )]
            );
        }
    }

    mod keyword_updates {
        use super::*;
        use dpp::{
            data_contract::conversion::value::v0::DataContractValueConversionMethodsV0,
            data_contracts::SystemDataContract,
            document::DocumentV0Getters,
            platform_value::{string_encoding::Encoding, Value},
            state_transition::{
                data_contract_create_transition::{
                    methods::DataContractCreateTransitionMethodsV0, DataContractCreateTransition,
                },
                StateTransition,
            },
            system_data_contracts::load_system_data_contract,
            tests::json_document::json_document_to_contract_with_ids,
        };
        use drive::{
            drive::document::query::QueryDocumentsOutcomeV0Methods,
            query::{DriveDocumentQuery, WhereClause, WhereOperator},
        };

        // ────────────────────────────────────────────────────────────────────────
        // helpers
        // ────────────────────────────────────────────────────────────────────────

        /// Creates a contract with the supplied keywords and commits it to Drive.
        /// Returns `(contract_id, create_transition)`.
        fn create_contract_with_keywords(
            platform: &mut TempPlatform<MockCoreRPCLike>,
            identity: &Identity,
            signer: &SimpleSigner,
            key: &IdentityPublicKey,
            keywords: &[&str],
            platform_version: &PlatformVersion,
        ) -> (Identifier, StateTransition) {
            let base = json_document_to_contract_with_ids(
                "tests/supporting_files/contract/keyword_test/keyword_base_contract.json",
                None,
                None,
                false,
                platform_version,
            )
            .expect("load base contract");

            let mut val = base.to_value(platform_version).expect("to_value");

            val["keywords"] = Value::Array(
                keywords
                    .iter()
                    .map(|k| Value::Text(k.to_string()))
                    .collect(),
            );

            let contract =
                DataContract::from_value(val, true, platform_version).expect("from_value");

            let create = DataContractCreateTransition::new_from_data_contract(
                contract,
                2,
                &identity.clone().into_partial_identity_info(),
                key.id(),
                signer,
                platform_version,
                None,
            )
            .expect("create transition");

            let tx_bytes = create.serialize_to_bytes().expect("serialize");

            let tx = platform.drive.grove.start_transaction();
            let platform_state = platform.state.load();

            let res = platform
                .platform
                .process_raw_state_transitions(
                    &[tx_bytes],
                    &platform_state,
                    &BlockInfo::default(),
                    &tx,
                    platform_version,
                    false,
                    None,
                )
                .expect("process create");

            assert_matches!(
                res.execution_results().as_slice(),
                [StateTransitionExecutionResult::SuccessfulExecution(_, _)]
            );

            platform
                .drive
                .grove
                .commit_transaction(tx)
                .unwrap()
                .expect("commit create");

            // pull id from unique_identifiers
            let contract_id = Identifier::from_string(
                create
                    .unique_identifiers()
                    .first()
                    .unwrap()
                    .as_str()
                    .split('-')
                    .last()
                    .unwrap(),
                Encoding::Base58,
            )
            .unwrap();

            (contract_id, create)
        }

        /// Convenience for building and applying an **update** transition that
        /// only changes the `keywords` array.
        fn apply_keyword_update(
            platform: &mut TempPlatform<MockCoreRPCLike>,
            contract_id: Identifier,
            identity: &Identity,
            signer: &SimpleSigner,
            key: &IdentityPublicKey,
            new_keywords: &[&str],
            platform_version: &PlatformVersion,
        ) -> Result<(), Vec<StateTransitionExecutionResult>> {
            // fetch existing contract
            let fetched = platform
                .drive
                .fetch_contract(contract_id.into(), None, None, None, platform_version)
                .value
                .unwrap()
                .unwrap();

            let mut val = fetched.contract.to_value(platform_version).unwrap();

            val["keywords"] = Value::Array(
                new_keywords
                    .iter()
                    .map(|k| Value::Text(k.to_string()))
                    .collect(),
            );

            let mut updated_contract =
                DataContract::from_value(val, true, platform_version).unwrap();
            updated_contract.set_version(2);

            let update = DataContractUpdateTransition::new_from_data_contract(
                updated_contract,
                &identity.clone().into_partial_identity_info(),
                key.id(),
                2,
                0,
                signer,
                platform_version,
                None,
            )
            .expect("build update");

            let bytes = update.serialize_to_bytes().unwrap();

            let tx = platform.drive.grove.start_transaction();
            let platform_state = platform.state.load();

            let outcome = platform
                .platform
                .process_raw_state_transitions(
                    &[bytes],
                    &platform_state,
                    &BlockInfo::default(),
                    &tx,
                    platform_version,
                    false,
                    None,
                )
                .expect("process update");

            if matches!(
                outcome.execution_results().as_slice(),
                [StateTransitionExecutionResult::SuccessfulExecution(_, _)]
            ) {
                platform
                    .drive
                    .grove
                    .commit_transaction(tx)
                    .unwrap()
                    .expect("commit update");
                Ok(())
            } else {
                Err(outcome.execution_results().to_vec())
            }
        }

        /// Helper to read all keyword docs for a contract id.
        fn keyword_docs_for_contract(
            platform: &TempPlatform<MockCoreRPCLike>,
            contract_id: Identifier,
            platform_version: &PlatformVersion,
        ) -> Vec<String> {
            let search_contract =
                load_system_data_contract(SystemDataContract::KeywordSearch, platform_version)
                    .unwrap();
            let doc_type = search_contract
                .document_type_for_name("contractKeywords")
                .unwrap();

            let mut query = DriveDocumentQuery {
                contract: &search_contract,
                document_type: doc_type,
                internal_clauses: Default::default(),
                offset: None,
                limit: None,
                order_by: Default::default(),
                start_at: None,
                start_at_included: false,
                block_time_ms: None,
            };
            query.internal_clauses.equal_clauses.insert(
                "contractId".to_string(),
                WhereClause {
                    field: "contractId".to_string(),
                    operator: WhereOperator::Equal,
                    value: contract_id.into(),
                },
            );

            let res = platform
                .drive
                .query_documents(query, None, false, None, None)
                .unwrap();

            res.documents()
                .iter()
                .map(|d| d.get("keyword").unwrap().as_str().unwrap().to_owned())
                .collect()
        }

        // ────────────────────────────────────────────────────────────────────────
        // negative cases – same validation as create
        // ────────────────────────────────────────────────────────────────────────

        macro_rules! invalid_update_test {
            ($name:ident, $keywords:expr, $error:pat_param) => {
                #[test]
                fn $name() {
                    let platform_version = PlatformVersion::latest();
                    let mut platform = TestPlatformBuilder::new()
                        .build_with_mock_rpc()
                        .set_genesis_state();

                    let (identity, signer, key) =
                        setup_identity(&mut platform, 958, dash_to_credits!(10.0));

                    // create initial contract with one keyword so update is allowed
                    let (cid, _) = create_contract_with_keywords(
                        &mut platform,
                        &identity,
                        &signer,
                        &key,
                        &["orig"],
                        &platform_version,
                    );

                    // try invalid update
                    let err = apply_keyword_update(
                        &mut platform,
                        cid,
                        &identity,
                        &signer,
                        &key,
                        &$keywords,
                        &platform_version,
                    )
                    .unwrap_err();

                    assert_matches!(
                        err.as_slice(),
                        [StateTransitionExecutionResult::PaidConsensusError(
                            ConsensusError::BasicError($error),
                            _
                        )]
                    );

                    // original keyword docs must still be there
                    let docs = keyword_docs_for_contract(&platform, cid, &platform_version);
                    assert_eq!(docs, vec!["orig"]);
                }
            };
        }

        invalid_update_test!(
            update_fails_too_many_keywords,
            [
                "kw0", "kw1", "kw2", "kw3", "kw4", "kw5", "kw6", "kw7", "kw8", "kw9", "kw10",
                "kw11", "kw12", "kw13", "kw14", "kw15", "kw16", "kw17", "kw18", "kw19", "kw20",
                "kw21", "kw22", "kw23", "kw24", "kw25", "kw26", "kw27", "kw28", "kw29", "kw30",
                "kw31", "kw32", "kw33", "kw34", "kw35", "kw36", "kw37", "kw38", "kw39", "kw40",
                "kw41", "kw42", "kw43", "kw44", "kw45", "kw46", "kw47", "kw48", "kw49", "kw50",
            ],
            BasicError::TooManyKeywordsError(_)
        );

        invalid_update_test!(
            update_fails_duplicate_keywords,
            ["dup", "dup"],
            BasicError::DuplicateKeywordsError(_)
        );

        invalid_update_test!(
            update_fails_keyword_too_short,
            ["hi"],
            BasicError::InvalidKeywordLengthError(_)
        );

        invalid_update_test!(
            update_fails_keyword_too_long,
            [&"x".repeat(51)],
            BasicError::InvalidKeywordLengthError(_)
        );

        // ────────────────────────────────────────────────────────────────────────
        // positive case – old docs removed, new docs inserted
        // ────────────────────────────────────────────────────────────────────────

        #[test]
        fn update_keywords_replaces_search_docs() {
            let platform_version = PlatformVersion::latest();
            let mut platform = TestPlatformBuilder::new()
                .build_with_mock_rpc()
                .set_genesis_state();

            let (identity, signer, key) = setup_identity(&mut platform, 958, dash_to_credits!(1.0));

            // initial contract with two keywords
            let (cid, _) = create_contract_with_keywords(
                &mut platform,
                &identity,
                &signer,
                &key,
                &["old1", "old2"],
                platform_version,
            );

            // verify initial docs
            let initial_docs = keyword_docs_for_contract(&platform, cid, &platform_version);
            assert_eq!(initial_docs.len(), 2);

            // apply update to ["newA", "newB", "newC"]
            apply_keyword_update(
                &mut platform,
                cid,
                &identity,
                &signer,
                &key,
                &["newA", "newB", "newC"],
                platform_version,
            )
            .expect("update should succeed");

            // fetch contract – keywords updated?
            let fetched = platform
                .drive
                .fetch_contract(cid.into(), None, None, None, platform_version)
                .value
                .unwrap()
                .unwrap();
            assert_eq!(
                *fetched.contract.keywords(),
                ["newa", "newb", "newc"]
                    .iter()
                    .map(|&s| s.to_string())
                    .collect::<Vec<String>>()
            );

            // search‑contract docs updated?
            let docs_after = keyword_docs_for_contract(&platform, cid, platform_version);
            assert_eq!(docs_after.len(), 3);
            assert!(docs_after.contains(&"newa".to_string()));
            assert!(docs_after.contains(&"newb".to_string()));
            assert!(docs_after.contains(&"newc".to_string()));
            // old docs gone
            assert!(!docs_after.contains(&"old1".to_string()));
            assert!(!docs_after.contains(&"old2".to_string()));
        }
    }

    mod description_updates {
        use super::*;
        use dpp::platform_value::btreemap_extensions::BTreeValueMapHelper;
        use dpp::{
            data_contract::conversion::value::v0::DataContractValueConversionMethodsV0,
            data_contracts::SystemDataContract,
            document::DocumentV0Getters,
            platform_value::{string_encoding::Encoding, Value},
            state_transition::{
                data_contract_create_transition::{
                    methods::DataContractCreateTransitionMethodsV0, DataContractCreateTransition,
                },
                StateTransition,
            },
            system_data_contracts::load_system_data_contract,
            tests::json_document::json_document_to_contract_with_ids,
        };
        use drive::{
            drive::document::query::QueryDocumentsOutcomeV0Methods,
            query::{DriveDocumentQuery, WhereClause, WhereOperator},
        };

        // ────────────────────────────────────────────────────────────────────────
        // helpers
        // ────────────────────────────────────────────────────────────────────────

        /// Creates a contract with the supplied description and commits it to Drive.
        /// Returns `(contract_id, create_transition)`.
        fn create_contract_with_description(
            platform: &mut TempPlatform<MockCoreRPCLike>,
            identity: &Identity,
            signer: &SimpleSigner,
            key: &IdentityPublicKey,
            description: &str,
            platform_version: &PlatformVersion,
        ) -> (Identifier, StateTransition) {
            let base = json_document_to_contract_with_ids(
                "tests/supporting_files/contract/keyword_test/keyword_base_contract.json",
                None,
                None,
                false,
                platform_version,
            )
            .expect("load base contract");

            let mut val = base.to_value(platform_version).expect("to_value");

            val["description"] = Value::Text(description.to_string());

            let contract =
                DataContract::from_value(val, true, platform_version).expect("from_value");

            let create = DataContractCreateTransition::new_from_data_contract(
                contract,
                2,
                &identity.clone().into_partial_identity_info(),
                key.id(),
                signer,
                platform_version,
                None,
            )
            .expect("create transition");

            let tx_bytes = create.serialize_to_bytes().expect("serialize");

            let tx = platform.drive.grove.start_transaction();
            let platform_state = platform.state.load();

            let res = platform
                .platform
                .process_raw_state_transitions(
                    &[tx_bytes],
                    &platform_state,
                    &BlockInfo::default(),
                    &tx,
                    platform_version,
                    false,
                    None,
                )
                .expect("process create");

            assert_matches!(
                res.execution_results().as_slice(),
                [StateTransitionExecutionResult::SuccessfulExecution(_, _)]
            );

            platform
                .drive
                .grove
                .commit_transaction(tx)
                .unwrap()
                .expect("commit create");

            // pull id from unique_identifiers
            let contract_id = Identifier::from_string(
                create
                    .unique_identifiers()
                    .first()
                    .unwrap()
                    .as_str()
                    .split('-')
                    .last()
                    .unwrap(),
                Encoding::Base58,
            )
            .unwrap();

            (contract_id, create)
        }

        /// Convenience for building and applying an **update** transition that
        /// only changes the `description` string.
        fn apply_description_update(
            platform: &mut TempPlatform<MockCoreRPCLike>,
            contract_id: Identifier,
            identity: &Identity,
            signer: &SimpleSigner,
            key: &IdentityPublicKey,
            new_description: &str,
            platform_version: &PlatformVersion,
        ) -> Result<(), Vec<StateTransitionExecutionResult>> {
            // fetch existing contract
            let fetched = platform
                .drive
                .fetch_contract(contract_id.into(), None, None, None, platform_version)
                .value
                .unwrap()
                .unwrap();

            let mut val = fetched.contract.to_value(platform_version).unwrap();

            val["description"] = Value::Text(new_description.to_string());

            let mut updated_contract =
                DataContract::from_value(val, true, platform_version).unwrap();
            updated_contract.set_version(2);

            let update = DataContractUpdateTransition::new_from_data_contract(
                updated_contract,
                &identity.clone().into_partial_identity_info(),
                key.id(),
                2,
                0,
                signer,
                platform_version,
                None,
            )
            .expect("build update");

            let bytes = update.serialize_to_bytes().unwrap();

            let tx = platform.drive.grove.start_transaction();
            let platform_state = platform.state.load();

            let outcome = platform
                .platform
                .process_raw_state_transitions(
                    &[bytes],
                    &platform_state,
                    &BlockInfo::default(),
                    &tx,
                    platform_version,
                    false,
                    None,
                )
                .expect("process update");

            if matches!(
                outcome.execution_results().as_slice(),
                [StateTransitionExecutionResult::SuccessfulExecution(_, _)]
            ) {
                platform
                    .drive
                    .grove
                    .commit_transaction(tx)
                    .unwrap()
                    .expect("commit update");
                Ok(())
            } else {
                Err(outcome.execution_results().to_vec())
            }
        }

        /// Helper to read all description docs for a contract id.
        fn description_docs_for_contract(
            platform: &TempPlatform<MockCoreRPCLike>,
            contract_id: Identifier,
            platform_version: &PlatformVersion,
        ) -> String {
            let search_contract =
                load_system_data_contract(SystemDataContract::KeywordSearch, platform_version)
                    .unwrap();
            let doc_type = search_contract
                .document_type_for_name("shortDescription")
                .unwrap();

            let mut query = DriveDocumentQuery {
                contract: &search_contract,
                document_type: doc_type,
                internal_clauses: Default::default(),
                offset: None,
                limit: None,
                order_by: Default::default(),
                start_at: None,
                start_at_included: false,
                block_time_ms: None,
            };
            query.internal_clauses.equal_clauses.insert(
                "contractId".to_string(),
                WhereClause {
                    field: "contractId".to_string(),
                    operator: WhereOperator::Equal,
                    value: contract_id.into(),
                },
            );

            let mut res = platform
                .drive
                .query_documents(query, None, false, None, None)
                .expect("expected query to succeed")
                .documents_owned();

            if res.is_empty() {
                panic!("expected a document description");
            }

            let first_document = res.remove(0);

            first_document
                .properties()
                .get_string("description")
                .expect("expected description to exist")
        }

        // ────────────────────────────────────────────────────────────────────────
        // negative cases – same validation as create
        // ────────────────────────────────────────────────────────────────────────

        macro_rules! invalid_update_test {
            ($name:ident, $description:expr, $error:pat_param) => {
                #[test]
                fn $name() {
                    let platform_version = PlatformVersion::latest();
                    let mut platform = TestPlatformBuilder::new()
                        .build_with_mock_rpc()
                        .set_genesis_state();

                    let (identity, signer, key) =
                        setup_identity(&mut platform, 958, dash_to_credits!(1.0));

                    // create initial contract with description so update is allowed
                    let (cid, _) = create_contract_with_description(
                        &mut platform,
                        &identity,
                        &signer,
                        &key,
                        &"orig",
                        &platform_version,
                    );

                    // try invalid update
                    let err = apply_description_update(
                        &mut platform,
                        cid,
                        &identity,
                        &signer,
                        &key,
                        &$description,
                        &platform_version,
                    )
                    .unwrap_err();

                    assert_matches!(
                        err.as_slice(),
                        [StateTransitionExecutionResult::PaidConsensusError(
                            ConsensusError::BasicError($error),
                            _
                        )]
                    );

                    // original description docs must still be there
                    let docs = description_docs_for_contract(&platform, cid, &platform_version);
                    assert_eq!(docs, "orig".to_string());
                }
            };
        }

        invalid_update_test!(
            update_fails_description_too_short,
            "hi",
            BasicError::InvalidDescriptionLengthError(_)
        );

        invalid_update_test!(
            update_fails_description_too_long,
            &"x".repeat(101),
            BasicError::InvalidDescriptionLengthError(_)
        );

        // ────────────────────────────────────────────────────────────────────────
        // positive case – old docs removed, new docs inserted
        // ────────────────────────────────────────────────────────────────────────

        #[test]
        fn update_description_replaces_search_docs() {
            let platform_version = PlatformVersion::latest();
            let mut platform = TestPlatformBuilder::new()
                .build_with_mock_rpc()
                .set_genesis_state();

            let (identity, signer, key) = setup_identity(&mut platform, 958, dash_to_credits!(1.0));

            // initial contract with description
            let (cid, _) = create_contract_with_description(
                &mut platform,
                &identity,
                &signer,
                &key,
                "old1",
                platform_version,
            );

            // verify initial docs
            let initial_docs = description_docs_for_contract(&platform, cid, platform_version);
            assert_eq!(initial_docs, "old1".to_string());

            // apply update to "newA"
            apply_description_update(
                &mut platform,
                cid,
                &identity,
                &signer,
                &key,
                "newA",
                platform_version,
            )
            .expect("update should succeed");

            // fetch contract – description updated?
            let fetched = platform
                .drive
                .fetch_contract(cid.into(), None, None, None, platform_version)
                .value
                .unwrap()
                .unwrap();
            assert_eq!(
                fetched.contract.description(),
                Some("newA".to_string()).as_ref()
            );

            // search‑contract docs updated?
            let docs_after = description_docs_for_contract(&platform, cid, platform_version);
            assert_eq!(docs_after, "newA".to_string());
            // old docs gone
            assert!(!docs_after.contains(&"old1".to_string()));
        }
    }
}
