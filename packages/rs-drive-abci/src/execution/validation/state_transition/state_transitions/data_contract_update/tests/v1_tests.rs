#[cfg(test)]
mod tests {
    use super::super::{
        apply_contract, setup_identity, setup_test, Identifier, Identity, IdentityPublicKey,
        TestData,
    };
    use crate::test::helpers::setup::{TempPlatform, TestPlatformBuilder};
    use dpp::block::block_info::BlockInfo;
    use dpp::consensus::ConsensusError;
    use dpp::dash_to_credits;
    use dpp::data_contract::accessors::v0::{DataContractV0Getters, DataContractV0Setters};
    use std::collections::BTreeMap;

    use dpp::identity::accessors::IdentityGettersV0;
    use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
    use dpp::platform_value::BinaryData;
    use dpp::serialization::PlatformSerializable;
    use dpp::state_transition::data_contract_update_transition::{
        DataContractUpdateTransition, DataContractUpdateTransitionV1,
    };

    use crate::platform_types::platform_state::PlatformStateV0Methods;
    use crate::platform_types::state_transitions_processing_result::StateTransitionExecutionResult;
    use assert_matches::assert_matches;
    use dpp::consensus::basic::BasicError;
    use dpp::data_contract::accessors::v1::{DataContractV1Getters, DataContractV1Setters};
    use dpp::data_contract::group::v0::GroupV0;
    use dpp::data_contract::group::Group;
    use dpp::identity::signer::Signer;
    use dpp::serialization::Signable;
    use dpp::state_transition::StateTransitionSingleSigned;
    use dpp::tests::fixtures::get_data_contract_fixture;
    use dpp::version::PlatformVersion;
    use drive::util::storage_flags::StorageFlags;
    use simple_signer::signer::SimpleSigner;

    mod group_tests {
        use super::*;
        use crate::platform_types::state_transitions_processing_result::StateTransitionExecutionResult::UnpaidConsensusError;
        use dpp::state_transition::StateTransition;

        #[test]
        fn test_data_contract_update_can_add_new_group_v1() {
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

            // Create V1 update transition to add a new group at position 2
            let new_group = Group::V0(GroupV0 {
                members: [
                    (identity.id(), 1),
                    (identity_2.id(), 2),
                    (identity_3.id(), 2),
                ]
                .into(),
                required_power: 3,
            });

            let mut new_groups = BTreeMap::new();
            new_groups.insert(2, new_group);

            let state_transition = DataContractUpdateTransitionV1 {
                update_contract_system_version: None,
                id: data_contract.id(),
                owner_id: data_contract.owner_id(),
                revision: 2,
                updated_schema_defs: BTreeMap::new(),
                new_schema_defs: BTreeMap::new(),
                updated_document_schemas: BTreeMap::new(),
                new_document_schemas: BTreeMap::new(),
                new_groups,
                new_tokens: BTreeMap::new(),
                remove_keywords: vec![],
                add_keywords: vec![],
                update_description: None,
                identity_contract_nonce: 1,
                user_fee_increase: 0,
                signature_public_key_id: key.id(),
                signature: BinaryData::default(),
            };

            let update_transition: DataContractUpdateTransition = state_transition.into();
            let mut state_transition: StateTransition = update_transition.into();

            // Sign the transition
            let signable_bytes = state_transition
                .signable_bytes()
                .expect("expected signable bytes");
            let signature = signer
                .sign(&key, &signable_bytes)
                .expect("expected to sign");
            state_transition.set_signature(signature);

            let data_contract_update_serialized_transition = state_transition
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
                [StateTransitionExecutionResult::SuccessfulExecution { .. }]
            );

            platform
                .drive
                .grove
                .commit_transaction(transaction)
                .unwrap()
                .expect("expected to commit transaction");
        }

