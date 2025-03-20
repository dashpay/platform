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
                &vec![claim_serialized_transition.clone()],
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
                &vec![claim_serialized_transition.clone()],
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
    use super::test_suite::*;
    use dpp::data_contract::associated_token::token_perpetual_distribution::distribution_function::DistributionFunction;

    // Given some token configuration,
    // When a claim is made at block 42,
    // Then the claim should be successful.
    #[test]

    fn test_block_based_perpetual_fixed_amount_50() {
        super::test_suite::check_heights_odd_no_current_rewards(
            DistributionFunction::FixedAmount { amount: 50 },
            &[41, 46, 50, 1000],
            &[100200, 100200, 100250],
            10,
        )
        .expect("\n-> fixed amount should pass");
    }

    /// Test case for overflow error.
    ///
    /// claim at height 1000000000000: claim failed: assertion 0 failed: expected SuccessfulExecution,
    /// got [InternalError(\"storage: protocol: overflow error: Overflow in FixedAmount evaluation\")]"
    #[test]
    fn test_block_based_perpetual_fixed_amount_1_000_000_000() {
        check_heights_odd_no_current_rewards(
            DistributionFunction::FixedAmount {
                amount: 1_000_000_000,
            },
            &[41, 46, 50, 51, 1_000_000_000_000],
            &[
                100_000 + 4 * 1_000_000_000,
                100_000 + 4 * 1_000_000_000,
                100_000 + 5 * 1_000_000_000,
                100_000 + 5 * 1_000_000_000,
                1, // 100_000 + (1_000_000_000_000 / 10) * 1_000_000_000, -- this will overflow
            ],
            10,
        )
        .expect("\n-> fixed amount should pass");
    }

    #[test]
    /// With a fixed amount of 0, we expect first claim to fetch 100_000 units (which are in the contract defintion),
    /// and fail for the rest of the claims.
    ///
    /// FAILS
    fn test_block_based_perpetual_fixed_amount_0() {
        check_heights(
            DistributionFunction::FixedAmount { amount: 0 },
            &[41, 46, 50, 100000],
            &[100000, 100000, 100000, 100000],
            &[true, false, false, false],
            None,
            10,
        )
        .expect("\nfixed amount zero increase\n");
    }

    #[test]
    fn test_block_based_perpetual_fixed_amount_u64_max() {
        check_heights_odd_no_current_rewards(
            DistributionFunction::FixedAmount { amount: u64::MAX },
            &[41, 46, 50, 1000],
            &[100200, 100200, 100250, 100250],
            10,
        )
        .expect("\nfixed amount u64::MAX should pass\n");
    }
}
mod block_based_perpetual_random {
    use dpp::data_contract::associated_token::token_perpetual_distribution::distribution_function::DistributionFunction;

    use super::test_suite::check_heights_odd_no_current_rewards;

    #[test]
    fn test_block_based_perpetual_random() {
        check_heights_odd_no_current_rewards(
            DistributionFunction::Random { min: 0, max: 100 },
            &[41, 46, 50, 59, 60],
            &[100192, 100192, 100263, 100263, 100310],
            10,
        )
        .expect("correct case 1");

        check_heights_odd_no_current_rewards(
            DistributionFunction::Random { min: 0, max: 0 },
            &[41],
            &[100192],
            10,
        )
        .expect("no rewards");
    }
}
mod matrix {
    use dpp::{
        balances::credits::TokenAmount,
        data_contract::associated_token::token_perpetual_distribution::distribution_function::DistributionFunction,
    };
    use rust_decimal::prelude::ToPrimitive;

    use crate::execution::validation::state_transition::batch::tests::token::distribution::perpetual::block_based::{block_based_perpetual_step_decreasing::expected_emission, test_suite::check_heights};

