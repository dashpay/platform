mod direct_selling;
mod distribution;

use super::*;
use crate::execution::validation::state_transition::tests::create_token_contract_with_owner_identity;
use crate::platform_types::state_transitions_processing_result::StateTransitionExecutionResult;
use crate::test::helpers::setup::TestPlatformBuilder;
use dpp::block::block_info::BlockInfo;
use dpp::consensus::basic::BasicError;
use dpp::consensus::state::state_error::StateError;
use dpp::consensus::ConsensusError;
use dpp::dash_to_credits;
use dpp::data_contract::associated_token::token_configuration::accessors::v0::TokenConfigurationV0Getters;
use dpp::data_contract::associated_token::token_configuration::accessors::v0::TokenConfigurationV0Setters;
use dpp::data_contract::associated_token::token_configuration::TokenConfiguration;
use dpp::data_contract::associated_token::token_configuration_convention::v0::TokenConfigurationConventionV0;
use dpp::data_contract::associated_token::token_distribution_rules::accessors::v0::TokenDistributionRulesV0Setters;
use dpp::data_contract::change_control_rules::authorized_action_takers::AuthorizedActionTakers;
use dpp::data_contract::change_control_rules::v0::ChangeControlRulesV0;
use dpp::data_contract::change_control_rules::ChangeControlRules;
use dpp::data_contract::group::v0::GroupV0;
use dpp::data_contract::group::Group;
use dpp::group::GroupStateTransitionInfo;
use dpp::group::GroupStateTransitionInfoStatus;
use dpp::identifier::Identifier;
use dpp::state_transition::batch_transition::methods::v1::DocumentsBatchTransitionMethodsV1;
use dpp::state_transition::batch_transition::BatchTransition;
use dpp::state_transition::batch_transition::TokenConfigUpdateTransition;
use rand::prelude::StdRng;

mod token_tests {
    use super::*;

    mod token_mint_tests {
        use super::*;

        mod token_mint_tests_normal_scenarios {
            use super::*;

            #[test]
            fn test_token_mint_by_owner_allowed_sending_to_self() {
                let platform_version = PlatformVersion::latest();
                let mut platform = TestPlatformBuilder::new()
                    .with_latest_protocol_version()
                    .build_with_mock_rpc()
                    .set_genesis_state();

                let mut rng = StdRng::seed_from_u64(49853);

                let platform_state = platform.state.load();

                let (identity, signer, key) =
                    setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));

                let (contract, token_id) = create_token_contract_with_owner_identity(
                    &mut platform,
                    identity.id(),
                    None::<fn(&mut TokenConfiguration)>,
                    None,
                    None,
                    platform_version,
                );

                let documents_batch_create_transition = BatchTransition::new_token_mint_transition(
                    token_id,
                    identity.id(),
                    contract.id(),
                    0,
                    1337,
                    Some(identity.id()),
                    None,
                    None,
                    &key,
                    2,
                    0,
                    &signer,
                    platform_version,
                    None,
                    None,
                    None,
                )
                .expect("expect to create documents batch transition");

                let documents_batch_create_serialized_transition =
                    documents_batch_create_transition
                        .serialize_to_bytes()
                        .expect("expected documents batch serialized state transition");

                let transaction = platform.drive.grove.start_transaction();

                let processing_result = platform
                    .platform
                    .process_raw_state_transitions(
                        &vec![documents_batch_create_serialized_transition.clone()],
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

                let token_balance = platform
                    .drive
                    .fetch_identity_token_balance(
                        token_id.to_buffer(),
                        identity.id().to_buffer(),
                        None,
                        platform_version,
                    )
                    .expect("expected to fetch token balance");
                assert_eq!(token_balance, Some(101337));
            }

            #[test]
            fn test_token_mint_by_owner_can_not_mint_past_max_supply() {
                let platform_version = PlatformVersion::latest();
                let mut platform = TestPlatformBuilder::new()
                    .with_latest_protocol_version()
                    .build_with_mock_rpc()
                    .set_genesis_state();

                let mut rng = StdRng::seed_from_u64(49853);

                let platform_state = platform.state.load();

                let (identity, signer, key) =
                    setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));

                let (contract, token_id) = create_token_contract_with_owner_identity(
                    &mut platform,
                    identity.id(),
                    Some(|token_configuration: &mut TokenConfiguration| {
                        token_configuration.set_max_supply(Some(1000000));
                    }),
                    None,
                    None,
                    platform_version,
                );

                let documents_batch_create_transition = BatchTransition::new_token_mint_transition(
                    token_id,
                    identity.id(),
                    contract.id(),
                    0,
                    2000000,
                    Some(identity.id()),
                    None,
                    None,
                    &key,
                    2,
                    0,
                    &signer,
                    platform_version,
                    None,
                    None,
                    None,
                )
                .expect("expect to create documents batch transition");

                let documents_batch_create_serialized_transition =
                    documents_batch_create_transition
                        .serialize_to_bytes()
                        .expect("expected documents batch serialized state transition");

                let transaction = platform.drive.grove.start_transaction();

                let processing_result = platform
                    .platform
                    .process_raw_state_transitions(
                        &vec![documents_batch_create_serialized_transition.clone()],
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
                    [StateTransitionExecutionResult::PaidConsensusError(
                        ConsensusError::StateError(StateError::TokenMintPastMaxSupplyError(_)),
                        _
                    )]
                );

                platform
                    .drive
                    .grove
                    .commit_transaction(transaction)
                    .unwrap()
                    .expect("expected to commit transaction");