        #[test]
        fn test_data_contract_update_can_not_add_new_group_with_gap_v1() {
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

            // Create an initial data contract with groups at positions 0 and 1
            let mut data_contract =
                get_data_contract_fixture(None, 0, platform_version.protocol_version)
                    .data_contract_owned();

            data_contract.set_owner_id(identity.id());

            {
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
                            (identity_2.id(), 1),
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

            // Try to add a new group at position 3 (gap - should have been position 2)
            let new_group = Group::V0(GroupV0 {
                members: [
                    (identity.id(), 1),
                    (identity_2.id(), 1),
                    (identity_3.id(), 1),
                ]
                .into(),
                required_power: 3,
            });

            let mut new_groups = BTreeMap::new();
            new_groups.insert(3, new_group);

            let state_transition = DataContractUpdateTransitionV1 {
                update_contract_system_version: None,
                id: data_contract.id(),
                owner_id: data_contract.owner_id(),
                revision: 2,
                updated_schema_defs: BTreeMap::new(),
                new_schema_defs: BTreeMap::new(),
                updated_document_schemas: BTreeMap::new(),
                new_document_schemas: BTreeMap::new(),
                new_groups,
                new_tokens: BTreeMap::new(),
                remove_keywords: vec![],
                add_keywords: vec![],
                update_description: None,
                identity_contract_nonce: 1,
                user_fee_increase: 0,
                signature_public_key_id: key.id(),
                signature: BinaryData::default(),
            };

            let update_transition: DataContractUpdateTransition = state_transition.into();
            let mut state_transition: StateTransition = update_transition.into();

            let signable_bytes = state_transition
                .signable_bytes()
                .expect("expected signable bytes");
            let signature = signer
                .sign(&key, &signable_bytes)
                .expect("expected to sign");
            state_transition.set_signature(signature);

            let data_contract_update_serialized_transition = state_transition
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
                [StateTransitionExecutionResult::PaidConsensusError {
                    error: ConsensusError::BasicError(
                        BasicError::NonContiguousContractGroupPositionsError(_)
                    ),
                    ..
                }]
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
        use dpp::state_transition::StateTransition;
        use dpp::data_contract::associated_token::token_configuration::accessors::v0::{TokenConfigurationV0Getters, TokenConfigurationV0Setters};
        use dpp::data_contract::associated_token::token_configuration::v0::TokenConfigurationV0;
        use dpp::data_contract::associated_token::token_configuration::TokenConfiguration;
        use dpp::data_contract::associated_token::token_configuration_convention::v0::TokenConfigurationConventionV0;
        use dpp::data_contract::associated_token::token_configuration_convention::TokenConfigurationConvention;
        use dpp::data_contract::associated_token::token_configuration_localization::v0::TokenConfigurationLocalizationV0;
        use dpp::data_contract::associated_token::token_configuration_localization::TokenConfigurationLocalization;

        #[test]
        fn test_data_contract_update_can_add_new_token_v1() {
            let mut platform = TestPlatformBuilder::new()
                .build_with_mock_rpc()
                .set_initial_state_structure();

            let (identity, signer, key) = setup_identity(&mut platform, 958, dash_to_credits!(1.0));

            let platform_state = platform.state.load();
            let platform_version = platform_state
                .current_platform_version()
                .expect("expected to get current platform version");

            // Create initial contract without tokens
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

            // Create a valid token configuration
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

            let mut new_tokens = BTreeMap::new();
            new_tokens.insert(0, valid_token_cfg);

            let state_transition = DataContractUpdateTransitionV1 {
                update_contract_system_version: None,
                id: data_contract.id(),
                owner_id: data_contract.owner_id(),
                revision: 2,
                updated_schema_defs: BTreeMap::new(),
                new_schema_defs: BTreeMap::new(),
                updated_document_schemas: BTreeMap::new(),
                new_document_schemas: BTreeMap::new(),
                new_groups: BTreeMap::new(),
                new_tokens,
                remove_keywords: vec![],
                add_keywords: vec![],
                update_description: None,
                identity_contract_nonce: 1,
                user_fee_increase: 0,
                signature_public_key_id: key.id(),
                signature: BinaryData::default(),
            };

            let update_transition: DataContractUpdateTransition = state_transition.into();
            let mut state_transition: StateTransition = update_transition.into();

            let signable_bytes = state_transition
                .signable_bytes()
                .expect("expected signable bytes");
            let signature = signer
                .sign(&key, &signable_bytes)
                .expect("expected to sign");
            state_transition.set_signature(signature);

            let tx_bytes = state_transition
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
                [StateTransitionExecutionResult::SuccessfulExecution { .. }]
            );

            platform
                .drive
                .grove
                .commit_transaction(transaction)
                .unwrap()
                .expect("expected to commit transaction");
        }

        #[test]
        fn test_data_contract_update_can_not_add_new_token_with_gap_v1() {
            let mut platform = TestPlatformBuilder::new()
                .build_with_mock_rpc()
                .set_initial_state_structure();

            let (identity, signer, key) = setup_identity(&mut platform, 958, dash_to_credits!(1.0));

            let platform_state = platform.state.load();
            let platform_version = platform_state
                .current_platform_version()
                .expect("expected to get current platform version");

            // Create initial contract with token at position 0
            let mut data_contract =
                get_data_contract_fixture(None, 0, platform_version.protocol_version)
                    .data_contract_owned();
            data_contract.set_owner_id(identity.id());

            let initial_token = {
                let mut cfg =
                    TokenConfiguration::V0(TokenConfigurationV0::default_most_restrictive());
                cfg.set_conventions(TokenConfigurationConvention::V0(
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
                cfg
            };
            data_contract.add_token(0, initial_token);

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

            // Try to add token at position 2 (gap - should be position 1)
            let new_token = {
                let mut cfg =
                    TokenConfiguration::V0(TokenConfigurationV0::default_most_restrictive());
                cfg.set_conventions(TokenConfigurationConvention::V0(
                    TokenConfigurationConventionV0 {
                        localizations: BTreeMap::from([(
                            "en".to_string(),
                            TokenConfigurationLocalization::V0(TokenConfigurationLocalizationV0 {
                                should_capitalize: true,
                                singular_form: "coin".to_string(),
                                plural_form: "coins".to_string(),
                            }),
                        )]),
                        decimals: 8,
                    },
                ));
                cfg
            };

            let mut new_tokens = BTreeMap::new();
            new_tokens.insert(2, new_token); // Position 2 creates a gap

            let state_transition = DataContractUpdateTransitionV1 {
                update_contract_system_version: None,
                id: data_contract.id(),
                owner_id: data_contract.owner_id(),
                revision: 2,
                updated_schema_defs: BTreeMap::new(),
                new_schema_defs: BTreeMap::new(),
                updated_document_schemas: BTreeMap::new(),
                new_document_schemas: BTreeMap::new(),
                new_groups: BTreeMap::new(),
                new_tokens,
                remove_keywords: vec![],
                add_keywords: vec![],
                update_description: None,
                identity_contract_nonce: 1,
                user_fee_increase: 0,
                signature_public_key_id: key.id(),
                signature: BinaryData::default(),
            };

            let update_transition: DataContractUpdateTransition = state_transition.into();
            let mut state_transition: StateTransition = update_transition.into();

            let signable_bytes = state_transition
                .signable_bytes()
                .expect("expected signable bytes");
            let signature = signer
                .sign(&key, &signable_bytes)
                .expect("expected to sign");
            state_transition.set_signature(signature);

            let tx_bytes = state_transition
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
                [StateTransitionExecutionResult::PaidConsensusError {
                    error: ConsensusError::BasicError(
                        BasicError::NonContiguousContractTokenPositionsError(_)
                    ),
                    ..
                }]
            );

            platform
                .drive
                .grove
                .commit_transaction(transaction)
                .unwrap()
                .expect("expected to commit transaction");
        }

        #[test]
        fn test_data_contract_update_can_not_add_new_token_with_large_base_supply_v1() {
            let mut platform = TestPlatformBuilder::new()
                .build_with_mock_rpc()
                .set_initial_state_structure();

            let (identity, signer, key) = setup_identity(&mut platform, 958, dash_to_credits!(1.0));

            let platform_state = platform.state.load();
            let platform_version = platform_state
                .current_platform_version()
                .expect("expected to get current platform version");

            // Create initial contract without tokens
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

            // Try to add token with base_supply > i64::MAX
            let mut huge_supply_cfg =
                TokenConfiguration::V0(TokenConfigurationV0::default_most_restrictive());
            huge_supply_cfg.set_base_supply(i64::MAX as u64 + 1);

            let mut new_tokens = BTreeMap::new();
            new_tokens.insert(0, huge_supply_cfg);

            let state_transition = DataContractUpdateTransitionV1 {
                update_contract_system_version: None,
                id: data_contract.id(),
                owner_id: data_contract.owner_id(),
                revision: 2,
                updated_schema_defs: BTreeMap::new(),
                new_schema_defs: BTreeMap::new(),
                updated_document_schemas: BTreeMap::new(),
                new_document_schemas: BTreeMap::new(),
                new_groups: BTreeMap::new(),
                new_tokens,
                remove_keywords: vec![],
                add_keywords: vec![],
                update_description: None,
                identity_contract_nonce: 1,
                user_fee_increase: 0,
                signature_public_key_id: key.id(),
                signature: BinaryData::default(),
            };

            let update_transition: DataContractUpdateTransition = state_transition.into();
            let mut state_transition: StateTransition = update_transition.into();

            let signable_bytes = state_transition
                .signable_bytes()
                .expect("expected signable bytes");
            let signature = signer
                .sign(&key, &signable_bytes)
                .expect("expected to sign");
            state_transition.set_signature(signature);

            let tx_bytes = state_transition
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
    }

    mod basic_validation_tests {
        use super::*;
        use dpp::consensus::basic::data_contract::{
            DataContractUpdateTransitionConflictingKeywordError,
            DataContractUpdateTransitionOverlappingFieldsError,
        };
        use dpp::platform_value::Value;
        use dpp::state_transition::StateTransition;

        #[test]
        fn test_overlapping_schema_defs_should_fail_v1() {
            let mut platform = TestPlatformBuilder::new()
                .build_with_mock_rpc()
                .set_initial_state_structure();

            let (identity, signer, key) = setup_identity(&mut platform, 958, dash_to_credits!(1.0));

            let platform_state = platform.state.load();
            let platform_version = platform_state
                .current_platform_version()
                .expect("expected to get current platform version");

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

            // Try to have the same key in both updated_schema_defs and new_schema_defs
            let mut updated_schema_defs = BTreeMap::new();
            updated_schema_defs.insert(
                "overlapping".to_string(),
                Value::Text("updated".to_string()),
            );

            let mut new_schema_defs = BTreeMap::new();
            new_schema_defs.insert("overlapping".to_string(), Value::Text("new".to_string()));

            let state_transition = DataContractUpdateTransitionV1 {
                update_contract_system_version: None,
                id: data_contract.id(),
                owner_id: data_contract.owner_id(),
                revision: 2,
                updated_schema_defs,
                new_schema_defs,
                updated_document_schemas: BTreeMap::new(),
                new_document_schemas: BTreeMap::new(),
                new_groups: BTreeMap::new(),
                new_tokens: BTreeMap::new(),
                remove_keywords: vec![],
                add_keywords: vec![],
                update_description: None,
                identity_contract_nonce: 1,
                user_fee_increase: 0,
                signature_public_key_id: key.id(),
                signature: BinaryData::default(),
            };

            let update_transition: DataContractUpdateTransition = state_transition.into();
            let mut state_transition: StateTransition = update_transition.into();

            let signable_bytes = state_transition
                .signable_bytes()
                .expect("expected signable bytes");
            let signature = signer
                .sign(&key, &signable_bytes)
                .expect("expected to sign");
            state_transition.set_signature(signature);

            let tx_bytes = state_transition
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
                [StateTransitionExecutionResult::UnpaidConsensusError(
                    ConsensusError::BasicError(
                        BasicError::DataContractUpdateTransitionOverlappingFieldsError(err)
                    )
                )] if err.field_type() == "schema_defs"
            );

            platform
                .drive
                .grove
                .commit_transaction(transaction)
                .unwrap()
                .expect("expected to commit transaction");
        }

        #[test]
        fn test_overlapping_document_schemas_should_fail_v1() {
            let mut platform = TestPlatformBuilder::new()
                .build_with_mock_rpc()
                .set_initial_state_structure();

            let (identity, signer, key) = setup_identity(&mut platform, 958, dash_to_credits!(1.0));

            let platform_state = platform.state.load();
            let platform_version = platform_state
                .current_platform_version()
                .expect("expected to get current platform version");

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

            // Try to have the same key in both updated_document_schemas and new_document_schemas
            let mut updated_document_schemas = BTreeMap::new();
            updated_document_schemas.insert(
                "overlappingDoc".to_string(),
                Value::Text("updated".to_string()),
            );

            let mut new_document_schemas = BTreeMap::new();
            new_document_schemas
                .insert("overlappingDoc".to_string(), Value::Text("new".to_string()));

            let state_transition = DataContractUpdateTransitionV1 {
                update_contract_system_version: None,
                id: data_contract.id(),
                owner_id: data_contract.owner_id(),
                revision: 2,
                updated_schema_defs: BTreeMap::new(),
                new_schema_defs: BTreeMap::new(),
                updated_document_schemas,
                new_document_schemas,
                new_groups: BTreeMap::new(),
                new_tokens: BTreeMap::new(),
                remove_keywords: vec![],
                add_keywords: vec![],
                update_description: None,
                identity_contract_nonce: 1,
                user_fee_increase: 0,
                signature_public_key_id: key.id(),
                signature: BinaryData::default(),
            };

            let update_transition: DataContractUpdateTransition = state_transition.into();
            let mut state_transition: StateTransition = update_transition.into();

            let signable_bytes = state_transition
                .signable_bytes()
                .expect("expected signable bytes");
            let signature = signer
                .sign(&key, &signable_bytes)
                .expect("expected to sign");
            state_transition.set_signature(signature);

            let tx_bytes = state_transition
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
                [StateTransitionExecutionResult::UnpaidConsensusError(
                    ConsensusError::BasicError(
                        BasicError::DataContractUpdateTransitionOverlappingFieldsError(err)
                    )
                )] if err.field_type() == "document_schemas"
            );

            platform
                .drive
                .grove
                .commit_transaction(transaction)
                .unwrap()
                .expect("expected to commit transaction");
        }

        #[test]
        fn test_conflicting_keywords_should_fail_v1() {
            let mut platform = TestPlatformBuilder::new()
                .build_with_mock_rpc()
                .set_initial_state_structure();

            let (identity, signer, key) = setup_identity(&mut platform, 958, dash_to_credits!(1.0));

            let platform_state = platform.state.load();
            let platform_version = platform_state
                .current_platform_version()
                .expect("expected to get current platform version");

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

            // Try to add and remove the same keyword
            let state_transition = DataContractUpdateTransitionV1 {
                update_contract_system_version: None,
                id: data_contract.id(),
                owner_id: data_contract.owner_id(),
                revision: 2,
                updated_schema_defs: BTreeMap::new(),
                new_schema_defs: BTreeMap::new(),
                updated_document_schemas: BTreeMap::new(),
                new_document_schemas: BTreeMap::new(),
                new_groups: BTreeMap::new(),
                new_tokens: BTreeMap::new(),
                remove_keywords: vec!["conflicting".to_string()],
                add_keywords: vec!["conflicting".to_string()],
                update_description: None,
                identity_contract_nonce: 1,
                user_fee_increase: 0,
                signature_public_key_id: key.id(),
                signature: BinaryData::default(),
            };

            let update_transition: DataContractUpdateTransition = state_transition.into();
            let mut state_transition: StateTransition = update_transition.into();

            let signable_bytes = state_transition
                .signable_bytes()
                .expect("expected signable bytes");
            let signature = signer
                .sign(&key, &signable_bytes)
                .expect("expected to sign");
            state_transition.set_signature(signature);

            let tx_bytes = state_transition
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
                [StateTransitionExecutionResult::UnpaidConsensusError(
                    ConsensusError::BasicError(
                        BasicError::DataContractUpdateTransitionConflictingKeywordError(err)
                    )
                )] if err.keyword() == "conflicting"
            );

            platform
                .drive
                .grove
                .commit_transaction(transaction)
                .unwrap()
                .expect("expected to commit transaction");
        }