    // #[test_case::test_matrix(
    //     [1,10], // step_count
    //     [0,1,u16::MAX,999], // decrease_per_interval_numerator
    //     [0,1,2,10,100,u16::MAX], // decrease_per_interval_denominator
    //      [None,Some(1),Some(10),Some(u64::MAX)], // s
    //     [0,1,100,100_000, 1_000_000, 10_000_000, 100_000_000, u64::MAX], // n
    //     [None,Some(1),Some(10),Some(u64::MAX)], // min_value
    //     [0,110, 100, 1000] // distribution_interval
    // )]
    #[test_case::test_matrix(
        [0,1,10], // step_count
        [0,1,10,u16::MAX], // decrease_per_interval_numerator
        [1], // decrease_per_interval_denominator
         [None], // s
        [0,1,100000], // n
        [None], // min_value
        [0,110, 100, 1000] // distribution_interval
    )]
    fn test_block_based_perpetual_step_decreasing_matrix(
        step_count: u32,
        decrease_per_interval_numerator: u16,
        decrease_per_interval_denominator: u16,
        s: Option<u64>,
        n: TokenAmount,
        min_value: Option<u64>,
        distribution_interval: u64,
    ) {
        //
        let dist = DistributionFunction::StepDecreasingAmount {
            step_count,
            decrease_per_interval_numerator,
            decrease_per_interval_denominator,
            s,
            n,
            min_value,
        };

        const VERY_HIGH_HEIGHT: u64 = 1_000_000;
        let claim_heights = if distribution_interval > 0 {
            let mut heights = (1..10)
                .map(|i| i * distribution_interval)
                .collect::<Vec<_>>();
            heights.push(VERY_HIGH_HEIGHT);

            heights
        } else {
            vec![1, 2, 3, 10, 100, VERY_HIGH_HEIGHT]
        };
        let expected_balances = claim_heights
            .iter()
            .map(|&h| {
                // initial balance, defined in contract js
                let mut expected_balance = 100_000;
                // loop over blocks, starting with S, with step PERPETUAL_DISTRIBUTION_INTERVAL
                if distribution_interval > 0 {
                    for i in (1..=h).step_by(distribution_interval as usize) {
                        expected_balance += expected_emission(i, &dist);
                    }
                }
                println!("expected balance at height {}: {}", h, expected_balance);
                expected_balance.to_u64().unwrap_or(0) // to handle tests that overflow
            })
            .collect::<Vec<_>>();
        // we expect all tests to pass
        let expect_pass = claim_heights.iter().map(|&_h| true).collect::<Vec<_>>();

        if let Err(e) = check_heights(
            dist.clone(),
            &claim_heights,
            &expected_balances,
            &expect_pass,
            None, //Some(S),
            distribution_interval,
        ) {
            // print dist to stderr
            panic!("test failed for distribution function {:?}: {}", dist, e);
        } else {
            println!("test passed for distribution function {:?}", dist);
        }
    }
}

mod block_based_perpetual_step_decreasing {
    use dpp::balances::credits::TokenAmount;
    use dpp::data_contract::associated_token::token_perpetual_distribution::distribution_function::DistributionFunction;
    use rust_decimal::prelude::ToPrimitive;
    use test_case::test_case;
    use crate::execution::validation::state_transition::batch::tests::token::distribution::perpetual::block_based::test_suite::check_heights;
    use super::test_suite::with_timeout;

    const TIMEOUT: tokio::time::Duration = tokio::time::Duration::from_secs(1);

