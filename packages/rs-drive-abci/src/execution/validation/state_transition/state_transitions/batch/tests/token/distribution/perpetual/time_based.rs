use super::*;
use crate::execution::validation::state_transition::tests::{
    create_token_contract_with_owner_identity, setup_identity,
};
use crate::test::helpers::setup::TestPlatformBuilder;
use dpp::dash_to_credits;
use dpp::data_contract::TokenConfiguration;
use dpp::state_transition::batch_transition::BatchTransition;
use platform_version::version::PlatformVersion;
use rand::prelude::StdRng;
mod perpetual_distribution_time {
    use dpp::block::epoch::Epoch;
    use dpp::data_contract::associated_token::token_distribution_key::TokenDistributionType;
    use dpp::data_contract::associated_token::token_perpetual_distribution::distribution_function::{DistributionFunction, MAX_DISTRIBUTION_PARAM, MAX_LINEAR_SLOPE_A_PARAM};
    use dpp::data_contract::associated_token::token_perpetual_distribution::distribution_recipient::TokenDistributionRecipient;
    use dpp::data_contract::associated_token::token_perpetual_distribution::reward_distribution_type::RewardDistributionType;
    use dpp::data_contract::associated_token::token_perpetual_distribution::TokenPerpetualDistribution;
    use dpp::data_contract::associated_token::token_perpetual_distribution::v0::TokenPerpetualDistributionV0;
    use crate::test::helpers::fast_forward_to_block::fast_forward_to_block;
    use super::*;
    #[test]
    fn test_token_perpetual_distribution_time_claim_linear_and_claim_again() {
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
                    .set_perpetual_distribution(Some(TokenPerpetualDistribution::V0(
                        TokenPerpetualDistributionV0 {
                            distribution_type: RewardDistributionType::TimeBasedDistribution {
                                // every 3600 seconds, meaning every hour
                                interval: 3_600_000,
                                function: DistributionFunction::FixedAmount { amount: 50 },
                            },
                            distribution_recipient: TokenDistributionRecipient::ContractOwner,
                        },
                    )));
            }),
            None,
            None,
            platform_version,
        );

        // 5 hours later
        fast_forward_to_block(&platform, 18_000_000, 40, 42, 1, false);

        let claim_transition = BatchTransition::new_token_claim_transition(
            token_id,
            identity.id(),
            contract.id(),
            0,
            TokenDistributionType::Perpetual,
            None,
            &key,
            2,
            0,
            &signer,
            platform_version,
            None,
        )
        .expect("expect to create documents batch transition");

        let claim_serialized_transition = claim_transition
            .serialize_to_bytes()
            .expect("expected documents batch serialized state transition");

        let transaction = platform.drive.grove.start_transaction();

        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &vec![claim_serialized_transition.clone()],
                &platform_state,
                &BlockInfo {
                    time_ms: 18_000_000,
                    height: 41,
                    core_height: 42,
                    epoch: Epoch::new(1).unwrap(),
                },
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
        // Since height is 42 we had 5 events * 50 (+ 100000 which was data contract owner base).
        assert_eq!(token_balance, Some(100250));

        fast_forward_to_block(&platform, 19_000_000, 45, 42, 1, false);

        let claim_transition = BatchTransition::new_token_claim_transition(
            token_id,
            identity.id(),
            contract.id(),
            0,
            TokenDistributionType::Perpetual,
            None,
            &key,
            3,
            0,
            &signer,
            platform_version,
            None,
        )
        .expect("expect to create documents batch transition");

        let claim_serialized_transition = claim_transition
            .serialize_to_bytes()
            .expect("expected documents batch serialized state transition");

        let transaction = platform.drive.grove.start_transaction();

        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &vec![claim_serialized_transition.clone()],
                &platform_state,
                &BlockInfo {
                    time_ms: 19_100_000,
                    height: 46,
                    core_height: 42,
                    epoch: Epoch::new(1).unwrap(),
                },
                &transaction,
                platform_version,
                false,
                None,
            )
            .expect("expected to process state transition");

        assert_matches!(
            processing_result.execution_results().as_slice(),
            [PaidConsensusError(
                ConsensusError::StateError(StateError::InvalidTokenClaimNoCurrentRewards(_)),
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
        assert_eq!(token_balance, Some(100250));

        fast_forward_to_block(&platform, 22_000_000, 49, 42, 1, false);

        let claim_transition = BatchTransition::new_token_claim_transition(
            token_id,
            identity.id(),
            contract.id(),
            0,
            TokenDistributionType::Perpetual,
            None,
            &key,
            4,
            0,
            &signer,
            platform_version,
            None,
        )
        .expect("expect to create documents batch transition");

        let claim_serialized_transition = claim_transition
            .serialize_to_bytes()
            .expect("expected documents batch serialized state transition");

        let transaction = platform.drive.grove.start_transaction();

        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &vec![claim_serialized_transition.clone()],
                &platform_state,
                &BlockInfo {
                    time_ms: 22_100_000,
                    height: 50,
                    core_height: 42,
                    epoch: Epoch::new(1).unwrap(),
                },
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
        // An extra event
        assert_eq!(token_balance, Some(100300));
    }

    #[test]
    fn test_token_perpetual_distribution_not_claimant() {
        let platform_version = PlatformVersion::latest();
        let mut platform = TestPlatformBuilder::new()
            .with_latest_protocol_version()
            .build_with_mock_rpc()
            .set_genesis_state();

        let mut rng = StdRng::seed_from_u64(49853);

        let platform_state = platform.state.load();

        let (identity, signer, key) =
            setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));

        let (identity_2, _, _) = setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));

        let (contract, token_id) = create_token_contract_with_owner_identity(
            &mut platform,
            identity.id(),
            Some(|token_configuration: &mut TokenConfiguration| {
                token_configuration
                    .distribution_rules_mut()
                    .set_perpetual_distribution(Some(TokenPerpetualDistribution::V0(
                        TokenPerpetualDistributionV0 {
                            distribution_type: RewardDistributionType::TimeBasedDistribution {
                                // every 3600 seconds, meaning every hour
                                interval: 3_600_000,
                                function: DistributionFunction::FixedAmount { amount: 50 },
                            },
                            // we give to identity 2
                            distribution_recipient: TokenDistributionRecipient::Identity(
                                identity_2.id(),
                            ),
                        },
                    )));
            }),
            None,
            None,
            platform_version,
        );

        fast_forward_to_block(&platform, 10_200_000_000, 40, 42, 1, false); //25 years later

        // claiming for identity 1 (contract owner)
        let claim_transition = BatchTransition::new_token_claim_transition(
            token_id,
            identity.id(),
            contract.id(),
            0,
            TokenDistributionType::Perpetual,
            None,
            &key,
            2,
            0,
            &signer,
            platform_version,
            None,
        )
        .expect("expect to create documents batch transition");

        let claim_serialized_transition = claim_transition
            .serialize_to_bytes()
            .expect("expected documents batch serialized state transition");

        let transaction = platform.drive.grove.start_transaction();

        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &vec![claim_serialized_transition.clone()],
                &platform_state,
                &BlockInfo {
                    time_ms: 10_200_100_000,
                    height: 41,
                    core_height: 42,
                    epoch: Epoch::new(1).unwrap(),
                },
                &transaction,
                platform_version,
                false,
                None,
            )
            .expect("expected to process state transition");

        assert_matches!(
            processing_result.execution_results().as_slice(),
            [PaidConsensusError(
                ConsensusError::StateError(StateError::InvalidTokenClaimWrongClaimant(_)),
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

        let token_balance_2 = platform
            .drive
            .fetch_identity_token_balance(
                token_id.to_buffer(),
                identity_2.id().to_buffer(),
                None,
                platform_version,
            )
            .expect("expected to fetch token balance");
        assert_eq!(token_balance_2, None);
    }

    #[test]
    fn test_token_perpetual_distribution_time_claim_linear_given_to_specific_identity() {
        let platform_version = PlatformVersion::latest();
        let mut platform = TestPlatformBuilder::new()
            .with_latest_protocol_version()
            .build_with_mock_rpc()
            .set_genesis_state();

        let mut rng = StdRng::seed_from_u64(49853);

        let platform_state = platform.state.load();

        let (identity, _, _) = setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));

        let (identity_2, signer_2, key_2) =
            setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));

        let (contract, token_id) = create_token_contract_with_owner_identity(
            &mut platform,
            identity.id(),
            Some(|token_configuration: &mut TokenConfiguration| {
                token_configuration
                    .distribution_rules_mut()
                    .set_perpetual_distribution(Some(TokenPerpetualDistribution::V0(
                        TokenPerpetualDistributionV0 {
                            distribution_type: RewardDistributionType::TimeBasedDistribution {
                                // every 3600 seconds, meaning every hour
                                interval: 3_600_000,
                                function: DistributionFunction::FixedAmount { amount: 50 },
                            },
                            distribution_recipient: TokenDistributionRecipient::Identity(
                                identity_2.id(),
                            ),
                        },
                    )));
            }),
            None,
            None,
            platform_version,
        );

        // 5 hours later
        fast_forward_to_block(&platform, 18_000_000, 40, 42, 1, false);

        let claim_transition = BatchTransition::new_token_claim_transition(
            token_id,
            identity_2.id(),
            contract.id(),
            0,
            TokenDistributionType::Perpetual,
            None,
            &key_2,
            2,
            0,
            &signer_2,
            platform_version,
            None,
        )
        .expect("expect to create documents batch transition");

        let claim_serialized_transition = claim_transition
            .serialize_to_bytes()
            .expect("expected documents batch serialized state transition");

        let transaction = platform.drive.grove.start_transaction();

        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &vec![claim_serialized_transition.clone()],
                &platform_state,
                &BlockInfo {
                    time_ms: 18_100_000,
                    height: 41,
                    core_height: 42,
                    epoch: Epoch::new(1).unwrap(),
                },
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
                identity_2.id().to_buffer(),
                None,
                platform_version,
            )
            .expect("expected to fetch token balance");
        // Since time is slightly over 5 hours we had 5 events x 5.
        assert_eq!(token_balance, Some(250));
    }

    #[test]
    fn test_token_perpetual_distribution_time_linear() {
        let platform_version = PlatformVersion::latest();
        let mut platform = TestPlatformBuilder::new()
            .with_latest_protocol_version()
            .build_with_mock_rpc()
            .set_genesis_state();

        let mut rng = StdRng::seed_from_u64(4981);

        let platform_state = platform.state.load();

        let (identity, _, _) = setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));

        let (identity_2, signer_2, key_2) =
            setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));

        let (contract, token_id) = create_token_contract_with_owner_identity(
            &mut platform,
            identity.id(),
            Some(|token_configuration: &mut TokenConfiguration| {
                token_configuration
                    .distribution_rules_mut()
                    .set_perpetual_distribution(Some(TokenPerpetualDistribution::V0(
                        TokenPerpetualDistributionV0 {
                            distribution_type: RewardDistributionType::TimeBasedDistribution {
                                // every 1 millisecond
                                interval: 1,
                                function: DistributionFunction::Linear {
                                    a: 1, // Increase in slope y = x
                                    d: 1, // No division, simple emission
                                    start_step: None,
                                    starting_amount: 0,
                                    min_value: None,
                                    max_value: None,
                                },
                            },
                            distribution_recipient: TokenDistributionRecipient::Identity(
                                identity_2.id(),
                            ),
                        },
                    )));
            }),
            None,
            None,
            platform_version,
        );

        // 5 hours later
        fast_forward_to_block(&platform, 18_000_000, 40, 42, 1, false);

        // We are only claiming for 256 cycles
        let claim_transition = BatchTransition::new_token_claim_transition(
            token_id,
            identity_2.id(),
            contract.id(),
            0,
            TokenDistributionType::Perpetual,
            None,
            &key_2,
            2,
            0,
            &signer_2,
            platform_version,
            None,
        )
        .expect("expect to create documents batch transition");

        let claim_serialized_transition = claim_transition
            .serialize_to_bytes()
            .expect("expected documents batch serialized state transition");

        let transaction = platform.drive.grove.start_transaction();

        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &vec![claim_serialized_transition.clone()],
                &platform_state,
                &BlockInfo {
                    time_ms: 18_100_000,
                    height: 41,
                    core_height: 42,
                    epoch: Epoch::new(1).unwrap(),
                },
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
                identity_2.id().to_buffer(),
                None,
                platform_version,
            )
            .expect("expected to fetch token balance");
        // Since time is slightly over 5 hours we had 5 events x 5.
        let redemption_cycles = platform_version.system_limits.max_token_redemption_cycles;
        let balance = redemption_cycles * (redemption_cycles + 1) / 2; // Euler's theorem
                                                                       // This works because we are adding 1 + 2 + 3 + 4 etc
        assert_eq!(token_balance, Some(balance as u64));

        // We are only claiming for 128 more cycles
        let claim_transition = BatchTransition::new_token_claim_transition(
            token_id,
            identity_2.id(),
            contract.id(),
            0,
            TokenDistributionType::Perpetual,
            None,
            &key_2,
            3,
            0,
            &signer_2,
            platform_version,
            None,
        )
        .expect("expect to create documents batch transition");

        let claim_serialized_transition = claim_transition
            .serialize_to_bytes()
            .expect("expected documents batch serialized state transition");

        let transaction = platform.drive.grove.start_transaction();

        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &vec![claim_serialized_transition.clone()],
                &platform_state,
                &BlockInfo {
                    time_ms: 18_100_000,
                    height: 41,
                    core_height: 42,
                    epoch: Epoch::new(1).unwrap(),
                },
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
                identity_2.id().to_buffer(),
                None,
                platform_version,
            )
            .expect("expected to fetch token balance");
        let redemption_cycles = redemption_cycles * 2;
        let balance = redemption_cycles * (redemption_cycles + 1) / 2; // Euler's theorem
        assert_eq!(token_balance, Some(balance as u64));
    }

    #[test]
    fn test_token_perpetual_distribution_time_linear_every_hour() {
        let platform_version = PlatformVersion::latest();
        let mut platform = TestPlatformBuilder::new()
            .with_latest_protocol_version()
            .build_with_mock_rpc()
            .set_genesis_state();

        let mut rng = StdRng::seed_from_u64(4981);

        let platform_state = platform.state.load();

        let (identity, _, _) = setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));

        let (identity_2, signer_2, key_2) =
            setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));

        let (contract, token_id) = create_token_contract_with_owner_identity(
            &mut platform,
            identity.id(),
            Some(|token_configuration: &mut TokenConfiguration| {
                token_configuration
                    .distribution_rules_mut()
                    .set_perpetual_distribution(Some(TokenPerpetualDistribution::V0(
                        TokenPerpetualDistributionV0 {
                            distribution_type: RewardDistributionType::TimeBasedDistribution {
                                // every hour
                                interval: 3_600_000,
                                function: DistributionFunction::Linear {
                                    a: 1, // Increase in slope y = x
                                    d: 1, // No division, simple emission
                                    start_step: None,
                                    starting_amount: 0,
                                    min_value: None,
                                    max_value: None,
                                },
                            },
                            distribution_recipient: TokenDistributionRecipient::Identity(
                                identity_2.id(),
                            ),
                        },
                    )));
            }),
            None,
            None,
            platform_version,
        );

        // 5 hours later
        fast_forward_to_block(&platform, 18_000_000, 40, 42, 1, false);

        // We are only claiming for 256 cycles
        let claim_transition = BatchTransition::new_token_claim_transition(
            token_id,
            identity_2.id(),
            contract.id(),
            0,
            TokenDistributionType::Perpetual,
            None,
            &key_2,
            2,
            0,
            &signer_2,
            platform_version,
            None,
        )
        .expect("expect to create documents batch transition");

        let claim_serialized_transition = claim_transition
            .serialize_to_bytes()
            .expect("expected documents batch serialized state transition");

        let transaction = platform.drive.grove.start_transaction();

        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &vec![claim_serialized_transition.clone()],
                &platform_state,
                &BlockInfo {
                    time_ms: 18_100_000,
                    height: 41,
                    core_height: 42,
                    epoch: Epoch::new(1).unwrap(),
                },
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
                identity_2.id().to_buffer(),
                None,
                platform_version,
            )
            .expect("expected to fetch token balance");
        // 1 at hour 1 = 36_000
        // 2 at hour 2 = 72_000
        // 3 at hour 3 = 108_000
        // 4 at hour 4 = 144_000
        // 5 at hour 5 = 180_000
        // Sum of 1 + 2 + 3 + 4 + 5 = 15
        assert_eq!(token_balance, Some(15));

        // We are claiming again shortly later, we should have nothing
        let claim_transition = BatchTransition::new_token_claim_transition(
            token_id,
            identity_2.id(),
            contract.id(),
            0,
            TokenDistributionType::Perpetual,
            None,
            &key_2,
            3,
            0,
            &signer_2,
            platform_version,
            None,
        )
        .expect("expect to create documents batch transition");

        let claim_serialized_transition = claim_transition
            .serialize_to_bytes()
            .expect("expected documents batch serialized state transition");

        let transaction = platform.drive.grove.start_transaction();

        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &vec![claim_serialized_transition.clone()],
                &platform_state,
                &BlockInfo {
                    time_ms: 18_100_000,
                    height: 41,
                    core_height: 42,
                    epoch: Epoch::new(1).unwrap(),
                },
                &transaction,
                platform_version,
                false,
                None,
            )
            .expect("expected to process state transition");

        assert_matches!(
            processing_result.execution_results().as_slice(),
            [PaidConsensusError(
                ConsensusError::StateError(StateError::InvalidTokenClaimNoCurrentRewards(_)),
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
                identity_2.id().to_buffer(),
                None,
                platform_version,
            )
            .expect("expected to fetch token balance");
        assert_eq!(token_balance, Some(15));
    }

    #[test]
    fn test_token_perpetual_distribution_time_linear_verify_contract_start() {
        let platform_version = PlatformVersion::latest();
        let mut platform = TestPlatformBuilder::new()
            .with_latest_protocol_version()
            .build_with_mock_rpc()
            .set_genesis_state();

        let mut rng = StdRng::seed_from_u64(4981);

        let platform_state = platform.state.load();

        let (identity, _, _) = setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));

        let (identity_2, signer_2, key_2) =
            setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));

        let (contract, token_id) = create_token_contract_with_owner_identity(
            &mut platform,
            identity.id(),
            Some(|token_configuration: &mut TokenConfiguration| {
                token_configuration
                    .distribution_rules_mut()
                    .set_perpetual_distribution(Some(TokenPerpetualDistribution::V0(
                        TokenPerpetualDistributionV0 {
                            distribution_type: RewardDistributionType::TimeBasedDistribution {
                                // every hour
                                interval: 3_600_000,
                                function: DistributionFunction::Linear {
                                    a: 1, // Increase in slope y = x
                                    d: 1, // No division, simple emission
                                    start_step: None,
                                    starting_amount: 0,
                                    min_value: None,
                                    max_value: None,
                                },
                            },
                            distribution_recipient: TokenDistributionRecipient::Identity(
                                identity_2.id(),
                            ),
                        },
                    )));
            }),
            Some(9_000_000),
            None,
            platform_version,
        );

        // 5 hours later
        fast_forward_to_block(&platform, 18_000_000, 40, 42, 1, false);

        // We are only claiming for 256 cycles
        let claim_transition = BatchTransition::new_token_claim_transition(
            token_id,
            identity_2.id(),
            contract.id(),
            0,
            TokenDistributionType::Perpetual,
            None,
            &key_2,
            2,
            0,
            &signer_2,
            platform_version,
            None,
        )
        .expect("expect to create documents batch transition");

        let claim_serialized_transition = claim_transition
            .serialize_to_bytes()
            .expect("expected documents batch serialized state transition");

        let transaction = platform.drive.grove.start_transaction();

        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &vec![claim_serialized_transition.clone()],
                &platform_state,
                &BlockInfo {
                    time_ms: 18_100_000,
                    height: 41,
                    core_height: 42,
                    epoch: Epoch::new(1).unwrap(),
                },
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
                identity_2.id().to_buffer(),
                None,
                platform_version,
            )
            .expect("expected to fetch token balance");
        // 1 at hour 3 = 108_000
        // 2 at hour 4 = 144_000
        // 3 at hour 5 = 180_000
        // Sum of 1 + 2 + 3  = 6
        assert_eq!(token_balance, Some(6));

        // We are claiming again, we should have nothing
        let claim_transition = BatchTransition::new_token_claim_transition(
            token_id,
            identity_2.id(),
            contract.id(),
            0,
            TokenDistributionType::Perpetual,
            None,
            &key_2,
            3,
            0,
            &signer_2,
            platform_version,
            None,
        )
        .expect("expect to create documents batch transition");

        let claim_serialized_transition = claim_transition
            .serialize_to_bytes()
            .expect("expected documents batch serialized state transition");

        let transaction = platform.drive.grove.start_transaction();

        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &vec![claim_serialized_transition.clone()],
                &platform_state,
                &BlockInfo {
                    time_ms: 18_100_000,
                    height: 41,
                    core_height: 42,
                    epoch: Epoch::new(1).unwrap(),
                },
                &transaction,
                platform_version,
                false,
                None,
            )
            .expect("expected to process state transition");

        assert_matches!(
            processing_result.execution_results().as_slice(),
            [PaidConsensusError(
                ConsensusError::StateError(StateError::InvalidTokenClaimNoCurrentRewards(_)),
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
                identity_2.id().to_buffer(),
                None,
                platform_version,
            )
            .expect("expected to fetch token balance");
        assert_eq!(token_balance, Some(6));
    }

    #[test]
    fn test_token_perpetual_distribution_time_linear_high_values() {
        let platform_version = PlatformVersion::latest();
        let mut platform = TestPlatformBuilder::new()
            .with_latest_protocol_version()
            .build_with_mock_rpc()
            .set_genesis_state();

        let mut rng = StdRng::seed_from_u64(4981);

        let platform_state = platform.state.load();

        let (identity, _, _) = setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));

        let (identity_2, signer_2, key_2) =
            setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));

        let (contract, token_id) = create_token_contract_with_owner_identity(
            &mut platform,
            identity.id(),
            Some(|token_configuration: &mut TokenConfiguration| {
                token_configuration
                    .distribution_rules_mut()
                    .set_perpetual_distribution(Some(TokenPerpetualDistribution::V0(
                        TokenPerpetualDistributionV0 {
                            distribution_type: RewardDistributionType::TimeBasedDistribution {
                                // every 1 millisecond
                                interval: 1,
                                function: DistributionFunction::Linear {
                                    a: MAX_LINEAR_SLOPE_A_PARAM as i64, // Strongest slope
                                    d: 1,                               // No division
                                    start_step: None,
                                    starting_amount: MAX_DISTRIBUTION_PARAM,
                                    min_value: None,
                                    max_value: None,
                                },
                            },
                            distribution_recipient: TokenDistributionRecipient::Identity(
                                identity_2.id(),
                            ),
                        },
                    )));
            }),
            None,
            None,
            platform_version,
        );

        // 5 hours later
        fast_forward_to_block(&platform, 18_000_000, 40, 42, 1, false);

        // We are only claiming for 256 cycles
        let claim_transition = BatchTransition::new_token_claim_transition(
            token_id,
            identity_2.id(),
            contract.id(),
            0,
            TokenDistributionType::Perpetual,
            None,
            &key_2,
            2,
            0,
            &signer_2,
            platform_version,
            None,
        )
        .expect("expect to create documents batch transition");

        let claim_serialized_transition = claim_transition
            .serialize_to_bytes()
            .expect("expected documents batch serialized state transition");

        let transaction = platform.drive.grove.start_transaction();

        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &vec![claim_serialized_transition.clone()],
                &platform_state,
                &BlockInfo {
                    time_ms: 18_100_000,
                    height: 41,
                    core_height: 42,
                    epoch: Epoch::new(1).unwrap(),
                },
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
                identity_2.id().to_buffer(),
                None,
                platform_version,
            )
            .expect("expected to fetch token balance");
        assert_eq!(token_balance, Some(36028797021077376));

        // We are only claiming for 256 more cycles
        let claim_transition = BatchTransition::new_token_claim_transition(
            token_id,
            identity_2.id(),
            contract.id(),
            0,
            TokenDistributionType::Perpetual,
            None,
            &key_2,
            3,
            0,
            &signer_2,
            platform_version,
            None,
        )
        .expect("expect to create documents batch transition");

        let claim_serialized_transition = claim_transition
            .serialize_to_bytes()
            .expect("expected documents batch serialized state transition");

        let transaction = platform.drive.grove.start_transaction();

        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &vec![claim_serialized_transition.clone()],
                &platform_state,
                &BlockInfo {
                    time_ms: 18_100_000,
                    height: 41,
                    core_height: 42,
                    epoch: Epoch::new(1).unwrap(),
                },
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
                identity_2.id().to_buffer(),
                None,
                platform_version,
            )
            .expect("expected to fetch token balance");
        assert_eq!(token_balance, Some(72057594046349056));
    }

    #[test]
    fn test_token_perpetual_distribution_time_linear_high_values_old_contract_should_handle_overflow(
    ) {
        let platform_version = PlatformVersion::latest();
        let mut platform = TestPlatformBuilder::new()
            .with_latest_protocol_version()
            .build_with_mock_rpc()
            .set_genesis_state();

        let mut rng = StdRng::seed_from_u64(4981);

        let platform_state = platform.state.load();

        let (identity, _, _) = setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));

        let (identity_2, signer_2, key_2) =
            setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));

        let (contract, token_id) = create_token_contract_with_owner_identity(
            &mut platform,
            identity.id(),
            Some(|token_configuration: &mut TokenConfiguration| {
                token_configuration
                    .distribution_rules_mut()
                    .set_perpetual_distribution(Some(TokenPerpetualDistribution::V0(
                        TokenPerpetualDistributionV0 {
                            distribution_type: RewardDistributionType::TimeBasedDistribution {
                                // every 1 millisecond
                                interval: 1,
                                function: DistributionFunction::Linear {
                                    a: MAX_LINEAR_SLOPE_A_PARAM as i64, // Strongest slope
                                    d: 1,                               // No division
                                    start_step: None,
                                    starting_amount: MAX_DISTRIBUTION_PARAM,
                                    min_value: None,
                                    max_value: None,
                                },
                            },
                            distribution_recipient: TokenDistributionRecipient::Identity(
                                identity_2.id(),
                            ),
                        },
                    )));
            }),
            None,
            None,
            platform_version,
        );

        // 5 hours later
        fast_forward_to_block(&platform, 18_000_000, 40, 42, 1, false);

        for i in 0..256 {
            // We are only claiming for 256 cycles
            let claim_transition = BatchTransition::new_token_claim_transition(
                token_id,
                identity_2.id(),
                contract.id(),
                0,
                TokenDistributionType::Perpetual,
                None,
                &key_2,
                2 + i,
                0,
                &signer_2,
                platform_version,
                None,
            )
            .expect("expect to create documents batch transition");

            let claim_serialized_transition = claim_transition
                .serialize_to_bytes()
                .expect("expected documents batch serialized state transition");

            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![claim_serialized_transition.clone()],
                    &platform_state,
                    &BlockInfo {
                        time_ms: 18_100_000,
                        height: 41,
                        core_height: 42,
                        epoch: Epoch::new(1).unwrap(),
                    },
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

        let token_balance = platform
            .drive
            .fetch_identity_token_balance(
                token_id.to_buffer(),
                identity_2.id().to_buffer(),
                None,
                platform_version,
            )
            .expect("expected to fetch token balance");
        // This is i64::Max
        assert_eq!(token_balance, Some(9223372036854675807));
    }
}