        #[test]
        fn test_description_too_short_should_fail_v1() {
            let mut platform = TestPlatformBuilder::new()
                .build_with_mock_rpc()
                .set_initial_state_structure();

            let (identity, signer, key) = setup_identity(&mut platform, 958, dash_to_credits!(1.0));

            let platform_state = platform.state.load();
            let platform_version = platform_state
                .current_platform_version()
                .expect("expected to get current platform version");

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

            // Try to set a description that is too short (< 3 chars)
            let state_transition = DataContractUpdateTransitionV1 {
                update_contract_system_version: None,
                id: data_contract.id(),
                owner_id: data_contract.owner_id(),
                revision: 2,
                updated_schema_defs: BTreeMap::new(),
                new_schema_defs: BTreeMap::new(),
                updated_document_schemas: BTreeMap::new(),
                new_document_schemas: BTreeMap::new(),
                new_groups: BTreeMap::new(),
                new_tokens: BTreeMap::new(),
                remove_keywords: vec![],
                add_keywords: vec![],
                update_description: Some(Some("ab".to_string())), // 2 chars, too short
                identity_contract_nonce: 1,
                user_fee_increase: 0,
                signature_public_key_id: key.id(),
                signature: BinaryData::default(),
            };

            let update_transition: DataContractUpdateTransition = state_transition.into();
            let mut state_transition: StateTransition = update_transition.into();

            let signable_bytes = state_transition
                .signable_bytes()
                .expect("expected signable bytes");
            let signature = signer
                .sign(&key, &signable_bytes)
                .expect("expected to sign");
            state_transition.set_signature(signature);

            let tx_bytes = state_transition
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
                [StateTransitionExecutionResult::UnpaidConsensusError(
                    ConsensusError::BasicError(BasicError::InvalidDescriptionLengthError(_))
                )]
            );

