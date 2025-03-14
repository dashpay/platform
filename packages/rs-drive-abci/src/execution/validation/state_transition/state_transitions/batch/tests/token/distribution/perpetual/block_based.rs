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
