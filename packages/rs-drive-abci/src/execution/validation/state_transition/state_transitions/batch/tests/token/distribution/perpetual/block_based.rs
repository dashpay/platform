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

/// Initial contract balance, as hardcoded in the contract definition (JSON file).
const INITIAL_BALANCE: u64 = 100_000;

mod perpetual_distribution_block {
    use dpp::block::epoch::Epoch;
    use dpp::data_contract::associated_token::token_distribution_key::TokenDistributionType;
    use dpp::data_contract::associated_token::token_perpetual_distribution::distribution_function::DistributionFunction;
    use dpp::data_contract::associated_token::token_perpetual_distribution::distribution_recipient::TokenDistributionRecipient;
    use dpp::data_contract::associated_token::token_perpetual_distribution::reward_distribution_type::RewardDistributionType;
    use dpp::data_contract::associated_token::token_perpetual_distribution::TokenPerpetualDistribution;
    use dpp::data_contract::associated_token::token_perpetual_distribution::v0::TokenPerpetualDistributionV0;
    use crate::test::helpers::fast_forward_to_block::fast_forward_to_block;
    use super::*;

    #[test]
    fn test_token_perpetual_distribution_block_claim_linear_and_claim_again() {
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
                            distribution_type: RewardDistributionType::BlockBasedDistribution {
                                interval: 10,
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

        fast_forward_to_block(&platform, 10_200_000_000, 40, 42, 1, false); //25 years later

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
            None,
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
                &[claim_serialized_transition.clone()],
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
        // Since height is 42 we had 4 events * 50 (+ 100000 which was data contract owner base).
        assert_eq!(token_balance, Some(100200));

        fast_forward_to_block(&platform, 10_200_000_000, 45, 42, 1, false);

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
            None,
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
                &[claim_serialized_transition.clone()],
                &platform_state,
                &BlockInfo {
                    time_ms: 10_200_100_000,
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
            [StateTransitionExecutionResult::PaidConsensusError(
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
        assert_eq!(token_balance, Some(100200));

        fast_forward_to_block(&platform, 10_200_000_000, 49, 42, 1, false);

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
            None,
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
                &[claim_serialized_transition.clone()],
                &platform_state,
                &BlockInfo {
                    time_ms: 10_200_100_000,
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
        assert_eq!(token_balance, Some(100250));
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
                            distribution_type: RewardDistributionType::BlockBasedDistribution {
                                interval: 10,
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
            None,
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
                &[claim_serialized_transition.clone()],
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
            [StateTransitionExecutionResult::PaidConsensusError(
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
    fn test_token_perpetual_distribution_block_claim_linear_given_to_specific_identity() {
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
                            distribution_type: RewardDistributionType::BlockBasedDistribution {
                                interval: 10,
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

        fast_forward_to_block(&platform, 10_200_000_000, 40, 42, 1, false); //25 years later

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
            None,
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
                &[claim_serialized_transition.clone()],
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
        // Since height is 42 we had 4 events x 5.
        assert_eq!(token_balance, Some(200));
    }
}

#[cfg(test)]
mod block_based_perpetual_fixed_amount {
    use crate::platform_types::state_transitions_processing_result::StateTransitionExecutionResult;

    use super::{test_suite::*, INITIAL_BALANCE};
    use dpp::{
        consensus::{state::state_error::StateError, ConsensusError},
        data_contract::associated_token::token_perpetual_distribution::distribution_function::DistributionFunction,
    };

    // Given some token configuration,
    // When a claim is made at block 42,
    // Then the claim should be successful.
    #[test]
    fn test_block_based_perpetual_fixed_amount_50() {
        super::test_suite::check_heights(
            DistributionFunction::FixedAmount { amount: 50 },
            &[
                TestStep::new(41, 100_200, true),
                TestStep::new(46, 100_200, false),
                TestStep::new(50, 100_250, true),
                TestStep::new(51, 100_250, false),
            ],
            None,
            10,
            None,
        )
        .expect("\n-> fixed amount should pass");
    }

    /// Test case for overflow error.
    ///
    /// TODO: Fails, please fix.
    ///
    /// claim at height 1000000000000: claim failed: assertion 0 failed: expected SuccessfulExecution,
    /// got [InternalError(\"storage: protocol: overflow error: Overflow in FixedAmount evaluation\")]"
    #[test]
    fn fail_test_block_based_perpetual_fixed_amount_1_000_000_000() {
        check_heights(
            DistributionFunction::FixedAmount {
                amount: 1_000_000_000,
            },
            &[
                TestStep::new(41, INITIAL_BALANCE + 4 * 1_000_000_000, true),
                TestStep::new(46, INITIAL_BALANCE + 4 * 1_000_000_000, false),
                TestStep::new(50, INITIAL_BALANCE + 5 * 1_000_000_000, true),
                TestStep::new(51, INITIAL_BALANCE + 5 * 1_000_000_000, false),
                TestStep::new(
                    1_000_000_000_000,
                    INITIAL_BALANCE + 5 * 1_000_000_000,
                    false,
                ),
            ],
            None,
            10,
            None,
        )
        .expect("\n-> fixed amount should pass");
    }

    #[test]
    /// Given a fixed amount distribution with value of 0,
    /// When we try to claim,
    /// Then we always fail and the balance remains unchanged.
    fn test_block_based_perpetual_fixed_amount_0() {
        check_heights(
            DistributionFunction::FixedAmount { amount: 0 },
            &[
                (41, 100000, false),
                (46, 100000, false),
                (50, 100000, false),
                (1000, 100000, false),
            ],
            None,
            10,
            None,
        )
        .expect("\nfixed amount zero increase\n");
    }

    #[test]
    /// Given a fixed amount distribution with value of 1_000_000 and max_supply of 200_000,
    /// When we try to claim,
    /// Then we always fail and the balance remains unchanged.
    fn test_fixed_amount_above_max_supply() {
        let test = TestStep {
            name: "test_fixed_amount_above_max_supply".to_string(),
            base_height: 41,
            base_time_ms: Default::default(),
            expected_balance: None,
            claim_transition_assertions: vec![|v| match v {
                [StateTransitionExecutionResult::PaidConsensusError(
                    ConsensusError::StateError(StateError::TokenMintPastMaxSupplyError(_)),
                    _,
                )] => Ok(()),
                _ => Err(format!("expected TokenMintPastMaxSupplyError, got {:?}", v)),
            }],
        };
        check_heights(
            DistributionFunction::FixedAmount { amount: 1_000_000 },
            &[test],
            None,
            10,
            Some(Some(200_000)),
        )
        .expect("\nfixed amount zero increase\n");
    }

    /// Given a fixed amount distribution with value of u64::MAX,
    /// When I claim tokens,
    /// Then I don't get an InternalError.
    #[test]
    fn fail_test_block_based_perpetual_fixed_amount_u64_max() {
        check_heights(
            DistributionFunction::FixedAmount { amount: u64::MAX },
            &[TestStep::new(41, 100_000, false)],
            None,
            10,
            None,
        )
        .expect("\nfixed amount u64::MAX should pass\n");
    }
}
mod block_based_perpetual_random {
    use std::{
        collections::BTreeMap,
        sync::{Arc, Mutex},
    };

    use crate::execution::validation::state_transition::batch::tests::token::distribution::perpetual::block_based::test_suite::TestSuite;

    use super::{
        test_suite::{check_heights, TestStep},
        INITIAL_BALANCE,
    };
    use dpp::data_contract::{
        associated_token::{
            token_configuration::accessors::v0::TokenConfigurationV0Getters,
            token_distribution_key::TokenDistributionType,
            token_distribution_rules::accessors::v0::TokenDistributionRulesV0Setters,
            token_perpetual_distribution::{
                distribution_function::DistributionFunction,
                distribution_recipient::TokenDistributionRecipient,
                reward_distribution_type::RewardDistributionType, v0::TokenPerpetualDistributionV0,
                TokenPerpetualDistribution,
            },
        },
        TokenConfiguration,
    };

    /// Given a random distribution function with min=0, max=100,
    /// When I claim tokens at various heights,
    /// Then I get deterministic balances at those heights.
    #[test]
    fn test_random_max_supply() -> Result<(), String> {
        let steps = [
            TestStep::new(41, 100_192, true),
            TestStep::new(46, 100_192, false),
            TestStep::new(50, 100_263, true),
            TestStep::new(59, 100_263, false),
            TestStep::new(60, 100_310, true),
        ];

        for max_supply in [None, Some(1_000_000)] {
            check_heights(
                DistributionFunction::Random { min: 0, max: 100 },
                &steps,
                None,
                10,
                Some(max_supply),
            )?;
        }
        Ok(())
    }

    /// Given a random distribution function with min=0, max=0,
    /// When I claim tokens at various heights,
    /// Then claim fails and I get the same balance at those heights.
    #[test]
    fn test_block_based_perpetual_random_0_0() {
        check_heights(
            DistributionFunction::Random { min: 0, max: 0 },
            &[
                TestStep::new(41, INITIAL_BALANCE, false),
                TestStep::new(50, INITIAL_BALANCE, false),
                TestStep::new(100, INITIAL_BALANCE, false),
            ],
            None,
            10,
            None,
        )
        .expect("no rewards");
    }
    #[test]
    fn fails_test_block_based_perpetual_random_0_max() {
        check_heights(
            DistributionFunction::Random {
                min: 0,
                max: u64::MAX,
            },
            &[
                TestStep::new(41, INITIAL_BALANCE, false),
                TestStep::new(50, INITIAL_BALANCE, false),
                TestStep::new(100, INITIAL_BALANCE, false),
            ],
            None,
            10,
            None,
        )
        .expect("no rewards");
    }

    /// Given a random distribution function with min=10, max=30,
    /// When I claim tokens at various heights,
    /// Then I get a distribution of balances that is close to the maximum entropy.
    #[test]
    fn test_block_based_perpetual_random_10_30_entropy() {
        const N: u64 = 200;
        const MIN: u64 = 10;
        const MAX: u64 = 30;
        let tests: Vec<_> = (1..=N)
            .map(|i| TestStep {
                name: format!("test_{}", i),
                base_height: i - 1,
                base_time_ms: Default::default(),

                expected_balance: None,
                claim_transition_assertions: Default::default(),
            })
            .collect();

        let balances = Arc::new(Mutex::new(Vec::new()));
        let balances_result = balances.clone();

        let mut suite = TestSuite::new(
            10_200_000_000,
            0,
            TokenDistributionType::Perpetual,
            Some(move |token_configuration: &mut TokenConfiguration| {
                token_configuration
                    .distribution_rules_mut()
                    .set_perpetual_distribution(Some(TokenPerpetualDistribution::V0(
                        TokenPerpetualDistributionV0 {
                            distribution_type: RewardDistributionType::BlockBasedDistribution {
                                interval: 1,
                                function: DistributionFunction::Random { min: MIN, max: MAX },
                            },
                            distribution_recipient: TokenDistributionRecipient::ContractOwner,
                        },
                    )));
            }),
        )
        .with_step_success_fn(move |balance: u64| {
            balances.lock().unwrap().push(balance);
        });

        suite.execute(&tests).expect("should execute");

        let data = balances_result.lock().unwrap();
        // substract balance from previous step (for first step, substract initial balance of 100_000)
        let diffs: Vec<u64> = data
            .iter()
            .scan(INITIAL_BALANCE, |prev, &x| {
                let diff = x - *prev;
                *prev = x;
                Some(diff)
            })
            .collect();

        let entropy = calculate_entropy(&diffs);
        let max_entropy: f64 = ((MAX - MIN) as f64).log2();
        let entropy_diff = (max_entropy - entropy).abs() / max_entropy;

        tracing::debug!("Data: {:?}", diffs);
        tracing::info!(
            "Entropy: {}, max entropy: {}, difference: {}%",
            entropy,
            max_entropy,
            entropy_diff * 100.0
        );

        // assert that the entropy is close to the maximum entropy
        assert!(
            entropy_diff < 0.05,
            "Entropy is not close to maximum entropy"
        );
    }

    // HELPERS //

    fn calculate_entropy(data: &[u64]) -> f64 {
        let mut counts = BTreeMap::new();
        let len = data.len() as f64;

        // Count the occurrences of each value
        for &value in data {
            *counts.entry(value).or_insert(0) += 1;
        }

        // Calculate the probability of each value and apply the Shannon entropy formula
        let mut entropy = 0.0;
        for &count in counts.values() {
            let probability = count as f64 / len;
            entropy -= probability * probability.log2();
        }

        entropy
    }
}

mod block_based_perpetual_step_decreasing {
    use dpp::balances::credits::TokenAmount;
    use dpp::data_contract::associated_token::token_configuration::accessors::v0::TokenConfigurationV0Getters;
    use dpp::data_contract::associated_token::token_distribution_key::TokenDistributionType;
    use dpp::data_contract::associated_token::token_distribution_rules::accessors::v0::TokenDistributionRulesV0Setters;
    use dpp::data_contract::associated_token::token_perpetual_distribution::distribution_function::DistributionFunction;
    use dpp::data_contract::associated_token::token_perpetual_distribution::distribution_recipient::TokenDistributionRecipient;
    use dpp::data_contract::associated_token::token_perpetual_distribution::reward_distribution_type::RewardDistributionType;
    use dpp::data_contract::associated_token::token_perpetual_distribution::v0::TokenPerpetualDistributionV0;
    use dpp::data_contract::associated_token::token_perpetual_distribution::TokenPerpetualDistribution;
    use dpp::data_contract::TokenConfiguration;
    use rust_decimal::prelude::ToPrimitive;
    use crate::{execution::validation::state_transition::batch::tests::token::distribution::perpetual::block_based::test_suite::check_heights, platform_types::state_transitions_processing_result::StateTransitionExecutionResult};
    use crate::execution::validation::state_transition::batch::tests::token::distribution::perpetual::block_based::INITIAL_BALANCE;

    use super::test_suite::{TestStep, TestSuite};

    #[test]
    fn claim_every_100_blocks() -> Result<(), String> {
        run_test(
            1,
            1,
            100,
            None,
            100_000,
            Some(1),
            Some((1..1000).step_by(100).collect()),
            1,
        )
    }

    #[test]
    fn claim_every_100_blocks_with_1_percent_increase() -> Result<(), String> {
        run_test(
            1,
            101,
            100,
            None,
            100_000,
            Some(1),
            Some((1..1000).step_by(100).collect()),
            1,
        )
    }

    #[test]
    fn claim_every_500_blocks_fails_due_to_max_token_redemption_cycles() -> Result<(), String> {
        let result = run_test(
            1,
            101,
            100,
            None,
            100_000,
            Some(1),
            Some((1..1000).step_by(500).collect()),
            1,
        );
        assert!(result.is_err_and(
            |s| s.contains("claim at height 501: expected balance Some(100510) but got 100138")
        ));
        Ok(())
    }

    #[test]
    fn fails_with_1000x_increase_overflow() -> Result<(), String> {
        run_test(1, 1000, 1, None, 100_000, Some(1), Some(vec![1, 7]), 1)
    }

    #[test]
    fn full_decrease_min_1_100() -> Result<(), String> {
        for min in [1, 100] {
            run_test(
                1,
                1,
                1,
                None,
                100_000,
                Some(min),
                Some(vec![1, 2, 3, 10, 100]),
                1,
            )
            .map_err(|e| format!("failed with min {}: {}", min, e))?;
        }

        Ok(())
    }

    #[test]
    fn fails_full_decrease_min_eq_u64_max() -> Result<(), String> {
        run_test(
            1,
            1,
            1,
            None,
            100_000,
            Some(u64::MAX),
            Some(vec![1, 2, 3, 10, 100]),
            1,
        )
    }
    #[test]
    fn no_decrease_changing_min() -> Result<(), String> {
        for min in [None, Some(0), Some(1), Some(100)] {
            run_test(1, 0, 1, None, 100_000, min, Some(vec![1, 2, 3, 10, 100]), 1)
                .map_err(|e| format!("failed with min {:?}: {}", min, e))?;
        }
        Ok(())
    }

    #[test]
    fn full_decrease_step_10_interval_1() -> Result<(), String> {
        run_test(10, 1, 1, None, 100_000, None, Some(vec![2, 7, 9]), 1)
    }

    #[test]
    fn full_decrease_start_5_step_10_interval_1() -> Result<(), String> {
        run_test(
            10,
            1,
            1,
            Some(5),
            100_000,
            None,
            Some(vec![2, 7, 9, 13, 14]),
            1,
        )
    }

    #[test]
    fn full_decrease_start_5_step_10_interval_1_err_at_15() -> Result<(), String> {
        let result = run_test(10, 1, 1, Some(5), 100_000, None, Some(vec![14, 15]), 1);
        assert!(result.is_err_and(|s| s.contains("claim at height 15: claim failed")));
        Ok(())
    }

    #[test]
    fn fails_half_decrease_changing_step_and_interval() -> Result<(), String> {
        for step in [5, 10] {
            for distribution_interval in [1, 5] {
                run_test(
                    step,
                    1,
                    2,
                    None,
                    100_000,
                    None,
                    Some(vec![5, 10, 18, 22, 100]),
                    distribution_interval,
                )
                .map_err(|e| {
                    format!(
                        "failed with step {} interval {}: {}",
                        step, distribution_interval, e
                    )
                })?;
            }
        }

        Ok(())
    }

    #[test]
    fn half_decrease_chainging_s() -> Result<(), String> {
        for s in [None, Some(1), Some(5)] {
            run_test(1, 10, 100, s, 100_000, None, Some(vec![5, 10, 15, 20]), 1)
                .map_err(|e| format!("failed with s {:?}: {}", s, e))?;
        }
        Ok(())
    }

    /// Test various combinations of [DistributionFunction::StepDecreasingAmount] distribution.
    #[allow(clippy::too_many_arguments)]
    fn run_test(
        step_count: u32,
        decrease_per_interval_numerator: u16,
        decrease_per_interval_denominator: u16,
        s: Option<u64>,
        n: TokenAmount,
        min_value: Option<u64>,
        claim_heights: Option<Vec<u64>>,
        distribution_interval: u64,
    ) -> Result<(), String> {
        let dist = DistributionFunction::StepDecreasingAmount {
            step_count,
            decrease_per_interval_numerator,
            decrease_per_interval_denominator,
            s,
            n,
            min_value,
        };
        let claim_heights =
            claim_heights.unwrap_or(vec![1, 2, 3, 4, 5, 10, 20, 30, 50, 100, 1_000_000]);

        let expected_balances = claim_heights
            .iter()
            .map(|&h| {
                // initial balance, defined in contract js
                let mut expected_balance: i128 = INITIAL_BALANCE as i128;
                // loop over blocks, starting with S, with step PERPETUAL_DISTRIBUTION_INTERVAL
                for i in (1..=h).step_by(distribution_interval as usize) {
                    expected_balance += expected_emission(i, &dist);
                }
                tracing::debug!("expected balance at height {}: {}", h, expected_balance);
                expected_balance.to_u64().unwrap_or_else(|| {
                    tracing::error!("overflow in expected balance at height {}", h);
                    0
                }) // to handle tests that overflow
            })
            .collect::<Vec<_>>();
        // we expect all tests to pass
        let claims = claim_heights
            .iter()
            .zip(expected_balances.iter())
            .map(|(&h, &b)| (h, b, true))
            .collect::<Vec<_>>();

        // we return Err(()) to make result comparision easier in test_case
        check_heights(
            dist,
            &claims,
            None, //Some(S),
            distribution_interval,
            None,
        )
        .inspect_err(|e| {
            tracing::error!(e);
        })
    }

    /// Given that we have a distribution function distributing some tokens,
    /// When I claim tokens with delay bigger than [platform_version.system_limits.max_token_redemption_cycles],
    /// Then I need to run the claim more than once to get correct balance.
    #[test]
    fn test_claim_more_than_max_token_redemption_cycles() {
        let dist = DistributionFunction::StepDecreasingAmount {
            step_count: 1,
            decrease_per_interval_numerator: 101,
            decrease_per_interval_denominator: 100,
            s: None,
            n: 100_000,
            min_value: Some(1),
        };

        let dist_clone = dist.clone();
        let mut suite = TestSuite::new(
            10_200_000_000,
            1,
            TokenDistributionType::Perpetual,
            Some(|token_configuration: &mut TokenConfiguration| {
                token_configuration
                    .distribution_rules_mut()
                    .set_perpetual_distribution(Some(TokenPerpetualDistribution::V0(
                        TokenPerpetualDistributionV0 {
                            distribution_type: RewardDistributionType::BlockBasedDistribution {
                                interval: 1,
                                function: dist_clone,
                            },
                            distribution_recipient: TokenDistributionRecipient::ContractOwner,
                        },
                    )));
            }),
        );

        for (height, balance) in [(1, 100_001), (101, 100_110), (501, 100_510)] {
            // claim at height 500;loop until we have no more coins
            let step = TestStep {
                name: format!("height {}", height),
                base_height: height - 1,
                base_time_ms: 10_200_000_000,
                expected_balance: None,
                claim_transition_assertions: vec![|v| match v {
                    [StateTransitionExecutionResult::SuccessfulExecution(_, _)] => Ok(()),
                    _ => Err(format!("got {:?}", v)),
                }],
            };

            let mut loops = 0;
            let err = loop {
                if let Err(err) = suite.execute_step(&step) {
                    break err;
                }
                loops += 1;
            };

            // max_token_redemption_cycles is 128
            if height == 501 {
                assert_eq!(loops, (501 - 101) / 128 + 1);
            } else {
                assert_eq!(loops, 1);
            }

            assert!(
                err.contains("InvalidTokenClaimNoCurrentRewards"),
                "expected InvalidTokenClaimNoCurrentRewards error, got {}",
                err
            );

            assert_eq!(
                suite
                    .get_balance()
                    .expect("get balance")
                    .unwrap_or_default(),
                balance,
                "expected balance at height {}: {}",
                height,
                balance
            );
        }
    }

    // ===== HELPER FUNCTIONS ===== //

    /// Calculate expected emission at provided height.
    ///
    /// We use [i128] to ensure we handle overflows better than the original code.
    ///
    // f(x) = n * (1 - (decrease_per_interval_numerator / decrease_per_interval_denominator))^((x - s) / step_count)
    pub(super) fn expected_emission(x: u64, dist: &DistributionFunction) -> i128 {
        let x = x as i128;
        let (
            step_count,
            decrease_per_interval_numerator,
            decrease_per_interval_denominator,
            s,
            n,
            min_value,
        ) = match dist {
            DistributionFunction::StepDecreasingAmount {
                step_count,
                decrease_per_interval_numerator,
                decrease_per_interval_denominator,
                s,
                n,
                min_value,
            } => (
                *step_count as i128,
                *decrease_per_interval_numerator as i128,
                *decrease_per_interval_denominator as i128,
                s.unwrap_or_default() as i128,
                *n as i128,
                min_value.unwrap_or(1) as i128,
            ),
            _ => panic!("expected StepDecreasingAmount"),
        };

        if x < s {
            n
        } else {
            // let's simplify it to a form like:
            //    f(x) = N * a ^ b
            let a = 1f64
                - (decrease_per_interval_numerator as f64
                    / decrease_per_interval_denominator as f64);
            let b = (x - s) / step_count; // integer by purpose, we want to round down
            let f_x = n as f64 * a.powi(b.to_i32().expect("overflow"));
            f_x.to_i128()
                .unwrap_or_else(|| {
                    tracing::error!("overflow in expected_emission({})", f_x);
                    0
                })
                .max(min_value)
        }
    }
}

mod block_based_perpetual_stepwise {
    use super::test_suite::check_heights;
    use dpp::data_contract::associated_token::token_perpetual_distribution::distribution_function::DistributionFunction;
    use std::collections::BTreeMap;

    #[test]
    fn fails_stepwise_correct() {
        let periods = BTreeMap::from([
            (0, 10_000),
            (20, 20_000),
            (45, 30_000),
            (50, 40_000),
            (70, 50_000),
        ]);

        let dist = DistributionFunction::Stepwise(periods);
        let distribution_interval = 10;

        // claims: height, balance, expect_pass
        let steps = [
            (1, 100_000, false),
            (9, 100_000, false),
            (10, 110_000, true),
            (11, 110_000, false),
            (19, 110_000, false),
            (20, 120_000, true),
            (21, 120_000, false),
            (24, 120_000, false),
            (35, 140_000, true), // since 20, we should get one more distribution of 20k at height 30
            (39, 140_000, false),
            (46, 160_000, true),
            (49, 160_000, false),
            (51, 180_000, true),
            (52, 180_000, false),
            (70, 270_000, true),
            (
                1_000_000,
                270_000 + 50_000 * (1_000_000 - 70_000) / distribution_interval,
                true,
            ),
        ];

        check_heights(
            dist,
            &steps,
            None, //Some(S),
            distribution_interval,
            None,
        )
        .inspect_err(|e| {
            tracing::error!("{}", e);
        })
        .expect("stepwise should pass");
    }
}

mod block_based_perpetual_linear {
    use super::test_suite::check_heights;
    use dpp::data_contract::associated_token::token_perpetual_distribution::distribution_function::DistributionFunction;

    /// Given linear distribution with d=0,
    /// When I create a token,
    /// Then I get an error.
    #[test]
    fn fails_divide_by_0() -> Result<(), String> {
        test_linear(
            1,                       // a
            0,                       // d
            None,                    // start_step
            100_000,                 // starting_amount
            None,                    // min_value
            None,                    // max_value
            &[(10, 100_000, false)], // heights
            1,                       // distribution_interval
        )
    }
    /// Given linear distribution with d=MAX and starting amount of 1,
    /// When I claim tokens,
    /// Then I have only one success, and subsequent claims fail because the calculated distribution is lower than 1
    #[test]
    fn divide_my_max() -> Result<(), String> {
        test_linear(
            1,                                            // a
            u64::MAX,                                     // d
            None,                                         // start_step
            0,                                            // starting_amount
            Some(0),                                      // min_value
            None,                                         // max_value
            &[(1, 100_000, false), (20, 100_000, false)], // heights
            1,
        )
    }

    #[test]
    fn min_eq_max() -> Result<(), String> {
        test_linear(
            1,
            1,
            None,
            0,
            Some(10),
            Some(10),
            &[(1, 100_010, true), (2, 100_020, true)],
            1,
        )
    }

    #[test]
    fn fx_eq_x_matrix() -> Result<(), String> {
        let steps = [
            (1, 100_001, true),
            (2, 100_003, true),
            (3, 100_006, true),
            (10, 100_055, true),
        ];

        for start_step in [None, Some(0)] {
            for min_value in [None, Some(0), Some(1)] {
                for max_value in [None, Some(1000)] {
                    test_linear(1, 1, start_step, 0, min_value, max_value, &steps, 1)?;
                }
            }
        }
        Ok(())
    }
    #[test]
    fn negative_a() -> Result<(), String> {
        for a in [-1, -100_000, i64::MIN] {
            test_linear(
                a,
                1,
                None,
                0,
                None,
                None,
                &[(1, 100_000, false), (20, 100_000, false)],
                1,
            )?;
        }
        Ok(())
    }

    #[test]
    fn fails_max_lt_min() -> Result<(), String> {
        for max in [0, 99] {
            test_linear(
                1,
                1,
                None,
                0,
                Some(100),
                Some(max),
                &[(1, 100_000, false), (20, 100_000, false)],
                1,
            )?;
        }
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn test_linear(
        a: i64,
        d: u64,
        start_step: Option<u64>,
        starting_amount: u64,
        min_value: Option<u64>,
        max_value: Option<u64>,
        steps: &[(u64, u64, bool)], // height, expected balance, expect pass
        distribution_interval: u64,
    ) -> Result<(), String> {
        // Linear distribution function
        //
        // # Formula
        // The formula for the linear distribution function is:

        // ```text
        // f(x) = (a * (x - start_moment) / d) + starting_amount
        // ```
        //
        let dist = DistributionFunction::Linear {
            a,
            d,
            start_step,
            starting_amount,
            min_value,
            max_value,
        };

        check_heights(
            dist,
            steps,
            None, //Some(S),
            distribution_interval,
            None,
        )
        .inspect_err(|e| {
            tracing::error!("{}", e);
        })
    }
}

mod block_based_perpetual_polynomial {
    use super::test_suite::{check_heights, TestStep, TestSuite};
    use crate::platform_types::state_transitions_processing_result::StateTransitionExecutionResult;
    use dpp::data_contract::{
        associated_token::{
            token_configuration::accessors::v0::TokenConfigurationV0Getters,
            token_distribution_key::TokenDistributionType,
            token_distribution_rules::accessors::v0::TokenDistributionRulesV0Setters,
            token_perpetual_distribution::{
                distribution_function::DistributionFunction::{self, Polynomial},
                distribution_recipient::TokenDistributionRecipient,
                reward_distribution_type::RewardDistributionType,
                v0::TokenPerpetualDistributionV0,
                TokenPerpetualDistribution,
            },
        },
        TokenConfiguration,
    };

    #[test]
    fn ones() -> Result<(), String> {
        test_polynomial(
            Polynomial {
                a: 1,
                d: 1,
                m: 1,
                n: 1,
                o: 1,
                start_moment: Some(1),
                b: 100_000,
                min_value: None,
                max_value: None,
            },
            &[(10, 1_100_055, true), (20, 2_100_210, true)],
            1,
        )
    }

    /// Divide by 0
    /// claim at height 10: claim failed: assertion 1 failed: expected SuccessfulExecution, got
    /// [InternalError(\"storage: protocol: divide by zero error: Polynomial function: divisor d is 0\")]\nexpected balance Some(1100055) but got 100000\n\n-->
    #[test]
    fn fails_divide_by_0() -> Result<(), String> {
        test_polynomial(
            Polynomial {
                a: 1,
                d: 0,
                m: 1,
                n: 1,
                o: 1,
                start_moment: Some(1),
                b: 100_000,
                min_value: None,
                max_value: None,
            },
            &[(10, 1_100_055, true), (20, 2_100_210, true)],
            1,
        )
    }

    #[test]
    fn max_lt_min_should_fail() -> Result<(), String> {
        test_polynomial(
            Polynomial {
                a: 1,
                d: 1,
                m: 1,
                n: 1,
                o: 1,
                start_moment: Some(1),
                b: 100_000,
                min_value: Some(100_000),
                max_value: Some(10_000),
            },
            &[(10, 100_000, false), (20, 100_000, false)],
            1,
        )
    }

    #[test]
    fn negative_a() -> Result<(), String> {
        test_polynomial(
            Polynomial {
                a: -1,
                d: 1,
                m: 1,
                n: 1,
                o: 1,
                start_moment: Some(1),
                b: 100_000,
                min_value: None,
                max_value: None,
            },
            &[(1, 199_999, true), (4, 499_990, true)],
            1,
        )
    }

    #[test]
    fn fails_a_min() -> Result<(), String> {
        test_polynomial(
            Polynomial {
                a: i64::MIN,
                d: 1,
                m: 1,
                n: 1,
                o: 1,
                start_moment: Some(1),
                b: 100_000,
                min_value: None,
                max_value: None,
            },
            &[(1, 100_000, false), (4, 100_000, true)],
            1,
        )
    }
    #[test]
    fn a_minus_1_b_0() -> Result<(), String> {
        test_polynomial(
            Polynomial {
                a: -1,
                d: 1,
                m: 1,
                n: 1,
                o: 1,
                start_moment: Some(1),
                b: 0,
                min_value: None,
                max_value: None,
            },
            &[(1, 100_000, false), (4, 100_000, false)],
            1,
        )
    }

    #[test]
    fn o_min() -> Result<(), String> {
        test_polynomial(
            Polynomial {
                a: 1,
                d: 1,
                m: 1,
                n: 1,
                o: i64::MIN,
                start_moment: Some(1),
                b: 0,
                min_value: None,
                max_value: None,
            },
            &[(1, 100_000, false), (4, 100_000, false)],
            1,
        )
    }

    #[test]
    fn o_max() -> Result<(), String> {
        test_polynomial(
            Polynomial {
                a: 1,
                d: 1,
                m: 1,
                n: 1,
                o: i64::MAX,
                start_moment: Some(1),
                b: 0,
                min_value: None,
                max_value: None,
            },
            &[(1, 100_000, false), (4, 100_000, false)],
            1,
        )
    }

    #[test]
    fn zero_pow_minus_1_at_h_1() -> Result<(), String> {
        test_polynomial(
            Polynomial {
                a: 1,
                d: 1,
                m: -1,
                n: 1,
                o: 0,
                start_moment: Some(1),
                b: 0,
                min_value: None,
                max_value: None,
            },
            &[(1, 100_000, false), (2, 100_001, true)],
            1,
        )
    }
    #[test]
    fn fails_zero_pow_minus_1_at_h_2() -> Result<(), String> {
        test_polynomial(
            Polynomial {
                a: 1,
                d: 1,
                m: 1,
                n: 2,
                o: 0,
                start_moment: Some(1),
                b: 0,
                min_value: None,
                max_value: None,
            },
            &[
                (1, 100_000, false), // this should fail, 0.pow(-1) is unspecified
                (2, 100_001, true),  // it's 1.pow(1/2) == 1
                (3, 100_002, true),  // 2.pow(1/2) == 1.41 - should round to 1
                (4, 100_004, true),  // 3.pow(1/2) == 1.73 - should round to 2; FAILS
                (5, 100_006, true),  // 4.pow(1/2) == 2
                (6, 100_008, true),  // 5.pow(1/2) == 2.23 - should round to 2
            ],
            1,
        )
    }

    #[test]
    fn o_max_m_2() -> Result<(), String> {
        test_polynomial(
            Polynomial {
                a: 1,
                d: 1,
                m: 2,
                n: 1,
                o: i64::MAX,
                start_moment: Some(1),
                b: 0,
                min_value: None,
                max_value: None,
            },
            &[(1, 100_000, false), (10, 100_000, false)],
            1,
        )
    }
    /// Test polynomial distribution function.
    ///
    /// `f(x) = (a * (x - s + o)^(m/n)) / d + b`
    fn test_polynomial(
        dist: DistributionFunction,
        steps: &[(u64, u64, bool)], // height, expected balance, expect pass
        distribution_interval: u64,
    ) -> Result<(), String> {
        check_heights(
            dist,
            steps,
            None, //Some(S),
            distribution_interval,
            None,
        )
        .inspect_err(|e| {
            tracing::error!("{}", e);
        })
    }

    /// Test various combinations of `m/n` in [DistributionFunction::Polynomial] distribution.
    ///
    /// We expect this test not to end with InternalError.
    #[test]
    fn test_poynomial_power() -> Result<(), String> {
        for m in [i64::MIN, -1, 0, 1, i64::MAX] {
            for n in [0, 1, u64::MAX] {
                let dist = Polynomial {
                    a: 1,
                    d: 1,
                    m,
                    n,
                    o: 1,
                    start_moment: Some(1),
                    b: 100_000,
                    min_value: None,
                    max_value: None,
                };

                let mut suite = TestSuite::new(
                    10_200_000_000,
                    0,
                    TokenDistributionType::Perpetual,
                    Some(move |token_configuration: &mut TokenConfiguration| {
                        token_configuration
                            .distribution_rules_mut()
                            .set_perpetual_distribution(Some(TokenPerpetualDistribution::V0(
                                TokenPerpetualDistributionV0 {
                                    distribution_type:
                                        RewardDistributionType::BlockBasedDistribution {
                                            interval: 1,
                                            function: dist,
                                        },
                                    distribution_recipient:
                                        TokenDistributionRecipient::ContractOwner,
                                },
                            )));
                    }),
                );

                suite = suite.with_contract_start_time(1);

                let step = TestStep {
                    base_height: 10,
                    base_time_ms: Default::default(),
                    expected_balance: None,
                    claim_transition_assertions: vec![
                        |results: &[StateTransitionExecutionResult]| -> Result<(), String> {
                            let err = results
                                .iter()
                                .find(|r| format!("{:?}", r).contains("InternalError"));

                            if let Some(e) = err {
                                Err(format!("InternalError: {:?}", e))
                            } else {
                                Ok(())
                            }
                        },
                    ],
                    name: "test".to_string(),
                };

                suite
                    .execute(&[step])
                    .inspect_err(|e| {
                        tracing::error!("{}", e);
                    })
                    .map_err(|e| format!("failed with m {} n {}: {}", m, n, e))?;
            }
        }

        Ok(())
    }
}

mod block_based_perpetual_logarithmic {

    use super::test_suite::check_heights;
    use dpp::data_contract::associated_token::token_perpetual_distribution::distribution_function::DistributionFunction::{self,Logarithmic};

    #[test]
    fn zeros() -> Result<(), String> {
        test_logarithmic(
            Logarithmic {
                a: 0,                  // a: i64,
                d: 0,                  // d: u64,
                m: 0,                  // m: u64,
                n: 0,                  // n: u64,
                o: 0,                  // o: i64,
                start_moment: Some(0), // start_moment: Option<u64>,
                b: 0,                  // b: TokenAmount,
                min_value: None,       // min_value: Option<u64>,
                max_value: None,       // max_value: Option<u64>,
            },
            &[(4, 100_000, true)],
            1,
        )
    }

    /// "fails: ones - use of ln instead of log as documented
    #[test]
    fn fails_ones() -> Result<(), String> {
        test_logarithmic(
            Logarithmic {
                a: 1,                  // a: i64,
                d: 1,                  // d: u64,
                m: 1,                  // m: u64,
                n: 1,                  // n: u64,
                o: 1,                  // o: i64,
                start_moment: Some(1), // start_moment: Option<u64>,
                b: 1,                  // b: TokenAmount,
                min_value: None,       // min_value: Option<u64>,
                max_value: None,       // max_value: Option<u64>,
            },
            &[
                (1, 100_001, true), // log(0)+1 = 1
                (2, 100_002, true), // log(1)+1 = 1
                (3, 100_003, true), // log(3)+1 = 1
                (4, 100_005, true), // log(4)+1 = 2 (log(4) == 0.6, rounded up to 1)
            ],
            1,
        )
    }
    #[test]
    fn fails_div_by_0() -> Result<(), String> {
        test_logarithmic(
            Logarithmic {
                a: 1,                  // a: i64,
                d: 1,                  // d: u64,
                m: 1,                  // m: u64,
                n: 0,                  // n: u64,
                o: 1,                  // o: i64,
                start_moment: Some(1), // start_moment: Option<u64>,
                b: 1,                  // b: TokenAmount,
                min_value: None,       // min_value: Option<u64>,
                max_value: None,       // max_value: Option<u64>,
            },
            &[(2, 100_002, false)],
            1,
        )
    }
    #[test]
    fn fails_log_0() -> Result<(), String> {
        test_logarithmic(
            Logarithmic {
                a: 1,                  // a: i64,
                d: 1,                  // d: u64,
                m: 0,                  // m: u64,
                n: 1,                  // n: u64,
                o: 1,                  // o: i64,
                start_moment: Some(1), // start_moment: Option<u64>,
                b: 1,                  // b: TokenAmount,
                min_value: None,       // min_value: Option<u64>,
                max_value: None,       // max_value: Option<u64>,
            },
            &[(1, 100_001, true), (5, 100_001, true)],
            1,
        )
    }

    /// min == max means linear
    #[test]
    fn min_eq_max() -> Result<(), String> {
        test_logarithmic(
            Logarithmic {
                a: 1,                  // a: i64,
                d: 1,                  // d: u64,
                m: 1,                  // m: u64,
                n: 1,                  // n: u64,
                o: 1,                  // o: i64,
                start_moment: Some(1), // start_moment: Option<u64>,
                b: 0,                  // b: TokenAmount,
                min_value: Some(10),   // min_value: Option<u64>,
                max_value: Some(10),   // max_value: Option<u64>,
            },
            &[(1, 100_010, true), (5, 100_050, true)],
            1,
        )
    }
    #[test]
    fn min_eq_max_interval_5() -> Result<(), String> {
        test_logarithmic(
            Logarithmic {
                a: 1,                  // a: i64,
                d: 1,                  // d: u64,
                m: 1,                  // m: u64,
                n: 1,                  // n: u64,
                o: 1,                  // o: i64,
                start_moment: Some(1), // start_moment: Option<u64>,
                b: 0,                  // b: TokenAmount,
                min_value: Some(10),   // min_value: Option<u64>,
                max_value: Some(10),   // max_value: Option<u64>,
            },
            &[(5, 100_010, true), (10, 100_020, true)],
            5,
        )
    }
    #[test]
    fn fails_min_gt_max() -> Result<(), String> {
        test_logarithmic(
            Logarithmic {
                a: 1,                  // a: i64,
                d: 1,                  // d: u64,
                m: 1,                  // m: u64,
                n: 1,                  // n: u64,
                o: 1,                  // o: i64,
                start_moment: Some(1), // start_moment: Option<u64>,
                b: 1,                  // b: TokenAmount,
                min_value: Some(10),   // min_value: Option<u64>,
                max_value: Some(5),    // max_value: Option<u64>,
            },
            &[(5, 100_000, false), (10, 100_000, false)],
            5,
        )
    }
    #[test]
    fn fails_a_min() -> Result<(), String> {
        test_logarithmic(
            Logarithmic {
                a: i64::MIN,           // a: i64,
                d: 1,                  // d: u64,
                m: 1,                  // m: u64,
                n: 1,                  // n: u64,
                o: 1,                  // o: i64,
                start_moment: Some(1), // start_moment: Option<u64>,
                b: 1,                  // b: TokenAmount,
                min_value: None,       // min_value: Option<u64>,
                max_value: None,       // max_value: Option<u64>,
            },
            &[
                (1, 100_000, false), // f(1) should be < 0, is 1
                (9, 100_000, false),
                (10, 100_000, false),
            ],
            1,
        )
    }
    #[test]
    fn a_max_overflows() -> Result<(), String> {
        test_logarithmic(
            Logarithmic {
                a: i64::MAX,           // a: i64,
                d: 1,                  // d: u64,
                m: 1,                  // m: u64,
                n: 1,                  // n: u64,
                o: 1,                  // o: i64,
                start_moment: Some(1), // start_moment: Option<u64>,
                b: 1,                  // b: TokenAmount,
                min_value: None,       // min_value: Option<u64>,
                max_value: None,       // max_value: Option<u64>,
            },
            &[
                (1, 100_000, false),
                (9, 100_000, false),
                (10, 100_000, false),
            ],
            1,
        )
    }
    #[test]
    fn a_0_b_0() -> Result<(), String> {
        test_logarithmic(
            Logarithmic {
                a: 0,                  // a: i64,
                d: 1,                  // d: u64,
                m: 1,                  // m: u64,
                n: 1,                  // n: u64,
                o: 1,                  // o: i64,
                start_moment: Some(1), // start_moment: Option<u64>,
                b: 0,                  // b: TokenAmount,
                min_value: None,       // min_value: Option<u64>,
                max_value: None,       // max_value: Option<u64>,
            },
            &[
                (1, 100_000, false),
                (9, 100_000, false),
                (10, 100_000, false),
            ],
            1,
        )
    }
    #[test]
    fn fails_log_negative() -> Result<(), String> {
        test_logarithmic(
            Logarithmic {
                a: 1,                  // a: i64,
                d: 1,                  // d: u64,
                m: 1,                  // m: u64,
                n: 1,                  // n: u64,
                o: -10,                // o: i64,
                start_moment: Some(1), // start_moment: Option<u64>,
                b: 0,                  // b: TokenAmount,
                min_value: None,       // min_value: Option<u64>,
                max_value: None,       // max_value: Option<u64>,
            },
            &[
                (1, 100_000, false),
                (9, 100_000, false),
                (10, 100_000, false),
            ],
            1,
        )
    }
    #[test]
    fn fails_o_min() -> Result<(), String> {
        test_logarithmic(
            Logarithmic {
                a: 1,                  // a: i64,
                d: 1,                  // d: u64,
                m: 1,                  // m: u64,
                n: 1,                  // n: u64,
                o: i64::MIN,           // o: i64,
                start_moment: Some(1), // start_moment: Option<u64>,
                b: 0,                  // b: TokenAmount,
                min_value: None,       // min_value: Option<u64>,
                max_value: None,       // max_value: Option<u64>,
            },
            &[
                (1, 100_000, false),
                (9, 100_000, false),
                (10, 100_000, false),
            ],
            1,
        )
    }
    #[test]
    fn fails_b_max() -> Result<(), String> {
        test_logarithmic(
            Logarithmic {
                a: 1,                  // a: i64,
                d: 1,                  // d: u64,
                m: 1,                  // m: u64,
                n: 1,                  // n: u64,
                o: 1,                  // o: i64,
                start_moment: Some(1), // start_moment: Option<u64>,
                b: u64::MAX,           // b: TokenAmount,
                min_value: None,       // min_value: Option<u64>,
                max_value: None,       // max_value: Option<u64>,
            },
            &[
                (1, 100_000, false),
                (9, 100_000, false),
                (10, 100_000, false),
            ],
            1,
        )
    }
    /// f(x) = (a * log(m * (x - s + o) / n)) / d + b
    fn test_logarithmic(
        dist: DistributionFunction,
        steps: &[(u64, u64, bool)], // height, expected balance, expect pass
        distribution_interval: u64,
    ) -> Result<(), String> {
        check_heights(
            dist,
            steps,
            None, //Some(S),
            distribution_interval,
            None,
        )
        .inspect_err(|e| {
            tracing::error!("{}", e);
        })
    }
}

mod block_based_perpetual_inverted_logarithmic {
    use super::test_suite::check_heights;
    use dpp::data_contract::associated_token::token_perpetual_distribution::distribution_function::DistributionFunction::{self,InvertedLogarithmic};

    #[test]
    fn fails_zeros() -> Result<(), String> {
        let dist = InvertedLogarithmic {
            a: 0,                  // a: i64,
            d: 0,                  // d: u64,
            m: 0,                  // m: u64,
            n: 0,                  // n: u64,
            o: 0,                  // o: i64,
            start_moment: Some(0), // start_moment: Option<u64>,
            b: 0,                  // b: TokenAmount,
            min_value: None,       // min_value: Option<u64>,
            max_value: None,       // max_value: Option<u64>,
        };
        let steps = [(4, 100_000, true)];

        run_test(dist, &steps, 1)
    }

    #[test]
    fn fails_ones() -> Result<(), String> {
        let dist = InvertedLogarithmic {
            a: 1,                  // a: i64,
            d: 1,                  // d: u64,
            m: 1,                  // m: u64,
            n: 1,                  // n: u64,
            o: 1,                  // o: i64,
            start_moment: Some(1), // start_moment: Option<u64>,
            b: 1,                  // b: TokenAmount,
            min_value: None,       // min_value: Option<u64>,
            max_value: None,       // max_value: Option<u64>,
        };
        let steps = [
            (1, 100_001, true),
            (2, 100_002, true),
            (3, 100_003, true),
            (4, 100_005, true),
        ];

        run_test(dist, &steps, 1)
    }

    #[test]
    fn fails_divide_by_zero() -> Result<(), String> {
        let dist = InvertedLogarithmic {
            a: 1,                  // a: i64,
            d: 0,                  // d: u64,
            m: 1,                  // m: u64,
            n: 1,                  // n: u64,
            o: 1,                  // o: i64,
            start_moment: Some(1), // start_moment: Option<u64>,
            b: 1,                  // b: TokenAmount,
            min_value: None,       // min_value: Option<u64>,
            max_value: None,       // max_value: Option<u64>,
        };
        let steps = [(2, 100_002, false)];

        run_test(dist, &steps, 1)
    }

    #[test]
    fn fails_n_zero_log_zero() -> Result<(), String> {
        let dist = InvertedLogarithmic {
            a: 1,                  // a: i64,
            d: 1,                  // d: u64,
            m: 1,                  // m: u64,
            n: 0,                  // n: u64,
            o: 1,                  // o: i64,
            start_moment: Some(1), // start_moment: Option<u64>,
            b: 1,                  // b: TokenAmount,
            min_value: None,       // min_value: Option<u64>,
            max_value: None,       // max_value: Option<u64>,
        };
        let steps = [(1, 100_001, true), (5, 100_001, true)];

        run_test(dist, &steps, 1)
    }

    #[test]
    fn min_eq_max_means_linear() -> Result<(), String> {
        let dist = InvertedLogarithmic {
            a: 1,                  // a: i64,
            d: 1,                  // d: u64,
            m: 1,                  // m: u64,
            n: 1,                  // n: u64,
            o: 1,                  // o: i64,
            start_moment: Some(1), // start_moment: Option<u64>,
            b: 0,                  // b: TokenAmount,
            min_value: Some(10),   // min_value: Option<u64>,
            max_value: Some(10),   // max_value: Option<u64>,
        };
        let steps = [(1, 100_010, true), (5, 100_050, true)];

        run_test(dist, &steps, 1)
    }

    #[test]
    fn min_eq_max_means_linear_interval_5() -> Result<(), String> {
        let dist = InvertedLogarithmic {
            a: 1,                  // a: i64,
            d: 1,                  // d: u64,
            m: 1,                  // m: u64,
            n: 1,                  // n: u64,
            o: 1,                  // o: i64,
            start_moment: Some(1), // start_moment: Option<u64>,
            b: 0,                  // b: TokenAmount,
            min_value: Some(10),   // min_value: Option<u64>,
            max_value: Some(10),   // max_value: Option<u64>,
        };
        let steps = [(5, 100_010, true), (10, 100_020, true)];

        run_test(dist, &steps, 5)
    }

    #[test]
    fn fails_min_gt_max() -> Result<(), String> {
        let dist = InvertedLogarithmic {
            a: 1,                  // a: i64,
            d: 1,                  // d: u64,
            m: 1,                  // m: u64,
            n: 1,                  // n: u64,
            o: 1,                  // o: i64,
            start_moment: Some(1), // start_moment: Option<u64>,
            b: 1,                  // b: TokenAmount,
            min_value: Some(10),   // min_value: Option<u64>,
            max_value: Some(5),    // max_value: Option<u64>,
        };
        let steps = [(5, 100_000, false), (10, 100_000, false)];

        run_test(dist, &steps, 5)
    }

    #[test]
    fn fails_a_min() -> Result<(), String> {
        let dist = InvertedLogarithmic {
            a: i64::MIN,           // a: i64,
            d: 1,                  // d: u64,
            m: 1,                  // m: u64,
            n: 1,                  // n: u64,
            o: 1,                  // o: i64,
            start_moment: Some(1), // start_moment: Option<u64>,
            b: 1,                  // b: TokenAmount,
            min_value: None,       // min_value: Option<u64>,
            max_value: None,       // max_value: Option<u64>,
        };
        let steps = [
            (1, 100_000, false), // f(1) should be < 0, is 1
            (9, 100_000, false),
            (10, 100_000, false),
        ];

        run_test(dist, &steps, 1)
    }

    #[test]
    fn fails_a_max() -> Result<(), String> {
        let dist = InvertedLogarithmic {
            a: i64::MAX,           // a: i64,
            d: 1,                  // d: u64,
            m: 1,                  // m: u64,
            n: 1,                  // n: u64,
            o: 1,                  // o: i64,
            start_moment: Some(1), // start_moment: Option<u64>,
            b: 1,                  // b: TokenAmount,
            min_value: None,       // min_value: Option<u64>,
            max_value: None,       // max_value: Option<u64>,
        };
        let steps = [
            (1, 100_001, true), // f(x) = 0 for x>1
            (9, 100_001, false),
            (10, 100_001, false),
        ];

        run_test(dist, &steps, 1)
    }

    #[test]
    fn fails_a_zero_b_zero() -> Result<(), String> {
        let dist = InvertedLogarithmic {
            a: 0,                  // a: i64,
            d: 1,                  // d: u64,
            m: 1,                  // m: u64,
            n: 1,                  // n: u64,
            o: 1,                  // o: i64,
            start_moment: Some(1), // start_moment: Option<u64>,
            b: 0,                  // b: TokenAmount,
            min_value: None,       // min_value: Option<u64>,
            max_value: None,       // max_value: Option<u64>,
        };
        let steps = [
            (1, 100_000, false),
            (9, 100_000, false),
            (10, 100_000, false),
        ];

        run_test(dist, &steps, 1)
    }

    #[test]
    fn fails_log_negative() -> Result<(), String> {
        let dist = InvertedLogarithmic {
            a: 1,                  // a: i64,
            d: 1,                  // d: u64,
            m: 1,                  // m: u64,
            n: 1,                  // n: u64,
            o: -10,                // o: i64,
            start_moment: Some(1), // start_moment: Option<u64>,
            b: 0,                  // b: TokenAmount,
            min_value: None,       // min_value: Option<u64>,
            max_value: None,       // max_value: Option<u64>,
        };
        let steps = [
            (1, 100_000, false),
            (9, 100_000, false),
            (10, 100_000, false),
        ];

        run_test(dist, &steps, 1)
    }

    #[test]
    fn fails_o_min() -> Result<(), String> {
        let dist = InvertedLogarithmic {
            a: 1,                  // a: i64,
            d: 1,                  // d: u64,
            m: 1,                  // m: u64,
            n: 1,                  // n: u64,
            o: i64::MIN,           // o: i64,
            start_moment: Some(1), // start_moment: Option<u64>,
            b: 0,                  // b: TokenAmount,
            min_value: None,       // min_value: Option<u64>,
            max_value: None,       // max_value: Option<u64>,
        };
        let steps = [
            (1, 100_000, false),
            (9, 100_000, false),
            (10, 100_000, false),
        ];

        run_test(dist, &steps, 1)
    }

    #[test]
    fn fails_b_max() -> Result<(), String> {
        let dist = InvertedLogarithmic {
            a: 1,                  // a: i64,
            d: 1,                  // d: u64,
            m: 1,                  // m: u64,
            n: 1,                  // n: u64,
            o: 1,                  // o: i64,
            start_moment: Some(1), // start_moment: Option<u64>,
            b: u64::MAX,           // b: TokenAmount,
            min_value: None,       // min_value: Option<u64>,
            max_value: None,       // max_value: Option<u64>,
        };
        let steps = [
            (1, 100_000, false),
            (9, 100_000, false),
            (10, 100_000, false),
        ];

        run_test(dist, &steps, 1)
    }
    /// f(x) = (a * log( n / (m * (x - s + o)) )) / d + b
    fn run_test(
        dist: DistributionFunction,
        steps: &[(u64, u64, bool)], // height, expected balance, expect pass
        distribution_interval: u64,
    ) -> Result<(), String> {
        check_heights(
            dist,
            steps,
            None, //Some(S),
            distribution_interval,
            None,
        )
        .inspect_err(|e| {
            tracing::error!("{}", e);
        })
    }
}

mod test_suite {
    use super::*;
    use crate::rpc::core::MockCoreRPCLike;
    use crate::test::helpers::fast_forward_to_block::fast_forward_to_block;
    use crate::test::helpers::setup::TempPlatform;
    use dpp::block::extended_block_info::v0::ExtendedBlockInfoV0Getters;
    use dpp::data_contract::associated_token::token_distribution_key::TokenDistributionType;
    use dpp::data_contract::associated_token::token_distribution_rules::accessors::v0::TokenDistributionRulesV0Getters;
    use dpp::data_contract::associated_token::token_distribution_rules::TokenDistributionRules;
    use dpp::data_contract::associated_token::token_perpetual_distribution::distribution_function::DistributionFunction;
    use dpp::data_contract::associated_token::token_perpetual_distribution::distribution_recipient::TokenDistributionRecipient;
    use dpp::data_contract::associated_token::token_perpetual_distribution::reward_distribution_type::RewardDistributionType;
    use dpp::data_contract::associated_token::token_perpetual_distribution::v0::TokenPerpetualDistributionV0;
    use dpp::data_contract::associated_token::token_perpetual_distribution::TokenPerpetualDistribution;
    use dpp::prelude::{DataContract, IdentityPublicKey, TimestampMillis};
    use simple_signer::signer::SimpleSigner;

    const TIMEOUT: tokio::time::Duration = tokio::time::Duration::from_secs(10);
    /// Run provided closure with timeout.
    /// TODO: Check if it works with sync code
    fn with_timeout(
        duration: tokio::time::Duration,
        f: impl FnOnce() -> Result<(), String> + Send + 'static,
    ) -> Result<(), String> {
        let rt = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()
            .unwrap();
        // thread executing our code
        let worker = rt.spawn_blocking(f);

        rt.block_on(async move { tokio::time::timeout(duration, worker).await })
            .map_err(|_| format!("test timed out after {:?}", TIMEOUT))?
            .map_err(|e| format!("join error: {:?}", e))?
    }

    /// Check that claim results at provided heights are as expected, and that balances match expectations.
    ///
    /// Note we take i128 into expected_balances, as we want to be able to detect overflows.
    ///
    /// # Arguments
    ///
    /// * `distribution_function` - configured distribution function to test
    /// * `claims` - heights at which claims will be made; they will see balance from previous height
    /// * `contract_start_time` - optional start time of the contract
    /// * `distribution_interval` - interval between distributions
    /// * `max_supply` - optional max supply of the token; if Some(), it will override max supply in contract JSON definition
    ///
    /// Note that for conveniance, you can provide `steps` as a [`TestStep`] or a slice of tuples, where each tuple contains:
    /// * `height` - height at which claim will be made
    /// * `expected_balance` - expected balance after claim was made
    /// * `expect_pass` - whether we expect the claim to pass or not
    ///
    pub(super) fn check_heights<C: Into<TestStep> + Clone>(
        distribution_function: DistributionFunction,
        steps: &[C],
        contract_start_time: Option<TimestampMillis>,
        distribution_interval: u64,
        max_supply: Option<Option<u64>>,
    ) -> Result<(), String> {
        let mut suite = TestSuite::new(
            10_200_000_000,
            0,
            TokenDistributionType::Perpetual,
            Some(move |token_configuration: &mut TokenConfiguration| {
                token_configuration
                    .distribution_rules_mut()
                    .set_perpetual_distribution(Some(TokenPerpetualDistribution::V0(
                        TokenPerpetualDistributionV0 {
                            distribution_type: RewardDistributionType::BlockBasedDistribution {
                                interval: distribution_interval,
                                function: distribution_function,
                            },
                            distribution_recipient: TokenDistributionRecipient::ContractOwner,
                        },
                    )));
            }),
        );
        if let Some(max_supply) = max_supply {
            suite = suite.with_max_supply(max_supply);
        }

        suite = suite.with_contract_start_time(contract_start_time.unwrap_or(1));

        let steps = steps
            .iter()
            .map(|item| item.clone().into())
            .collect::<Vec<TestStep>>();

        with_timeout(TIMEOUT, move || suite.execute(&steps))
    }

    pub(super) type TokenConfigFn = dyn FnOnce(&mut TokenConfiguration) + Send + Sync;
    /// Test engine to run tests for different token distribution functions.
    pub(crate) struct TestSuite {
        platform: TempPlatform<MockCoreRPCLike>,
        platform_version: &'static PlatformVersion,
        identity: dpp::prelude::Identity,
        signer: SimpleSigner,
        identity_public_key: IdentityPublicKey,
        token_id: Option<dpp::prelude::Identifier>,
        contract: Option<DataContract>,
        start_time: Option<TimestampMillis>,
        token_distribution_type: TokenDistributionType,
        token_configuration_modification: Option<Box<TokenConfigFn>>,
        epoch_index: u16,
        nonce: u64,
        time_between_blocks: u64,

        /// function that will be called after successful claim.
        ///
        /// ## Arguments
        ///
        /// * `u64` - balance after claim
        on_step_success: Box<dyn Fn(u64) + Send + Sync>,
    }

    impl TestSuite {
        /// Create new test suite that will start at provided genesis time and create token contract with provided
        /// configuration.
        pub(crate) fn new<C: FnOnce(&mut TokenConfiguration) + Send + Sync + 'static>(
            genesis_time_ms: u64,
            time_between_blocks: u64,
            token_distribution_type: TokenDistributionType,
            token_configuration_modification: Option<C>,
        ) -> Self {
            let platform_version = PlatformVersion::latest();
            let mut platform = TestPlatformBuilder::new()
                .with_latest_protocol_version()
                .build_with_mock_rpc()
                .set_genesis_state();

            Self::setup_logs();

            let mut rng = StdRng::seed_from_u64(49853);

            let (identity, signer, identity_public_key) =
                setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));

            let me = Self {
                platform,
                platform_version,
                identity,
                signer,
                identity_public_key,
                token_id: None,   // lazy initialization in get_contract/get_token_id
                contract: None,   // lazy initialization in get_contract/get_token_id
                start_time: None, // optional, configured using with_contract_start_time
                token_distribution_type,
                epoch_index: 1,
                nonce: 1,
                time_between_blocks,
                token_configuration_modification: None, // setup later
                on_step_success: Box::new(|_| {}),
            }
            .with_genesis(1, genesis_time_ms);

            if let Some(token_configuration_modification) = token_configuration_modification {
                me.with_token_configuration_modification_fn(token_configuration_modification)
            } else {
                me
            }
        }

        /// Appends new token configuration modification function after existing ones.
        pub(crate) fn with_token_configuration_modification_fn(
            mut self,
            token_configuration_modification: impl FnOnce(&mut TokenConfiguration)
                + Send
                + Sync
                + 'static,
        ) -> Self {
            if let Some(previous) = self.token_configuration_modification.take() {
                let f = Box::new(move |token_configuration: &mut TokenConfiguration| {
                    previous(token_configuration);
                    token_configuration_modification(token_configuration);
                });

                self.token_configuration_modification = Some(f);
            } else {
                // no previous modifications
                let f = Box::new(token_configuration_modification);
                self.token_configuration_modification = Some(f);
            };

            self
        }
        /// Appends a token configuration modification that will change max supply.
        pub(crate) fn with_max_supply(self, max_supply: Option<u64>) -> Self {
            self.with_token_configuration_modification_fn(
                move |token_configuration: &mut TokenConfiguration| {
                    token_configuration.set_max_supply(max_supply);
                },
            )
        }

        /// Enable logging for tests
        fn setup_logs() {
            tracing_subscriber::fmt::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::new(
            "info,dash_sdk=trace,dash_sdk::platform::fetch=debug,drive_proof_verifier=debug,main=debug,h2=info,drive_abci::execution=trace",
        ))
        .pretty()
        .with_ansi(true)
        .with_writer(std::io::stdout)
        .try_init()
        .ok();
        }

        /// Lazily initialize and return token contract. Also sets token id.
        fn get_contract(&mut self) -> DataContract {
            if let Some(ref contract) = self.contract {
                return contract.clone();
            }
            // we `take()` to avoid moving from reference; this means subsequent calls will fail, but we will already have
            // the contract and token id initialized so it should never happen
            let token_config_fn = if let Some(tc) = self.token_configuration_modification.take() {
                let closure = |token_configuration: &mut TokenConfiguration| {
                    // call previous token configuration modification
                    tc(token_configuration);

                    // execute distribution function validation
                    if let Err(e) = validate_distribution_function(
                        token_configuration,
                        self.start_time.unwrap_or(0),
                    ) {
                        panic!("{}", e);
                    };
                };
                Some(closure)
            } else {
                None
            };

            let (contract, token_id) = create_token_contract_with_owner_identity(
                &mut self.platform,
                self.identity.id(),
                token_config_fn,
                self.start_time,
                None,
                self.platform_version,
            );

            self.token_id = Some(token_id);
            self.contract = Some(contract.clone());

            contract
        }

        /// Get token ID or create if needed.
        fn get_token_id(&mut self) -> Identifier {
            if self.token_id.is_none() {
                self.get_contract(); // lazy initialization of token id and contract
            }

            self.token_id
                .expect("expected token id to be initialized in get_contract")
        }

        fn next_identity_nonce(&mut self) -> u64 {
            self.nonce += 1;

            self.nonce
        }

        /// Submit a claim transition and assert the results
        pub(crate) fn claim(&mut self, assertions: Vec<AssertionFn>) -> Result<(), String> {
            let committed_block_info = self.block_info();
            let nonce = self.next_identity_nonce();
            // next block config
            let new_block_info = BlockInfo {
                time_ms: committed_block_info.time_ms + self.time_between_blocks,
                height: committed_block_info.height + 1,
                // no change here
                core_height: committed_block_info.core_height,
                ..committed_block_info
            };

            let claim_transition = BatchTransition::new_token_claim_transition(
                self.get_token_id(),
                self.identity.id(),
                self.get_contract().id(),
                0,
                self.token_distribution_type,
                None,
                &self.identity_public_key,
                nonce,
                0,
                &self.signer,
                self.platform_version,
                None,
                None,
                None,
            )
            .expect("expect to create documents batch transition");

            let claim_serialized_transition = claim_transition
                .serialize_to_bytes()
                .expect("expected documents batch serialized state transition");

            let transaction = self.platform.drive.grove.start_transaction();
            let platform_state = self.platform.state.load();

            let processing_result = self
                .platform
                .platform
                .process_raw_state_transitions(
                    &[claim_serialized_transition.clone()],
                    &platform_state,
                    &new_block_info,
                    &transaction,
                    self.platform_version,
                    false,
                    None,
                )
                .expect("expected to process state transition");

            for (i, assertion) in assertions.iter().enumerate() {
                if let Err(e) = assertion(processing_result.execution_results().as_slice()) {
                    return Err(format!("assertion {} failed: {}", i, e));
                }
            }

            self.platform
                .drive
                .grove
                .commit_transaction(transaction)
                .unwrap()
                .expect("expected to commit transaction");

            Ok(())
        }

        /// Retrieve token balance for the identity and assert it matches expected value.
        pub(crate) fn get_balance(&mut self) -> Result<Option<u64>, String> {
            let token_id = self.get_token_id().to_buffer();

            let balance = self
                .platform
                .drive
                .fetch_identity_token_balance(
                    token_id,
                    self.identity.id().to_buffer(),
                    None,
                    self.platform_version,
                )
                .map_err(|e| format!("failed to fetch token balance: {}", e));

            tracing::trace!("retrieved balance: {:?}", balance);
            balance
        }

        /// Retrieve token balance for the identity and assert it matches expected value.
        pub(crate) fn assert_balance(
            &mut self,
            expected_balance: Option<u64>,
        ) -> Result<(), String> {
            let token_balance = self.get_balance()?;

            if token_balance != expected_balance {
                return Err(format!(
                    "expected balance {:?} but got {:?}",
                    expected_balance, token_balance
                ));
            }

            Ok(())
        }

        fn block_info(&self) -> BlockInfo {
            *self
                .platform
                .state
                .load()
                .last_committed_block_info()
                .as_ref()
                .expect("expected last committed block info")
                .basic_info()
        }
        /// initialize genesis state
        fn with_genesis(self, genesis_core_height: u32, genesis_time_ms: u64) -> Self {
            fast_forward_to_block(
                &self.platform,
                genesis_time_ms,
                1,
                genesis_core_height,
                self.epoch_index,
                false,
            );

            self
        }

        /// Configure custom contract start time; must be called before contract is
        /// initialized.
        pub(super) fn with_contract_start_time(mut self, start_time: TimestampMillis) -> Self {
            if self.contract.is_some() {
                panic!("with_contract_start_time must be called before contract is initialized");
            }
            self.start_time = Some(start_time);
            self
        }

        pub(super) fn with_step_success_fn<'a>(
            mut self,
            step_success_fn: impl Fn(u64) + Send + Sync + 'static,
        ) -> Self
        where
            Self: 'a,
        {
            // fn f(s: TestSuite<C>) {
            //     step_success_fn(s);
            // };
            self.on_step_success = Box::new(step_success_fn);
            self
        }

        /// execute test steps, one by one
        pub(super) fn execute(&mut self, tests: &[TestStep]) -> Result<(), String> {
            let mut errors = String::new();
            for test_case in tests {
                let result = self.execute_step(test_case);
                if let Err(e) = result {
                    errors += format!("\n--> {}: {}\n", test_case.name, e).as_str();
                }
            }

            if errors.is_empty() {
                Ok(())
            } else {
                Err(errors)
            }
        }

        /// Execute a single test step. It fasts forwards to the block height of the test case,
        /// executes the claim and checks the balance.
        pub(super) fn execute_step(&mut self, test_case: &TestStep) -> Result<(), String> {
            let current_height = self.block_info().height;
            let current_core_height = self.block_info().core_height;

            let block_time = if test_case.base_height >= current_height {
                test_case.base_time_ms
                    + self.time_between_blocks * (test_case.base_height - current_height)
            } else {
                // workaround for fast_forward_to_block not allowing to go back in time
                test_case.base_time_ms
            };

            fast_forward_to_block(
                &self.platform,
                block_time,
                test_case.base_height,
                current_core_height,
                self.epoch_index,
                false,
            );
            let mut result = Vec::new();
            if let Err(e) = self.claim(test_case.claim_transition_assertions.clone()) {
                result.push(format!("claim failed: {}", e))
            }

            let balance = self
                .get_balance()
                .map_err(|e| format!("failed to get balance: {}", e))?
                .ok_or("expected balance to be present, but got None".to_string())?;

            if test_case
                .expected_balance
                .is_some_and(|expected_balance| expected_balance != balance)
            {
                result.push(format!(
                    "expected balance {:?} but got {:?}",
                    test_case.expected_balance, balance
                ));
            }

            if result.is_empty() {
                tracing::trace!(
                    "step successful, base height: {}, balance: {}",
                    test_case.base_height,
                    balance
                );
                (self.on_step_success)(balance);
                Ok(())
            } else {
                Err(result.join("\n"))
            }
        }
    }

    /// dyn FnOnce(&mut TokenConfiguration) + Send + Sync;
    fn validate_distribution_function(
        token_configuration: &mut TokenConfiguration,
        contract_start_time: u64,
    ) -> Result<(), String> {
        let TokenConfiguration::V0(token_config) = token_configuration;

        let TokenDistributionRules::V0(dist_rules) = token_config.distribution_rules();

        let TokenPerpetualDistribution::V0(perpetual_distribution) = dist_rules
            .perpetual_distribution()
            .expect("expected perpetual distribution");

        perpetual_distribution
            .distribution_type
            .function()
            .validate(contract_start_time)
            .map_err(|e| format!("invalid distribution function: {:?}", e))?;

        Ok(())
    }

    pub(crate) type AssertionFn = fn(&[StateTransitionExecutionResult]) -> Result<(), String>;

    /// Individual step of a test case.
    #[derive(Clone, Debug)]
    pub(crate) struct TestStep {
        pub(crate) name: String,
        /// height of block just before the claim
        pub(crate) base_height: u64,
        /// time of block before the claim
        pub(crate) base_time_ms: u64,
        /// expected balance is a function that should return the expected balance after committing block
        /// at provided height and time
        pub(crate) expected_balance: Option<u64>,
        /// assertion functions that must be met after executing the claim state transition
        pub(crate) claim_transition_assertions: Vec<AssertionFn>,
    }

    impl TestStep {
        /// Create a new test step with provided claim height and expected balance.
        /// If expect_success is true, we expect the claim to be successful.
        /// If false, we expect the claim to fail.
        ///
        /// If expected_balance is None, we don't check the balance.
        pub(super) fn new(claim_height: u64, expected_balance: u64, expect_success: bool) -> Self {
            let trace_assertion: AssertionFn = |processing_results: &[_]| {
                tracing::trace!(
                    "transaction assertion check for processing results: {:?}",
                    processing_results
                );
                Ok(())
            };
            let assertions: Vec<AssertionFn> = if expect_success {
                vec![
                    |processing_results: &[_]| {
                        tracing::trace!(?processing_results, "expect success");
                        Ok(())
                    },
                    |processing_results: &[_]| match processing_results {
                        [StateTransitionExecutionResult::SuccessfulExecution(_, _)] => Ok(()),
                        _ => Err(format!(
                            "expected SuccessfulExecution, got {:?}",
                            processing_results
                        )),
                    },
                    trace_assertion,
                ]
            } else {
                vec![
                    |processing_results: &[_]| {
                        tracing::trace!(?processing_results, "expect failure");
                        Ok(())
                    },
                    |processing_results: &[_]| match processing_results {
                        [StateTransitionExecutionResult::SuccessfulExecution(_, _)] => {
                            Err("expected error, got SuccessfulExecution".into())
                        }
                        [StateTransitionExecutionResult::InternalError(e)] => {
                            Err(format!("expected normal error, got InternalError: {}", e))
                        }
                        _ => Ok(()),
                    },
                    trace_assertion,
                ]
            };
            Self {
                name: format!("claim at height {}", claim_height),
                base_height: claim_height - 1,
                base_time_ms: 10_200_000_000,
                expected_balance: Some(expected_balance),
                claim_transition_assertions: assertions,
            }
        }
    }

    impl From<(u64, u64, bool)> for TestStep {
        fn from(
            (claim_height, expected_balance, expect_claim_successful): (u64, u64, bool),
        ) -> Self {
            Self::new(claim_height, expected_balance, expect_claim_successful)
        }
    }
}
