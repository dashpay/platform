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
mod fixed_amount {
    use crate::platform_types::state_transitions_processing_result::StateTransitionExecutionResult;

    use super::{test_suite::*, INITIAL_BALANCE};
    use dpp::{
        consensus::{state::state_error::StateError, ConsensusError},
        data_contract::associated_token::token_perpetual_distribution::distribution_function::DistributionFunction,
    };
    use dpp::data_contract::associated_token::token_perpetual_distribution::distribution_function::{MAX_DISTRIBUTION_CYCLES_PARAM, MAX_DISTRIBUTION_PARAM};

    #[test]
    fn fixed_amount_1_interval_1() -> Result<(), String> {
        check_heights(
            DistributionFunction::FixedAmount { amount: 1 },
            &[
                TestStep::new(1, 100_001, true),
                TestStep::new(2, 100_002, true),
                TestStep::new(3, 100_003, true),
                TestStep::new(50, 100_050, true),
            ],
            None,
            1,
            None,
        )
    }

    // Given some token configuration,
    // When a claim is made at block 41 and 50,
    // Then the claim should be successful.
    // If we claim again in the interval it should not be successful.
    #[test]
    fn fixed_amount_50_interval_10() {
        check_heights(
            DistributionFunction::FixedAmount { amount: 50 },
            &[
                TestStep::new(1, 100_000, false),
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
    ///
    /// claim at height 1000000000000: claim failed: assertion 0 failed: expected SuccessfulExecution,
    /// got [InternalError(\"storage: protocol: overflow error: Overflow in FixedAmount evaluation\")]"
    #[test]
    fn fixed_amount_at_trillionth_block() {
        check_heights(
            DistributionFunction::FixedAmount {
                amount: 1_000_000_000,
            },
            &[
                TestStep::new(41, INITIAL_BALANCE + 4 * 1_000_000_000, true),
                TestStep::new(46, INITIAL_BALANCE + 4 * 1_000_000_000, false),
                TestStep::new(50, INITIAL_BALANCE + 5 * 1_000_000_000, true),
                TestStep::new(51, INITIAL_BALANCE + 5 * 1_000_000_000, false),
                // We will be getting MAX_DISTRIBUTION_CYCLES_PARAM intervals of 1_000_000_000 tokens, and we already had 5
                TestStep::new(
                    1_000_000_000_000,
                    INITIAL_BALANCE + (MAX_DISTRIBUTION_CYCLES_PARAM + 5) * 1_000_000_000,
                    true,
                ),
                // We will be getting another MAX_DISTRIBUTION_CYCLES_PARAM intervals of 1_000_000_000 tokens, and we already had 5 + MAX_DISTRIBUTION_CYCLES_PARAM
                TestStep::new(
                    1_000_000_000_000,
                    INITIAL_BALANCE + (MAX_DISTRIBUTION_CYCLES_PARAM * 2 + 5) * 1_000_000_000,
                    true,
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
    fn fixed_amount_0() {
        check_heights(
            DistributionFunction::FixedAmount { amount: 0 },
            &[(41, 100000, false)],
            None,
            10,
            None,
        )
        .expect_err("\namount should not be 0\n");
    }

    #[test]
    /// Given a fixed amount distribution with value of 1_000_000 and max_supply of 200_000,
    /// When we try to claim,
    /// Then we always fail and the balance remains unchanged.
    fn fixed_amount_gt_max_supply() {
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
    fn test_block_based_perpetual_fixed_amount_u64_max_should_error_at_validation() {
        check_heights(
            DistributionFunction::FixedAmount { amount: u64::MAX },
            &[TestStep::new(41, 100_000, false)],
            None,
            10,
            None,
        )
        .expect_err("u64::Max is too much for DistributionFunction::FixedAmount");
    }

    /// Given a fixed amount distribution with value of u64::MAX,
    /// When I claim tokens,
    /// Then I don't get an InternalError.
    #[test]
    fn test_block_based_perpetual_fixed_amount_max_distribution() {
        check_heights(
            DistributionFunction::FixedAmount {
                amount: MAX_DISTRIBUTION_PARAM,
            },
            &[TestStep::new(
                41,
                4 * MAX_DISTRIBUTION_PARAM + 100_000,
                true,
            )],
            None,
            10,
            None,
        )
        .expect("MAX_DISTRIBUTION_PARAM should be valid DistributionFunction::FixedAmount");
    }
}
mod random {
    use std::{
        collections::BTreeMap,
        sync::{Arc, Mutex},
    };

    use crate::execution::validation::state_transition::batch::tests::token::distribution::perpetual::block_based::test_suite::TestSuite;

    use super::{
        test_suite::{check_heights, TestStep},
        INITIAL_BALANCE,
    };
    use dpp::data_contract::associated_token::token_perpetual_distribution::distribution_function::MAX_DISTRIBUTION_PARAM;
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
    #[ignore]
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
    #[ignore]
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
    #[ignore]
    fn test_block_based_perpetual_random_0_u64_max_should_error_at_validation() {
        check_heights(
            DistributionFunction::Random {
                min: 0,
                max: u64::MAX,
            },
            &[TestStep::new(41, INITIAL_BALANCE, false)],
            None,
            10,
            None,
        )
        .expect_err("max is too much for DistributionFunction::Random");
    }

    #[test]
    #[ignore]
    fn test_block_based_perpetual_random_0_max_distribution_param() {
        check_heights(
            DistributionFunction::Random {
                min: 0,
                max: MAX_DISTRIBUTION_PARAM,
            },
            &[
                TestStep::new(41, 382777733174502, true),
                TestStep::new(50, 447703202535488, true),
                TestStep::new(100, 1080112432401531, true),
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
    #[ignore]
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
        // subtract balance from previous step (for first step, subtract initial balance of 100_000)
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

mod step_decreasing {
    use dpp::balances::credits::TokenAmount;
    use dpp::data_contract::associated_token::token_perpetual_distribution::distribution_function::{DistributionFunction, MAX_DISTRIBUTION_PARAM};
    use dpp::prelude::{BlockHeight, BlockHeightInterval};
    use crate::{execution::validation::state_transition::batch::tests::token::distribution::perpetual::block_based::test_suite::check_heights};
    use crate::execution::validation::state_transition::batch::tests::token::distribution::perpetual::block_based::INITIAL_BALANCE;

    const DECREASING_ONE_PERCENT_100K: [TokenAmount; 500] = [
        100000, 99000, 98010, 97029, 96058, 95097, 94146, 93204, 92271, 91348, 90434, 89529, 88633,
        87746, 86868, 85999, 85139, 84287, 83444, 82609, 81782, 80964, 80154, 79352, 78558, 77772,
        76994, 76224, 75461, 74706, 73958, 73218, 72485, 71760, 71042, 70331, 69627, 68930, 68240,
        67557, 66881, 66212, 65549, 64893, 64244, 63601, 62964, 62334, 61710, 61092, 60481, 59876,
        59277, 58684, 58097, 57516, 56940, 56370, 55806, 55247, 54694, 54147, 53605, 53068, 52537,
        52011, 51490, 50975, 50465, 49960, 49460, 48965, 48475, 47990, 47510, 47034, 46563, 46097,
        45636, 45179, 44727, 44279, 43836, 43397, 42963, 42533, 42107, 41685, 41268, 40855, 40446,
        40041, 39640, 39243, 38850, 38461, 38076, 37695, 37318, 36944, 36574, 36208, 35845, 35486,
        35131, 34779, 34431, 34086, 33745, 33407, 33072, 32741, 32413, 32088, 31767, 31449, 31134,
        30822, 30513, 30207, 29904, 29604, 29307, 29013, 28722, 28434, 28149, 27867, 27588, 27312,
        27038, 26767, 26499, 26234, 25971, 25711, 25453, 25198, 24946, 24696, 24449, 24204, 23961,
        23721, 23483, 23248, 23015, 22784, 22556, 22330, 22106, 21884, 21665, 21448, 21233, 21020,
        20809, 20600, 20394, 20190, 19988, 19788, 19590, 19394, 19200, 19008, 18817, 18628, 18441,
        18256, 18073, 17892, 17713, 17535, 17359, 17185, 17013, 16842, 16673, 16506, 16340, 16176,
        16014, 15853, 15694, 15537, 15381, 15227, 15074, 14923, 14773, 14625, 14478, 14333, 14189,
        14047, 13906, 13766, 13628, 13491, 13356, 13222, 13089, 12958, 12828, 12699, 12572, 12446,
        12321, 12197, 12075, 11954, 11834, 11715, 11597, 11481, 11366, 11252, 11139, 11027, 10916,
        10806, 10697, 10590, 10484, 10379, 10275, 10172, 10070, 9969, 9869, 9770, 9672, 9575, 9479,
        9384, 9290, 9197, 9105, 9013, 8922, 8832, 8743, 8655, 8568, 8482, 8397, 8313, 8229, 8146,
        8064, 7983, 7903, 7823, 7744, 7666, 7589, 7513, 7437, 7362, 7288, 7215, 7142, 7070, 6999,
        6929, 6859, 6790, 6722, 6654, 6587, 6521, 6455, 6390, 6326, 6262, 6199, 6137, 6075, 6014,
        5953, 5893, 5834, 5775, 5717, 5659, 5602, 5545, 5489, 5434, 5379, 5325, 5271, 5218, 5165,
        5113, 5061, 5010, 4959, 4909, 4859, 4810, 4761, 4713, 4665, 4618, 4571, 4525, 4479, 4434,
        4389, 4345, 4301, 4257, 4214, 4171, 4129, 4087, 4046, 4005, 3964, 3924, 3884, 3845, 3806,
        3767, 3729, 3691, 3654, 3617, 3580, 3544, 3508, 3472, 3437, 3402, 3367, 3333, 3299, 3266,
        3233, 3200, 3168, 3136, 3104, 3072, 3041, 3010, 2979, 2949, 2919, 2889, 2860, 2831, 2802,
        2773, 2745, 2717, 2689, 2662, 2635, 2608, 2581, 2555, 2529, 2503, 2477, 2452, 2427, 2402,
        2377, 2353, 2329, 2305, 2281, 2258, 2235, 2212, 2189, 2167, 2145, 2123, 2101, 2079, 2058,
        2037, 2016, 1995, 1975, 1955, 1935, 1915, 1895, 1876, 1857, 1838, 1819, 1800, 1782, 1764,
        1746, 1728, 1710, 1692, 1675, 1658, 1641, 1624, 1607, 1590, 1574, 1558, 1542, 1526, 1510,
        1494, 1479, 1464, 1449, 1434, 1419, 1404, 1389, 1375, 1361, 1347, 1333, 1319, 1305, 1291,
        1278, 1265, 1252, 1239, 1226, 1213, 1200, 1188, 1176, 1164, 1152, 1140, 1128, 1116, 1104,
        1092, 1081, 1070, 1059, 1048, 1037, 1026, 1015, 1004, 993, 983, 973, 963, 953, 943, 933,
        923, 913, 903, 893, 884, 875, 866, 857, 848, 839, 830, 821, 812, 803, 794, 786, 778, 770,
        762, 754, 746, 738, 730, 722, 714, 706, 698, 691, 684, 677, 670, 663, 656, 649, 642, 635,
        628, 621, 614,
    ];

    fn sum_till_for_100k_step_1_interval_1(
        distribution_heights: Vec<BlockHeight>,
    ) -> Vec<TokenAmount> {
        distribution_heights
            .into_iter()
            .map(|height| {
                (1..=height)
                    .map(|height| DECREASING_ONE_PERCENT_100K[height as usize])
                    .sum::<TokenAmount>()
                    + INITIAL_BALANCE
            })
            .collect()
    }

    const DECREASING_HALF_100K: [TokenAmount; 20] = [
        100000, 50000, 25000, 12500, 6250, 3125, 1562, 781, 390, 195, 97, 48, 24, 12, 6, 3, 1, 0,
        0, 0,
    ];

    fn sum_till_for_100k_halving(
        distribution_heights: Vec<BlockHeight>,
        reduce_every_block_count: u32,
        interval: BlockHeightInterval,
        start_decreasing_step: u64,
    ) -> Vec<TokenAmount> {
        distribution_heights
            .into_iter()
            .map(|height| {
                // How many full intervals have passed by `height`?
                let end = height / interval;

                // If not even 1 interval, return the initial balance
                if end < 1 {
                    return INITIAL_BALANCE;
                }

                // Sum each interval’s distribution
                let sum_halved = (1..=end)
                    .map(|i| {
                        if i < start_decreasing_step {
                            // Before start offset => always distribute the first entry
                            DECREASING_HALF_100K[0]
                        } else {
                            // After offset => normal indexing
                            let offset_index = ((i - start_decreasing_step) as usize)
                                / (reduce_every_block_count as usize);

                            DECREASING_HALF_100K.get(offset_index).copied().unwrap_or(0)
                        }
                    })
                    .sum::<TokenAmount>();

                INITIAL_BALANCE + sum_halved
            })
            .collect()
    }

    #[test]
    fn claim_every_block() {
        run_test(
            1,
            1,
            100,
            None,
            None,
            10_000,
            0,
            Some(1),
            (1..5).step_by(1).collect(),
            1,
            vec![
                INITIAL_BALANCE + 9_900,
                INITIAL_BALANCE + 9_900 + 9_801,
                INITIAL_BALANCE + 9_900 + 9_801 + 9_702,
                INITIAL_BALANCE + 9_900 + 9_801 + 9_702 + 9_604,
            ],
        )
        .expect("expected to succeed");
    }

    #[test]
    fn claim_every_5_blocks() {
        run_test(
            1,
            1,
            100,
            None,
            None,
            10_000,
            0,
            Some(1),
            vec![1, 6, 11],
            1,
            vec![
                INITIAL_BALANCE + 9_900,
                INITIAL_BALANCE + 9_900 + 9_801 + 9_702 + 9_604 + 9_507 + 9_411,
                INITIAL_BALANCE
                    + 9_900
                    + 9_801
                    + 9_702
                    + 9_604
                    + 9_507
                    + 9_411
                    + 9_316
                    + 9_222
                    + 9_129
                    + 9_037
                    + 8_946,
            ],
        )
        .expect("expected to succeed");
    }

    #[test]
    fn claim_with_1_percent_increase_should_fail() {
        let result_str = run_test(
            1,
            101,
            100,
            None,
            None,
            100_000,
            0,
            Some(1),
            (1..1000).step_by(100).collect(),
            1,
            vec![],
        )
        .expect_err("should not allow to increase");
        assert!(
            result_str.contains("Invalid parameter tuple in token distribution function: `decrease_per_interval_numerator` must be smaller than `decrease_per_interval_denominator`"),
            "Unexpected panic message: {result_str}"
        );
    }

    #[test]
    fn claim_with_no_decrease_should_fail() {
        let result_str = run_test(
            1,
            0,
            100,
            None,
            None,
            100_000,
            0,
            Some(1),
            (1..1000).step_by(100).collect(),
            1,
            vec![],
        )
        .expect_err("should not allow to increase");
        assert!(
            result_str.contains("Invalid parameter `decrease_per_interval_numerator` in token distribution function. Expected range: 1 to 65535"),
            "Unexpected panic message: {result_str}"
        );
    }

    #[test]
    fn claim_every_10_blocks_on_100k() {
        let steps = (1..500).step_by(10).collect::<Vec<_>>();
        run_test(
            1,
            1,
            100,
            None,
            Some(1024),
            100_000,
            0,
            Some(1),
            steps.clone(),
            1,
            sum_till_for_100k_step_1_interval_1(steps),
        )
        .expect("should pass");
    }

    #[test]
    fn claim_every_block_on_100k_128_default_steps() {
        let steps = (1..140).step_by(1).collect::<Vec<_>>();
        let start_steps = (1..129).step_by(1).collect::<Vec<_>>();
        let start_steps_expected_amounts = sum_till_for_100k_step_1_interval_1(start_steps.clone());
        let later_steps = (129..140).step_by(1).collect::<Vec<_>>();
        let later_steps_expected_amounts = later_steps
            .iter()
            .map(|_| *start_steps_expected_amounts.last().unwrap())
            .collect::<Vec<_>>();
        let mut expected_amounts = start_steps_expected_amounts;
        expected_amounts.extend(later_steps_expected_amounts);
        run_test(
            1,
            1,
            100,
            None,
            None,
            100_000,
            0,
            Some(1),
            steps.clone(),
            1,
            expected_amounts,
        )
        .expect("should pass");
    }

    #[test]
    fn claim_every_block_on_100k_128_default_steps_with_trailing_distribution() {
        let steps = (1..200).step_by(1).collect::<Vec<_>>();
        let start_steps = (1..129).step_by(1).collect::<Vec<_>>();
        let start_steps_expected_amounts = sum_till_for_100k_step_1_interval_1(start_steps.clone());
        let later_steps = (129..200).step_by(1).collect::<Vec<_>>();
        let later_steps_expected_amounts = later_steps
            .iter()
            .map(|&i| *start_steps_expected_amounts.last().unwrap() + (i - 128) * 10)
            .collect::<Vec<_>>();
        let mut expected_amounts = start_steps_expected_amounts;
        expected_amounts.extend(later_steps_expected_amounts);
        run_test(
            1,
            1,
            100,
            None,
            None,
            100_000,
            // 10 credits per step afterward
            10,
            Some(1),
            steps.clone(),
            1,
            expected_amounts,
        )
        .expect("should pass");
    }

    #[test]
    fn claim_every_10_blocks_on_100k_128_default_steps() {
        let steps = (1..500).step_by(10).collect::<Vec<_>>();
        let start_steps = (1..128).step_by(10).collect::<Vec<_>>();
        let start_steps_expected_amounts = sum_till_for_100k_step_1_interval_1(start_steps);
        let step_128_amount = sum_till_for_100k_step_1_interval_1(vec![128]).remove(0);
        let later_steps = (141..500).step_by(10).collect::<Vec<_>>();
        let later_steps_expected_amounts = later_steps
            .iter()
            .map(|_| step_128_amount)
            .collect::<Vec<_>>();
        let mut expected_amounts = start_steps_expected_amounts;
        expected_amounts.push(step_128_amount); // at 131.
        expected_amounts.extend(later_steps_expected_amounts);
        run_test(
            1,
            1,
            100,
            None,
            None,
            100_000,
            0,
            Some(1),
            steps.clone(),
            1,
            expected_amounts,
        )
        .expect("should pass");
    }

    #[test]
    fn claim_128_default_steps_480_max_token_redemption_cycles() {
        // We can only claim 128 events at a time.
        // The step_wise distribution stops after 500 from the start.
        let claim_heights = vec![1, 400, 400, 400, 400, 401, 450, 500];
        // 129 is the first claim for 400 because we can only do 128 cycles at a time
        // Then 257 because we are doing 128 cycles and 129 + 128 = 257
        // The last one is 480 because our max steps is 480
        let expected_amounts =
            sum_till_for_100k_step_1_interval_1(vec![1, 129, 257, 385, 400, 401, 450, 480]);
        run_test(
            1,
            1,
            100,
            None,
            Some(480),
            100_000,
            0,
            Some(1),
            // This will give us 1, 151, 301, 400, 401, 450 for result values
            claim_heights,
            1,
            expected_amounts,
        )
        .expect("should pass");
    }

    #[test]
    fn decrease_where_min_would_not_matter_min_1_100() {
        let claim_heights = vec![1, 2, 3, 10, 100];
        let expected_amounts = sum_till_for_100k_step_1_interval_1(claim_heights.clone());
        for min in [1, 100] {
            run_test(
                1,
                1,
                100,
                None,
                None,
                100_000,
                0,
                Some(min),
                claim_heights.clone(),
                1,
                expected_amounts.clone(),
            )
            .map_err(|e| format!("failed with min {}: {}", min, e))
            .expect("should pass");
        }
    }

    #[test]
    fn heavy_decrease_to_min_with_min_various_values() {
        let claim_heights = vec![1, 2, 3, 10, 100];
        for min in [1, 10] {
            let expected_amounts = vec![
                INITIAL_BALANCE + min,
                INITIAL_BALANCE + 2 * min,
                INITIAL_BALANCE + 3 * min,
                INITIAL_BALANCE + 10 * min,
                INITIAL_BALANCE + 100 * min,
            ];
            run_test(
                1,
                u16::MAX - 1,
                u16::MAX,
                None,
                None,
                100_000,
                0,
                Some(min),
                claim_heights.clone(),
                1,
                expected_amounts,
            )
            .map_err(|e| format!("failed with min {}: {}", min, e))
            .expect("should pass");
        }
    }

    #[test]
    fn full_decrease_min_eq_u64_max() {
        let result_str = run_test(
            1,
            u16::MAX - 1,
            u16::MAX,
            None,
            None,
            MAX_DISTRIBUTION_PARAM,
            0,
            Some(u64::MAX),
            vec![1, 2, 3, 10, 100],
            1,
            vec![],
        )
        .expect_err("should fail");
        assert!(
            result_str.contains("Invalid parameter tuple in token distribution function: `n` must be greater than or equal to `min_value`"),
            "Unexpected panic message: {result_str}"
        );
    }
    #[test]
    fn full_decrease_min_eq_max_distribution() {
        run_test(
            1,
            u16::MAX - 1,
            u16::MAX,
            None,
            None,
            MAX_DISTRIBUTION_PARAM,
            0,
            Some(MAX_DISTRIBUTION_PARAM),
            vec![1, 2, 10],
            1,
            vec![
                MAX_DISTRIBUTION_PARAM + INITIAL_BALANCE,
                MAX_DISTRIBUTION_PARAM * 2 + INITIAL_BALANCE,
                MAX_DISTRIBUTION_PARAM * 10 + INITIAL_BALANCE,
            ],
        )
        .expect("should succeed");
    }

    #[test]
    fn distribute_max_distribution_param_every_step() {
        let claim_heights = (1..65_536).step_by(128).collect::<Vec<_>>();
        let expected_balances = claim_heights
            .iter()
            .map(|&height| {
                MAX_DISTRIBUTION_PARAM
                    .saturating_mul(height)
                    .saturating_add(INITIAL_BALANCE)
                    .min(i64::MAX as u64)
            })
            .collect();
        run_test(
            1,
            u16::MAX - 1,
            u16::MAX,
            None,
            None,
            MAX_DISTRIBUTION_PARAM,
            MAX_DISTRIBUTION_PARAM,
            Some(MAX_DISTRIBUTION_PARAM),
            claim_heights,
            1,
            expected_balances,
        )
        .expect("should succeed");
    }

    #[test]
    fn start_over_max_distribution_param_should_fail() {
        let result_str = run_test(
            1,
            1,
            u16::MAX,
            None,
            None,
            MAX_DISTRIBUTION_PARAM + 1,
            0,
            None,
            vec![1, 2, 10],
            1,
            vec![],
        )
        .expect_err("should fail");
        assert!(
            result_str.contains("Invalid parameter `n` in token distribution function. Expected range: 1 to 281474976710655"),
            "Unexpected panic message: {result_str}"
        );
    }

    #[test]
    fn half_decrease_changing_step_5_distribution_interval_1() {
        let step = 5; // Every 5 blocks the amount divides by 1/2
        let distribution_interval = 1; // The payout happens every block
        let claim_heights = vec![5, 10, 18, 22, 100];
        let expected_balances =
            sum_till_for_100k_halving(claim_heights.clone(), step, distribution_interval, 0);
        run_test(
            step,
            1,
            2,
            None,
            None,
            100_000,
            0,
            None,
            claim_heights,
            distribution_interval,
            expected_balances,
        )
        .expect("should pass");
    }

    #[test]
    fn half_decrease_changing_step_5_distribution_interval_5() {
        let step = 5; // Every 25 blocks (5 x distribution interval) the amount divides by 1/2
        let distribution_interval = 5; // The payout happens every 5 blocks
        let claim_heights = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 18, 22, 25, 26, 51, 100];
        let expected_balances =
            sum_till_for_100k_halving(claim_heights.clone(), step, distribution_interval, 0);
        run_test(
            step,
            1,
            2,
            None,
            None,
            100_000,
            0,
            None,
            claim_heights,
            distribution_interval,
            expected_balances,
        )
        .expect("should pass");
    }

    #[test]
    fn half_decrease_changing_step_24_distribution_interval_1000() {
        let step = 24; // Every 24000 blocks (24 x distribution interval) the amount divides by 1/2
        let distribution_interval = 1000; // The payout happens every 400 blocks
        let claim_heights = vec![3000, 45000, 60000, 300000, 300000];
        let value_heights = vec![3000, 45000, 60000, 60000 + 128 * 1000, 300000];
        let expected_balances =
            sum_till_for_100k_halving(value_heights, step, distribution_interval, 0);
        run_test(
            step,
            1,
            2,
            None,
            None,
            100_000,
            0,
            None,
            claim_heights,
            distribution_interval,
            expected_balances,
        )
        .expect("should pass");
    }

    #[test]
    fn half_decrease_changing_step_24_distribution_interval_1000_start_height_2000() {
        let step = 24; // Every 24000 blocks (24 x distribution interval) the amount divides by 1/2
        let distribution_interval = 1000; // The payout happens every 400 blocks
        let claim_heights = vec![3000, 23000, 24000, 25000, 43000, 44000, 300000, 300000];
        let start_height = 2000;
        let value_heights = vec![
            3000,
            23000,
            24000,
            25000,
            43000,
            44000,
            44000 + 128 * 1000,
            300000,
        ];
        let expected_balances = sum_till_for_100k_halving(
            value_heights,
            step,
            distribution_interval,
            start_height / distribution_interval,
        );
        run_test(
            step,
            1,
            2,
            Some(start_height / distribution_interval),
            None,
            100_000,
            0,
            None,
            claim_heights,
            distribution_interval,
            expected_balances,
        )
        .expect("should pass");
    }

    /// Test various combinations of [DistributionFunction::StepDecreasingAmount] distribution.
    #[allow(clippy::too_many_arguments)]
    fn run_test(
        step_count: u32,
        decrease_per_interval_numerator: u16,
        decrease_per_interval_denominator: u16,
        start_decreasing_offset: Option<BlockHeight>,
        max_interval_count: Option<u16>,
        distribution_start_amount: TokenAmount,
        trailing_distribution_interval_amount: TokenAmount,
        min_value: Option<TokenAmount>,
        claim_heights: Vec<BlockHeight>,
        distribution_interval: BlockHeightInterval,
        mut expected_balances: Vec<TokenAmount>,
    ) -> Result<(), String> {
        let dist = DistributionFunction::StepDecreasingAmount {
            step_count,
            decrease_per_interval_numerator,
            decrease_per_interval_denominator,
            start_decreasing_offset,
            max_interval_count,
            distribution_start_amount,
            trailing_distribution_interval_amount,
            min_value,
        };

        if claim_heights.len() != expected_balances.len() {
            expected_balances = (0..claim_heights.len()).map(|_| 0u64).collect();
        }

        let mut prev = None;
        let claims = claim_heights
            .iter()
            .zip(expected_balances.iter())
            .map(|(&h, &b)| {
                let is_increase = match prev {
                    Some(p) => b > p || b == i64::MAX as u64,
                    None => b > INITIAL_BALANCE,
                };
                prev = Some(b);
                (h, b, is_increase)
            })
            .collect::<Vec<_>>();

        // we return Err(()) to make result comparison easier in test_case
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
}

mod stepwise {
    use super::test_suite::check_heights;
    use dpp::data_contract::associated_token::token_perpetual_distribution::distribution_function::DistributionFunction;
    use std::collections::BTreeMap;

    #[test]
    fn distribution_stepwise_correct() {
        let distribution_interval = 10;
        let periods = BTreeMap::from([
            (0, 10_000), // h 1-20
            (2, 20_000), // h 20+
            (45, 30_000),
            (50, 40_000),
            (70, 50_000),
        ]);

        let dist = DistributionFunction::Stepwise(periods);

        // claims: height, balance, expect_pass
        let steps = [
            (1, 100_000, false),
            (9, 100_000, false),
            (10, 110_000, true),
            (11, 110_000, false),
            (19, 110_000, false),
            (20, 130_000, true),
            (21, 130_000, false),
            (24, 130_000, false),
            (35, 150_000, true),
            (39, 150_000, false),
            (46, 170_000, true),
            (49, 170_000, false),
            (51, 190_000, true),
            (200, 490_000, true),
            (300, 690_000, true),
            (
                1_000_000, 6_370_000, // because we only do 128 steps at a time.
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

mod linear {
    use super::test_suite::check_heights;
    use dpp::data_contract::associated_token::token_perpetual_distribution::distribution_function::{DistributionFunction, MAX_LINEAR_SLOPE_A_PARAM, MIN_LINEAR_SLOPE_A_PARAM};

    #[test]
    fn linear_distribution_divide_by_max() -> Result<(), String> {
        // Given linear distribution with d=MAX and starting amount of 1,
        // We expect no claim rewards
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
    fn linear_distribution_x_matrix() -> Result<(), String> {
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
    fn linear_distribution_slopes() -> Result<(), String> {
        for (a, steps) in [
            (-1, [(1, 100_000, false), (20, 100_000, false)]),
            (1, [(1, 100_001, true), (20, 100_210, true)]),
            (
                MIN_LINEAR_SLOPE_A_PARAM,
                [(1, 100_000, false), (20, 100_000, false)],
            ),
            (
                MAX_LINEAR_SLOPE_A_PARAM as i64,
                [(1, 100_256, true), (20, 153_760, true)],
            ),
        ] {
            test_linear(a, 1, None, 0, None, None, &steps, 1)?;
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

#[cfg(test)]
mod exponential {
    use super::test_suite::{check_heights, TestStep, TestSuite};
    use crate::platform_types::state_transitions_processing_result::StateTransitionExecutionResult;
    use dpp::data_contract::{
        associated_token::{
            token_configuration::accessors::v0::TokenConfigurationV0Getters,
            token_distribution_key::TokenDistributionType,
            token_distribution_rules::accessors::v0::TokenDistributionRulesV0Setters,
            token_perpetual_distribution::{
                distribution_function::DistributionFunction::{self, Exponential},
                distribution_recipient::TokenDistributionRecipient,
                reward_distribution_type::RewardDistributionType,
                v0::TokenPerpetualDistributionV0,
                TokenPerpetualDistribution,
            },
        },
        TokenConfiguration,
    };
    use dpp::data_contract::associated_token::token_perpetual_distribution::distribution_function::{MAX_DISTRIBUTION_PARAM, MAX_EXP_A_PARAM, MAX_EXP_M_PARAM, MAX_EXP_N_PARAM, MIN_EXP_M_PARAM};

    // ─────────────────────────────────────────────────────────────────────────
    // helper – one‑liner wrapper around `check_heights` (same as polynomial)
    // ─────────────────────────────────────────────────────────────────────────
    fn test_exponential(
        dist: DistributionFunction,
        steps: &[(u64, u64, bool)], // (height, expected balance, expect‑pass)
        distribution_interval: u64,
    ) -> Result<(), String> {
        check_heights(dist, steps, None, distribution_interval, None)
            .inspect_err(|e| tracing::error!("{e}"))
    }

    // ─────────────────────────────────────────────────────────────────────────
    // 1.  Basic positive‑growth example  (m > 0)
    // ─────────────────────────────────────────────────────────────────────────
    #[test]
    fn exponential_distribution_growth_basic() -> Result<(), String> {
        test_exponential(
            Exponential {
                a: 1,
                d: 1,
                m: 1, // positive  ⇒ growth
                n: 1,
                o: 0,
                start_moment: Some(1),
                b: 0,
                min_value: None,
                max_value: Some(1_000_000),
            },
            // heights 10 and 20 should both succeed – balances are illustrative
            &[(10, 112_814, true), (20, 6_799_881, true)],
            1,
        )
    }

    // ─────────────────────────────────────────────────────────────────────────
    // 2.  Basic negative‑decay example  (m < 0)
    // ─────────────────────────────────────────────────────────────────────────
    #[test]
    fn exponential_distribution_decay_basic() -> Result<(), String> {
        test_exponential(
            Exponential {
                a: 5,
                d: 1,
                m: -1, // negative  ⇒ decay
                n: 1,
                o: 0,
                start_moment: Some(1),
                b: 100_000,
                min_value: Some(50_000),
                max_value: None,
            },
            &[(1, 200_005, true), (4, 500_006, true)],
            1,
        )
    }

    // ─────────────────────────────────────────────────────────────────────────
    // 3.  o at −MAX_DISTRIBUTION_PARAM ⇒ argument very negative ▶ min / 0
    // ─────────────────────────────────────────────────────────────────────────
    #[test]
    fn exponential_distribution_o_min() -> Result<(), String> {
        test_exponential(
            Exponential {
                a: 1,
                d: 1,
                m: 1,
                n: 1,
                o: -(MAX_DISTRIBUTION_PARAM as i64),
                start_moment: Some(1),
                b: 0,
                min_value: None,
                max_value: Some(MAX_DISTRIBUTION_PARAM),
            },
            &[(1, 100_000, false), (4, 100_000, false)],
            1,
        )
    }

    // ─────────────────────────────────────────────────────────────────────────
    // 4.  o at +MAX_DISTRIBUTION_PARAM  (huge positive shift)
    // ─────────────────────────────────────────────────────────────────────────
    #[test]
    fn exponential_distribution_o_max() -> Result<(), String> {
        test_exponential(
            Exponential {
                a: MAX_EXP_A_PARAM,
                d: 1,
                m: -1,
                n: 32,
                o: MAX_DISTRIBUTION_PARAM as i64,
                start_moment: Some(1),
                b: 10,
                min_value: None,
                max_value: Some(MAX_DISTRIBUTION_PARAM),
            },
            &[(1, 100010, true), (10, 100100, true)],
            1,
        )
    }

    // ─────────────────────────────────────────────────────────────────────────
    // 5.  Exhaustive combination of extreme parameter values
    //     ‑ ensure no `InternalError`
    // ─────────────────────────────────────────────────────────────────────────
    #[test]
    fn exponential_distribution_extreme_values() -> Result<(), String> {
        for m in [MIN_EXP_M_PARAM, -1, 1, MAX_EXP_M_PARAM as i64] {
            for n in [1, MAX_EXP_N_PARAM] {
                for a in [1, MAX_EXP_A_PARAM] {
                    let dist = Exponential {
                        a,
                        d: 1,
                        m,
                        n,
                        o: 0,
                        start_moment: Some(1),
                        b: 0,
                        min_value: None,
                        max_value: Some(MAX_DISTRIBUTION_PARAM),
                    };

                    let mut suite = TestSuite::new(
                        10_200_000_000, // initial balance
                        0,              // owner balance
                        TokenDistributionType::Perpetual,
                        Some(move |cfg: &mut TokenConfiguration| {
                            cfg.distribution_rules_mut()
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
                        name: "extreme".into(),
                    };

                    suite
                        .execute(&[step])
                        .map_err(|e| format!("failed with a {a} m {m} n {n}: {e}"))?;
                }
            }
        }
        Ok(())
    }
}

mod polynomial {
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
    use dpp::data_contract::associated_token::token_perpetual_distribution::distribution_function::{MAX_DISTRIBUTION_PARAM, MAX_POL_A_PARAM, MAX_POL_M_PARAM, MAX_POL_N_PARAM, MIN_POL_A_PARAM, MIN_POL_M_PARAM};

    #[test]
    fn polynomial_distribution_basic() -> Result<(), String> {
        test_polynomial(
            Polynomial {
                a: 1,
                d: 1,
                m: 2,
                n: 1,
                o: 1,
                start_moment: Some(1),
                b: 0,
                min_value: None,
                max_value: None,
            },
            &[(10, 100_385, true), (20, 102_870, true)],
            1,
        )
    }

    #[test]
    fn polynomial_distribution_negative_a() -> Result<(), String> {
        test_polynomial(
            Polynomial {
                a: -1,
                d: 1,
                m: 3,
                n: 1,
                o: 1,
                start_moment: Some(1),
                b: 100_000,
                min_value: None,
                max_value: None,
            },
            &[(1, 199_999, true), (4, 499_900, true)],
            1,
        )
    }

    #[test]
    fn polynomial_distribution_a_minus_1_b_0() -> Result<(), String> {
        test_polynomial(
            Polynomial {
                a: -1,
                d: 1,
                m: 2,
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

    ///  Given a polynomial distribution function with o=-MAX_DISTRIBUTION_PARAM, we should
    /// have no rewards
    #[test]
    fn polynomial_distribution_o_min() -> Result<(), String> {
        test_polynomial(
            Polynomial {
                a: 1,
                d: 1,
                m: 2,
                n: 1,
                o: -(MAX_DISTRIBUTION_PARAM as i64),
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
    fn polynomial_distribution_pow_minus_1_at_h_2() -> Result<(), String> {
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
                (4, 100_003, true),  // 3.pow(1/2) == 1.73 - should round to 1
                (5, 100_005, true),  // 4.pow(1/2) == 2
                (6, 100_007, true),  // 5.pow(1/2) == 2.23 - should round to 2
            ],
            1,
        )
    }

    #[test]
    fn polynomial_distribution_o_max() -> Result<(), String> {
        test_polynomial(
            Polynomial {
                a: 1,
                d: 1,
                m: 2,
                n: 1,
                o: MAX_DISTRIBUTION_PARAM as i64,
                start_moment: Some(1),
                b: 0,
                min_value: None,
                max_value: None,
            },
            &[(1, 281474976810655, true), (10, 2814749767206550, true)],
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

    /// Test various combinations of `m/n` in `[DistributionFunction::Polynomial]` distribution.
    ///
    /// We expect this test not to end with InternalError.
    #[test]
    fn polynomial_distribution_power_extreme_values() -> Result<(), String> {
        for m in [MIN_POL_M_PARAM, MAX_POL_M_PARAM] {
            for n in [1, MAX_POL_N_PARAM] {
                for a in [MIN_POL_A_PARAM, MAX_POL_A_PARAM] {
                    for b in [0, MAX_DISTRIBUTION_PARAM] {
                        for o in [
                            -(MAX_DISTRIBUTION_PARAM as i64),
                            0,
                            MAX_DISTRIBUTION_PARAM as i64,
                        ] {
                            let dist = Polynomial {
                                a,
                                d: 1,
                                m,
                                n,
                                o,
                                start_moment: Some(1),
                                b,
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
                                        .set_perpetual_distribution(
                                        Some(TokenPerpetualDistribution::V0(
                                            TokenPerpetualDistributionV0 {
                                                distribution_type:
                                                    RewardDistributionType::BlockBasedDistribution {
                                                        interval: 1,
                                                        function: dist,
                                                    },
                                                distribution_recipient:
                                                    TokenDistributionRecipient::ContractOwner,
                                            },
                                        )),
                                    );
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
                }
            }
        }

        Ok(())
    }
}

mod logarithmic {

    use super::test_suite::check_heights;
    use dpp::data_contract::associated_token::token_perpetual_distribution::distribution_function::DistributionFunction::{self,Logarithmic};
    use dpp::data_contract::associated_token::token_perpetual_distribution::distribution_function::{MAX_DISTRIBUTION_PARAM, MAX_LOG_A_PARAM, MIN_LOG_A_PARAM};

    #[test]
    fn log_distribution_basic() -> Result<(), String> {
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
                (1, 100_001, true), // ln(0)+1 = 1
                (2, 100_002, true), // ln(1)+1 = 1
                (3, 100_004, true), // ln(3)+1 = 2
                (4, 100_006, true), // ln(4)+1 = 2
            ],
            1,
        )
    }

    #[test]
    fn log_distribution_1_div_u64_max() -> Result<(), String> {
        // n is very big here, so we would expect to get 0
        test_logarithmic(
            Logarithmic {
                a: 1,                  // a: i64,
                d: 1,                  // d: u64,
                m: 1,                  // m: u64,
                n: u64::MAX,           // n: u64,
                o: 0,                  // o: i64,
                start_moment: Some(0), // start_moment: Option<u64>,
                b: 0,                  // b: TokenAmount,
                min_value: None,       // min_value: Option<u64>,
                max_value: None,       // max_value: Option<u64>,
            },
            &[(1, 100_000, false), (5, 100_000, false)],
            1,
        )
    }

    #[test]
    fn log_distribution_neg_1_div_u64_max() -> Result<(), String> {
        // n is very big here, so we would expect to get 0
        test_logarithmic(
            Logarithmic {
                a: -1,                 // a: i64,
                d: 1,                  // d: u64,
                m: 1,                  // m: u64,
                n: u64::MAX,           // n: u64,
                o: 0,                  // o: i64,
                start_moment: Some(0), // start_moment: Option<u64>,
                b: 0,                  // b: TokenAmount,
                min_value: None,       // min_value: Option<u64>,
                max_value: None,       // max_value: Option<u64>,
            },
            &[(1, 100_044, true), (5, 100_214, true)],
            1,
        )
    }

    #[test]
    fn log_distribution_a_min() -> Result<(), String> {
        test_logarithmic(
            Logarithmic {
                a: MIN_LOG_A_PARAM,    // a: i64,
                d: 1,                  // d: u64,
                m: 1,                  // m: u64,
                n: 1,                  // n: u64,
                o: 1,                  // o: i64,
                start_moment: Some(1), // start_moment: Option<u64>,
                b: 1,                  // b: TokenAmount,
                min_value: None,       // min_value: Option<u64>,
                max_value: None,       // max_value: Option<u64>,
            },
            // f(x) = (a * log(m * (x - s + o) / n)) / d + b
            &[
                (1, 100_001, true),
                (2, 100_001, false),
                (9, 100_001, false),
                (10, 100_001, false),
            ],
            1,
        )
    }

    #[test]
    fn log_distribution_max_amounts() {
        test_logarithmic(
            Logarithmic {
                a: MAX_LOG_A_PARAM,               // a: i64,
                d: 1,                             // d: u64,
                m: MAX_DISTRIBUTION_PARAM,        // m: u64,
                n: 1,                             // n: u64,
                o: MAX_DISTRIBUTION_PARAM as i64, // o: i64,
                start_moment: Some(0),            // start_moment: Option<u64>,
                b: MAX_DISTRIBUTION_PARAM,        // b: TokenAmount,
                min_value: None,                  // min_value: Option<u64>,
                max_value: None,                  // max_value: Option<u64>,
            },
            &[
                (1, 281474978991040, true),
                (9, 2533274810119360, true),
                (10, 2814749789010400, true),
                (200, 38843547087063520, true),
            ],
            1,
        )
        .expect("expect to pass");
    }

    #[test]
    fn log_distribution_with_b_max() -> Result<(), String> {
        test_logarithmic(
            Logarithmic {
                a: 1,                      // a: i64,
                d: 1,                      // d: u64,
                m: 1,                      // m: u64,
                n: 1,                      // n: u64,
                o: 1,                      // o: i64,
                start_moment: Some(1),     // start_moment: Option<u64>,
                b: MAX_DISTRIBUTION_PARAM, // b: TokenAmount,
                min_value: None,           // min_value: Option<u64>,
                max_value: None,           // max_value: Option<u64>,
            },
            &[
                (1, 281474976810655, true), // We start at 1
                (9, 2533274790495904, true),
                (10, 2814749767206561, true),
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

mod inverted_logarithmic {
    use super::test_suite::check_heights;
    use dpp::data_contract::associated_token::token_perpetual_distribution::distribution_function::DistributionFunction::{self,InvertedLogarithmic};

    #[test]
    fn inv_log_distribution_very_low_emission() -> Result<(), String> {
        // At block 2 no more can ever be claimed because the function is decreasing
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
            (2, 100_001, false),
            (50000, 100_001, false),
        ];
        let x_1 = dist.evaluate(0, 1).expect("expected to evaluate");
        assert_eq!(x_1, 1); // This is ln (1/ (1 - 1 + 1)), or basically ln(1) = 1
        let x_2 = dist.evaluate(0, 2).expect("expected to evaluate");
        assert_eq!(x_2, 0); // This is ln (1/ (1 - 1 + 2)), or basically ln(1/2) = 0
        run_test(dist, &steps, 1)
    }

    #[test]
    fn inv_log_distribution_reduced_emission() -> Result<(), String> {
        //       y
        //       ↑
        // 10000 |*
        //  9000 | *
        //  8000 |  *
        //  7000 |   *
        //  6000 |    *
        //  5000 |     *
        //  4000 |       *
        //  3000 |         *
        //  2000 |           *
        //  1000 |              *
        //     0 +-------------------*----------→ x
        //         0     2000   4000   6000   8000
        let dist = InvertedLogarithmic {
            a: 10000,              // a: i64,
            d: 1,                  // d: u64,
            m: 1,                  // m: u64,
            n: 5000,               // n: u64,
            o: 0,                  // o: i64,
            start_moment: Some(0), // start_moment: Option<u64>,
            b: 0,                  // b: TokenAmount,
            min_value: None,       // min_value: Option<u64>,
            max_value: None,       // max_value: Option<u64>,
        };
        let x_1 = dist.evaluate(0, 1).expect("expected to evaluate");
        let x_2 = dist.evaluate(0, 2).expect("expected to evaluate");
        let x_1000 = dist.evaluate(0, 1000).expect("expected to evaluate");
        let x_4000 = dist.evaluate(0, 4000).expect("expected to evaluate");
        let x_5000 = dist.evaluate(0, 5000).expect("expected to evaluate");
        let x_6000 = dist.evaluate(0, 6000).expect("expected to evaluate");
        assert_eq!(x_1, 85171);
        assert_eq!(x_2, 78240);
        assert_eq!(x_1000, 16094);
        assert_eq!(x_4000, 2231);
        assert_eq!(x_5000, 0);
        assert_eq!(x_6000, 0);
        let steps = [
            (1, 185_171, true),
            (2, 263_411, true),
            (1000, 6_110_958, true),
        ];

        run_test(dist, &steps, 1)
    }

    #[test]
    fn inv_log_distribution_reduced_emission_passing_0() -> Result<(), String> {
        //         y
        //         ↑
        //     350 |*
        //     300 | *
        //     250 |  *
        //     200 |   *
        //     150 |     *
        //     100 |       *
        //      50 |         *
        //       0 +-------------*--------------→ x
        //         0     100    200   300   400
        let dist = InvertedLogarithmic {
            a: 100,                // a: i64,
            d: 1,                  // d: u64,
            m: 1,                  // m: u64,
            n: 200,                // n: u64,
            o: 0,                  // o: i64,
            start_moment: Some(0), // start_moment: Option<u64>,
            b: 0,                  // b: TokenAmount,
            min_value: None,       // min_value: Option<u64>,
            max_value: None,       // max_value: Option<u64>,
        };
        let steps = [
            (1, 100529, true),
            (2, 100989, true),
            (100, 116559, true),
            (210, 119546, true),
            (300, 119546, false), // past 200 we won't get any more
        ];

        run_test(dist, &steps, 1)
    }

    #[test]
    fn inv_log_distribution_negative_a_increase_emission() -> Result<(), String> {
        //         y
        //          ↑
        //    10000 |
        //     9000 |
        //     8000 |
        //     7000 |                                                    *
        //     6000 |                                 *
        //     5000 |                    *
        //     4000 |           *
        //     3000 |      *
        //     2000 |  *
        //     1000 *
        //        0 +-------------------------------------------→ x
        //          0       5k     10k     15k     20k     25k     30k
        let dist = InvertedLogarithmic {
            a: -2200,              // a: i64,
            d: 1,                  // d: u64,
            m: 1,                  // m: u64,
            n: 10000,              // n: u64,
            o: 3000,               // o: i64,
            start_moment: Some(0), // start_moment: Option<u64>,
            b: 4000,               // b: TokenAmount,
            min_value: None,       // min_value: Option<u64>,
            max_value: None,       // max_value: Option<u64>,
        };
        let x_1 = dist.evaluate(0, 1).expect("expected to evaluate");
        let x_2 = dist.evaluate(0, 2).expect("expected to evaluate");
        let x_1000 = dist.evaluate(0, 1000).expect("expected to evaluate");
        let x_4000 = dist.evaluate(0, 4000).expect("expected to evaluate");
        assert_eq!(x_1, 1351);
        assert_eq!(x_2, 1352);
        assert_eq!(x_1000, 1984);
        assert_eq!(x_4000, 3215);
        let steps = [
            (1, 101351, true),
            (2, 102703, true),
            (100, 238739, true),
            (210, 399539, true),
            (300, 537282, true),
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
    use dpp::data_contract::associated_token::token_keeps_history_rules::accessors::v0::TokenKeepsHistoryRulesV0Setters;
    use dpp::data_contract::associated_token::token_perpetual_distribution::distribution_function::DistributionFunction;
    use dpp::data_contract::associated_token::token_perpetual_distribution::distribution_recipient::TokenDistributionRecipient;
    use dpp::data_contract::associated_token::token_perpetual_distribution::reward_distribution_type::RewardDistributionType;
    use dpp::data_contract::associated_token::token_perpetual_distribution::v0::TokenPerpetualDistributionV0;
    use dpp::data_contract::associated_token::token_perpetual_distribution::TokenPerpetualDistribution;
    use dpp::prelude::{DataContract, IdentityPublicKey, TimestampMillis};
    use drive::drive::Drive;
    use simple_signer::signer::SimpleSigner;
    use std::sync::Arc;
    use dpp::state_transition::batch_transition::batched_transition::BatchedTransition;
    use dpp::state_transition::batch_transition::batched_transition::token_transition::TokenTransitionV0Methods;
    use dpp::state_transition::proof_result::StateTransitionProofResult;
    use dpp::state_transition::StateTransition;

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
    /// Note that for convenience, you can provide `steps` as a [`TestStep`] or a slice of tuples, where each tuple contains:
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
                    .keeps_history_mut()
                    .set_all_keeps_history(true);
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

        suite.execute(&steps)
    }

    pub(super) type TokenConfigFn = dyn FnOnce(&mut TokenConfiguration) + Send + Sync;
    /// Test engine to run tests for different token distribution functions.
    pub(crate) struct TestSuite {
        platform: TempPlatform<MockCoreRPCLike>,
        platform_version: &'static PlatformVersion,
        identity: dpp::prelude::Identity,
        signer: SimpleSigner,
        identity_public_key: IdentityPublicKey,
        token_id: Option<Identifier>,
        contract: Option<DataContract>,
        token_configuration: Option<TokenConfiguration>,
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
                token_id: None, // lazy initialization in get_contract/get_token_id
                contract: None, // lazy initialization in get_contract/get_token_id
                token_configuration: None, // lazy initialization in get_token_config/get_token_id
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

        /// Lazily initialize and return the token configuration.
        ///
        /// If the token configuration was already created, returns it.
        /// Otherwise, it will initialize the token contract first (if necessary)
        /// and extract the configuration from it.
        fn get_token_config(&mut self) -> Result<TokenConfiguration, String> {
            if let Some(ref token_config) = self.token_configuration {
                return Ok(token_config.clone());
            }

            // If contract is not yet initialized, initialize it
            let contract = self.get_contract()?;

            // Now extract TokenConfiguration from contract
            let token_config = contract
                .expected_token_configuration(0)
                .expect("expected to get token configuration")
                .clone();

            self.token_configuration = Some(token_config.clone());

            Ok(token_config)
        }

        /// Lazily initialize and return token contract. Also sets token id.
        fn get_contract(&mut self) -> Result<DataContract, String> {
            if let Some(ref contract) = self.contract {
                return Ok(contract.clone());
            }
            // we `take()` to avoid moving from reference; this means subsequent calls will fail, but we will already have
            // the contract and token id initialized so it should never happen
            let token_config_fn = if let Some(tc) = self.token_configuration_modification.take() {
                let closure = |token_configuration: &mut TokenConfiguration| {
                    // call previous token configuration modification
                    tc(token_configuration);
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

            // Now extract TokenConfiguration from contract
            let token_config = contract
                .expected_token_configuration(0)
                .expect("expected to get token configuration")
                .clone();

            self.token_configuration = Some(token_config.clone());

            self.token_id = Some(token_id);
            self.contract = Some(contract.clone());

            // execute distribution function validation
            validate_distribution_function(&token_config, self.start_time.unwrap_or(0))
                .map_err(|e| e.to_string())?;

            Ok(contract)
        }

        /// Get token ID or create if needed.
        fn get_token_id(&mut self) -> Identifier {
            if self.token_id.is_none() {
                self.get_contract().ok(); // lazy initialization of token id and contract
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

            let contract = self.get_contract()?;

            let claim_transition = BatchTransition::new_token_claim_transition(
                self.get_token_id(),
                self.identity.id(),
                contract.id(),
                0,
                self.token_distribution_type,
                None,
                &self.identity_public_key,
                nonce,
                0,
                &self.signer,
                self.platform_version,
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

            if processing_result.valid_count() == 1 {
                let proof_result = self
                    .platform
                    .drive
                    .prove_state_transition(&claim_transition, None, self.platform_version)
                    .map_err(|e| e.to_string())?;

                if let Some(proof_error) = proof_result.first_error() {
                    return Err(format!(
                        "proof_result is not valid with error {}",
                        proof_error
                    ));
                }

                let proof_data = proof_result.into_data().map_err(|e| e.to_string())?;

                let contract = Arc::from(self.get_contract()?);

                let (_, verification_result) =
                    Drive::verify_state_transition_was_executed_with_proof(
                        &claim_transition,
                        &BlockInfo::default(),
                        &proof_data,
                        &|id: &Identifier| {
                            if *id == contract.id() {
                                Ok(Some(contract.clone()))
                            } else {
                                Ok(None)
                            }
                        },
                        self.platform_version,
                    )
                    .map_err(|e| e.to_string())?;

                let StateTransitionProofResult::VerifiedTokenActionWithDocument(document) =
                    verification_result
                else {
                    return Err(format!(
                        "verification_result expected claim document, but got: {:?}",
                        verification_result
                    ));
                };

                // --- Extract TokenTransition from claim_transition ---
                let StateTransition::Batch(BatchTransition::V1(batch_v1)) = claim_transition else {
                    return Err("expected BatchTransition::V1".to_string());
                };

                // Assume only one transition was in the batch
                let [batched_transition] = batch_v1.transitions.as_slice() else {
                    return Err(format!(
                        "expected exactly one batched transition, got {}",
                        batch_v1.transitions.len()
                    ));
                };

                let BatchedTransition::Token(token_transition) = batched_transition else {
                    return Err("expected BatchedTransition::Token".to_string());
                };

                // --- Now build expected document ---
                let expected_document = token_transition
                    .build_historical_document(
                        self.get_token_id(),
                        self.identity.id(),
                        nonce,
                        &BlockInfo::default(),
                        &self.get_token_config()?,
                        self.platform_version,
                    )
                    .map_err(|e| e.to_string())?;

                if !document
                    .is_equal_ignoring_time_based_fields(
                        &expected_document,
                        Some(vec!["amount"]),
                        self.platform_version,
                    )
                    .map_err(|e| e.to_string())?
                {
                    return Err(
                        "Expected historical document for claim does not match proof document"
                            .to_string(),
                    );
                }
            }

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
        token_configuration: &TokenConfiguration,
        contract_start_time: u64,
    ) -> Result<(), String> {
        let TokenConfiguration::V0(token_config) = token_configuration;

        let TokenDistributionRules::V0(dist_rules) = token_config.distribution_rules();

        let TokenPerpetualDistribution::V0(perpetual_distribution) = dist_rules
            .perpetual_distribution()
            .expect("expected perpetual distribution");

        let consensus_result = perpetual_distribution
            .distribution_type
            .function()
            .validate(contract_start_time, PlatformVersion::latest())
            .map_err(|e| format!("invalid distribution function: {:?}", e))?;

        if let Some(error) = consensus_result.first_error() {
            return Err(error.to_string());
        }

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