            platform
                .drive
                .grove
                .commit_transaction(transaction)
                .unwrap()
                .expect("expected to commit transaction");
        }

        #[test]
        fn test_description_too_long_should_fail_v1() {
            let mut platform = TestPlatformBuilder::new()
                .build_with_mock_rpc()
                .set_initial_state_structure();

            let (identity, signer, key) = setup_identity(&mut platform, 958, dash_to_credits!(1.0));

            let platform_state = platform.state.load();
            let platform_version = platform_state
                .current_platform_version()
                .expect("expected to get current platform version");

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

            // Try to set a description that is too long (> 100 chars)
            let state_transition = DataContractUpdateTransitionV1 {
                update_contract_system_version: None,
                id: data_contract.id(),
                owner_id: data_contract.owner_id(),
                revision: 2,
                updated_schema_defs: BTreeMap::new(),
                new_schema_defs: BTreeMap::new(),
                updated_document_schemas: BTreeMap::new(),
                new_document_schemas: BTreeMap::new(),
                new_groups: BTreeMap::new(),
                new_tokens: BTreeMap::new(),
                remove_keywords: vec![],
                add_keywords: vec![],
                update_description: Some(Some("x".repeat(101))), // 101 chars, too long
                identity_contract_nonce: 1,
                user_fee_increase: 0,
                signature_public_key_id: key.id(),
                signature: BinaryData::default(),
            };

            let update_transition: DataContractUpdateTransition = state_transition.into();
            let mut state_transition: StateTransition = update_transition.into();

            let signable_bytes = state_transition
                .signable_bytes()
                .expect("expected signable bytes");
            let signature = signer
                .sign(&key, &signable_bytes)
                .expect("expected to sign");
            state_transition.set_signature(signature);

            let tx_bytes = state_transition
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
                [StateTransitionExecutionResult::UnpaidConsensusError(
                    ConsensusError::BasicError(BasicError::InvalidDescriptionLengthError(_))
                )]
            );

            platform
                .drive
                .grove
                .commit_transaction(transaction)
                .unwrap()
                .expect("expected to commit transaction");
        }
    }
}