    #[test_case(
        1,// step_count
        1,// decrease_per_interval_numerator
        100,// decrease_per_interval_denominator
        None,// s
        100_000,// n
        Some(1),// min_value
      Some((1..1000).step_by(100).collect()),// claim_heights
        1; // distribution_interval
        "claim every 100 blocks"
    )]
    #[test_case(
        1,// step_count
        1,// decrease_per_interval_numerator
        100,// decrease_per_interval_denominator
        None,// s
        100_000,// n
        Some(1),// min_value
      Some((1..1000).step_by(500).collect()),// claim_heights
        1; // distribution_interval
        "claim every 500 blocks"
    )]
    #[test_matrix(
        1,// step_count
        101,// decrease_per_interval_numerator
        100,// decrease_per_interval_denominator
        None,// s
        100_000,// n
        Some(1),// min_value
        [Some((1..1000).step_by(100).collect()),Some((1..1000).step_by(500).collect())],// claim_heights
        1; // distribution_interval
        "1% increase, varying claim heights"
    )]
    #[test_case(
        1,// step_count
        1000,// decrease_per_interval_numerator
        1,// decrease_per_interval_denominator
        None,// s
        100_000,// n
        Some(1),// min_value
        Some(vec![1,7]), // claim_heights
        1; // distribution_interval
        "1000x increase, overflow"
    )]
    #[test_case(
        1,// step_count
        1,// decrease_per_interval_numerator
        1,// decrease_per_interval_denominator
        None,// s
        100_000,// n
        Some(1),// min_value
        Some(vec![1,2,3,10,100]), // claim_heights // ,300,500,800,1_000,1_000_000
        1; // distribution_interval
        "100% decrease, various min values"
    )]
    #[test_matrix(
        1,// step_count
        0,// decrease_per_interval_numerator
        1,// decrease_per_interval_denominator
        None,// s
        100_000,// n
        [None,Some(0),Some(1),Some(100)],// min_value
        Some(vec![1,2,3,10,100]), // claim_heights // ,300,500,800,1_000,1_000_000
        1; // distribution_interval
        "no decrease, irrelevant min values"
    )]
    #[test_matrix(
        [5,10],// step_count
        1,// decrease_per_interval_numerator
        2,// decrease_per_interval_denominator
        None,// s
        100_000,// n
        None,// min_value
        Some(vec![5,10,100]), // claim_heights // ,300,500,800,1_000,1_000_000
        [1,5]; // distribution_interval
        "1/2 decrease, changing step"
    )]
    #[test_matrix(
        [1,10],// step_count
        1,// decrease_per_interval_numerator
        2,// decrease_per_interval_denominator
        [None,Some(1),Some(5)],// s
        100_000,// n
        None,// min_value
        Some(vec![5,10,100]), // claim_heights // ,300,500,800,1_000,1_000_000
        [1,5]; // distribution_interval
        "1/2 decrease, changing S"
    )]

    /// Test various combinations of [DistributionFunction::StepDecreasingAmount] distribution.
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
                let mut expected_balance: i128 = 100_000;
                // loop over blocks, starting with S, with step PERPETUAL_DISTRIBUTION_INTERVAL
                for i in (1..=h).step_by(distribution_interval as usize) {
                    expected_balance += expected_emission(i, &dist);
                }
                println!("expected balance at height {}: {}", h, expected_balance);
                expected_balance.to_u64().unwrap_or_else(|| {
                    println!("ERR: overflow in expected balance at height {}", h);
                    0
                }) // to handle tests that overflow
            })
            .collect::<Vec<_>>();
        // we expect all tests to pass
        let expect_pass = claim_heights.iter().map(|&_h| true).collect::<Vec<_>>();

        with_timeout(TIMEOUT, move || {
            check_heights(
                dist,
                &claim_heights,
                &expected_balances,
                &expect_pass,
                None, //Some(S),
                distribution_interval,
            )
        })
        .inspect_err(|e| {
            println!("{}", e);
        })
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
                    println!("ERR: overflow in expected_emission({})", f_x);
                    0
                })
                .max(min_value)
        }
    }
}