                let token_balance = platform
                    .drive
                    .fetch_identity_token_balance(
                        token_id.to_buffer(),
                        identity.id().to_buffer(),
                        None,
                        platform_version,
                    )
                    .expect("expected to fetch token balance");
                assert_eq!(token_balance, Some(100000));
            }

            #[test]
            fn test_token_mint_by_owner_allowed_sending_to_other() {
                let platform_version = PlatformVersion::latest();
                let mut platform = TestPlatformBuilder::new()
                    .with_latest_protocol_version()
                    .build_with_mock_rpc()
                    .set_genesis_state();

                let mut rng = StdRng::seed_from_u64(49853);

                let platform_state = platform.state.load();

                let (identity, signer, key) =
                    setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));

                let (receiver, _, _) =
                    setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));

                let (contract, token_id) = create_token_contract_with_owner_identity(
                    &mut platform,
                    identity.id(),
                    None::<fn(&mut TokenConfiguration)>,
                    None,
                    None,
                    platform_version,
                );

                let documents_batch_create_transition = BatchTransition::new_token_mint_transition(
                    token_id,
                    identity.id(),
                    contract.id(),
                    0,
                    1337,
                    Some(receiver.id()),
                    None,
                    None,
                    &key,
                    2,
                    0,
                    &signer,
                    platform_version,
                    None,
                    None,
                    None,
                )
                .expect("expect to create documents batch transition");

                let documents_batch_create_serialized_transition =
                    documents_batch_create_transition
                        .serialize_to_bytes()
                        .expect("expected documents batch serialized state transition");

                let transaction = platform.drive.grove.start_transaction();

                let processing_result = platform
                    .platform
                    .process_raw_state_transitions(
                        &vec![documents_batch_create_serialized_transition.clone()],
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

                let token_balance = platform
                    .drive
                    .fetch_identity_token_balance(
                        token_id.to_buffer(),
                        receiver.id().to_buffer(),
                        None,
                        platform_version,
                    )
                    .expect("expected to fetch token balance");
                assert_eq!(token_balance, Some(1337));
            }

            #[test]
            fn test_token_mint_sending_to_non_existing_identity_causes_error() {
                let platform_version = PlatformVersion::latest();
                let mut platform = TestPlatformBuilder::new()
                    .with_latest_protocol_version()
                    .build_with_mock_rpc()
                    .set_genesis_state();

                let mut rng = StdRng::seed_from_u64(49853);

                let platform_state = platform.state.load();

                let (identity, signer, key) =
                    setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));

                let receiver = Identifier::random_with_rng(&mut rng);

                let (contract, token_id) = create_token_contract_with_owner_identity(
                    &mut platform,
                    identity.id(),
                    None::<fn(&mut TokenConfiguration)>,
                    None,
                    None,
                    platform_version,
                );

                let documents_batch_create_transition = BatchTransition::new_token_mint_transition(
                    token_id,
                    identity.id(),
                    contract.id(),
                    0,
                    1337,
                    Some(receiver),
                    None,
                    None,
                    &key,
                    2,
                    0,
                    &signer,
                    platform_version,
                    None,
                    None,
                    None,
                )
                .expect("expect to create documents batch transition");

                let documents_batch_create_serialized_transition =
                    documents_batch_create_transition
                        .serialize_to_bytes()
                        .expect("expected documents batch serialized state transition");

                let transaction = platform.drive.grove.start_transaction();

                let processing_result = platform
                    .platform
                    .process_raw_state_transitions(
                        &vec![documents_batch_create_serialized_transition.clone()],
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
                    [StateTransitionExecutionResult::PaidConsensusError(
                        ConsensusError::StateError(StateError::RecipientIdentityDoesNotExistError(
                            _
                        )),
                        _
                    )]
                );

                platform
                    .drive
                    .grove
                    .commit_transaction(transaction)
                    .unwrap()
                    .expect("expected to commit transaction");

                let token_balance = platform
                    .drive
                    .fetch_identity_token_balance(
                        token_id.to_buffer(),
                        receiver.to_buffer(),
                        None,
                        platform_version,
                    )
                    .expect("expected to fetch token balance");
                assert_eq!(token_balance, None);
            }

            #[test]
            fn test_token_mint_by_owner_no_destination_causes_error() {
                let platform_version = PlatformVersion::latest();
                let mut platform = TestPlatformBuilder::new()
                    .with_latest_protocol_version()
                    .build_with_mock_rpc()
                    .set_genesis_state();

                let mut rng = StdRng::seed_from_u64(49853);

                let platform_state = platform.state.load();

                let (identity, signer, key) =
                    setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));

                let (contract, token_id) = create_token_contract_with_owner_identity(
                    &mut platform,
                    identity.id(),
                    None::<fn(&mut TokenConfiguration)>,
                    None,
                    None,
                    platform_version,
                );

                let documents_batch_create_transition = BatchTransition::new_token_mint_transition(
                    token_id,
                    identity.id(),
                    contract.id(),
                    0,
                    1337,
                    None,
                    None,
                    None,
                    &key,
                    2,
                    0,
                    &signer,
                    platform_version,
                    None,
                    None,
                    None,
                )
                .expect("expect to create documents batch transition");

                let documents_batch_create_serialized_transition =
                    documents_batch_create_transition
                        .serialize_to_bytes()
                        .expect("expected documents batch serialized state transition");

                let transaction = platform.drive.grove.start_transaction();

                let processing_result = platform
                    .platform
                    .process_raw_state_transitions(
                        &vec![documents_batch_create_serialized_transition.clone()],
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
                    [StateTransitionExecutionResult::PaidConsensusError(
                        ConsensusError::BasicError(
                            BasicError::DestinationIdentityForTokenMintingNotSetError(_)
                        ),
                        _
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

        mod token_mint_tests_no_recipient_minting {
            use super::*;

            #[test]
            fn test_token_mint_by_owned_id_allowed_sending_to_self() {
                let platform_version = PlatformVersion::latest();
                let mut platform = TestPlatformBuilder::new()
                    .with_latest_protocol_version()
                    .build_with_mock_rpc()
                    .set_genesis_state();

                let mut rng = StdRng::seed_from_u64(49853);

                let platform_state = platform.state.load();

                let (identity, signer, key) =
                    setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));

                let (contract, token_id) = create_token_contract_with_owner_identity(
                    &mut platform,
                    identity.id(),
                    Some(|token_configuration: &mut TokenConfiguration| {
                        token_configuration
                            .distribution_rules_mut()
                            .set_minting_allow_choosing_destination(false);
                    }),
                    None,
                    None,
                    platform_version,
                );

                let documents_batch_create_transition = BatchTransition::new_token_mint_transition(
                    token_id,
                    identity.id(),
                    contract.id(),
                    0,
                    1337,
                    Some(identity.id()),
                    None,
                    None,
                    &key,
                    2,
                    0,
                    &signer,
                    platform_version,
                    None,
                    None,
                    None,
                )
                .expect("expect to create documents batch transition");

                let documents_batch_create_serialized_transition =
                    documents_batch_create_transition
                        .serialize_to_bytes()
                        .expect("expected documents batch serialized state transition");

                let transaction = platform.drive.grove.start_transaction();

                let processing_result = platform
                    .platform
                    .process_raw_state_transitions(
                        &vec![documents_batch_create_serialized_transition.clone()],
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
                    [StateTransitionExecutionResult::PaidConsensusError(
                        ConsensusError::BasicError(
                            BasicError::ChoosingTokenMintRecipientNotAllowedError(_)
                        ),
                        _
                    )]
                );

                platform
                    .drive
                    .grove
                    .commit_transaction(transaction)
                    .unwrap()
                    .expect("expected to commit transaction");

                let token_balance = platform
                    .drive
                    .fetch_identity_token_balance(
                        token_id.to_buffer(),
                        identity.id().to_buffer(),
                        None,
                        platform_version,
                    )
                    .expect("expected to fetch token balance");
                assert_eq!(token_balance, Some(100000));
            }

            #[test]
            fn test_token_mint_by_owned_id_allowed_sending_to_other() {
                let platform_version = PlatformVersion::latest();
                let mut platform = TestPlatformBuilder::new()
                    .with_latest_protocol_version()
                    .build_with_mock_rpc()
                    .set_genesis_state();

                let mut rng = StdRng::seed_from_u64(49853);

                let platform_state = platform.state.load();

                let (identity, signer, key) =
                    setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));

                let (receiver, _, _) =
                    setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));

                let (contract, token_id) = create_token_contract_with_owner_identity(
                    &mut platform,
                    identity.id(),
                    Some(|token_configuration: &mut TokenConfiguration| {
                        token_configuration
                            .distribution_rules_mut()
                            .set_minting_allow_choosing_destination(false);
                    }),
                    None,
                    None,
                    platform_version,
                );

                let documents_batch_create_transition = BatchTransition::new_token_mint_transition(
                    token_id,
                    identity.id(),
                    contract.id(),
                    0,
                    1337,
                    Some(receiver.id()),
                    None,
                    None,
                    &key,
                    2,
                    0,
                    &signer,
                    platform_version,
                    None,
                    None,
                    None,
                )
                .expect("expect to create documents batch transition");

                let documents_batch_create_serialized_transition =
                    documents_batch_create_transition
                        .serialize_to_bytes()
                        .expect("expected documents batch serialized state transition");

                let transaction = platform.drive.grove.start_transaction();

                let processing_result = platform
                    .platform
                    .process_raw_state_transitions(
                        &vec![documents_batch_create_serialized_transition.clone()],
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
                    [StateTransitionExecutionResult::PaidConsensusError(
                        ConsensusError::BasicError(
                            BasicError::ChoosingTokenMintRecipientNotAllowedError(_)
                        ),
                        _
                    )]
                );

                platform
                    .drive
                    .grove
                    .commit_transaction(transaction)
                    .unwrap()
                    .expect("expected to commit transaction");

                let token_balance = platform
                    .drive
                    .fetch_identity_token_balance(
                        token_id.to_buffer(),
                        receiver.id().to_buffer(),
                        None,
                        platform_version,
                    )
                    .expect("expected to fetch token balance");
                assert_eq!(token_balance, None);
            }

            #[test]
            fn test_token_mint_by_owned_id_no_destination_causes_error() {
                let platform_version = PlatformVersion::latest();
                let mut platform = TestPlatformBuilder::new()
                    .with_latest_protocol_version()
                    .build_with_mock_rpc()
                    .set_genesis_state();

                let mut rng = StdRng::seed_from_u64(49853);

                let platform_state = platform.state.load();

                let (identity, signer, key) =
                    setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));

                let (contract, token_id) = create_token_contract_with_owner_identity(
                    &mut platform,
                    identity.id(),
                    Some(|token_configuration: &mut TokenConfiguration| {
                        token_configuration
                            .distribution_rules_mut()
                            .set_minting_allow_choosing_destination(false);
                    }),
                    None,
                    None,
                    platform_version,
                );

                let documents_batch_create_transition = BatchTransition::new_token_mint_transition(
                    token_id,
                    identity.id(),
                    contract.id(),
                    0,
                    1337,
                    None,
                    None,
                    None,
                    &key,
                    2,
                    0,
                    &signer,
                    platform_version,
                    None,
                    None,
                    None,
                )
                .expect("expect to create documents batch transition");

                let documents_batch_create_serialized_transition =
                    documents_batch_create_transition
                        .serialize_to_bytes()
                        .expect("expected documents batch serialized state transition");

                let transaction = platform.drive.grove.start_transaction();

                let processing_result = platform
                    .platform
                    .process_raw_state_transitions(
                        &vec![documents_batch_create_serialized_transition.clone()],
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
                    [StateTransitionExecutionResult::PaidConsensusError(
                        ConsensusError::BasicError(
                            BasicError::DestinationIdentityForTokenMintingNotSetError(_)
                        ),
                        _
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

        mod token_mint_tests_contract_has_recipient {
            use super::*;

            #[test]
            fn test_token_mint_by_owned_id_allowed_sending_to_self() {
                let platform_version = PlatformVersion::latest();
                let mut platform = TestPlatformBuilder::new()
                    .with_latest_protocol_version()
                    .build_with_mock_rpc()
                    .set_genesis_state();

                let mut rng = StdRng::seed_from_u64(49853);

                let platform_state = platform.state.load();

                let (identity, signer, key) =
                    setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));

                let (contract, token_id) = create_token_contract_with_owner_identity(
                    &mut platform,
                    identity.id(),
                    Some(|token_configuration: &mut TokenConfiguration| {
                        token_configuration
                            .distribution_rules_mut()
                            .set_minting_allow_choosing_destination(false);
                        token_configuration
                            .distribution_rules_mut()
                            .set_new_tokens_destination_identity(Some(identity.id()));
                    }),
                    None,
                    None,
                    platform_version,
                );

                let documents_batch_create_transition = BatchTransition::new_token_mint_transition(
                    token_id,
                    identity.id(),
                    contract.id(),
                    0,
                    1337,
                    Some(identity.id()),
                    None,
                    None,
                    &key,
                    2,
                    0,
                    &signer,
                    platform_version,
                    None,
                    None,
                    None,
                )
                .expect("expect to create documents batch transition");

                let documents_batch_create_serialized_transition =
                    documents_batch_create_transition
                        .serialize_to_bytes()
                        .expect("expected documents batch serialized state transition");

                let transaction = platform.drive.grove.start_transaction();

                let processing_result = platform
                    .platform
                    .process_raw_state_transitions(
                        &vec![documents_batch_create_serialized_transition.clone()],
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
                    [StateTransitionExecutionResult::PaidConsensusError(
                        ConsensusError::BasicError(
                            BasicError::ChoosingTokenMintRecipientNotAllowedError(_)
                        ),
                        _
                    )]
                );

                platform
                    .drive
                    .grove
                    .commit_transaction(transaction)
                    .unwrap()
                    .expect("expected to commit transaction");

                let token_balance = platform
                    .drive
                    .fetch_identity_token_balance(
                        token_id.to_buffer(),
                        identity.id().to_buffer(),
                        None,
                        platform_version,
                    )
                    .expect("expected to fetch token balance");
                assert_eq!(token_balance, Some(100000));
            }

            #[test]
            fn test_token_mint_by_owned_id_allowed_sending_to_other() {
                let platform_version = PlatformVersion::latest();
                let mut platform = TestPlatformBuilder::new()
                    .with_latest_protocol_version()
                    .build_with_mock_rpc()
                    .set_genesis_state();

                let mut rng = StdRng::seed_from_u64(49853);

                let platform_state = platform.state.load();

                let (identity, signer, key) =
                    setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));

                let (receiver, _, _) =
                    setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));

                let (contract, token_id) = create_token_contract_with_owner_identity(
                    &mut platform,
                    identity.id(),
                    Some(|token_configuration: &mut TokenConfiguration| {
                        token_configuration
                            .distribution_rules_mut()
                            .set_minting_allow_choosing_destination(false);
                        token_configuration
                            .distribution_rules_mut()
                            .set_new_tokens_destination_identity(Some(identity.id()));
                    }),
                    None,
                    None,
                    platform_version,
                );

                let documents_batch_create_transition = BatchTransition::new_token_mint_transition(
                    token_id,
                    identity.id(),
                    contract.id(),
                    0,
                    1337,
                    Some(receiver.id()),
                    None,
                    None,
                    &key,
                    2,
                    0,
                    &signer,
                    platform_version,
                    None,
                    None,
                    None,
                )
                .expect("expect to create documents batch transition");

                let documents_batch_create_serialized_transition =
                    documents_batch_create_transition
                        .serialize_to_bytes()
                        .expect("expected documents batch serialized state transition");

                let transaction = platform.drive.grove.start_transaction();

                let processing_result = platform
                    .platform
                    .process_raw_state_transitions(
                        &vec![documents_batch_create_serialized_transition.clone()],
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
                    [StateTransitionExecutionResult::PaidConsensusError(
                        ConsensusError::BasicError(
                            BasicError::ChoosingTokenMintRecipientNotAllowedError(_)
                        ),
                        _
                    )]
                );

                platform
                    .drive
                    .grove
                    .commit_transaction(transaction)
                    .unwrap()
                    .expect("expected to commit transaction");

                let token_balance = platform
                    .drive
                    .fetch_identity_token_balance(
                        token_id.to_buffer(),
                        receiver.id().to_buffer(),
                        None,
                        platform_version,
                    )
                    .expect("expected to fetch token balance");
                assert_eq!(token_balance, None);
            }

            #[test]
            fn test_token_mint_by_owned_id_no_set_destination_should_use_contracts() {
                let platform_version = PlatformVersion::latest();
                let mut platform = TestPlatformBuilder::new()
                    .with_latest_protocol_version()
                    .build_with_mock_rpc()
                    .set_genesis_state();

                let mut rng = StdRng::seed_from_u64(49853);

                let platform_state = platform.state.load();

                let (identity, signer, key) =
                    setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));

                let (contract, token_id) = create_token_contract_with_owner_identity(
                    &mut platform,
                    identity.id(),
                    Some(|token_configuration: &mut TokenConfiguration| {
                        token_configuration
                            .distribution_rules_mut()
                            .set_minting_allow_choosing_destination(false);
                        token_configuration
                            .distribution_rules_mut()
                            .set_new_tokens_destination_identity(Some(identity.id()));
                    }),
                    None,
                    None,
                    platform_version,
                );

                let documents_batch_create_transition = BatchTransition::new_token_mint_transition(
                    token_id,
                    identity.id(),
                    contract.id(),
                    0,
                    1337,
                    None,
                    None,
                    None,
                    &key,
                    2,
                    0,
                    &signer,
                    platform_version,
                    None,
                    None,
                    None,
                )
                .expect("expect to create documents batch transition");

                let documents_batch_create_serialized_transition =
                    documents_batch_create_transition
                        .serialize_to_bytes()
                        .expect("expected documents batch serialized state transition");

                let transaction = platform.drive.grove.start_transaction();

                let processing_result = platform
                    .platform
                    .process_raw_state_transitions(
                        &vec![documents_batch_create_serialized_transition.clone()],
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

                let token_balance = platform
                    .drive
                    .fetch_identity_token_balance(
                        token_id.to_buffer(),
                        identity.id().to_buffer(),
                        None,
                        platform_version,
                    )
                    .expect("expected to fetch token balance");
                assert_eq!(token_balance, Some(101337));
            }
        }

        mod token_mint_tests_authorization_scenarios {
            use super::*;
            use dpp::data_contract::change_control_rules::authorized_action_takers::AuthorizedActionTakers;
            use dpp::data_contract::change_control_rules::v0::ChangeControlRulesV0;
            use dpp::data_contract::change_control_rules::ChangeControlRules;
            use dpp::data_contract::group::v0::GroupV0;
            use dpp::data_contract::group::Group;
            use dpp::group::{GroupStateTransitionInfo, GroupStateTransitionInfoStatus};
            use dpp::state_transition::batch_transition::TokenMintTransition;

            #[test]
            fn test_token_mint_by_owner_sending_to_self_minting_not_allowed() {
                let platform_version = PlatformVersion::latest();
                let mut platform = TestPlatformBuilder::new()
                    .with_latest_protocol_version()
                    .build_with_mock_rpc()
                    .set_genesis_state();

                let mut rng = StdRng::seed_from_u64(49853);

                let platform_state = platform.state.load();

                let (identity, signer, key) =
                    setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));

                let (contract, token_id) = create_token_contract_with_owner_identity(
                    &mut platform,
                    identity.id(),
                    Some(|token_configuration: &mut TokenConfiguration| {
                        token_configuration.set_manual_minting_rules(ChangeControlRules::V0(
                            ChangeControlRulesV0 {
                                authorized_to_make_change: AuthorizedActionTakers::NoOne,
                                admin_action_takers: AuthorizedActionTakers::NoOne,
                                changing_authorized_action_takers_to_no_one_allowed: false,
                                changing_admin_action_takers_to_no_one_allowed: false,
                                self_changing_admin_action_takers_allowed: false,
                            },
                        ));
                    }),
                    None,
                    None,
                    platform_version,
                );

                let documents_batch_create_transition = BatchTransition::new_token_mint_transition(
                    token_id,
                    identity.id(),
                    contract.id(),
                    0,
                    1337,
                    Some(identity.id()),
                    None,
                    None,
                    &key,
                    2,
                    0,
                    &signer,
                    platform_version,
                    None,
                    None,
                    None,
                )
                .expect("expect to create documents batch transition");

                let documents_batch_create_serialized_transition =
                    documents_batch_create_transition
                        .serialize_to_bytes()
                        .expect("expected documents batch serialized state transition");

                let transaction = platform.drive.grove.start_transaction();

                let processing_result = platform
                    .platform
                    .process_raw_state_transitions(
                        &vec![documents_batch_create_serialized_transition.clone()],
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
                    [StateTransitionExecutionResult::PaidConsensusError(
                        ConsensusError::StateError(StateError::UnauthorizedTokenActionError(_)),
                        _
                    )]
                );

                platform
                    .drive
                    .grove
                    .commit_transaction(transaction)
                    .unwrap()
                    .expect("expected to commit transaction");

                let token_balance = platform
                    .drive
                    .fetch_identity_token_balance(
                        token_id.to_buffer(),
                        identity.id().to_buffer(),
                        None,
                        platform_version,
                    )
                    .expect("expected to fetch token balance");
                assert_eq!(token_balance, Some(100000));
            }

            #[test]
            fn test_token_mint_by_owner_sending_to_self_minting_only_allowed_by_group() {
                let platform_version = PlatformVersion::latest();
                let mut platform = TestPlatformBuilder::new()
                    .with_latest_protocol_version()
                    .build_with_mock_rpc()
                    .set_genesis_state();

                let mut rng = StdRng::seed_from_u64(49853);

                let platform_state = platform.state.load();

                let (identity, signer, key) =
                    setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));

                let (identity_2, _, _) =
                    setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));

                let (contract, token_id) = create_token_contract_with_owner_identity(
                    &mut platform,
                    identity.id(),
                    Some(|token_configuration: &mut TokenConfiguration| {
                        token_configuration.set_manual_minting_rules(ChangeControlRules::V0(
                            ChangeControlRulesV0 {
                                authorized_to_make_change: AuthorizedActionTakers::Group(0),
                                admin_action_takers: AuthorizedActionTakers::NoOne,
                                changing_authorized_action_takers_to_no_one_allowed: false,
                                changing_admin_action_takers_to_no_one_allowed: false,
                                self_changing_admin_action_takers_allowed: false,
                            },
                        ));
                    }),
                    None,
                    Some(
                        [(
                            0,
                            Group::V0(GroupV0 {
                                members: [(identity.id(), 5), (identity_2.id(), 5)].into(),
                                required_power: 10,
                            }),
                        )]
                        .into(),
                    ),
                    platform_version,
                );

                let documents_batch_create_transition = BatchTransition::new_token_mint_transition(
                    token_id,
                    identity.id(),
                    contract.id(),
                    0,
                    1337,
                    Some(identity.id()),
                    None,
                    None,
                    &key,
                    2,
                    0,
                    &signer,
                    platform_version,
                    None,
                    None,
                    None,
                )
                .expect("expect to create documents batch transition");

                let documents_batch_create_serialized_transition =
                    documents_batch_create_transition
                        .serialize_to_bytes()
                        .expect("expected documents batch serialized state transition");

                let transaction = platform.drive.grove.start_transaction();

                let processing_result = platform
                    .platform
                    .process_raw_state_transitions(
                        &vec![documents_batch_create_serialized_transition.clone()],
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
                    [StateTransitionExecutionResult::PaidConsensusError(
                        ConsensusError::StateError(StateError::UnauthorizedTokenActionError(_)),
                        _
                    )]
                );

                platform
                    .drive
                    .grove
                    .commit_transaction(transaction)
                    .unwrap()
                    .expect("expected to commit transaction");

                let token_balance = platform
                    .drive
                    .fetch_identity_token_balance(
                        token_id.to_buffer(),
                        identity.id().to_buffer(),
                        None,
                        platform_version,
                    )
                    .expect("expected to fetch token balance");
                assert_eq!(token_balance, Some(100000));
            }

            #[test]
            fn test_token_mint_by_owner_sending_to_self_minting_only_allowed_by_group_enough_member_power(
            ) {
                // We are using a group, but our member alone has enough power in the group to do the action
                let platform_version = PlatformVersion::latest();
                let mut platform = TestPlatformBuilder::new()
                    .with_latest_protocol_version()
                    .build_with_mock_rpc()
                    .set_genesis_state();

                let mut rng = StdRng::seed_from_u64(49853);

                let platform_state = platform.state.load();

                let (identity, signer, key) =
                    setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));

                let (identity_2, _, _) =
                    setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));

                let (contract, token_id) = create_token_contract_with_owner_identity(
                    &mut platform,
                    identity.id(),
                    Some(|token_configuration: &mut TokenConfiguration| {
                        token_configuration.set_manual_minting_rules(ChangeControlRules::V0(
                            ChangeControlRulesV0 {
                                authorized_to_make_change: AuthorizedActionTakers::Group(0),
                                admin_action_takers: AuthorizedActionTakers::NoOne,
                                changing_authorized_action_takers_to_no_one_allowed: false,
                                changing_admin_action_takers_to_no_one_allowed: false,
                                self_changing_admin_action_takers_allowed: false,
                            },
                        ));
                    }),
                    None,
                    Some(
                        [(
                            0,
                            Group::V0(GroupV0 {
                                members: [(identity.id(), 5), (identity_2.id(), 1)].into(),
                                required_power: 5,
                            }),
                        )]
                        .into(),
                    ),
                    platform_version,
                );

                let documents_batch_create_transition = BatchTransition::new_token_mint_transition(
                    token_id,
                    identity.id(),
                    contract.id(),
                    0,
                    1337,
                    Some(identity.id()),
                    None,
                    None,
                    &key,
                    2,
                    0,
                    &signer,
                    platform_version,
                    None,
                    None,
                    None,
                )
                .expect("expect to create documents batch transition");

                let documents_batch_create_serialized_transition =
                    documents_batch_create_transition
                        .serialize_to_bytes()
                        .expect("expected documents batch serialized state transition");

                let transaction = platform.drive.grove.start_transaction();

                let processing_result = platform
                    .platform
                    .process_raw_state_transitions(
                        &vec![documents_batch_create_serialized_transition.clone()],
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

                let token_balance = platform
                    .drive
                    .fetch_identity_token_balance(
                        token_id.to_buffer(),
                        identity.id().to_buffer(),
                        None,
                        platform_version,
                    )
                    .expect("expected to fetch token balance");
                assert_eq!(token_balance, Some(101337));
            }

            #[test]
            fn test_token_mint_by_owner_requires_group_other_member() {
                // We are using a group, and two members need to sign for the event to happen
                let platform_version = PlatformVersion::latest();
                let mut platform = TestPlatformBuilder::new()
                    .with_latest_protocol_version()
                    .build_with_mock_rpc()
                    .set_genesis_state();

                let mut rng = StdRng::seed_from_u64(49853);

                let platform_state = platform.state.load();

                let (identity, signer, key) =
                    setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));

                let (identity_2, signer2, key2) =
                    setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));

                let (contract, token_id) = create_token_contract_with_owner_identity(
                    &mut platform,
                    identity.id(),
                    Some(|token_configuration: &mut TokenConfiguration| {
                        token_configuration.set_manual_minting_rules(ChangeControlRules::V0(
                            ChangeControlRulesV0 {
                                authorized_to_make_change: AuthorizedActionTakers::Group(0),
                                admin_action_takers: AuthorizedActionTakers::NoOne,
                                changing_authorized_action_takers_to_no_one_allowed: false,
                                changing_admin_action_takers_to_no_one_allowed: false,
                                self_changing_admin_action_takers_allowed: false,
                            },
                        ));
                    }),
                    None,
                    Some(
                        [(
                            0,
                            Group::V0(GroupV0 {
                                members: [(identity.id(), 1), (identity_2.id(), 1)].into(),
                                required_power: 2,
                            }),
                        )]
                        .into(),
                    ),
                    platform_version,
                );

                let token_mint_transition = BatchTransition::new_token_mint_transition(
                    token_id,
                    identity.id(),
                    contract.id(),
                    0,
                    1337,
                    Some(identity.id()),
                    None,
                    Some(GroupStateTransitionInfoStatus::GroupStateTransitionInfoProposer(0)),
                    &key,
                    2,
                    0,
                    &signer,
                    platform_version,
                    None,
                    None,
                    None,
                )
                .expect("expect to create documents batch transition");

                let token_mint_serialized_transition = token_mint_transition
                    .serialize_to_bytes()
                    .expect("expected documents batch serialized state transition");

                let transaction = platform.drive.grove.start_transaction();

                let processing_result = platform
                    .platform
                    .process_raw_state_transitions(
                        &vec![token_mint_serialized_transition.clone()],
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

                let token_balance = platform
                    .drive
                    .fetch_identity_token_balance(
                        token_id.to_buffer(),
                        identity.id().to_buffer(),
                        None,
                        platform_version,
                    )
                    .expect("expected to fetch token balance");
                assert_eq!(token_balance, Some(100000));

                // Now we need to get the second identity to also sign it
                let action_id = TokenMintTransition::calculate_action_id_with_fields(
                    token_id.as_bytes(),
                    identity.id().as_bytes(),
                    2,
                    1337,
                );
                let confirm_token_mint_transition = BatchTransition::new_token_mint_transition(
                    token_id,
                    identity_2.id(),
                    contract.id(),
                    0,
                    1337,
                    Some(identity.id()),
                    None,
                    Some(
                        GroupStateTransitionInfoStatus::GroupStateTransitionInfoOtherSigner(
                            GroupStateTransitionInfo {
                                group_contract_position: 0,
                                action_id,
                                action_is_proposer: false,
                            },
                        ),
                    ),
                    &key2,
                    2,
                    0,
                    &signer2,
                    platform_version,
                    None,
                    None,
                    None,
                )
                .expect("expect to create documents batch transition");

                let confirm_token_mint_serialized_transition = confirm_token_mint_transition
                    .serialize_to_bytes()
                    .expect("expected documents batch serialized state transition");

                let transaction = platform.drive.grove.start_transaction();

                let processing_result = platform
                    .platform
                    .process_raw_state_transitions(
                        &vec![confirm_token_mint_serialized_transition.clone()],
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

                let token_balance = platform
                    .drive
                    .fetch_identity_token_balance(
                        token_id.to_buffer(),
                        identity.id().to_buffer(),
                        None,
                        platform_version,
                    )
                    .expect("expected to fetch token balance");
                assert_eq!(token_balance, Some(101337));

                let token_balance = platform
                    .drive
                    .fetch_identity_token_balance(
                        token_id.to_buffer(),
                        identity_2.id().to_buffer(),
                        None,
                        platform_version,
                    )
                    .expect("expected to fetch token balance");
                assert_eq!(token_balance, None);
            }

            #[test]
            fn test_token_mint_by_owner_requires_group_resubmitting_causes_error() {
                // We are using a group, and two members need to sign for the event to happen
                let platform_version = PlatformVersion::latest();
                let mut platform = TestPlatformBuilder::new()
                    .with_latest_protocol_version()
                    .build_with_mock_rpc()
                    .set_genesis_state();

                let mut rng = StdRng::seed_from_u64(49853);

                let platform_state = platform.state.load();

                let (identity, signer, key) =
                    setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));

                let (identity_2, _, _) =
                    setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));

                let (contract, token_id) = create_token_contract_with_owner_identity(
                    &mut platform,
                    identity.id(),
                    Some(|token_configuration: &mut TokenConfiguration| {
                        token_configuration.set_manual_minting_rules(ChangeControlRules::V0(
                            ChangeControlRulesV0 {
                                authorized_to_make_change: AuthorizedActionTakers::Group(0),
                                admin_action_takers: AuthorizedActionTakers::NoOne,
                                changing_authorized_action_takers_to_no_one_allowed: false,
                                changing_admin_action_takers_to_no_one_allowed: false,
                                self_changing_admin_action_takers_allowed: false,
                            },
                        ));
                    }),
                    None,
                    Some(
                        [(
                            0,
                            Group::V0(GroupV0 {
                                members: [(identity.id(), 1), (identity_2.id(), 1)].into(),
                                required_power: 2,
                            }),
                        )]
                        .into(),
                    ),
                    platform_version,
                );

                let token_mint_transition = BatchTransition::new_token_mint_transition(
                    token_id,
                    identity.id(),
                    contract.id(),
                    0,
                    1337,
                    Some(identity.id()),
                    None,
                    Some(GroupStateTransitionInfoStatus::GroupStateTransitionInfoProposer(0)),
                    &key,
                    2,
                    0,
                    &signer,
                    platform_version,
                    None,
                    None,
                    None,
                )
                .expect("expect to create documents batch transition");

                let token_mint_serialized_transition = token_mint_transition
                    .serialize_to_bytes()
                    .expect("expected documents batch serialized state transition");

                let transaction = platform.drive.grove.start_transaction();

                let processing_result = platform
                    .platform
                    .process_raw_state_transitions(
                        &vec![token_mint_serialized_transition.clone()],
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

                let token_balance = platform
                    .drive
                    .fetch_identity_token_balance(
                        token_id.to_buffer(),
                        identity.id().to_buffer(),
                        None,
                        platform_version,
                    )
                    .expect("expected to fetch token balance");
                assert_eq!(token_balance, Some(100000));

                // Now we need to get the second identity to also sign it, but we are going to resubmit with first
                // This will create an error
                let action_id = TokenMintTransition::calculate_action_id_with_fields(
                    token_id.as_bytes(),
                    identity.id().as_bytes(),
                    2,
                    1337,
                );
                let confirm_token_mint_transition = BatchTransition::new_token_mint_transition(
                    token_id,
                    identity.id(),
                    contract.id(),
                    0,
                    1337,
                    Some(identity.id()),
                    None,
                    Some(
                        GroupStateTransitionInfoStatus::GroupStateTransitionInfoOtherSigner(
                            GroupStateTransitionInfo {
                                group_contract_position: 0,
                                action_id,
                                action_is_proposer: false,
                            },
                        ),
                    ),
                    &key,
                    3,
                    0,
                    &signer,
                    platform_version,
                    None,
                    None,
                    None,
                )
                .expect("expect to create documents batch transition");

                let confirm_token_mint_serialized_transition = confirm_token_mint_transition
                    .serialize_to_bytes()
                    .expect("expected documents batch serialized state transition");

                let transaction = platform.drive.grove.start_transaction();

                let processing_result = platform
                    .platform
                    .process_raw_state_transitions(
                        &vec![confirm_token_mint_serialized_transition.clone()],
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
                    [StateTransitionExecutionResult::PaidConsensusError(
                        ConsensusError::StateError(
                            StateError::GroupActionAlreadySignedByIdentityError(_)
                        ),
                        _
                    )]
                );

                platform
                    .drive
                    .grove
                    .commit_transaction(transaction)
                    .unwrap()
                    .expect("expected to commit transaction");

                let token_balance = platform
                    .drive
                    .fetch_identity_token_balance(
                        token_id.to_buffer(),
                        identity.id().to_buffer(),
                        None,
                        platform_version,
                    )
                    .expect("expected to fetch token balance");
                assert_eq!(token_balance, Some(100000));

                let token_balance = platform
                    .drive
                    .fetch_identity_token_balance(
                        token_id.to_buffer(),
                        identity_2.id().to_buffer(),
                        None,
                        platform_version,
                    )
                    .expect("expected to fetch token balance");
                assert_eq!(token_balance, None);
            }

            #[test]
            fn test_token_mint_by_owner_requires_group_other_member_resubmitting_causes_error() {
                // We are using a group, and two members need to sign for the event to happen
                let platform_version = PlatformVersion::latest();
                let mut platform = TestPlatformBuilder::new()
                    .with_latest_protocol_version()
                    .build_with_mock_rpc()
                    .set_genesis_state();

                let mut rng = StdRng::seed_from_u64(49853);

                let platform_state = platform.state.load();

                let (identity, signer, key) =
                    setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));

                let (identity_2, signer2, key2) =
                    setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));

                let (identity_3, _, _) =
                    setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));

                let (contract, token_id) = create_token_contract_with_owner_identity(
                    &mut platform,
                    identity.id(),
                    Some(|token_configuration: &mut TokenConfiguration| {
                        token_configuration.set_manual_minting_rules(ChangeControlRules::V0(
                            ChangeControlRulesV0 {
                                authorized_to_make_change: AuthorizedActionTakers::Group(0),
                                admin_action_takers: AuthorizedActionTakers::NoOne,
                                changing_authorized_action_takers_to_no_one_allowed: false,
                                changing_admin_action_takers_to_no_one_allowed: false,
                                self_changing_admin_action_takers_allowed: false,
                            },
                        ));
                    }),
                    None,
                    Some(
                        [(
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
                        )]
                        .into(),
                    ),
                    platform_version,
                );

                let token_mint_transition = BatchTransition::new_token_mint_transition(
                    token_id,
                    identity.id(),
                    contract.id(),
                    0,
                    1337,
                    Some(identity.id()),
                    None,
                    Some(GroupStateTransitionInfoStatus::GroupStateTransitionInfoProposer(0)),
                    &key,
                    2,
                    0,
                    &signer,
                    platform_version,
                    None,
                    None,
                    None,
                )
                .expect("expect to create documents batch transition");

                let token_mint_serialized_transition = token_mint_transition
                    .serialize_to_bytes()
                    .expect("expected documents batch serialized state transition");

                let transaction = platform.drive.grove.start_transaction();

                let processing_result = platform
                    .platform
                    .process_raw_state_transitions(
                        &vec![token_mint_serialized_transition.clone()],
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

                let token_balance = platform
                    .drive
                    .fetch_identity_token_balance(
                        token_id.to_buffer(),
                        identity.id().to_buffer(),
                        None,
                        platform_version,
                    )
                    .expect("expected to fetch token balance");
                assert_eq!(token_balance, Some(100000));

                // Now we need to get the second identity to also sign it
                let action_id = TokenMintTransition::calculate_action_id_with_fields(
                    token_id.as_bytes(),
                    identity.id().as_bytes(),
                    2,
                    1337,
                );
                let confirm_token_mint_transition = BatchTransition::new_token_mint_transition(
                    token_id,
                    identity_2.id(),
                    contract.id(),
                    0,
                    1337,
                    Some(identity.id()),
                    None,
                    Some(
                        GroupStateTransitionInfoStatus::GroupStateTransitionInfoOtherSigner(
                            GroupStateTransitionInfo {
                                group_contract_position: 0,
                                action_id,
                                action_is_proposer: false,
                            },
                        ),
                    ),
                    &key2,
                    2,
                    0,
                    &signer2,
                    platform_version,
                    None,
                    None,
                    None,
                )
                .expect("expect to create documents batch transition");

                let confirm_token_mint_serialized_transition = confirm_token_mint_transition
                    .serialize_to_bytes()
                    .expect("expected documents batch serialized state transition");

                let transaction = platform.drive.grove.start_transaction();

                let processing_result = platform
                    .platform
                    .process_raw_state_transitions(
                        &vec![confirm_token_mint_serialized_transition.clone()],
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

                let token_balance = platform
                    .drive
                    .fetch_identity_token_balance(
                        token_id.to_buffer(),
                        identity.id().to_buffer(),
                        None,
                        platform_version,
                    )
                    .expect("expected to fetch token balance");
                assert_eq!(token_balance, Some(100000));

                let token_balance = platform
                    .drive
                    .fetch_identity_token_balance(
                        token_id.to_buffer(),
                        identity_2.id().to_buffer(),
                        None,
                        platform_version,
                    )
                    .expect("expected to fetch token balance");
                assert_eq!(token_balance, None);

                // Now we need to get the second identity to sign it again to cause the error
                let confirm_token_mint_transition = BatchTransition::new_token_mint_transition(
                    token_id,
                    identity_2.id(),
                    contract.id(),
                    0,
                    1337,
                    Some(identity.id()),
                    None,
                    Some(
                        GroupStateTransitionInfoStatus::GroupStateTransitionInfoOtherSigner(
                            GroupStateTransitionInfo {
                                group_contract_position: 0,
                                action_id,
                                action_is_proposer: false,
                            },
                        ),
                    ),
                    &key2,
                    3,
                    0,
                    &signer2,
                    platform_version,
                    None,
                    None,
                    None,
                )
                .expect("expect to create documents batch transition");

                let confirm_token_mint_serialized_transition = confirm_token_mint_transition
                    .serialize_to_bytes()
                    .expect("expected documents batch serialized state transition");

                let transaction = platform.drive.grove.start_transaction();

                let processing_result = platform
                    .platform
                    .process_raw_state_transitions(
                        &vec![confirm_token_mint_serialized_transition.clone()],
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
                    [StateTransitionExecutionResult::PaidConsensusError(
                        ConsensusError::StateError(
                            StateError::GroupActionAlreadySignedByIdentityError(_)
                        ),
                        _
                    )]
                );

                platform
                    .drive
                    .grove
                    .commit_transaction(transaction)
                    .unwrap()
                    .expect("expected to commit transaction");

                let token_balance = platform
                    .drive
                    .fetch_identity_token_balance(
                        token_id.to_buffer(),
                        identity.id().to_buffer(),
                        None,
                        platform_version,
                    )
                    .expect("expected to fetch token balance");
                assert_eq!(token_balance, Some(100000));

                let token_balance = platform
                    .drive
                    .fetch_identity_token_balance(
                        token_id.to_buffer(),
                        identity_2.id().to_buffer(),
                        None,
                        platform_version,
                    )
                    .expect("expected to fetch token balance");
                assert_eq!(token_balance, None);
            }

            #[test]
            fn test_token_mint_by_owner_requires_group_other_member_submitting_after_completion_causes_error(
            ) {
                // We are using a group, and two members need to sign for the event to happen
                let platform_version = PlatformVersion::latest();
                let mut platform = TestPlatformBuilder::new()
                    .with_latest_protocol_version()
                    .build_with_mock_rpc()
                    .set_genesis_state();

                let mut rng = StdRng::seed_from_u64(49853);

                let platform_state = platform.state.load();

                let (identity, signer, key) =
                    setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));

                let (identity_2, signer2, key2) =
                    setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));

                let (identity_3, signer3, key3) =
                    setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));

                let (contract, token_id) = create_token_contract_with_owner_identity(
                    &mut platform,
                    identity.id(),
                    Some(|token_configuration: &mut TokenConfiguration| {
                        token_configuration.set_manual_minting_rules(ChangeControlRules::V0(
                            ChangeControlRulesV0 {
                                authorized_to_make_change: AuthorizedActionTakers::Group(0),
                                admin_action_takers: AuthorizedActionTakers::NoOne,
                                changing_authorized_action_takers_to_no_one_allowed: false,
                                changing_admin_action_takers_to_no_one_allowed: false,
                                self_changing_admin_action_takers_allowed: false,
                            },
                        ));
                    }),
                    None,
                    Some(
                        [(
                            0,
                            Group::V0(GroupV0 {
                                members: [
                                    (identity.id(), 1),
                                    (identity_2.id(), 1),
                                    (identity_3.id(), 1),
                                ]
                                .into(),
                                required_power: 2,
                            }),
                        )]
                        .into(),
                    ),
                    platform_version,
                );

                let token_mint_transition = BatchTransition::new_token_mint_transition(
                    token_id,
                    identity.id(),
                    contract.id(),
                    0,
                    1337,
                    Some(identity.id()),
                    None,
                    Some(GroupStateTransitionInfoStatus::GroupStateTransitionInfoProposer(0)),
                    &key,
                    2,
                    0,
                    &signer,
                    platform_version,
                    None,
                    None,
                    None,
                )
                .expect("expect to create documents batch transition");

                let token_mint_serialized_transition = token_mint_transition
                    .serialize_to_bytes()
                    .expect("expected documents batch serialized state transition");

                let transaction = platform.drive.grove.start_transaction();

                let processing_result = platform
                    .platform
                    .process_raw_state_transitions(
                        &vec![token_mint_serialized_transition.clone()],
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

                let token_balance = platform
                    .drive
                    .fetch_identity_token_balance(
                        token_id.to_buffer(),
                        identity.id().to_buffer(),
                        None,
                        platform_version,
                    )
                    .expect("expected to fetch token balance");
                assert_eq!(token_balance, Some(100000));

                // Now we need to get the second identity to also sign it
                let action_id = TokenMintTransition::calculate_action_id_with_fields(
                    token_id.as_bytes(),
                    identity.id().as_bytes(),
                    2,
                    1337,
                );
                let confirm_token_mint_transition = BatchTransition::new_token_mint_transition(
                    token_id,
                    identity_2.id(),
                    contract.id(),
                    0,
                    1337,
                    Some(identity.id()),
                    None,
                    Some(
                        GroupStateTransitionInfoStatus::GroupStateTransitionInfoOtherSigner(
                            GroupStateTransitionInfo {
                                group_contract_position: 0,
                                action_id,
                                action_is_proposer: false,
                            },
                        ),
                    ),
                    &key2,
                    2,
                    0,
                    &signer2,
                    platform_version,
                    None,
                    None,
                    None,
                )
                .expect("expect to create documents batch transition");

                let confirm_token_mint_serialized_transition = confirm_token_mint_transition
                    .serialize_to_bytes()
                    .expect("expected documents batch serialized state transition");

                let transaction = platform.drive.grove.start_transaction();

                let processing_result = platform
                    .platform
                    .process_raw_state_transitions(
                        &vec![confirm_token_mint_serialized_transition.clone()],
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

                let token_balance = platform
                    .drive
                    .fetch_identity_token_balance(
                        token_id.to_buffer(),
                        identity.id().to_buffer(),
                        None,
                        platform_version,
                    )
                    .expect("expected to fetch token balance");
                assert_eq!(token_balance, Some(101337));

                let token_balance = platform
                    .drive
                    .fetch_identity_token_balance(
                        token_id.to_buffer(),
                        identity_2.id().to_buffer(),
                        None,
                        platform_version,
                    )
                    .expect("expected to fetch token balance");
                assert_eq!(token_balance, None);

                // Now we need to get the second identity to sign it again to cause the error
                let confirm_token_mint_transition = BatchTransition::new_token_mint_transition(
                    token_id,
                    identity_3.id(),
                    contract.id(),
                    0,
                    1337,
                    Some(identity.id()),
                    None,
                    Some(
                        GroupStateTransitionInfoStatus::GroupStateTransitionInfoOtherSigner(
                            GroupStateTransitionInfo {
                                group_contract_position: 0,
                                action_id,
                                action_is_proposer: false,
                            },
                        ),
                    ),
                    &key3,
                    2,
                    0,
                    &signer3,
                    platform_version,
                    None,
                    None,
                    None,
                )
                .expect("expect to create documents batch transition");

                let confirm_token_mint_serialized_transition = confirm_token_mint_transition
                    .serialize_to_bytes()
                    .expect("expected documents batch serialized state transition");

                let transaction = platform.drive.grove.start_transaction();

                let processing_result = platform
                    .platform
                    .process_raw_state_transitions(
                        &vec![confirm_token_mint_serialized_transition.clone()],
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
                    [StateTransitionExecutionResult::PaidConsensusError(
                        ConsensusError::StateError(StateError::GroupActionAlreadyCompletedError(_)),
                        _
                    )]
                );

                platform
                    .drive
                    .grove
                    .commit_transaction(transaction)
                    .unwrap()
                    .expect("expected to commit transaction");

                let token_balance = platform
                    .drive
                    .fetch_identity_token_balance(
                        token_id.to_buffer(),
                        identity.id().to_buffer(),
                        None,
                        platform_version,
                    )
                    .expect("expected to fetch token balance");
                assert_eq!(token_balance, Some(101337));

                let token_balance = platform
                    .drive
                    .fetch_identity_token_balance(
                        token_id.to_buffer(),
                        identity_2.id().to_buffer(),
                        None,
                        platform_version,
                    )
                    .expect("expected to fetch token balance");
                assert_eq!(token_balance, None);
            }

            #[test]
            fn test_token_mint_by_owner_requires_group_proposer_not_in_group() {
                // We are using a group, and two members need to sign for the event to happen
                let platform_version = PlatformVersion::latest();
                let mut platform = TestPlatformBuilder::new()
                    .with_latest_protocol_version()
                    .build_with_mock_rpc()
                    .set_genesis_state();

                let mut rng = StdRng::seed_from_u64(49853);

                let platform_state = platform.state.load();

                let (identity, signer, key) =
                    setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));

                let (identity_2, _, _) =
                    setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));

                let (identity_3, _, _) =
                    setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));

                let (contract, token_id) = create_token_contract_with_owner_identity(
                    &mut platform,
                    identity.id(),
                    Some(|token_configuration: &mut TokenConfiguration| {
                        token_configuration.set_manual_minting_rules(ChangeControlRules::V0(
                            ChangeControlRulesV0 {
                                authorized_to_make_change: AuthorizedActionTakers::Group(0),
                                admin_action_takers: AuthorizedActionTakers::NoOne,
                                changing_authorized_action_takers_to_no_one_allowed: false,
                                changing_admin_action_takers_to_no_one_allowed: false,
                                self_changing_admin_action_takers_allowed: false,
                            },
                        ));
                    }),
                    None,
                    Some(
                        [(
                            0,
                            Group::V0(GroupV0 {
                                members: [(identity_3.id(), 1), (identity_2.id(), 1)].into(),
                                required_power: 2,
                            }),
                        )]
                        .into(),
                    ),
                    platform_version,
                );

                let token_mint_transition = BatchTransition::new_token_mint_transition(
                    token_id,
                    identity.id(),
                    contract.id(),
                    0,
                    1337,
                    Some(identity.id()),
                    None,
                    Some(GroupStateTransitionInfoStatus::GroupStateTransitionInfoProposer(0)),
                    &key,
                    2,
                    0,
                    &signer,
                    platform_version,
                    None,
                    None,
                    None,
                )
                .expect("expect to create documents batch transition");

                let token_mint_serialized_transition = token_mint_transition
                    .serialize_to_bytes()
                    .expect("expected documents batch serialized state transition");

                let transaction = platform.drive.grove.start_transaction();

                let processing_result = platform
                    .platform
                    .process_raw_state_transitions(
                        &vec![token_mint_serialized_transition.clone()],
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
                    [StateTransitionExecutionResult::PaidConsensusError(
                        ConsensusError::StateError(StateError::IdentityNotMemberOfGroupError(_)),
                        _
                    )]
                );

                platform
                    .drive
                    .grove
                    .commit_transaction(transaction)
                    .unwrap()
                    .expect("expected to commit transaction");

                let token_balance = platform
                    .drive
                    .fetch_identity_token_balance(
                        token_id.to_buffer(),
                        identity.id().to_buffer(),
                        None,
                        platform_version,
                    )
                    .expect("expected to fetch token balance");
                assert_eq!(token_balance, Some(100000));
            }

            #[test]
            fn test_token_mint_by_owner_requires_group_other_signer_not_part_of_group() {
                // We are using a group, and two members need to sign for the event to happen
                let platform_version = PlatformVersion::latest();
                let mut platform = TestPlatformBuilder::new()
                    .with_latest_protocol_version()
                    .build_with_mock_rpc()
                    .set_genesis_state();

                let mut rng = StdRng::seed_from_u64(49853);

                let platform_state = platform.state.load();

                let (identity, signer, key) =
                    setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));

                let (identity_2, signer2, key2) =
                    setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));

                let (identity_3, _, _) =
                    setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));

                let (contract, token_id) = create_token_contract_with_owner_identity(
                    &mut platform,
                    identity.id(),
                    Some(|token_configuration: &mut TokenConfiguration| {
                        token_configuration.set_manual_minting_rules(ChangeControlRules::V0(
                            ChangeControlRulesV0 {
                                authorized_to_make_change: AuthorizedActionTakers::Group(0),
                                admin_action_takers: AuthorizedActionTakers::NoOne,
                                changing_authorized_action_takers_to_no_one_allowed: false,
                                changing_admin_action_takers_to_no_one_allowed: false,
                                self_changing_admin_action_takers_allowed: false,
                            },
                        ));
                    }),
                    None,
                    Some(
                        [(
                            0,
                            Group::V0(GroupV0 {
                                members: [(identity.id(), 1), (identity_3.id(), 1)].into(),
                                required_power: 2,
                            }),
                        )]
                        .into(),
                    ),
                    platform_version,
                );

                let token_mint_transition = BatchTransition::new_token_mint_transition(
                    token_id,
                    identity.id(),
                    contract.id(),
                    0,
                    1337,
                    Some(identity.id()),
                    None,
                    Some(GroupStateTransitionInfoStatus::GroupStateTransitionInfoProposer(0)),
                    &key,
                    2,
                    0,
                    &signer,
                    platform_version,
                    None,
                    None,
                    None,
                )
                .expect("expect to create documents batch transition");

                let token_mint_serialized_transition = token_mint_transition
                    .serialize_to_bytes()
                    .expect("expected documents batch serialized state transition");

                let transaction = platform.drive.grove.start_transaction();

                let processing_result = platform
                    .platform
                    .process_raw_state_transitions(
                        &vec![token_mint_serialized_transition.clone()],
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

                let token_balance = platform
                    .drive
                    .fetch_identity_token_balance(
                        token_id.to_buffer(),
                        identity.id().to_buffer(),
                        None,
                        platform_version,
                    )
                    .expect("expected to fetch token balance");
                assert_eq!(token_balance, Some(100000));

                // Now we need to get the second identity to also sign it
                let action_id = TokenMintTransition::calculate_action_id_with_fields(
                    token_id.as_bytes(),
                    identity.id().as_bytes(),
                    2,
                    1337,
                );
                let confirm_token_mint_transition = BatchTransition::new_token_mint_transition(
                    token_id,
                    identity_2.id(),
                    contract.id(),
                    0,
                    1337,
                    Some(identity.id()),
                    None,
                    Some(
                        GroupStateTransitionInfoStatus::GroupStateTransitionInfoOtherSigner(
                            GroupStateTransitionInfo {
                                group_contract_position: 0,
                                action_id,
                                action_is_proposer: false,
                            },
                        ),
                    ),
                    &key2,
                    2,
                    0,
                    &signer2,
                    platform_version,
                    None,
                    None,
                    None,
                )
                .expect("expect to create documents batch transition");

                let confirm_token_mint_serialized_transition = confirm_token_mint_transition
                    .serialize_to_bytes()
                    .expect("expected documents batch serialized state transition");

                let transaction = platform.drive.grove.start_transaction();

                let processing_result = platform
                    .platform
                    .process_raw_state_transitions(
                        &vec![confirm_token_mint_serialized_transition.clone()],
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
                    [StateTransitionExecutionResult::PaidConsensusError(
                        ConsensusError::StateError(StateError::IdentityNotMemberOfGroupError(_)),
                        _
                    )]
                );

                platform
                    .drive
                    .grove
                    .commit_transaction(transaction)
                    .unwrap()
                    .expect("expected to commit transaction");

                let token_balance = platform
                    .drive
                    .fetch_identity_token_balance(
                        token_id.to_buffer(),
                        identity.id().to_buffer(),
                        None,
                        platform_version,
                    )
                    .expect("expected to fetch token balance");
                assert_eq!(token_balance, Some(100000));

                let token_balance = platform
                    .drive
                    .fetch_identity_token_balance(
                        token_id.to_buffer(),
                        identity_2.id().to_buffer(),
                        None,
                        platform_version,
                    )
                    .expect("expected to fetch token balance");
                assert_eq!(token_balance, None);
            }

            #[test]
            fn test_token_mint_other_signer_going_first_causes_error() {
                // We are using a group, and the second member gets a bit hasty and signs first
                let platform_version = PlatformVersion::latest();
                let mut platform = TestPlatformBuilder::new()
                    .with_latest_protocol_version()
                    .build_with_mock_rpc()
                    .set_genesis_state();

                let mut rng = StdRng::seed_from_u64(49853);

                let platform_state = platform.state.load();

                let (identity, _, _) =
                    setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));

                let (identity_2, signer2, key2) =
                    setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));

                let (contract, token_id) = create_token_contract_with_owner_identity(
                    &mut platform,
                    identity.id(),
                    Some(|token_configuration: &mut TokenConfiguration| {
                        token_configuration.set_manual_minting_rules(ChangeControlRules::V0(
                            ChangeControlRulesV0 {
                                authorized_to_make_change: AuthorizedActionTakers::Group(0),
                                admin_action_takers: AuthorizedActionTakers::NoOne,
                                changing_authorized_action_takers_to_no_one_allowed: false,
                                changing_admin_action_takers_to_no_one_allowed: false,
                                self_changing_admin_action_takers_allowed: false,
                            },
                        ));
                    }),
                    None,
                    Some(
                        [(
                            0,
                            Group::V0(GroupV0 {
                                members: [(identity.id(), 1), (identity_2.id(), 1)].into(),
                                required_power: 2,
                            }),
                        )]
                        .into(),
                    ),
                    platform_version,
                );

                // The second identity to also sign it
                let action_id = TokenMintTransition::calculate_action_id_with_fields(
                    token_id.as_bytes(),
                    identity.id().as_bytes(),
                    2,
                    1337,
                );
                let confirm_token_mint_transition = BatchTransition::new_token_mint_transition(
                    token_id,
                    identity_2.id(),
                    contract.id(),
                    0,
                    1337,
                    Some(identity.id()),
                    None,
                    Some(
                        GroupStateTransitionInfoStatus::GroupStateTransitionInfoOtherSigner(
                            GroupStateTransitionInfo {
                                group_contract_position: 0,
                                action_id,
                                action_is_proposer: false,
                            },
                        ),
                    ),
                    &key2,
                    2,
                    0,
                    &signer2,
                    platform_version,
                    None,
                    None,
                    None,
                )
                .expect("expect to create documents batch transition");

                let confirm_token_mint_serialized_transition = confirm_token_mint_transition
                    .serialize_to_bytes()
                    .expect("expected documents batch serialized state transition");

                let transaction = platform.drive.grove.start_transaction();

                let processing_result = platform
                    .platform
                    .process_raw_state_transitions(
                        &vec![confirm_token_mint_serialized_transition.clone()],
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
                    [StateTransitionExecutionResult::PaidConsensusError(
                        ConsensusError::StateError(StateError::GroupActionDoesNotExistError(_)),
                        _
                    )]
                );

                platform
                    .drive
                    .grove
                    .commit_transaction(transaction)
                    .unwrap()
                    .expect("expected to commit transaction");

                let token_balance = platform
                    .drive
                    .fetch_identity_token_balance(
                        token_id.to_buffer(),
                        identity.id().to_buffer(),
                        None,
                        platform_version,
                    )
                    .expect("expected to fetch token balance");
                assert_eq!(token_balance, Some(100000));

                let token_balance = platform
                    .drive
                    .fetch_identity_token_balance(
                        token_id.to_buffer(),
                        identity_2.id().to_buffer(),
                        None,
                        platform_version,
                    )
                    .expect("expected to fetch token balance");
                assert_eq!(token_balance, None);
            }
        }
    }

    mod token_burn_tests {
        use super::*;

        #[test]
        fn test_token_burn() {
            let platform_version = PlatformVersion::latest();
            let mut platform = TestPlatformBuilder::new()
                .with_latest_protocol_version()
                .build_with_mock_rpc()
                .set_genesis_state();

            let mut rng = StdRng::seed_from_u64(49853);

            let platform_state = platform.state.load();

            let (identity, signer, key) =
                setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));

            let (contract, token_id) = create_token_contract_with_owner_identity(
                &mut platform,
                identity.id(),
                None::<fn(&mut TokenConfiguration)>,
                None,
                None,
                platform_version,
            );

            let documents_batch_create_transition = BatchTransition::new_token_burn_transition(
                token_id,
                identity.id(),
                contract.id(),
                0,
                1337,
                None,
                None,
                &key,
                2,
                0,
                &signer,
                platform_version,
                None,
                None,
                None,
            )
            .expect("expect to create documents batch transition");

            let documents_batch_create_serialized_transition = documents_batch_create_transition
                .serialize_to_bytes()
                .expect("expected documents batch serialized state transition");

            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![documents_batch_create_serialized_transition.clone()],
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

            let token_balance = platform
                .drive
                .fetch_identity_token_balance(
                    token_id.to_buffer(),
                    identity.id().to_buffer(),
                    None,
                    platform_version,
                )
                .expect("expected to fetch token balance");
            let expected_amount = 100000 - 1337;
            assert_eq!(token_balance, Some(expected_amount));
        }

        #[test]
        fn test_token_burn_trying_to_burn_more_than_we_have() {
            let platform_version = PlatformVersion::latest();
            let mut platform = TestPlatformBuilder::new()
                .with_latest_protocol_version()
                .build_with_mock_rpc()
                .set_genesis_state();

            let mut rng = StdRng::seed_from_u64(49853);

            let platform_state = platform.state.load();

            let (identity, signer, key) =
                setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));

            let (contract, token_id) = create_token_contract_with_owner_identity(
                &mut platform,
                identity.id(),
                None::<fn(&mut TokenConfiguration)>,
                None,
                None,
                platform_version,
            );

            let documents_batch_create_transition = BatchTransition::new_token_burn_transition(
                token_id,
                identity.id(),
                contract.id(),
                0,
                200000,
                None,
                None,
                &key,
                2,
                0,
                &signer,
                platform_version,
                None,
                None,
                None,
            )
            .expect("expect to create documents batch transition");

            let documents_batch_create_serialized_transition = documents_batch_create_transition
                .serialize_to_bytes()
                .expect("expected documents batch serialized state transition");

            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![documents_batch_create_serialized_transition.clone()],
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
                [StateTransitionExecutionResult::PaidConsensusError(
                    ConsensusError::StateError(
                        StateError::IdentityDoesNotHaveEnoughTokenBalanceError(_)
                    ),
                    _
                )]
            );

            platform
                .drive
                .grove
                .commit_transaction(transaction)
                .unwrap()
                .expect("expected to commit transaction");

            let token_balance = platform
                .drive
                .fetch_identity_token_balance(
                    token_id.to_buffer(),
                    identity.id().to_buffer(),
                    None,
                    platform_version,
                )
                .expect("expected to fetch token balance");
            assert_eq!(token_balance, Some(100000)); // nothing was burned
        }

        #[test]
        fn test_token_burn_gives_error_if_trying_to_burn_from_not_allowed_identity() {
            let platform_version = PlatformVersion::latest();
            let mut platform = TestPlatformBuilder::new()
                .with_latest_protocol_version()
                .build_with_mock_rpc()
                .set_genesis_state();

            let mut rng = StdRng::seed_from_u64(49853);

            let platform_state = platform.state.load();

            let (contract_owner_identity, _, _) =
                setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));

            let (identity, signer, key) =
                setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));

            let (contract, token_id) = create_token_contract_with_owner_identity(
                &mut platform,
                contract_owner_identity.id(),
                None::<fn(&mut TokenConfiguration)>,
                None,
                None,
                platform_version,
            );

            let documents_batch_create_transition = BatchTransition::new_token_burn_transition(
                token_id,
                identity.id(),
                contract.id(),
                0,
                1337,
                None,
                None,
                &key,
                2,
                0,
                &signer,
                platform_version,
                None,
                None,
                None,
            )
            .expect("expect to create documents batch transition");

            let documents_batch_create_serialized_transition = documents_batch_create_transition
                .serialize_to_bytes()
                .expect("expected documents batch serialized state transition");

            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![documents_batch_create_serialized_transition.clone()],
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
                [StateTransitionExecutionResult::PaidConsensusError(
                    ConsensusError::StateError(StateError::UnauthorizedTokenActionError(_)),
                    _
                )]
            );

            platform
                .drive
                .grove
                .commit_transaction(transaction)
                .unwrap()
                .expect("expected to commit transaction");

            let token_balance = platform
                .drive
                .fetch_identity_token_balance(
                    token_id.to_buffer(),
                    contract_owner_identity.id().to_buffer(),
                    None,
                    platform_version,
                )
                .expect("expected to fetch token balance");
            assert_eq!(token_balance, Some(100000));

            let token_balance = platform
                .drive
                .fetch_identity_token_balance(
                    token_id.to_buffer(),
                    identity.id().to_buffer(),
                    None,
                    platform_version,
                )
                .expect("expected to fetch token balance");
            assert_eq!(token_balance, None);
        }
    }

    mod token_transfer_tests {
        use dpp::data_contract::change_control_rules::authorized_action_takers::AuthorizedActionTakers;
        use dpp::data_contract::change_control_rules::ChangeControlRules;
        use dpp::data_contract::change_control_rules::v0::ChangeControlRulesV0;
        use dpp::data_contract::group::Group;
        use dpp::state_transition::batch_transition::TokenMintTransition;
        use dpp::data_contract::group::v0::GroupV0;
        use dpp::group::{GroupStateTransitionInfo, GroupStateTransitionInfoStatus};
        use dpp::identity::SecurityLevel;
        use dpp::state_transition::batch_transition::accessors::DocumentsBatchTransitionAccessorsV0;
        use dpp::state_transition::batch_transition::batched_transition::token_transition::TokenTransition;
        use dpp::state_transition::StateTransition;
        use dpp::state_transition::batch_transition::batched_transition::BatchedTransitionMutRef;
        use dpp::state_transition::batch_transition::token_base_transition::token_base_transition_accessors::TokenBaseTransitionAccessors;
        use dpp::state_transition::batch_transition::token_base_transition::v0::v0_methods::TokenBaseTransitionV0Methods;
        use super::*;

        #[test]
        fn test_token_transfer() {
            let platform_version = PlatformVersion::latest();
            let mut platform = TestPlatformBuilder::new()
                .with_latest_protocol_version()
                .build_with_mock_rpc()
                .set_genesis_state();

            let mut rng = StdRng::seed_from_u64(49853);

            let platform_state = platform.state.load();

            let (identity, signer, key) =
                setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));

            let (recipient, _, _) = setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));

            let (contract, token_id) = create_token_contract_with_owner_identity(
                &mut platform,
                identity.id(),
                None::<fn(&mut TokenConfiguration)>,
                None,
                None,
                platform_version,
            );

            let token_transfer_transition = BatchTransition::new_token_transfer_transition(
                token_id,
                identity.id(),
                contract.id(),
                0,
                1337,
                recipient.id(),
                None,
                None,
                None,
                &key,
                2,
                0,
                &signer,
                platform_version,
                None,
                None,
                None,
            )
            .expect("expect to create documents batch transition");

            let token_transfer_serialized_transition = token_transfer_transition
                .serialize_to_bytes()
                .expect("expected documents batch serialized state transition");

            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![token_transfer_serialized_transition.clone()],
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

            let token_balance = platform
                .drive
                .fetch_identity_token_balance(
                    token_id.to_buffer(),
                    identity.id().to_buffer(),
                    None,
                    platform_version,
                )
                .expect("expected to fetch token balance");
            let expected_amount = 100000 - 1337;
            assert_eq!(token_balance, Some(expected_amount));

            let token_balance = platform
                .drive
                .fetch_identity_token_balance(
                    token_id.to_buffer(),
                    recipient.id().to_buffer(),
                    None,
                    platform_version,
                )
                .expect("expected to fetch token balance");
            let expected_amount = 1337;
            assert_eq!(token_balance, Some(expected_amount));
        }

        #[test]
        fn test_token_transfer_to_ourself_should_fail() {
            let platform_version = PlatformVersion::latest();
            let mut platform = TestPlatformBuilder::new()
                .with_latest_protocol_version()
                .build_with_mock_rpc()
                .set_genesis_state();

            let mut rng = StdRng::seed_from_u64(49853);

            let platform_state = platform.state.load();

            let (identity, signer, key) =
                setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));

            let (contract, token_id) = create_token_contract_with_owner_identity(
                &mut platform,
                identity.id(),
                None::<fn(&mut TokenConfiguration)>,
                None,
                None,
                platform_version,
            );

            let token_transfer_transition = BatchTransition::new_token_transfer_transition(
                token_id,
                identity.id(),
                contract.id(),
                0,
                1337,
                identity.id(),
                None,
                None,
                None,
                &key,
                2,
                0,
                &signer,
                platform_version,
                None,
                None,
                None,
            )
            .expect("expect to create documents batch transition");

            let token_transfer_serialized_transition = token_transfer_transition
                .serialize_to_bytes()
                .expect("expected documents batch serialized state transition");

            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![token_transfer_serialized_transition.clone()],
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
                    ConsensusError::BasicError(BasicError::TokenTransferToOurselfError(_))
                )]
            );

            platform
                .drive
                .grove
                .commit_transaction(transaction)
                .unwrap()
                .expect("expected to commit transaction");

            let token_balance = platform
                .drive
                .fetch_identity_token_balance(
                    token_id.to_buffer(),
                    identity.id().to_buffer(),
                    None,
                    platform_version,
                )
                .expect("expected to fetch token balance");
            assert_eq!(token_balance, Some(100000));
        }

        #[test]
        fn test_token_transfer_trying_to_send_more_than_we_have() {
            let platform_version = PlatformVersion::latest();
            let mut platform = TestPlatformBuilder::new()
                .with_latest_protocol_version()
                .build_with_mock_rpc()
                .set_genesis_state();

            let mut rng = StdRng::seed_from_u64(49853);

            let platform_state = platform.state.load();

            let (identity, signer, key) =
                setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));

            let (recipient, _, _) = setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));

            let (contract, token_id) = create_token_contract_with_owner_identity(
                &mut platform,
                identity.id(),
                None::<fn(&mut TokenConfiguration)>,
                None,
                None,
                platform_version,
            );

            let token_transfer_transition = BatchTransition::new_token_transfer_transition(
                token_id,
                identity.id(),
                contract.id(),
                0,
                200000,
                recipient.id(),
                None,
                None,
                None,
                &key,
                2,
                0,
                &signer,
                platform_version,
                None,
                None,
                None,
            )
            .expect("expect to create documents batch transition");

            let token_transfer_serialized_transition = token_transfer_transition
                .serialize_to_bytes()
                .expect("expected documents batch serialized state transition");

            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![token_transfer_serialized_transition.clone()],
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
                [StateTransitionExecutionResult::PaidConsensusError(
                    ConsensusError::StateError(
                        StateError::IdentityDoesNotHaveEnoughTokenBalanceError(_)
                    ),
                    _
                )]
            );

            platform
                .drive
                .grove
                .commit_transaction(transaction)
                .unwrap()
                .expect("expected to commit transaction");

            let token_balance = platform
                .drive
                .fetch_identity_token_balance(
                    token_id.to_buffer(),
                    identity.id().to_buffer(),
                    None,
                    platform_version,
                )
                .expect("expected to fetch token balance");
            let expected_amount = 100000;
            assert_eq!(token_balance, Some(expected_amount));

            let token_balance = platform
                .drive
                .fetch_identity_token_balance(
                    token_id.to_buffer(),
                    recipient.id().to_buffer(),
                    None,
                    platform_version,
                )
                .expect("expected to fetch token balance");
            assert_eq!(token_balance, None);
        }

        #[test]
        fn test_token_transfer_adding_group_info_causes_error() {
            let platform_version = PlatformVersion::latest();
            let mut platform = TestPlatformBuilder::new()
                .with_latest_protocol_version()
                .build_with_mock_rpc()
                .set_genesis_state();

            let mut rng = StdRng::seed_from_u64(49853);

            let platform_state = platform.state.load();

            let (identity, signer, key) =
                setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));

            let (recipient, _, _) = setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));

            // let's start by creating a real action

            let (contract, token_id) = create_token_contract_with_owner_identity(
                &mut platform,
                identity.id(),
                Some(|token_configuration: &mut TokenConfiguration| {
                    token_configuration.set_manual_minting_rules(ChangeControlRules::V0(
                        ChangeControlRulesV0 {
                            authorized_to_make_change: AuthorizedActionTakers::Group(0),
                            admin_action_takers: AuthorizedActionTakers::NoOne,
                            changing_authorized_action_takers_to_no_one_allowed: false,
                            changing_admin_action_takers_to_no_one_allowed: false,
                            self_changing_admin_action_takers_allowed: false,
                        },
                    ));
                }),
                None,
                Some(
                    [(
                        0,
                        Group::V0(GroupV0 {
                            members: [(identity.id(), 1), (recipient.id(), 1)].into(),
                            required_power: 2,
                        }),
                    )]
                    .into(),
                ),
                platform_version,
            );

            let token_mint_transition = BatchTransition::new_token_mint_transition(
                token_id,
                identity.id(),
                contract.id(),
                0,
                1337,
                Some(identity.id()),
                None,
                Some(GroupStateTransitionInfoStatus::GroupStateTransitionInfoProposer(0)),
                &key,
                2,
                0,
                &signer,
                platform_version,
                None,
                None,
                None,
            )
            .expect("expect to create documents batch transition");

            let token_mint_serialized_transition = token_mint_transition
                .serialize_to_bytes()
                .expect("expected documents batch serialized state transition");

            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![token_mint_serialized_transition.clone()],
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

            let action_id = TokenMintTransition::calculate_action_id_with_fields(
                token_id.as_bytes(),
                identity.id().as_bytes(),
                2,
                1337,
            );

            let mut token_transfer_transition = BatchTransition::new_token_transfer_transition(
                token_id,
                identity.id(),
                contract.id(),
                0,
                200000,
                recipient.id(),
                None,
                None,
                None,
                &key,
                3,
                0,
                &signer,
                platform_version,
                None,
                None,
                None,
            )
            .expect("expect to create documents batch transition");

            // here we add fake info
            if let StateTransition::Batch(batch) = &mut token_transfer_transition {
                let first_transition = batch
                    .first_transition_mut()
                    .expect("expected_first_transition");
                if let BatchedTransitionMutRef::Token(token) = first_transition {
                    if let TokenTransition::Transfer(transfer) = token {
                        transfer
                            .base_mut()
                            .set_using_group_info(Some(GroupStateTransitionInfo {
                                group_contract_position: 0,
                                action_id,
                                action_is_proposer: true,
                            }))
                    }
                }
            }

            token_transfer_transition
                .sign_external(&key, &signer, Some(|_, _| Ok(SecurityLevel::HIGH)))
                .expect("expected to resign transaction");

            let token_transfer_serialized_transition = token_transfer_transition
                .serialize_to_bytes()
                .expect("expected documents batch serialized state transition");

            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![token_transfer_serialized_transition.clone()],
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
                    ConsensusError::BasicError(BasicError::GroupActionNotAllowedOnTransitionError(
                        _
                    ))
                )]
            );

            platform
                .drive
                .grove
                .commit_transaction(transaction)
                .unwrap()
                .expect("expected to commit transaction");

            let token_balance = platform
                .drive
                .fetch_identity_token_balance(
                    token_id.to_buffer(),
                    identity.id().to_buffer(),
                    None,
                    platform_version,
                )
                .expect("expected to fetch token balance");
            let expected_amount = 100000;
            assert_eq!(token_balance, Some(expected_amount));

            let token_balance = platform
                .drive
                .fetch_identity_token_balance(
                    token_id.to_buffer(),
                    recipient.id().to_buffer(),
                    None,
                    platform_version,
                )
                .expect("expected to fetch token balance");
            assert_eq!(token_balance, None);
        }
    }

    mod token_freeze_tests {
        use super::*;
        use dpp::tokens::info::v0::IdentityTokenInfoV0Accessors;

        #[test]
        fn test_token_freeze() {
            let platform_version = PlatformVersion::latest();
            let mut platform = TestPlatformBuilder::new()
                .with_latest_protocol_version()
                .build_with_mock_rpc()
                .set_genesis_state();

            let mut rng = StdRng::seed_from_u64(49853);

            let platform_state = platform.state.load();

            let (identity, signer, key) =
                setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));

            let (identity_2, _, _) =
                setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));

            let (contract, token_id) = create_token_contract_with_owner_identity(
                &mut platform,
                identity.id(),
                Some(|token_configuration: &mut TokenConfiguration| {
                    token_configuration.set_freeze_rules(ChangeControlRules::V0(
                        ChangeControlRulesV0 {
                            authorized_to_make_change: AuthorizedActionTakers::ContractOwner,
                            admin_action_takers: AuthorizedActionTakers::NoOne,
                            changing_authorized_action_takers_to_no_one_allowed: false,
                            changing_admin_action_takers_to_no_one_allowed: false,
                            self_changing_admin_action_takers_allowed: false,
                        },
                    ));
                }),
                None,
                None,
                platform_version,
            );

            let freeze_transition = BatchTransition::new_token_freeze_transition(
                token_id,
                identity.id(),
                contract.id(),
                0,
                identity_2.id(),
                None,
                None,
                &key,
                2,
                0,
                &signer,
                platform_version,
                None,
                None,
                None,
            )
            .expect("expect to create documents batch transition");

            let freeze_serialized_transition = freeze_transition
                .serialize_to_bytes()
                .expect("expected documents batch serialized state transition");

            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![freeze_serialized_transition.clone()],
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

            let token_frozen = platform
                .drive
                .fetch_identity_token_info(
                    token_id.to_buffer(),
                    identity_2.id().to_buffer(),
                    None,
                    platform_version,
                )
                .expect("expected to fetch token info")
                .map(|info| info.frozen());
            assert_eq!(token_frozen, Some(true));
        }

        #[test]
        fn test_token_freeze_and_unfreeze() {
            let platform_version = PlatformVersion::latest();
            let mut platform = TestPlatformBuilder::new()
                .with_latest_protocol_version()
                .build_with_mock_rpc()
                .set_genesis_state();

            let mut rng = StdRng::seed_from_u64(49853);

            let platform_state = platform.state.load();

            let (identity, signer, key) =
                setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));

            let (identity_2, _, _) =
                setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));

            let (contract, token_id) = create_token_contract_with_owner_identity(
                &mut platform,
                identity.id(),
                Some(|token_configuration: &mut TokenConfiguration| {
                    token_configuration.set_freeze_rules(ChangeControlRules::V0(
                        ChangeControlRulesV0 {
                            authorized_to_make_change: AuthorizedActionTakers::ContractOwner,
                            admin_action_takers: AuthorizedActionTakers::NoOne,
                            changing_authorized_action_takers_to_no_one_allowed: false,
                            changing_admin_action_takers_to_no_one_allowed: false,
                            self_changing_admin_action_takers_allowed: false,
                        },
                    ));
                    token_configuration.set_unfreeze_rules(ChangeControlRules::V0(
                        ChangeControlRulesV0 {
                            authorized_to_make_change: AuthorizedActionTakers::ContractOwner,
                            admin_action_takers: AuthorizedActionTakers::NoOne,
                            changing_authorized_action_takers_to_no_one_allowed: false,
                            changing_admin_action_takers_to_no_one_allowed: false,
                            self_changing_admin_action_takers_allowed: false,
                        },
                    ));
                }),
                None,
                None,
                platform_version,
            );

            let freeze_transition = BatchTransition::new_token_freeze_transition(
                token_id,
                identity.id(),
                contract.id(),
                0,
                identity_2.id(),
                None,
                None,
                &key,
                2,
                0,
                &signer,
                platform_version,
                None,
                None,
                None,
            )
            .expect("expect to create documents batch transition");

            let freeze_serialized_transition = freeze_transition
                .serialize_to_bytes()
                .expect("expected documents batch serialized state transition");

            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![freeze_serialized_transition.clone()],
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

            let token_frozen = platform
                .drive
                .fetch_identity_token_info(
                    token_id.to_buffer(),
                    identity_2.id().to_buffer(),
                    None,
                    platform_version,
                )
                .expect("expected to fetch token info")
                .map(|info| info.frozen());
            assert_eq!(token_frozen, Some(true));

            let unfreeze_transition = BatchTransition::new_token_unfreeze_transition(
                token_id,
                identity.id(),
                contract.id(),
                0,
                identity_2.id(),
                None,
                None,
                &key,
                3,
                0,
                &signer,
                platform_version,
                None,
                None,
                None,
            )
            .expect("expect to create documents batch transition");

            let unfreeze_serialized_transition = unfreeze_transition
                .serialize_to_bytes()
                .expect("expected documents batch serialized state transition");

            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![unfreeze_serialized_transition.clone()],
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

            let token_frozen = platform
                .drive
                .fetch_identity_token_info(
                    token_id.to_buffer(),
                    identity_2.id().to_buffer(),
                    None,
                    platform_version,
                )
                .expect("expected to fetch token info")
                .map(|info| info.frozen());
            assert_eq!(token_frozen, Some(false));
        }

        #[test]
        fn test_token_frozen_receive_balance_allowed_sending_not_allowed_till_unfrozen() {
            let platform_version = PlatformVersion::latest();
            let mut platform = TestPlatformBuilder::new()
                .with_latest_protocol_version()
                .build_with_mock_rpc()
                .set_genesis_state();

            let mut rng = StdRng::seed_from_u64(49853);

            let platform_state = platform.state.load();

            let (identity, signer, key) =
                setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));

            let (recipient, signer2, key2) =
                setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));

            let (contract, token_id) = create_token_contract_with_owner_identity(
                &mut platform,
                identity.id(),
                Some(|token_configuration: &mut TokenConfiguration| {
                    token_configuration.set_freeze_rules(ChangeControlRules::V0(
                        ChangeControlRulesV0 {
                            authorized_to_make_change: AuthorizedActionTakers::ContractOwner,
                            admin_action_takers: AuthorizedActionTakers::NoOne,
                            changing_authorized_action_takers_to_no_one_allowed: false,
                            changing_admin_action_takers_to_no_one_allowed: false,
                            self_changing_admin_action_takers_allowed: false,
                        },
                    ));
                    token_configuration.set_unfreeze_rules(ChangeControlRules::V0(
                        ChangeControlRulesV0 {
                            authorized_to_make_change: AuthorizedActionTakers::ContractOwner,
                            admin_action_takers: AuthorizedActionTakers::NoOne,
                            changing_authorized_action_takers_to_no_one_allowed: false,
                            changing_admin_action_takers_to_no_one_allowed: false,
                            self_changing_admin_action_takers_allowed: false,
                        },
                    ));
                }),
                None,
                None,
                platform_version,
            );

            let freeze_transition = BatchTransition::new_token_freeze_transition(
                token_id,
                identity.id(),
                contract.id(),
                0,
                recipient.id(),
                None,
                None,
                &key,
                2,
                0,
                &signer,
                platform_version,
                None,
                None,
                None,
            )
            .expect("expect to create documents batch transition");

            let freeze_serialized_transition = freeze_transition
                .serialize_to_bytes()
                .expect("expected documents batch serialized state transition");

            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![freeze_serialized_transition.clone()],
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

            let token_frozen = platform
                .drive
                .fetch_identity_token_info(
                    token_id.to_buffer(),
                    recipient.id().to_buffer(),
                    None,
                    platform_version,
                )
                .expect("expected to fetch token info")
                .map(|info| info.frozen());
            assert_eq!(token_frozen, Some(true));

            let token_transfer_transition = BatchTransition::new_token_transfer_transition(
                token_id,
                identity.id(),
                contract.id(),
                0,
                1337,
                recipient.id(),
                None,
                None,
                None,
                &key,
                3,
                0,
                &signer,
                platform_version,
                None,
                None,
                None,
            )
            .expect("expect to create documents batch transition");

            let token_transfer_serialized_transition = token_transfer_transition
                .serialize_to_bytes()
                .expect("expected documents batch serialized state transition");

            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![token_transfer_serialized_transition.clone()],
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

            let token_balance = platform
                .drive
                .fetch_identity_token_balance(
                    token_id.to_buffer(),
                    identity.id().to_buffer(),
                    None,
                    platform_version,
                )
                .expect("expected to fetch token balance");
            let expected_amount = 100000 - 1337;
            assert_eq!(token_balance, Some(expected_amount));

            let token_balance = platform
                .drive
                .fetch_identity_token_balance(
                    token_id.to_buffer(),
                    recipient.id().to_buffer(),
                    None,
                    platform_version,
                )
                .expect("expected to fetch token balance");
            let expected_amount = 1337;
            assert_eq!(token_balance, Some(expected_amount));

            //now let's try sending our balance

            let token_transfer_back_transition = BatchTransition::new_token_transfer_transition(
                token_id,
                recipient.id(),
                contract.id(),
                0,
                300,
                identity.id(),
                None,
                None,
                None,
                &key2,
                2,
                0,
                &signer2,
                platform_version,
                None,
                None,
                None,
            )
            .expect("expect to create documents batch transition");

            let token_transfer_back_serialized_transition = token_transfer_back_transition
                .serialize_to_bytes()
                .expect("expected documents batch serialized state transition");

            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![token_transfer_back_serialized_transition.clone()],
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
                [StateTransitionExecutionResult::PaidConsensusError(
                    ConsensusError::StateError(StateError::IdentityTokenAccountFrozenError(_)),
                    _
                )]
            );

            platform
                .drive
                .grove
                .commit_transaction(transaction)
                .unwrap()
                .expect("expected to commit transaction");

            // We expect no change

            let token_balance = platform
                .drive
                .fetch_identity_token_balance(
                    token_id.to_buffer(),
                    identity.id().to_buffer(),
                    None,
                    platform_version,
                )
                .expect("expected to fetch token balance");
            let expected_amount = 100000 - 1337;
            assert_eq!(token_balance, Some(expected_amount));

            let token_balance = platform
                .drive
                .fetch_identity_token_balance(
                    token_id.to_buffer(),
                    recipient.id().to_buffer(),
                    None,
                    platform_version,
                )
                .expect("expected to fetch token balance");
            let expected_amount = 1337;
            assert_eq!(token_balance, Some(expected_amount));

            let unfreeze_transition = BatchTransition::new_token_unfreeze_transition(
                token_id,
                identity.id(),
                contract.id(),
                0,
                recipient.id(),
                None,
                None,
                &key,
                4,
                0,
                &signer,
                platform_version,
                None,
                None,
                None,
            )
            .expect("expect to create documents batch transition");

            let unfreeze_serialized_transition = unfreeze_transition
                .serialize_to_bytes()
                .expect("expected documents batch serialized state transition");

            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![unfreeze_serialized_transition.clone()],
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

            let token_frozen = platform
                .drive
                .fetch_identity_token_info(
                    token_id.to_buffer(),
                    recipient.id().to_buffer(),
                    None,
                    platform_version,
                )
                .expect("expected to fetch token info")
                .map(|info| info.frozen());
            assert_eq!(token_frozen, Some(false));

            let token_transfer_transition = BatchTransition::new_token_transfer_transition(
                token_id,
                recipient.id(),
                contract.id(),
                0,
                300,
                identity.id(),
                None,
                None,
                None,
                &key2,
                3,
                0,
                &signer2,
                platform_version,
                None,
                None,
                None,
            )
            .expect("expect to create documents batch transition");

            let token_transfer_serialized_transition = token_transfer_transition
                .serialize_to_bytes()
                .expect("expected documents batch serialized state transition");

            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![token_transfer_serialized_transition.clone()],
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

            let token_balance = platform
                .drive
                .fetch_identity_token_balance(
                    token_id.to_buffer(),
                    identity.id().to_buffer(),
                    None,
                    platform_version,
                )
                .expect("expected to fetch token balance");
            let expected_amount = 100000 - 1337 + 300;
            assert_eq!(token_balance, Some(expected_amount));

            let token_balance = platform
                .drive
                .fetch_identity_token_balance(
                    token_id.to_buffer(),
                    recipient.id().to_buffer(),
                    None,
                    platform_version,
                )
                .expect("expected to fetch token balance");
            let expected_amount = 1337 - 300;
            assert_eq!(token_balance, Some(expected_amount));
        }

        #[test]
        fn test_token_frozen_receive_balance_may_not_be_allowed() {
            let platform_version = PlatformVersion::latest();
            let mut platform = TestPlatformBuilder::new()
                .with_latest_protocol_version()
                .build_with_mock_rpc()
                .set_genesis_state();

            let mut rng = StdRng::seed_from_u64(49853);

            let platform_state = platform.state.load();

            let (identity, signer, key) =
                setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));

            let (recipient, _, _) = setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));

            let (contract, token_id) = create_token_contract_with_owner_identity(
                &mut platform,
                identity.id(),
                Some(|token_configuration: &mut TokenConfiguration| {
                    token_configuration.allow_transfer_to_frozen_balance(false);

                    token_configuration.set_freeze_rules(ChangeControlRules::V0(
                        ChangeControlRulesV0 {
                            authorized_to_make_change: AuthorizedActionTakers::ContractOwner,
                            admin_action_takers: AuthorizedActionTakers::NoOne,
                            changing_authorized_action_takers_to_no_one_allowed: false,
                            changing_admin_action_takers_to_no_one_allowed: false,
                            self_changing_admin_action_takers_allowed: false,
                        },
                    ));
                    token_configuration.set_unfreeze_rules(ChangeControlRules::V0(
                        ChangeControlRulesV0 {
                            authorized_to_make_change: AuthorizedActionTakers::ContractOwner,
                            admin_action_takers: AuthorizedActionTakers::NoOne,
                            changing_authorized_action_takers_to_no_one_allowed: false,
                            changing_admin_action_takers_to_no_one_allowed: false,
                            self_changing_admin_action_takers_allowed: false,
                        },
                    ));
                }),
                None,
                None,
                platform_version,
            );

            let freeze_transition = BatchTransition::new_token_freeze_transition(
                token_id,
                identity.id(),
                contract.id(),
                0,
                recipient.id(),
                None,
                None,
                &key,
                2,
                0,
                &signer,
                platform_version,
                None,
                None,
                None,
            )
            .expect("expect to create documents batch transition");

            let freeze_serialized_transition = freeze_transition
                .serialize_to_bytes()
                .expect("expected documents batch serialized state transition");

            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![freeze_serialized_transition.clone()],
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

            let token_frozen = platform
                .drive
                .fetch_identity_token_info(
                    token_id.to_buffer(),
                    recipient.id().to_buffer(),
                    None,
                    platform_version,
                )
                .expect("expected to fetch token info")
                .map(|info| info.frozen());
            assert_eq!(token_frozen, Some(true));

            let token_transfer_transition = BatchTransition::new_token_transfer_transition(
                token_id,
                identity.id(),
                contract.id(),
                0,
                1337,
                recipient.id(),
                None,
                None,
                None,
                &key,
                3,
                0,
                &signer,
                platform_version,
                None,
                None,
                None,
            )
            .expect("expect to create documents batch transition");

            let token_transfer_serialized_transition = token_transfer_transition
                .serialize_to_bytes()
                .expect("expected documents batch serialized state transition");

            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![token_transfer_serialized_transition.clone()],
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
                [StateTransitionExecutionResult::PaidConsensusError(
                    ConsensusError::StateError(StateError::IdentityTokenAccountFrozenError(_)),
                    _
                )]
            );

            platform
                .drive
                .grove
                .commit_transaction(transaction)
                .unwrap()
                .expect("expected to commit transaction");

            let token_balance = platform
                .drive
                .fetch_identity_token_balance(
                    token_id.to_buffer(),
                    identity.id().to_buffer(),
                    None,
                    platform_version,
                )
                .expect("expected to fetch token balance");
            let expected_amount = 100000;
            assert_eq!(token_balance, Some(expected_amount));

            let token_balance = platform
                .drive
                .fetch_identity_token_balance(
                    token_id.to_buffer(),
                    recipient.id().to_buffer(),
                    None,
                    platform_version,
                )
                .expect("expected to fetch token balance");
            assert_eq!(token_balance, None);
        }
    }

    mod token_config_update_tests {
        use super::*;
        use dpp::data_contract::accessors::v1::DataContractV1Getters;
        use dpp::data_contract::associated_token::token_configuration_convention::TokenConfigurationConvention;
        use dpp::data_contract::associated_token::token_configuration_item::TokenConfigurationChangeItem;

        mod non_group {
            use super::*;
            #[test]
            fn test_token_config_update_by_owner_changing_total_max_supply() {
                let platform_version = PlatformVersion::latest();
                let mut platform = TestPlatformBuilder::new()
                    .with_latest_protocol_version()
                    .build_with_mock_rpc()
                    .set_genesis_state();

                let mut rng = StdRng::seed_from_u64(49853);

                let platform_state = platform.state.load();

                let (identity, signer, key) =
                    setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));

                let (contract, token_id) = create_token_contract_with_owner_identity(
                    &mut platform,
                    identity.id(),
                    Some(|token_configuration: &mut TokenConfiguration| {
                        token_configuration.set_max_supply_change_rules(ChangeControlRules::V0(
                            ChangeControlRulesV0 {
                                authorized_to_make_change: AuthorizedActionTakers::ContractOwner,
                                admin_action_takers: AuthorizedActionTakers::NoOne,
                                changing_authorized_action_takers_to_no_one_allowed: false,
                                changing_admin_action_takers_to_no_one_allowed: false,
                                self_changing_admin_action_takers_allowed: false,
                            },
                        ));
                    }),
                    None,
                    None,
                    platform_version,
                );

                let config_update_transition = BatchTransition::new_token_config_update_transition(
                    token_id,
                    identity.id(),
                    contract.id(),
                    0,
                    TokenConfigurationChangeItem::MaxSupply(Some(1000000)),
                    None,
                    None,
                    &key,
                    2,
                    0,
                    &signer,
                    platform_version,
                    None,
                    None,
                    None,
                )
                .expect("expect to create documents batch transition");

                let config_update_transition_serialized_transition = config_update_transition
                    .serialize_to_bytes()
                    .expect("expected documents batch serialized state transition");

                let transaction = platform.drive.grove.start_transaction();

                let processing_result = platform
                    .platform
                    .process_raw_state_transitions(
                        &vec![config_update_transition_serialized_transition.clone()],
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

                let contract = platform
                    .drive
                    .fetch_contract(
                        contract.id().to_buffer(),
                        None,
                        None,
                        None,
                        platform_version,
                    )
                    .unwrap()
                    .expect("expected to fetch token balance")
                    .expect("expected contract");
                let updated_token_config = contract
                    .contract
                    .expected_token_configuration(0)
                    .expect("expected token configuration");
                assert_eq!(updated_token_config.max_supply(), Some(1000000));
            }

            #[test]
            fn test_token_config_update_by_owner_changing_total_max_supply_to_less_than_current_supply(
            ) {
                let platform_version = PlatformVersion::latest();
                let mut platform = TestPlatformBuilder::new()
                    .with_latest_protocol_version()
                    .build_with_mock_rpc()
                    .set_genesis_state();

                let mut rng = StdRng::seed_from_u64(49853);

                let platform_state = platform.state.load();

                let (identity, signer, key) =
                    setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));

                let (contract, token_id) = create_token_contract_with_owner_identity(
                    &mut platform,
                    identity.id(),
                    Some(|token_configuration: &mut TokenConfiguration| {
                        token_configuration.set_max_supply_change_rules(ChangeControlRules::V0(
                            ChangeControlRulesV0 {
                                authorized_to_make_change: AuthorizedActionTakers::ContractOwner,
                                admin_action_takers: AuthorizedActionTakers::NoOne,
                                changing_authorized_action_takers_to_no_one_allowed: false,
                                changing_admin_action_takers_to_no_one_allowed: false,
                                self_changing_admin_action_takers_allowed: false,
                            },
                        ));
                    }),
                    None,
                    None,
                    platform_version,
                );

                let config_update_transition = BatchTransition::new_token_config_update_transition(
                    token_id,
                    identity.id(),
                    contract.id(),
                    0,
                    TokenConfigurationChangeItem::MaxSupply(Some(1000)),
                    None,
                    None,
                    &key,
                    2,
                    0,
                    &signer,
                    platform_version,
                    None,
                    None,
                    None,
                )
                .expect("expect to create documents batch transition");

                let config_update_transition_serialized_transition = config_update_transition
                    .serialize_to_bytes()
                    .expect("expected documents batch serialized state transition");

                let transaction = platform.drive.grove.start_transaction();

                let processing_result = platform
                    .platform
                    .process_raw_state_transitions(
                        &vec![config_update_transition_serialized_transition.clone()],
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
                    [StateTransitionExecutionResult::PaidConsensusError(
                        ConsensusError::StateError(
                            StateError::TokenSettingMaxSupplyToLessThanCurrentSupplyError(_)
                        ),
                        _
                    )]
                );

                platform
                    .drive
                    .grove
                    .commit_transaction(transaction)
                    .unwrap()
                    .expect("expected to commit transaction");

                let contract = platform
                    .drive
                    .fetch_contract(
                        contract.id().to_buffer(),
                        None,
                        None,
                        None,
                        platform_version,
                    )
                    .unwrap()
                    .expect("expected to fetch token balance")
                    .expect("expected contract");
                let updated_token_config = contract
                    .contract
                    .expected_token_configuration(0)
                    .expect("expected token configuration");
                assert_eq!(updated_token_config.max_supply(), None);
            }

            #[test]
            fn test_token_config_update_by_owner_change_admin_to_another_identity() {
                let platform_version = PlatformVersion::latest();
                let mut platform = TestPlatformBuilder::new()
                    .with_latest_protocol_version()
                    .build_with_mock_rpc()
                    .set_genesis_state();

                let mut rng = StdRng::seed_from_u64(49853);

                let platform_state = platform.state.load();

                let (identity, signer, key) =
                    setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));

                let (identity_2, signer_2, key_2) =
                    setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));

                let (contract, token_id) = create_token_contract_with_owner_identity(
                    &mut platform,
                    identity.id(),
                    Some(|token_configuration: &mut TokenConfiguration| {
                        token_configuration.set_max_supply_change_rules(ChangeControlRules::V0(
                            ChangeControlRulesV0 {
                                authorized_to_make_change: AuthorizedActionTakers::ContractOwner,
                                admin_action_takers: AuthorizedActionTakers::ContractOwner,
                                changing_authorized_action_takers_to_no_one_allowed: false,
                                changing_admin_action_takers_to_no_one_allowed: false,
                                self_changing_admin_action_takers_allowed: false,
                            },
                        ));
                    }),
                    None,
                    None,
                    platform_version,
                );

                let config_update_transition = BatchTransition::new_token_config_update_transition(
                    token_id,
                    identity.id(),
                    contract.id(),
                    0,
                    TokenConfigurationChangeItem::MaxSupplyControlGroup(
                        AuthorizedActionTakers::Identity(identity_2.id()),
                    ),
                    None,
                    None,
                    &key,
                    2,
                    0,
                    &signer,
                    platform_version,
                    None,
                    None,
                    None,
                )
                .expect("expect to create documents batch transition");

                let config_update_transition_serialized_transition = config_update_transition
                    .serialize_to_bytes()
                    .expect("expected documents batch serialized state transition");

                let transaction = platform.drive.grove.start_transaction();

                let processing_result = platform
                    .platform
                    .process_raw_state_transitions(
                        &vec![config_update_transition_serialized_transition.clone()],
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

                let config_update_transition = BatchTransition::new_token_config_update_transition(
                    token_id,
                    identity_2.id(),
                    contract.id(),
                    0,
                    TokenConfigurationChangeItem::MaxSupply(Some(1000000)),
                    None,
                    None,
                    &key_2,
                    2,
                    0,
                    &signer_2,
                    platform_version,
                    None,
                    None,
                    None,
                )
                .expect("expect to create documents batch transition");

                let config_update_transition_serialized_transition = config_update_transition
                    .serialize_to_bytes()
                    .expect("expected documents batch serialized state transition");

                let transaction = platform.drive.grove.start_transaction();

                let processing_result = platform
                    .platform
                    .process_raw_state_transitions(
                        &vec![config_update_transition_serialized_transition.clone()],
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

                let contract = platform
                    .drive
                    .fetch_contract(
                        contract.id().to_buffer(),
                        None,
                        None,
                        None,
                        platform_version,
                    )
                    .unwrap()
                    .expect("expected to fetch token balance")
                    .expect("expected contract");
                let updated_token_config = contract
                    .contract
                    .expected_token_configuration(0)
                    .expect("expected token configuration");
                assert_eq!(updated_token_config.max_supply(), Some(1000000));
            }

            #[test]
            fn test_token_config_update_by_owner_change_admin_to_a_non_existent_identity_error() {
                let platform_version = PlatformVersion::latest();
                let mut platform = TestPlatformBuilder::new()
                    .with_latest_protocol_version()
                    .build_with_mock_rpc()
                    .set_genesis_state();

                let mut rng = StdRng::seed_from_u64(49853);

                let platform_state = platform.state.load();

                let (identity, signer, key) =
                    setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));

                let identity_2_id = Identifier::random_with_rng(&mut rng);

                let (contract, token_id) = create_token_contract_with_owner_identity(
                    &mut platform,
                    identity.id(),
                    Some(|token_configuration: &mut TokenConfiguration| {
                        token_configuration.set_max_supply_change_rules(ChangeControlRules::V0(
                            ChangeControlRulesV0 {
                                authorized_to_make_change: AuthorizedActionTakers::ContractOwner,
                                admin_action_takers: AuthorizedActionTakers::ContractOwner,
                                changing_authorized_action_takers_to_no_one_allowed: false,
                                changing_admin_action_takers_to_no_one_allowed: false,
                                self_changing_admin_action_takers_allowed: false,
                            },
                        ));
                    }),
                    None,
                    None,
                    platform_version,
                );

                let config_update_transition = BatchTransition::new_token_config_update_transition(
                    token_id,
                    identity.id(),
                    contract.id(),
                    0,
                    TokenConfigurationChangeItem::MaxSupplyControlGroup(
                        AuthorizedActionTakers::Identity(identity_2_id),
                    ),
                    None,
                    None,
                    &key,
                    2,
                    0,
                    &signer,
                    platform_version,
                    None,
                    None,
                    None,
                )
                .expect("expect to create documents batch transition");

                let config_update_transition_serialized_transition = config_update_transition
                    .serialize_to_bytes()
                    .expect("expected documents batch serialized state transition");

                let transaction = platform.drive.grove.start_transaction();

                let processing_result = platform
                    .platform
                    .process_raw_state_transitions(
                        &vec![config_update_transition_serialized_transition.clone()],
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
                    [StateTransitionExecutionResult::PaidConsensusError(
                        ConsensusError::StateError(
                            StateError::NewAuthorizedActionTakerIdentityDoesNotExistError(_)
                        ),
                        _
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
            fn test_token_config_update_by_owner_change_admin_to_a_non_existent_group_error() {
                let platform_version = PlatformVersion::latest();
                let mut platform = TestPlatformBuilder::new()
                    .with_latest_protocol_version()
                    .build_with_mock_rpc()
                    .set_genesis_state();

                let mut rng = StdRng::seed_from_u64(49853);

                let platform_state = platform.state.load();

                let (identity, signer, key) =
                    setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));

                let (contract, token_id) = create_token_contract_with_owner_identity(
                    &mut platform,
                    identity.id(),
                    Some(|token_configuration: &mut TokenConfiguration| {
                        token_configuration.set_max_supply_change_rules(ChangeControlRules::V0(
                            ChangeControlRulesV0 {
                                authorized_to_make_change: AuthorizedActionTakers::ContractOwner,
                                admin_action_takers: AuthorizedActionTakers::ContractOwner,
                                changing_authorized_action_takers_to_no_one_allowed: false,
                                changing_admin_action_takers_to_no_one_allowed: false,
                                self_changing_admin_action_takers_allowed: false,
                            },
                        ));
                    }),
                    None,
                    None,
                    platform_version,
                );

                let config_update_transition = BatchTransition::new_token_config_update_transition(
                    token_id,
                    identity.id(),
                    contract.id(),
                    0,
                    TokenConfigurationChangeItem::MaxSupplyControlGroup(
                        AuthorizedActionTakers::Group(0),
                    ),
                    None,
                    None,
                    &key,
                    2,
                    0,
                    &signer,
                    platform_version,
                    None,
                    None,
                    None,
                )
                .expect("expect to create documents batch transition");

                let config_update_transition_serialized_transition = config_update_transition
                    .serialize_to_bytes()
                    .expect("expected documents batch serialized state transition");

                let transaction = platform.drive.grove.start_transaction();

                let processing_result = platform
                    .platform
                    .process_raw_state_transitions(
                        &vec![config_update_transition_serialized_transition.clone()],
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
                    [StateTransitionExecutionResult::PaidConsensusError(
                        ConsensusError::StateError(
                            StateError::NewAuthorizedActionTakerGroupDoesNotExistError(_)
                        ),
                        _
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
            fn test_token_config_update_by_owner_change_admin_to_main_group_not_set_error() {
                let platform_version = PlatformVersion::latest();
                let mut platform = TestPlatformBuilder::new()
                    .with_latest_protocol_version()
                    .build_with_mock_rpc()
                    .set_genesis_state();

                let mut rng = StdRng::seed_from_u64(49853);

                let platform_state = platform.state.load();

                let (identity, signer, key) =
                    setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));

                let (contract, token_id) = create_token_contract_with_owner_identity(
                    &mut platform,
                    identity.id(),
                    Some(|token_configuration: &mut TokenConfiguration| {
                        token_configuration.set_max_supply_change_rules(ChangeControlRules::V0(
                            ChangeControlRulesV0 {
                                authorized_to_make_change: AuthorizedActionTakers::ContractOwner,
                                admin_action_takers: AuthorizedActionTakers::ContractOwner,
                                changing_authorized_action_takers_to_no_one_allowed: false,
                                changing_admin_action_takers_to_no_one_allowed: false,
                                self_changing_admin_action_takers_allowed: false,
                            },
                        ));
                    }),
                    None,
                    None,
                    platform_version,
                );

                let config_update_transition = BatchTransition::new_token_config_update_transition(
                    token_id,
                    identity.id(),
                    contract.id(),
                    0,
                    TokenConfigurationChangeItem::MaxSupplyControlGroup(
                        AuthorizedActionTakers::MainGroup,
                    ),
                    None,
                    None,
                    &key,
                    2,
                    0,
                    &signer,
                    platform_version,
                    None,
                    None,
                    None,
                )
                .expect("expect to create documents batch transition");

                let config_update_transition_serialized_transition = config_update_transition
                    .serialize_to_bytes()
                    .expect("expected documents batch serialized state transition");

                let transaction = platform.drive.grove.start_transaction();

                let processing_result = platform
                    .platform
                    .process_raw_state_transitions(
                        &vec![config_update_transition_serialized_transition.clone()],
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
                    [StateTransitionExecutionResult::PaidConsensusError(
                        ConsensusError::StateError(
                            StateError::NewAuthorizedActionTakerMainGroupNotSetError(_)
                        ),
                        _
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

        mod with_group {
            use super::*;
            use dpp::data_contract::associated_token::token_configuration_localization::v0::TokenConfigurationLocalizationV0;

            #[test]
            fn test_token_config_update_by_group_member_changing_total_max_supply_not_using_group_gives_error(
            ) {
                let platform_version = PlatformVersion::latest();
                let mut platform = TestPlatformBuilder::new()
                    .with_latest_protocol_version()
                    .build_with_mock_rpc()
                    .set_genesis_state();

                let mut rng = StdRng::seed_from_u64(49853);

                let platform_state = platform.state.load();

                let (identity, signer, key) =
                    setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));

                let (identity_2, _, _) =
                    setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));

                let (contract, token_id) = create_token_contract_with_owner_identity(
                    &mut platform,
                    identity.id(),
                    Some(|token_configuration: &mut TokenConfiguration| {
                        token_configuration.set_max_supply_change_rules(ChangeControlRules::V0(
                            ChangeControlRulesV0 {
                                authorized_to_make_change: AuthorizedActionTakers::Group(0),
                                admin_action_takers: AuthorizedActionTakers::NoOne,
                                changing_authorized_action_takers_to_no_one_allowed: false,
                                changing_admin_action_takers_to_no_one_allowed: false,
                                self_changing_admin_action_takers_allowed: false,
                            },
                        ));
                    }),
                    None,
                    Some(
                        [(
                            0,
                            Group::V0(GroupV0 {
                                members: [(identity.id(), 1), (identity_2.id(), 1)].into(),
                                required_power: 2,
                            }),
                        )]
                        .into(),
                    ),
                    platform_version,
                );

                let config_update_transition = BatchTransition::new_token_config_update_transition(
                    token_id,
                    identity.id(),
                    contract.id(),
                    0,
                    TokenConfigurationChangeItem::MaxSupply(Some(1000000)),
                    None,
                    None,
                    &key,
                    2,
                    0,
                    &signer,
                    platform_version,
                    None,
                    None,
                    None,
                )
                .expect("expect to create documents batch transition");

                let config_update_transition_serialized_transition = config_update_transition
                    .serialize_to_bytes()
                    .expect("expected documents batch serialized state transition");

                let transaction = platform.drive.grove.start_transaction();

                let processing_result = platform
                    .platform
                    .process_raw_state_transitions(
                        &vec![config_update_transition_serialized_transition.clone()],
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
                    [StateTransitionExecutionResult::PaidConsensusError(
                        ConsensusError::StateError(StateError::UnauthorizedTokenActionError(_)),
                        _
                    )]
                );

                platform
                    .drive
                    .grove
                    .commit_transaction(transaction)
                    .unwrap()
                    .expect("expected to commit transaction");

                let contract = platform
                    .drive
                    .fetch_contract(
                        contract.id().to_buffer(),
                        None,
                        None,
                        None,
                        platform_version,
                    )
                    .unwrap()
                    .expect("expected to fetch token balance")
                    .expect("expected contract");
                let updated_token_config = contract
                    .contract
                    .expected_token_configuration(0)
                    .expect("expected token configuration");
                assert_eq!(updated_token_config.max_supply(), None);
            }

            #[test]
            fn test_token_config_update_by_group_member_changing_total_max_supply() {
                let platform_version = PlatformVersion::latest();
                let mut platform = TestPlatformBuilder::new()
                    .with_latest_protocol_version()
                    .build_with_mock_rpc()
                    .set_genesis_state();

                let mut rng = StdRng::seed_from_u64(49853);

                let platform_state = platform.state.load();

                let (identity, signer, key) =
                    setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));

                let (identity_2, signer_2, key_2) =
                    setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));

                let (contract, token_id) = create_token_contract_with_owner_identity(
                    &mut platform,
                    identity.id(),
                    Some(|token_configuration: &mut TokenConfiguration| {
                        token_configuration.set_max_supply_change_rules(ChangeControlRules::V0(
                            ChangeControlRulesV0 {
                                authorized_to_make_change: AuthorizedActionTakers::Group(0),
                                admin_action_takers: AuthorizedActionTakers::NoOne,
                                changing_authorized_action_takers_to_no_one_allowed: false,
                                changing_admin_action_takers_to_no_one_allowed: false,
                                self_changing_admin_action_takers_allowed: false,
                            },
                        ));
                    }),
                    None,
                    Some(
                        [(
                            0,
                            Group::V0(GroupV0 {
                                members: [(identity.id(), 1), (identity_2.id(), 1)].into(),
                                required_power: 2,
                            }),
                        )]
                        .into(),
                    ),
                    platform_version,
                );

                let action_id = TokenConfigUpdateTransition::calculate_action_id_with_fields(
                    token_id.as_bytes(),
                    identity.id().as_bytes(),
                    2,
                    TokenConfigurationChangeItem::MaxSupply(Some(1000000)).u8_item_index(),
                );

                let config_update_transition = BatchTransition::new_token_config_update_transition(
                    token_id,
                    identity.id(),
                    contract.id(),
                    0,
                    TokenConfigurationChangeItem::MaxSupply(Some(1000000)),
                    None,
                    Some(GroupStateTransitionInfoStatus::GroupStateTransitionInfoProposer(0)),
                    &key,
                    2,
                    0,
                    &signer,
                    platform_version,
                    None,
                    None,
                    None,
                )
                .expect("expect to create documents batch transition");

                let config_update_transition_serialized_transition = config_update_transition
                    .serialize_to_bytes()
                    .expect("expected documents batch serialized state transition");

                let transaction = platform.drive.grove.start_transaction();

                let processing_result = platform
                    .platform
                    .process_raw_state_transitions(
                        &vec![config_update_transition_serialized_transition.clone()],
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

                let new_contract = platform
                    .drive
                    .fetch_contract(
                        contract.id().to_buffer(),
                        None,
                        None,
                        None,
                        platform_version,
                    )
                    .unwrap()
                    .expect("expected to fetch token balance")
                    .expect("expected contract");
                let updated_token_config = new_contract
                    .contract
                    .expected_token_configuration(0)
                    .expect("expected token configuration");
                assert_eq!(updated_token_config.max_supply(), None);

                let config_update_transition = BatchTransition::new_token_config_update_transition(
                    token_id,
                    identity_2.id(),
                    contract.id(),
                    0,
                    TokenConfigurationChangeItem::MaxSupply(Some(1000000)),
                    None,
                    Some(
                        GroupStateTransitionInfoStatus::GroupStateTransitionInfoOtherSigner(
                            GroupStateTransitionInfo {
                                group_contract_position: 0,
                                action_id,
                                action_is_proposer: false,
                            },
                        ),
                    ),
                    &key_2,
                    2,
                    0,
                    &signer_2,
                    platform_version,
                    None,
                    None,
                    None,
                )
                .expect("expect to create documents batch transition");

                let config_update_transition_serialized_transition = config_update_transition
                    .serialize_to_bytes()
                    .expect("expected documents batch serialized state transition");

                let transaction = platform.drive.grove.start_transaction();

                let processing_result = platform
                    .platform
                    .process_raw_state_transitions(
                        &vec![config_update_transition_serialized_transition.clone()],
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

                let new_contract = platform
                    .drive
                    .fetch_contract(
                        contract.id().to_buffer(),
                        None,
                        None,
                        None,
                        platform_version,
                    )
                    .unwrap()
                    .expect("expected to fetch token balance")
                    .expect("expected contract");
                let updated_token_config = new_contract
                    .contract
                    .expected_token_configuration(0)
                    .expect("expected token configuration");
                assert_eq!(updated_token_config.max_supply(), Some(1000000));
            }

            #[test]
            fn test_token_config_change_own_admin_group_give_control_power_and_change_admin_back() {
                let platform_version = PlatformVersion::latest();
                let mut platform = TestPlatformBuilder::new()
                    .with_latest_protocol_version()
                    .build_with_mock_rpc()
                    .set_genesis_state();

                let mut rng = StdRng::seed_from_u64(49853);

                let platform_state = platform.state.load();

                let (identity, signer, key) =
                    setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));

                let (identity_2, signer_2, key_2) =
                    setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));

                let (identity_3, signer_3, key_3) =
                    setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));

                let (identity_4, signer_4, key_4) =
                    setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));

                let (identity_5, signer_5, key_5) =
                    setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));

                let (contract, token_id) = create_token_contract_with_owner_identity(
                    &mut platform,
                    identity.id(),
                    Some(|token_configuration: &mut TokenConfiguration| {
                        token_configuration.set_conventions_change_rules(ChangeControlRules::V0(
                            ChangeControlRulesV0 {
                                authorized_to_make_change: AuthorizedActionTakers::Group(0),
                                admin_action_takers: AuthorizedActionTakers::Group(1),
                                changing_authorized_action_takers_to_no_one_allowed: false,
                                changing_admin_action_takers_to_no_one_allowed: false,
                                self_changing_admin_action_takers_allowed: true,
                            },
                        ));
                    }),
                    None,
                    Some(
                        [
                            (
                                0,
                                Group::V0(GroupV0 {
                                    members: [(identity.id(), 1), (identity_2.id(), 1)].into(),
                                    required_power: 2,
                                }),
                            ),
                            (
                                1,
                                Group::V0(GroupV0 {
                                    members: [
                                        (identity_3.id(), 1),
                                        (identity_4.id(), 1),
                                        (identity_5.id(), 1),
                                    ]
                                    .into(),
                                    required_power: 2,
                                }),
                            ),
                        ]
                        .into(),
                    ),
                    platform_version,
                );

                let action_id = TokenConfigUpdateTransition::calculate_action_id_with_fields(
                    token_id.as_bytes(),
                    identity_3.id().as_bytes(),
                    2,
                    TokenConfigurationChangeItem::ConventionsAdminGroup(
                        AuthorizedActionTakers::Group(0),
                    )
                    .u8_item_index(),
                );

                let config_update_transition = BatchTransition::new_token_config_update_transition(
                    token_id,
                    identity_3.id(),
                    contract.id(),
                    0,
                    TokenConfigurationChangeItem::ConventionsAdminGroup(
                        AuthorizedActionTakers::Group(0),
                    ),
                    None,
                    Some(GroupStateTransitionInfoStatus::GroupStateTransitionInfoProposer(1)),
                    &key_3,
                    2,
                    0,
                    &signer_3,
                    platform_version,
                    None,
                    None,
                    None,
                )
                .expect("expect to create documents batch transition");

                let config_update_transition_serialized_transition = config_update_transition
                    .serialize_to_bytes()
                    .expect("expected documents batch serialized state transition");

                let transaction = platform.drive.grove.start_transaction();

                let processing_result = platform
                    .platform
                    .process_raw_state_transitions(
                        &vec![config_update_transition_serialized_transition.clone()],
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

                let new_contract = platform
                    .drive
                    .fetch_contract(
                        contract.id().to_buffer(),
                        None,
                        None,
                        None,
                        platform_version,
                    )
                    .unwrap()
                    .expect("expected to fetch token balance")
                    .expect("expected contract");
                let updated_token_config = new_contract
                    .contract
                    .expected_token_configuration(0)
                    .expect("expected token configuration");
                assert_eq!(
                    updated_token_config
                        .conventions_change_rules()
                        .admin_action_takers(),
                    &AuthorizedActionTakers::Group(1)
                );

                let config_update_transition = BatchTransition::new_token_config_update_transition(
                    token_id,
                    identity_4.id(),
                    contract.id(),
                    0,
                    TokenConfigurationChangeItem::ConventionsAdminGroup(
                        AuthorizedActionTakers::Group(0),
                    ),
                    None,
                    Some(
                        GroupStateTransitionInfoStatus::GroupStateTransitionInfoOtherSigner(
                            GroupStateTransitionInfo {
                                group_contract_position: 1,
                                action_id,
                                action_is_proposer: false,
                            },
                        ),
                    ),
                    &key_4,
                    2,
                    0,
                    &signer_4,
                    platform_version,
                    None,
                    None,
                    None,
                )
                .expect("expect to create documents batch transition");

                let config_update_transition_serialized_transition = config_update_transition
                    .serialize_to_bytes()
                    .expect("expected documents batch serialized state transition");

                let transaction = platform.drive.grove.start_transaction();

                let processing_result = platform
                    .platform
                    .process_raw_state_transitions(
                        &vec![config_update_transition_serialized_transition.clone()],
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

                let new_contract = platform
                    .drive
                    .fetch_contract(
                        contract.id().to_buffer(),
                        None,
                        None,
                        None,
                        platform_version,
                    )
                    .unwrap()
                    .expect("expected to fetch token balance")
                    .expect("expected contract");
                let updated_token_config = new_contract
                    .contract
                    .expected_token_configuration(0)
                    .expect("expected token configuration");
                assert_eq!(
                    updated_token_config
                        .conventions_change_rules()
                        .admin_action_takers(),
                    &AuthorizedActionTakers::Group(0)
                );
                assert_eq!(new_contract.contract.version(), 2);

                // 5 is late to the game, admin control has already been transferred, he should get an error
                let config_update_transition = BatchTransition::new_token_config_update_transition(
                    token_id,
                    identity_5.id(),
                    contract.id(),
                    0,
                    TokenConfigurationChangeItem::ConventionsAdminGroup(
                        AuthorizedActionTakers::Group(0),
                    ),
                    None,
                    Some(
                        GroupStateTransitionInfoStatus::GroupStateTransitionInfoOtherSigner(
                            GroupStateTransitionInfo {
                                group_contract_position: 1,
                                action_id,
                                action_is_proposer: false,
                            },
                        ),
                    ),
                    &key_5,
                    2,
                    0,
                    &signer_5,
                    platform_version,
                    None,
                    None,
                    None,
                )
                .expect("expect to create documents batch transition");

                let config_update_transition_serialized_transition = config_update_transition
                    .serialize_to_bytes()
                    .expect("expected documents batch serialized state transition");

                let transaction = platform.drive.grove.start_transaction();

                let processing_result = platform
                    .platform
                    .process_raw_state_transitions(
                        &vec![config_update_transition_serialized_transition.clone()],
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
                    [StateTransitionExecutionResult::PaidConsensusError(
                        ConsensusError::StateError(StateError::GroupActionAlreadyCompletedError(_)),
                        _
                    )]
                );

                platform
                    .drive
                    .grove
                    .commit_transaction(transaction)
                    .unwrap()
                    .expect("expected to commit transaction");

                // Let's try if he proposes it now

                let config_update_transition = BatchTransition::new_token_config_update_transition(
                    token_id,
                    identity_5.id(),
                    contract.id(),
                    0,
                    TokenConfigurationChangeItem::ConventionsAdminGroup(
                        AuthorizedActionTakers::Group(0),
                    ),
                    None,
                    Some(GroupStateTransitionInfoStatus::GroupStateTransitionInfoProposer(1)),
                    &key_5,
                    3,
                    0,
                    &signer_5,
                    platform_version,
                    None,
                    None,
                    None,
                )
                .expect("expect to create documents batch transition");

                let config_update_transition_serialized_transition = config_update_transition
                    .serialize_to_bytes()
                    .expect("expected documents batch serialized state transition");

                let transaction = platform.drive.grove.start_transaction();

                let processing_result = platform
                    .platform
                    .process_raw_state_transitions(
                        &vec![config_update_transition_serialized_transition.clone()],
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
                    [StateTransitionExecutionResult::PaidConsensusError(
                        ConsensusError::StateError(StateError::UnauthorizedTokenActionError(_)),
                        _
                    )]
                );

                platform
                    .drive
                    .grove
                    .commit_transaction(transaction)
                    .unwrap()
                    .expect("expected to commit transaction");

                // Now let's have Group 0 change the control of the conventions to identity 2 only

                let action_id_change_control =
                    TokenConfigUpdateTransition::calculate_action_id_with_fields(
                        token_id.as_bytes(),
                        identity.id().as_bytes(),
                        2,
                        TokenConfigurationChangeItem::ConventionsControlGroup(
                            AuthorizedActionTakers::Identity(identity_2.id()),
                        )
                        .u8_item_index(),
                    );

                let config_update_transition = BatchTransition::new_token_config_update_transition(
                    token_id,
                    identity.id(),
                    contract.id(),
                    0,
                    TokenConfigurationChangeItem::ConventionsControlGroup(
                        AuthorizedActionTakers::Identity(identity_2.id()),
                    ),
                    None,
                    Some(GroupStateTransitionInfoStatus::GroupStateTransitionInfoProposer(0)),
                    &key,
                    2,
                    0,
                    &signer,
                    platform_version,
                    None,
                    None,
                    None,
                )
                .expect("expect to create documents batch transition");

                let config_update_transition_serialized_transition = config_update_transition
                    .serialize_to_bytes()
                    .expect("expected documents batch serialized state transition");

                let transaction = platform.drive.grove.start_transaction();

                let processing_result = platform
                    .platform
                    .process_raw_state_transitions(
                        &vec![config_update_transition_serialized_transition.clone()],
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

                let config_update_transition = BatchTransition::new_token_config_update_transition(
                    token_id,
                    identity_2.id(),
                    contract.id(),
                    0,
                    TokenConfigurationChangeItem::ConventionsControlGroup(
                        AuthorizedActionTakers::Identity(identity_2.id()),
                    ),
                    None,
                    Some(
                        GroupStateTransitionInfoStatus::GroupStateTransitionInfoOtherSigner(
                            GroupStateTransitionInfo {
                                group_contract_position: 0,
                                action_id: action_id_change_control,
                                action_is_proposer: false,
                            },
                        ),
                    ),
                    &key_2,
                    2,
                    0,
                    &signer_2,
                    platform_version,
                    None,
                    None,
                    None,
                )
                .expect("expect to create documents batch transition");

                let config_update_transition_serialized_transition = config_update_transition
                    .serialize_to_bytes()
                    .expect("expected documents batch serialized state transition");

                let transaction = platform.drive.grove.start_transaction();

                let processing_result = platform
                    .platform
                    .process_raw_state_transitions(
                        &vec![config_update_transition_serialized_transition.clone()],
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

                let new_contract = platform
                    .drive
                    .fetch_contract(
                        contract.id().to_buffer(),
                        None,
                        None,
                        None,
                        platform_version,
                    )
                    .unwrap()
                    .expect("expected to fetch token balance")
                    .expect("expected contract");
                let updated_token_config = new_contract
                    .contract
                    .expected_token_configuration(0)
                    .expect("expected token configuration");
                assert_eq!(
                    updated_token_config
                        .conventions_change_rules()
                        .authorized_to_make_change_action_takers(),
                    &AuthorizedActionTakers::Identity(identity_2.id())
                );
                assert_eq!(new_contract.contract.version(), 3);

                // Now let's have Group 0 hand it back to Group 1

                let action_id_return = TokenConfigUpdateTransition::calculate_action_id_with_fields(
                    token_id.as_bytes(),
                    identity.id().as_bytes(),
                    3,
                    TokenConfigurationChangeItem::ConventionsAdminGroup(
                        AuthorizedActionTakers::Group(1),
                    )
                    .u8_item_index(),
                );

                let config_update_transition = BatchTransition::new_token_config_update_transition(
                    token_id,
                    identity.id(),
                    contract.id(),
                    0,
                    TokenConfigurationChangeItem::ConventionsAdminGroup(
                        AuthorizedActionTakers::Group(1),
                    ),
                    None,
                    Some(GroupStateTransitionInfoStatus::GroupStateTransitionInfoProposer(0)),
                    &key,
                    3,
                    0,
                    &signer,
                    platform_version,
                    None,
                    None,
                    None,
                )
                .expect("expect to create documents batch transition");

                let config_update_transition_serialized_transition = config_update_transition
                    .serialize_to_bytes()
                    .expect("expected documents batch serialized state transition");

                let transaction = platform.drive.grove.start_transaction();

                let processing_result = platform
                    .platform
                    .process_raw_state_transitions(
                        &vec![config_update_transition_serialized_transition.clone()],
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

                let config_update_transition = BatchTransition::new_token_config_update_transition(
                    token_id,
                    identity_2.id(),
                    contract.id(),
                    0,
                    TokenConfigurationChangeItem::ConventionsAdminGroup(
                        AuthorizedActionTakers::Group(1),
                    ),
                    None,
                    Some(
                        GroupStateTransitionInfoStatus::GroupStateTransitionInfoOtherSigner(
                            GroupStateTransitionInfo {
                                group_contract_position: 0,
                                action_id: action_id_return,
                                action_is_proposer: false,
                            },
                        ),
                    ),
                    &key_2,
                    3,
                    0,
                    &signer_2,
                    platform_version,
                    None,
                    None,
                    None,
                )
                .expect("expect to create documents batch transition");

                let config_update_transition_serialized_transition = config_update_transition
                    .serialize_to_bytes()
                    .expect("expected documents batch serialized state transition");

                let transaction = platform.drive.grove.start_transaction();

                let processing_result = platform
                    .platform
                    .process_raw_state_transitions(
                        &vec![config_update_transition_serialized_transition.clone()],
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

                let new_contract = platform
                    .drive
                    .fetch_contract(
                        contract.id().to_buffer(),
                        None,
                        None,
                        None,
                        platform_version,
                    )
                    .unwrap()
                    .expect("expected to fetch token balance")
                    .expect("expected contract");
                let updated_token_config = new_contract
                    .contract
                    .expected_token_configuration(0)
                    .expect("expected token configuration");
                assert_eq!(
                    updated_token_config
                        .conventions_change_rules()
                        .admin_action_takers(),
                    &AuthorizedActionTakers::Group(1)
                );
                assert_eq!(new_contract.contract.version(), 4);

                // Not let's try identity 3 to change the conventions (should fail)

                let config_update_transition = BatchTransition::new_token_config_update_transition(
                    token_id,
                    identity_3.id(),
                    contract.id(),
                    0,
                    TokenConfigurationChangeItem::Conventions(TokenConfigurationConvention::V0(
                        TokenConfigurationConventionV0 {
                            localizations: [(
                                "en".to_string(),
                                TokenConfigurationLocalizationV0 {
                                    should_capitalize: true,
                                    singular_form: "garzon".to_string(),
                                    plural_form: "garzons".to_string(),
                                }
                                .into(),
                            )]
                            .into(),
                            decimals: 8,
                        },
                    )),
                    None,
                    None,
                    &key_3,
                    3,
                    0,
                    &signer_3,
                    platform_version,
                    None,
                    None,
                    None,
                )
                .expect("expect to create documents batch transition");

                let config_update_transition_serialized_transition = config_update_transition
                    .serialize_to_bytes()
                    .expect("expected documents batch serialized state transition");

                let transaction = platform.drive.grove.start_transaction();

                let processing_result = platform
                    .platform
                    .process_raw_state_transitions(
                        &vec![config_update_transition_serialized_transition.clone()],
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
                    [StateTransitionExecutionResult::PaidConsensusError(
                        ConsensusError::StateError(StateError::UnauthorizedTokenActionError(_)),
                        _
                    )]
                );

                platform
                    .drive
                    .grove
                    .commit_transaction(transaction)
                    .unwrap()
                    .expect("expected to commit transaction");

                // Not let's try identity 2 to change the conventions (should succeed)

                let config_update_transition = BatchTransition::new_token_config_update_transition(
                    token_id,
                    identity_2.id(),
                    contract.id(),
                    0,
                    TokenConfigurationChangeItem::Conventions(TokenConfigurationConvention::V0(
                        TokenConfigurationConventionV0 {
                            localizations: [(
                                "en".to_string(),
                                TokenConfigurationLocalizationV0 {
                                    should_capitalize: true,
                                    singular_form: "garzon".to_string(),
                                    plural_form: "garzons".to_string(),
                                }
                                .into(),
                            )]
                            .into(),
                            decimals: 8,
                        },
                    )),
                    None,
                    None,
                    &key_2,
                    4,
                    0,
                    &signer_2,
                    platform_version,
                    None,
                    None,
                    None,
                )
                .expect("expect to create documents batch transition");

                let config_update_transition_serialized_transition = config_update_transition
                    .serialize_to_bytes()
                    .expect("expected documents batch serialized state transition");

                let transaction = platform.drive.grove.start_transaction();

                let processing_result = platform
                    .platform
                    .process_raw_state_transitions(
                        &vec![config_update_transition_serialized_transition.clone()],
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
    }
}
