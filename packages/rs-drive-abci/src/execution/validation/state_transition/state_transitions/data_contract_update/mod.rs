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
use crate::platform_types::platform_state::v0::PlatformStateV0Methods;
use crate::rpc::core::CoreRPCLike;

impl StateTransitionActionTransformerV0 for DataContractUpdateTransition {
    fn transform_into_action<C: CoreRPCLike>(
        &self,
        platform: &PlatformRef<C>,
        _block_info: &BlockInfo,
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
            0 => {
                self.transform_into_action_v0(validation_mode, execution_context, platform_version)
            }
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

        signer.add_key(master_key.clone(), master_private_key.clone());

        let (critical_public_key, private_key) =
            IdentityPublicKey::random_ecdsa_critical_level_authentication_key_with_rng(
                1,
                &mut rng,
                platform_version,
            )
            .expect("expected to get key pair");

        signer.add_key(critical_public_key.clone(), private_key.clone());

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

        let (identity, signer, key) = setup_identity(&mut platform, 958, dash_to_credits!(0.1));

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
                &vec![data_contract_update_serialized_transition.clone()],
                &platform_state,
                &BlockInfo::default(),
                &transaction,
                platform_version,
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
}