mod test_suite {
    use super::*;
    use crate::rpc::core::MockCoreRPCLike;
    use crate::test::helpers::fast_forward_to_block::fast_forward_to_block;
    use crate::test::helpers::setup::TempPlatform;
    use dpp::block::extended_block_info::v0::ExtendedBlockInfoV0Getters;
    use dpp::data_contract::associated_token::token_distribution_key::TokenDistributionType;
    use dpp::data_contract::associated_token::token_perpetual_distribution::distribution_function::DistributionFunction;
    use dpp::data_contract::associated_token::token_perpetual_distribution::distribution_recipient::TokenDistributionRecipient;
    use dpp::data_contract::associated_token::token_perpetual_distribution::reward_distribution_type::RewardDistributionType;
    use dpp::data_contract::associated_token::token_perpetual_distribution::v0::TokenPerpetualDistributionV0;
    use dpp::data_contract::associated_token::token_perpetual_distribution::TokenPerpetualDistribution;
    use dpp::prelude::{DataContract, IdentityPublicKey};
    use simple_signer::signer::SimpleSigner;

    /// Run provided closure with timeout.
    pub(super) fn with_timeout(
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
            .map_err(|e| format!("timeout after {:?}", e))?
            .map_err(|e| format!("join error: {:?}", e))?
    }

    /// Check that claim results at provided heights are as expected, and that balances match expectations.
    ///
    /// Note we take i128 into expected_balances, as we want to be able to detect overflows.
    pub(super) fn check_heights(
        distribution_function: DistributionFunction,
        claim_heights: &[u64],
        expected_balances: &[u64],
        expect_pass: &[bool],
        contract_start_height: Option<u64>,
        distribution_interval: u64,
    ) -> Result<(), String> {
        let mut suite = TestSuite::new(
            10_200_000_000,
            0,
            TokenDistributionType::Perpetual,
            Some(|token_configuration: &mut TokenConfiguration| {
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
        if let Some(start) = contract_start_height {
            suite = suite.with_contract_start_time(start);
        }

        let mut tests = Vec::new();
        for (i, height) in claim_heights.iter().enumerate() {
            let assertions: Vec<AssertionFn> = if expect_pass[i] {
                vec![|processing_results: &[_]| match processing_results {
                    [StateTransitionExecutionResult::SuccessfulExecution(_, _)] => Ok(()),
                    _ => Err(format!(
                        "expected SuccessfulExecution, got {:?}",
                        processing_results
                    )),
                }]
            } else {
                vec![|processing_results: &[_]| match processing_results {
                    [StateTransitionExecutionResult::SuccessfulExecution(_, _)] => {
                        Err("expected error, got SuccessfulExecution".into())
                    }
                    _ => Ok(()),
                }]
            };

            tests.push(TestCase {
                name: format!("claim at height {}", height),
                base_height: *height - 1,
                base_time_ms: 10_200_000_000,
                expected_balance: expected_balances[i],
                assertions,
            });
        }

        suite.execute(&tests)
    }
    /// This test checks claims at provided heights, where every second height does not have any rewards to claim.
    ///
    /// # Arguments
    ///
    /// * `distribution_function` - configured distribution function to test
    /// * `claim_heights` - heights at which claims will be made; they will see balance from previous height
    /// * `expected_balances` - expected balances after claims were made and block from `heights` was committed
    ///
    pub(super) fn check_heights_odd_no_current_rewards(
        distribution_function: DistributionFunction,
        claim_heights: &[u64],
        expected_balances: &[u64],
        distribution_interval: u64,
    ) -> Result<(), String> {
        let mut suite = TestSuite::new(
            10_200_000_000,
            0,
            TokenDistributionType::Perpetual,
            Some(|token_configuration: &mut TokenConfiguration| {
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

        let mut tests = Vec::new();
        for (i, height) in claim_heights.iter().enumerate() {
            let assertions: Vec<AssertionFn> = if i % 2 == 0 {
                vec![|processing_results: &[_]| match processing_results {
                    [StateTransitionExecutionResult::SuccessfulExecution(_, _)] => Ok(()),
                    _ => Err(format!(
                        "expected SuccessfulExecution, got {:?}",
                        processing_results
                    )),
                }]
            } else {
                vec![|processing_results: &[_]| match processing_results {
                    [StateTransitionExecutionResult::PaidConsensusError(
                        ConsensusError::StateError(StateError::InvalidTokenClaimNoCurrentRewards(
                            _,
                        )),
                        _,
                    )] => Ok(()),
                    _ => Err(format!(
                        "expected InvalidTokenClaimNoCurrentRewards, got {:?}",
                        processing_results
                    )),
                }]
            };

            tests.push(TestCase {
                name: format!("claim at height {}", height),
                base_height: *height - 1,
                base_time_ms: 10_200_000_000,
                expected_balance: expected_balances[i],
                assertions,
            });
        }

        suite.execute(&tests)
    }

    pub(crate) struct TestSuite<C> {
        platform: TempPlatform<MockCoreRPCLike>,
        platform_version: &'static PlatformVersion,
        identity: dpp::prelude::Identity,
        signer: SimpleSigner,
        identity_public_key: IdentityPublicKey,
        token_id: Option<dpp::prelude::Identifier>,
        contract: Option<DataContract>,
        start_time: Option<u64>,
        token_distribution_type: TokenDistributionType,
        token_configuration_modification: Option<C>,
        epoch_index: u16,
        nonce: u64,
        time_between_blocks: u64,
    }

    impl<C: FnOnce(&mut TokenConfiguration)> TestSuite<C> {
        pub(crate) fn new(
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

            let mut rng = StdRng::seed_from_u64(49853);

            let (identity, signer, identity_public_key) =
                setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));

            Self {
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
                token_configuration_modification,
            }
            .with_genesis(1, genesis_time_ms)
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

        // submit a claim transition and assert the results
        pub(crate) fn assert_claim(&mut self, assertions: Vec<AssertionFn>) -> Result<(), String> {
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
                    &vec![claim_serialized_transition.clone()],
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

        pub(crate) fn assert_balance(
            &mut self,
            expected_balance: Option<u64>,
        ) -> Result<(), String> {
            let token_id = self.get_token_id().to_buffer();
            let token_balance = self
                .platform
                .drive
                .fetch_identity_token_balance(
                    token_id,
                    self.identity.id().to_buffer(),
                    None,
                    self.platform_version,
                )
                .expect("expected to fetch token balance");

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
        pub(super) fn with_contract_start_time(mut self, start_time: u64) -> Self {
            if self.contract.is_some() {
                panic!("with_contract_start_time must be called before contract is initialized");
            }
            self.start_time = Some(start_time);
            self
        }
        /// execute test cases
        pub(super) fn execute(&mut self, tests: &[TestCase]) -> Result<(), String> {
            let mut errors = String::new();
            for test_case in tests {
                let result = self.execute_test_case(test_case);
                if let Err(e) = result {
                    errors += format!("\n--> {}: {}", test_case.name, e).as_str();
                }
            }

            if errors.is_empty() {
                Ok(())
            } else {
                Err(errors)
            }
        }

        pub(super) fn execute_test_case(&mut self, test_case: &TestCase) -> Result<(), String> {
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
            self.assert_claim(test_case.assertions.clone())
                .map_err(|e| format!("claim failed: {}", e))?;
            self.assert_balance(Some(test_case.expected_balance))
                .map_err(|e| format!("invalid balance: {}", e))?;

            Ok(())
        }
    }

    pub(crate) type AssertionFn = fn(&[StateTransitionExecutionResult]) -> Result<(), String>;
    pub(crate) struct TestCase {
        pub(crate) name: String,
        /// height of block just before the claim
        pub(crate) base_height: u64,
        /// time of block before the claim
        pub(crate) base_time_ms: u64,
        /// expected balance is a function that should return the expected balance after committing block
        /// at provided height and time
        pub(crate) expected_balance: u64,
        /// assertion functions that will be executed on the claim
        pub(crate) assertions: Vec<AssertionFn>,
    }
}
