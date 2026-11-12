use super::*;
use crate::execution::validation::state_transition::tests::{
    create_token_contract_with_owner_identity_with_start_epoch, setup_identity,
};
use crate::rpc::core::MockCoreRPCLike;
use crate::test::helpers::setup::{TempPlatform, TestPlatformBuilder};
use dpp::dash_to_credits;
use dpp::data_contract::TokenConfiguration;
use dpp::state_transition::batch_transition::BatchTransition;
use platform_version::version::PlatformVersion;
use rand::prelude::StdRng;

/// Initial contract balance, as hardcoded in the contract definition (JSON file).
const INITIAL_BALANCE: u64 = 100_000;

mod perpetual_distribution_epoch {
    use super::*;
    use crate::platform_types::platform_state::PlatformStateV0Methods;
    use crate::platform_types::state_transitions_processing_result::StateTransitionsProcessingResult;
    use dpp::block::epoch::{Epoch, EpochIndex};
    use dpp::block::finalized_epoch_info::v0::FinalizedEpochInfoV0;
    use dpp::block::finalized_epoch_info::FinalizedEpochInfo;
    use dpp::data_contract::associated_token::token_distribution_key::TokenDistributionType;
    use dpp::data_contract::associated_token::token_perpetual_distribution::distribution_function::DistributionFunction;
    use dpp::data_contract::associated_token::token_perpetual_distribution::distribution_recipient::TokenDistributionRecipient;
    use dpp::data_contract::associated_token::token_perpetual_distribution::reward_distribution_type::RewardDistributionType;
    use dpp::data_contract::associated_token::token_perpetual_distribution::v0::TokenPerpetualDistributionV0;
    use dpp::data_contract::associated_token::token_perpetual_distribution::TokenPerpetualDistribution;
    use dpp::identity::{Identity, IdentityPublicKey};
    use dpp::prelude::IdentityNonce;
    use dpp::version::ProtocolVersion;
    use simple_signer::signer::SimpleSigner;
    use std::collections::BTreeMap;
    use std::ops::RangeInclusive;
    use std::sync::Arc;

    /// The contract is registered in epoch 6 and pays a fixed amount every two epochs. A nonzero
    /// start with an interval above one is the shape whose cycle cap overflowed `u16` up to
    /// protocol version 13: a fixed amount allows 32_767 cycles, and 6 + 2 * 32_767 wraps to 4,
    /// below the start.
    const CONTRACT_CREATION_EPOCH: EpochIndex = 6;
    const EPOCH_INTERVAL: EpochIndex = 2;
    const AMOUNT_PER_CYCLE: u64 = 50;

    struct EpochTokenFixture {
        platform: TempPlatform<MockCoreRPCLike>,
        identity: Identity,
        signer: SimpleSigner,
        key: IdentityPublicKey,
        contract_id: Identifier,
        token_id: Identifier,
    }

    fn setup(
        protocol_version: ProtocolVersion,
        contract_creation_epoch: EpochIndex,
        interval: EpochIndex,
        function: DistributionFunction,
        distribution_recipient: TokenDistributionRecipient,
    ) -> EpochTokenFixture {
        let platform_version =
            PlatformVersion::get(protocol_version).expect("expected platform version");
        let mut platform = TestPlatformBuilder::new()
            .with_initial_protocol_version(protocol_version)
            .build_with_mock_rpc()
            .set_genesis_state();

        let mut rng = StdRng::seed_from_u64(49853);

        let (identity, signer, key) =
            setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));

        let (contract, token_id) = create_token_contract_with_owner_identity_with_start_epoch(
            &mut platform,
            identity.id(),
            Some(|token_configuration: &mut TokenConfiguration| {
                token_configuration
                    .distribution_rules_mut()
                    .set_perpetual_distribution(Some(TokenPerpetualDistribution::V0(
                        TokenPerpetualDistributionV0 {
                            distribution_type: RewardDistributionType::EpochBasedDistribution {
                                interval,
                                function,
                            },
                            distribution_recipient,
                        },
                    )));
            }),
            None,
            None,
            None,
            Some(contract_creation_epoch),
            platform_version,
        );

        EpochTokenFixture {
            platform,
            identity,
            signer,
            key,
            contract_id: contract.id(),
            token_id,
        }
    }

    impl EpochTokenFixture {
        /// Signs a perpetual claim with `nonce`, processes it in a block of `epoch` and commits
        /// the outcome.
        async fn claim_in_epoch(
            &self,
            nonce: IdentityNonce,
            epoch: EpochIndex,
            platform_version: &PlatformVersion,
        ) -> StateTransitionsProcessingResult {
            let platform_state = self.platform.state.load();

            let claim_transition = BatchTransition::new_token_claim_transition(
                self.token_id,
                self.identity.id(),
                self.contract_id,
                0,
                TokenDistributionType::Perpetual,
                None,
                &self.key,
                nonce,
                0,
                &self.signer,
                platform_version,
                None,
            )
            .await
            .expect("expected to create claim transition");

            let claim_serialized_transition = claim_transition
                .serialize_to_bytes()
                .expect("expected serialized claim transition");

            let transaction = self.platform.drive.grove.start_transaction();

            let processing_result = self
                .platform
                .platform
                .process_raw_state_transitions(
                    &[claim_serialized_transition],
                    &platform_state,
                    &BlockInfo {
                        time_ms: 10_200_100_000 + u64::from(epoch) * 1_000_000,
                        height: 100 + u64::from(epoch),
                        core_height: 42,
                        epoch: Epoch::new(epoch).expect("expected a valid epoch"),
                    },
                    &transaction,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected to process state transition");

            self.platform
                .drive
                .grove
                .commit_transaction(transaction)
                .unwrap()
                .expect("expected to commit transaction");

            processing_result
        }

        /// Records, for every epoch in `epochs`, `blocks_in_epoch` blocks of which the claimant
        /// proposed `proposed_by_claimant`, as the epoch change does once an epoch is over.
        fn seed_finalized_epochs(
            &self,
            epochs: RangeInclusive<EpochIndex>,
            blocks_in_epoch: u64,
            proposed_by_claimant: u64,
            platform_version: &PlatformVersion,
        ) {
            self.seed_finalized_epochs_proposing(
                epochs,
                blocks_in_epoch,
                |_| proposed_by_claimant,
                platform_version,
            )
        }

        /// Like `seed_finalized_epochs`, with the claimant's blocks chosen per epoch.
        fn seed_finalized_epochs_proposing(
            &self,
            epochs: impl IntoIterator<Item = EpochIndex>,
            blocks_in_epoch: u64,
            proposed_by_claimant: impl Fn(EpochIndex) -> u64,
            platform_version: &PlatformVersion,
        ) {
            let transaction = self.platform.drive.grove.start_transaction();
            for epoch_index in epochs {
                let proposed_by_claimant = proposed_by_claimant(epoch_index);
                let finalized_epoch_info = FinalizedEpochInfo::V0(FinalizedEpochInfoV0 {
                    first_block_time: u64::from(epoch_index) * 1_000_000,
                    first_block_height: u64::from(epoch_index) * blocks_in_epoch,
                    total_blocks_in_epoch: blocks_in_epoch,
                    first_core_block_height: 42,
                    next_epoch_start_core_block_height: 42,
                    total_processing_fees: 0,
                    total_distributed_storage_fees: 0,
                    total_created_storage_fees: 0,
                    core_block_rewards: 0,
                    block_proposers: BTreeMap::from([(self.identity.id(), proposed_by_claimant)]),
                    fee_multiplier_permille: 1_000,
                    protocol_version: platform_version.protocol_version,
                });
                let operation = self
                    .platform
                    .drive
                    .add_epoch_final_info_operation(
                        &Epoch::new(epoch_index).expect("expected a valid epoch"),
                        finalized_epoch_info,
                        platform_version,
                    )
                    .expect("expected a finalized epoch info operation");
                self.platform
                    .drive
                    .grove_apply_operation(
                        operation,
                        false,
                        Some(&transaction),
                        &platform_version.drive,
                    )
                    .expect("expected to store the finalized epoch info");
            }
            self.platform
                .drive
                .grove
                .commit_transaction(transaction)
                .unwrap()
                .expect("expected to commit the finalized epoch infos");
        }

        /// Moves the platform state to `protocol_version`, as the first block of an upgrade does.
        fn upgrade_to(&self, protocol_version: ProtocolVersion) {
            let mut upgraded_state = self.platform.state.load().as_ref().clone();
            upgraded_state.set_current_protocol_version_in_consensus(protocol_version);
            upgraded_state.set_next_epoch_protocol_version(protocol_version);
            self.platform.state.store(Arc::new(upgraded_state));
        }

        fn owner_token_balance(&self, platform_version: &PlatformVersion) -> Option<u64> {
            self.platform
                .drive
                .fetch_identity_token_balance(
                    self.token_id.to_buffer(),
                    self.identity.id().to_buffer(),
                    None,
                    platform_version,
                )
                .expect("expected to fetch token balance")
        }
    }

    #[tokio::test]
    async fn should_claim_epoch_rewards_with_a_nonzero_start_from_protocol_version_14() {
        let platform_version = PlatformVersion::latest();
        let fixture = setup(
            platform_version.protocol_version,
            CONTRACT_CREATION_EPOCH,
            EPOCH_INTERVAL,
            DistributionFunction::FixedAmount {
                amount: AMOUNT_PER_CYCLE,
            },
            TokenDistributionRecipient::ContractOwner,
        );

        // Epoch 10: the cycle that began at epoch 8 is complete; the one beginning now is not.
        // Up to protocol version 13 this claim was refused every time.
        let processing_result = fixture.claim_in_epoch(2, 10, platform_version).await;
        assert_matches!(
            processing_result.execution_results().as_slice(),
            [StateTransitionExecutionResult::SuccessfulExecution { .. }]
        );
        assert_eq!(
            fixture.owner_token_balance(platform_version),
            Some(INITIAL_BALANCE + AMOUNT_PER_CYCLE)
        );

        // Epoch 11: the cycle that began at epoch 10 is still running, so there is nothing new;
        // a refusal, not a fault.
        let processing_result = fixture.claim_in_epoch(3, 11, platform_version).await;
        assert_matches!(
            processing_result.execution_results().as_slice(),
            [StateTransitionExecutionResult::PaidConsensusError {
                error: ConsensusError::StateError(StateError::InvalidTokenClaimNoCurrentRewards(_)),
                ..
            }]
        );
        assert_eq!(
            fixture.owner_token_balance(platform_version),
            Some(INITIAL_BALANCE + AMOUNT_PER_CYCLE)
        );

        // Epoch 12: the claim now starts from the last paid moment, epoch 8, another nonzero
        // start, and redeems the cycle that began at epoch 10.
        let processing_result = fixture.claim_in_epoch(4, 12, platform_version).await;
        assert_matches!(
            processing_result.execution_results().as_slice(),
            [StateTransitionExecutionResult::SuccessfulExecution { .. }]
        );
        assert_eq!(
            fixture.owner_token_balance(platform_version),
            Some(INITIAL_BALANCE + 2 * AMOUNT_PER_CYCLE)
        );
    }

    /// Protocol version 13 could pay an interval-two token only while its start was below two,
    /// where the cap did not wrap yet. That claim leaves the last paid moment off a cycle boundary
    /// (the cap was the current cycle moment less one) and, for a fixed amount, one cycle short:
    /// `steps_till` drops a cycle when the start sits on a boundary and the end does not. After
    /// the upgrade the same count pays one cycle extra for a start off a boundary and an end on
    /// it, so every cycle is paid exactly once overall, and from then on every stored moment sits
    /// on a boundary and the counts agree.
    #[tokio::test]
    async fn should_pay_every_cycle_once_after_a_version_13_claim_left_the_start_off_a_boundary() {
        let v13 = PlatformVersion::get(13).expect("expected protocol version 13");
        let v14 = PlatformVersion::latest();
        let fixture = setup(
            v13.protocol_version,
            0,
            EPOCH_INTERVAL,
            DistributionFunction::FixedAmount {
                amount: AMOUNT_PER_CYCLE,
            },
            TokenDistributionRecipient::ContractOwner,
        );

        // Epoch 10 under version 13: the cycles at epochs 2, 4, 6 and 8 are complete, the cap is
        // 9, and the count pays three of them; the stored moment is 9.
        let processing_result = fixture.claim_in_epoch(2, 10, v13).await;
        assert_matches!(
            processing_result.execution_results().as_slice(),
            [StateTransitionExecutionResult::SuccessfulExecution { .. }]
        );
        assert_eq!(
            fixture.owner_token_balance(v13),
            Some(INITIAL_BALANCE + 3 * AMOUNT_PER_CYCLE)
        );

        fixture.upgrade_to(v14.protocol_version);

        // Epoch 14 under version 14: the cap is the completed cycle at 12, and the count from the
        // stored moment 9 pays the cycles at 10 and 12 plus the one dropped before: six in all.
        let processing_result = fixture.claim_in_epoch(3, 14, v14).await;
        assert_matches!(
            processing_result.execution_results().as_slice(),
            [StateTransitionExecutionResult::SuccessfulExecution { .. }]
        );
        assert_eq!(
            fixture.owner_token_balance(v14),
            Some(INITIAL_BALANCE + 6 * AMOUNT_PER_CYCLE)
        );

        // Epoch 16: from the stored moment 12, on a boundary, exactly the cycle at 14.
        let processing_result = fixture.claim_in_epoch(4, 16, v14).await;
        assert_matches!(
            processing_result.execution_results().as_slice(),
            [StateTransitionExecutionResult::SuccessfulExecution { .. }]
        );
        assert_eq!(
            fixture.owner_token_balance(v14),
            Some(INITIAL_BALANCE + 7 * AMOUNT_PER_CYCLE)
        );
    }

    /// An evonode distribution weights every cycle by the claimant's share of the blocks
    /// proposed in the epochs that cycle spans. Up to protocol version 13 the per-cycle
    /// evaluator read a cycle's step index as an epoch, so for an interval above one it asked
    /// for epochs before the distribution started, outside the epoch window the claim loads,
    /// and the claim failed as an internal error.
    #[tokio::test]
    async fn should_pay_evonode_rewards_by_the_epochs_each_cycle_spans_from_protocol_version_14() {
        let platform_version = PlatformVersion::latest();
        let fixture = setup(
            platform_version.protocol_version,
            CONTRACT_CREATION_EPOCH,
            EPOCH_INTERVAL,
            // 500 tokens at the creation cycle, 100 more each cycle after it.
            DistributionFunction::Linear {
                a: 100,
                d: 1,
                start_step: None,
                starting_amount: 500,
                min_value: None,
                max_value: None,
            },
            TokenDistributionRecipient::EvonodesByParticipation,
        );
        // Every epoch from 6 to 13 has 10 blocks, half of them proposed by the claimant.
        fixture.seed_finalized_epochs(6..=13, 10, 5, platform_version);

        // Epoch 12: the cycles at epochs 8 and 10 are complete. They are step indexes 4 and 5
        // from the creation index 3, paying 600 and 700, and each is weighted by the claimant's
        // half share of the blocks in the two epochs it spans.
        let processing_result = fixture.claim_in_epoch(2, 12, platform_version).await;
        assert_matches!(
            processing_result.execution_results().as_slice(),
            [StateTransitionExecutionResult::SuccessfulExecution { .. }]
        );
        assert_eq!(
            fixture.owner_token_balance(platform_version),
            Some(INITIAL_BALANCE + 650)
        );

        // Epoch 14: the cycle at epoch 12 pays 800, halved.
        let processing_result = fixture.claim_in_epoch(3, 14, platform_version).await;
        assert_matches!(
            processing_result.execution_results().as_slice(),
            [StateTransitionExecutionResult::SuccessfulExecution { .. }]
        );
        assert_eq!(
            fixture.owner_token_balance(platform_version),
            Some(INITIAL_BALANCE + 1_050)
        );
    }

    /// Protocol version 13 is frozen with the wrapping cap: the same claim is refused as having
    /// no rewards. The test profile checks arithmetic, so this also proves the frozen path
    /// reaches that refusal without an overflow panic, and replaying such a block cannot halt a
    /// node.
    #[tokio::test]
    async fn should_keep_refusing_epoch_rewards_with_a_nonzero_start_at_protocol_version_13() {
        let platform_version = PlatformVersion::get(13).expect("expected protocol version 13");
        let fixture = setup(
            platform_version.protocol_version,
            CONTRACT_CREATION_EPOCH,
            EPOCH_INTERVAL,
            DistributionFunction::FixedAmount {
                amount: AMOUNT_PER_CYCLE,
            },
            TokenDistributionRecipient::ContractOwner,
        );

        let processing_result = fixture.claim_in_epoch(2, 10, platform_version).await;
        assert_matches!(
            processing_result.execution_results().as_slice(),
            [StateTransitionExecutionResult::PaidConsensusError {
                error: ConsensusError::StateError(StateError::InvalidTokenClaimNoCurrentRewards(_)),
                ..
            }]
        );
        assert_eq!(
            fixture.owner_token_balance(platform_version),
            Some(INITIAL_BALANCE)
        );
    }

    /// Blocks recorded in every finalized epoch of the evonode distributions below.
    const BLOCKS_PER_EPOCH: u64 = 10;

    /// The emission of each evonode distribution below per cycle when it is a fixed amount.
    const FIXED_AMOUNT_PER_CYCLE: u64 = 1_000;

    /// 500 tokens at the distribution's first cycle, 100 more each cycle after it.
    fn linear() -> DistributionFunction {
        DistributionFunction::Linear {
            a: 100,
            d: 1,
            start_step: None,
            starting_amount: 500,
            min_value: None,
            max_value: None,
        }
    }

    fn fixed_amount() -> DistributionFunction {
        DistributionFunction::FixedAmount {
            amount: FIXED_AMOUNT_PER_CYCLE,
        }
    }

    /// The claimant's blocks in each epoch, out of `BLOCKS_PER_EPOCH`: two before epoch 100
    /// and eight from it, so a claim that weighs one span of epochs by another's share pays a
    /// visibly different amount.
    fn blocks_proposed_by_claimant(epoch: EpochIndex) -> u64 {
        if epoch < 100 {
            2
        } else {
            8
        }
    }

    /// What the cycles ending in `(start, end]` pay the claimant in all: each cycle's emission
    /// weighted by the claimant's share of the blocks in the epochs it spans, `proposed` giving
    /// its blocks in each epoch out of `BLOCKS_PER_EPOCH`. The emissions below are chosen so
    /// that a fixed amount's share over several cycles, which the evaluator weighs as one
    /// span, never rounds differently.
    fn participation_share(
        function: &DistributionFunction,
        distribution_start: EpochIndex,
        interval: EpochIndex,
        (start, end): (EpochIndex, EpochIndex),
        proposed: impl Fn(EpochIndex) -> u64,
        platform_version: &PlatformVersion,
    ) -> u64 {
        (start / interval + 1..=end / interval)
            .map(|cycle| {
                let emission = function
                    .evaluate(
                        u64::from(distribution_start / interval),
                        u64::from(cycle),
                        platform_version,
                    )
                    .expect("expected the cycle's emission");
                let cycle_moment = cycle * interval;
                let proposed_in_cycle: u64 = (cycle_moment - interval + 1..=cycle_moment)
                    .map(&proposed)
                    .sum();
                emission * proposed_in_cycle / (u64::from(interval) * BLOCKS_PER_EPOCH)
            })
            .sum()
    }

    /// An evonode 150 epochs behind (a first claim on a distribution paying every epoch since
    /// epoch 0) is paid in two claims: the first reads 100 finalized epochs and pays through
    /// the last of them, the second continues from there. Up to protocol version 13 the claim
    /// read the same 100 epochs but evaluated its whole range: a function other than a fixed
    /// amount failed as an internal error, a fixed amount applied the share of the epochs read
    /// to all 150.
    #[tokio::test]
    async fn should_pay_an_evonode_150_epochs_behind_over_two_claims_from_protocol_version_14() {
        let platform_version = PlatformVersion::latest();
        for function in [linear(), fixed_amount()] {
            let fixture = setup(
                platform_version.protocol_version,
                0,
                1,
                function.clone(),
                TokenDistributionRecipient::EvonodesByParticipation,
            );
            fixture.seed_finalized_epochs_proposing(
                0..=150,
                BLOCKS_PER_EPOCH,
                blocks_proposed_by_claimant,
                platform_version,
            );
            let share = |claimed: (EpochIndex, EpochIndex)| {
                participation_share(
                    &function,
                    0,
                    1,
                    claimed,
                    blocks_proposed_by_claimant,
                    platform_version,
                )
            };

            // Epoch 151: epochs 1 to 100 of the 150 finished.
            let processing_result = fixture.claim_in_epoch(2, 151, platform_version).await;
            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::SuccessfulExecution { .. }],
                "{function}"
            );
            assert_eq!(
                fixture.owner_token_balance(platform_version),
                Some(INITIAL_BALANCE + share((0, 100))),
                "{function}"
            );

            // The same epoch: the second claim starts after epoch 100 and pays the other 50.
            let processing_result = fixture.claim_in_epoch(3, 151, platform_version).await;
            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::SuccessfulExecution { .. }],
                "{function}"
            );
            assert_eq!(
                fixture.owner_token_balance(platform_version),
                Some(INITIAL_BALANCE + share((0, 150))),
                "{function}"
            );

            // Every finished epoch is paid, once.
            let processing_result = fixture.claim_in_epoch(4, 151, platform_version).await;
            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::PaidConsensusError {
                    error: ConsensusError::StateError(
                        StateError::InvalidTokenClaimNoCurrentRewards(_)
                    ),
                    ..
                }],
                "{function}"
            );
        }

        // The fixed amount pays 1,000 an epoch, a fifth of it before epoch 100 and four fifths
        // from it: 99 * 200 + 800 in the first claim, 50 * 800 in the second.
        assert_eq!(
            participation_share(
                &fixed_amount(),
                0,
                1,
                (0, 100),
                blocks_proposed_by_claimant,
                platform_version,
            ),
            20_600
        );
        assert_eq!(
            participation_share(
                &fixed_amount(),
                0,
                1,
                (100, 150),
                blocks_proposed_by_claimant,
                platform_version,
            ),
            40_000
        );
    }

    /// A first claim on a distribution that began 130 epochs before the claimant's first
    /// claim: up to protocol version 13 this failed at every epoch after, so the evonode could
    /// never be paid.
    #[tokio::test]
    async fn should_pay_an_evonode_that_never_claimed_a_distribution_begun_long_ago_from_protocol_version_14(
    ) {
        let platform_version = PlatformVersion::latest();
        let fixture = setup(
            platform_version.protocol_version,
            30,
            1,
            linear(),
            TokenDistributionRecipient::EvonodesByParticipation,
        );
        fixture.seed_finalized_epochs_proposing(
            30..=160,
            BLOCKS_PER_EPOCH,
            blocks_proposed_by_claimant,
            platform_version,
        );
        let share = |claimed: (EpochIndex, EpochIndex)| {
            participation_share(
                &linear(),
                30,
                1,
                claimed,
                blocks_proposed_by_claimant,
                platform_version,
            )
        };

        // Epoch 161: the first claim reads epochs 31 to 130.
        let processing_result = fixture.claim_in_epoch(2, 161, platform_version).await;
        assert_matches!(
            processing_result.execution_results().as_slice(),
            [StateTransitionExecutionResult::SuccessfulExecution { .. }]
        );
        assert_eq!(
            fixture.owner_token_balance(platform_version),
            Some(INITIAL_BALANCE + share((30, 130)))
        );

        let processing_result = fixture.claim_in_epoch(3, 161, platform_version).await;
        assert_matches!(
            processing_result.execution_results().as_slice(),
            [StateTransitionExecutionResult::SuccessfulExecution { .. }]
        );
        assert_eq!(
            fixture.owner_token_balance(platform_version),
            Some(INITIAL_BALANCE + share((30, 160)))
        );
    }

    /// Protocol version 13 is frozen: a claim reaching past the 100 epochs it read fails as an
    /// internal error for a function other than a fixed amount, at every later epoch too, and
    /// a fixed amount pays the share of the epochs read for the whole range.
    #[tokio::test]
    async fn should_keep_failing_or_mispaying_evonode_claims_past_the_epochs_read_at_protocol_version_13(
    ) {
        let platform_version = PlatformVersion::get(13).expect("expected protocol version 13");

        let fixture = setup(
            platform_version.protocol_version,
            0,
            1,
            linear(),
            TokenDistributionRecipient::EvonodesByParticipation,
        );
        fixture.seed_finalized_epochs_proposing(
            0..=160,
            BLOCKS_PER_EPOCH,
            blocks_proposed_by_claimant,
            platform_version,
        );
        // Epoch 151 claims epochs 1 to 128 but read only 0 to 99; epoch 161 claims the same.
        for (nonce, epoch) in [(2, 151), (3, 161)] {
            let processing_result = fixture.claim_in_epoch(nonce, epoch, platform_version).await;
            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::InternalError(message)]
                    if message.contains("missing epoch info for epochs 100..=100")
            );
            assert_eq!(
                fixture.owner_token_balance(platform_version),
                Some(INITIAL_BALANCE)
            );
        }

        let fixture = setup(
            platform_version.protocol_version,
            0,
            1,
            fixed_amount(),
            TokenDistributionRecipient::EvonodesByParticipation,
        );
        fixture.seed_finalized_epochs_proposing(
            0..=150,
            BLOCKS_PER_EPOCH,
            blocks_proposed_by_claimant,
            platform_version,
        );
        let processing_result = fixture.claim_in_epoch(2, 151, platform_version).await;
        assert_matches!(
            processing_result.execution_results().as_slice(),
            [StateTransitionExecutionResult::SuccessfulExecution { .. }]
        );
        // All 150 epochs at the claimant's share of epochs 1 to 99, a fifth: 30,000, where
        // its blocks earned 60,600.
        assert_eq!(
            fixture.owner_token_balance(platform_version),
            Some(INITIAL_BALANCE + 30_000)
        );
        assert_eq!(
            participation_share(
                &fixed_amount(),
                0,
                1,
                (0, 150),
                blocks_proposed_by_claimant,
                platform_version,
            ),
            60_600
        );
        // And the last paid moment moved to epoch 150: nothing is left to claim.
        let processing_result = fixture.claim_in_epoch(3, 151, platform_version).await;
        assert_matches!(
            processing_result.execution_results().as_slice(),
            [StateTransitionExecutionResult::PaidConsensusError {
                error: ConsensusError::StateError(StateError::InvalidTokenClaimNoCurrentRewards(_)),
                ..
            }]
        );
    }

    /// A claim within the epochs one read covers is paid the same at both protocol versions.
    #[tokio::test]
    async fn should_pay_an_evonode_claim_within_the_read_limit_the_same_at_protocol_versions_13_and_14(
    ) {
        for platform_version in [
            PlatformVersion::get(13).expect("expected protocol version 13"),
            PlatformVersion::latest(),
        ] {
            for function in [linear(), fixed_amount()] {
                let fixture = setup(
                    platform_version.protocol_version,
                    0,
                    1,
                    function.clone(),
                    TokenDistributionRecipient::EvonodesByParticipation,
                );
                fixture.seed_finalized_epochs_proposing(
                    0..=60,
                    BLOCKS_PER_EPOCH,
                    |epoch| 1 + u64::from(epoch) % 9,
                    platform_version,
                );

                let processing_result = fixture.claim_in_epoch(2, 61, platform_version).await;
                assert_matches!(
                    processing_result.execution_results().as_slice(),
                    [StateTransitionExecutionResult::SuccessfulExecution { .. }],
                    "v{} {function}",
                    platform_version.protocol_version
                );
                assert_eq!(
                    fixture.owner_token_balance(platform_version),
                    Some(
                        INITIAL_BALANCE
                            + participation_share(
                                &function,
                                0,
                                1,
                                (0, 60),
                                |epoch| 1 + u64::from(epoch) % 9,
                                platform_version,
                            )
                    ),
                    "v{} {function}",
                    platform_version.protocol_version
                );

                let processing_result = fixture.claim_in_epoch(3, 61, platform_version).await;
                assert_matches!(
                    processing_result.execution_results().as_slice(),
                    [StateTransitionExecutionResult::PaidConsensusError {
                        error: ConsensusError::StateError(
                            StateError::InvalidTokenClaimNoCurrentRewards(_)
                        ),
                        ..
                    }],
                    "v{} {function}",
                    platform_version.protocol_version
                );
            }
        }
    }

    /// With a cycle of three epochs the first claim reads epochs 1 to 100 and pays the 33
    /// cycles ending by epoch 99; epoch 100 opens a cycle the claim did not read to its end,
    /// so the second claim reads it again and pays the 19 cycles up to epoch 156, the last one
    /// finished at epoch 161.
    #[tokio::test]
    async fn should_pay_multi_epoch_cycles_through_the_last_whole_cycle_read_from_protocol_version_14(
    ) {
        let platform_version = PlatformVersion::latest();
        for function in [linear(), fixed_amount()] {
            let fixture = setup(
                platform_version.protocol_version,
                0,
                3,
                function.clone(),
                TokenDistributionRecipient::EvonodesByParticipation,
            );
            fixture.seed_finalized_epochs_proposing(
                0..=160,
                BLOCKS_PER_EPOCH,
                blocks_proposed_by_claimant,
                platform_version,
            );
            let share = |claimed: (EpochIndex, EpochIndex)| {
                participation_share(
                    &function,
                    0,
                    3,
                    claimed,
                    blocks_proposed_by_claimant,
                    platform_version,
                )
            };

            let processing_result = fixture.claim_in_epoch(2, 161, platform_version).await;
            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::SuccessfulExecution { .. }],
                "{function}"
            );
            assert_eq!(
                fixture.owner_token_balance(platform_version),
                Some(INITIAL_BALANCE + share((0, 99))),
                "{function}"
            );

            let processing_result = fixture.claim_in_epoch(3, 161, platform_version).await;
            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::SuccessfulExecution { .. }],
                "{function}"
            );
            assert_eq!(
                fixture.owner_token_balance(platform_version),
                Some(INITIAL_BALANCE + share((0, 156))),
                "{function}"
            );
        }
    }

    /// A cycle of 120 epochs is longer than the 100 epochs a claim reads otherwise: the claim
    /// reads the whole cycle, or it could never pay one.
    #[tokio::test]
    async fn should_read_a_whole_cycle_longer_than_the_read_limit_from_protocol_version_14() {
        let platform_version = PlatformVersion::latest();
        let fixture = setup(
            platform_version.protocol_version,
            0,
            120,
            linear(),
            TokenDistributionRecipient::EvonodesByParticipation,
        );
        fixture.seed_finalized_epochs_proposing(
            0..=240,
            BLOCKS_PER_EPOCH,
            blocks_proposed_by_claimant,
            platform_version,
        );

        // Epoch 241: the cycle of epochs 1 to 120 is finished, the one of 121 to 240 is still
        // running as far as cycles go (the cycle at 240 began). It pays 600, weighted by the
        // claimant's 99 * 2 + 21 * 8 of its 1,200 blocks: 183.
        let processing_result = fixture.claim_in_epoch(2, 241, platform_version).await;
        assert_matches!(
            processing_result.execution_results().as_slice(),
            [StateTransitionExecutionResult::SuccessfulExecution { .. }]
        );
        assert_eq!(
            fixture.owner_token_balance(platform_version),
            Some(INITIAL_BALANCE + 183)
        );
        assert_eq!(
            participation_share(
                &linear(),
                0,
                120,
                (0, 120),
                blocks_proposed_by_claimant,
                platform_version,
            ),
            183
        );
    }

    /// An epoch in which no block was produced is never finalized. From protocol version 14 it
    /// counts as an epoch without blocks, since later epochs are finalized; up to 13 the claim
    /// failed as an internal error.
    #[tokio::test]
    async fn should_count_an_epoch_without_blocks_as_no_participation_from_protocol_version_14() {
        for platform_version in [
            PlatformVersion::get(13).expect("expected protocol version 13"),
            PlatformVersion::latest(),
        ] {
            let fixture = setup(
                platform_version.protocol_version,
                0,
                1,
                linear(),
                TokenDistributionRecipient::EvonodesByParticipation,
            );
            // The chain produced no block in epoch 5.
            fixture.seed_finalized_epochs_proposing(
                (0..=10).filter(|epoch| *epoch != 5),
                BLOCKS_PER_EPOCH,
                blocks_proposed_by_claimant,
                platform_version,
            );

            let processing_result = fixture.claim_in_epoch(2, 11, platform_version).await;
            if platform_version.protocol_version < 14 {
                assert_matches!(
                    processing_result.execution_results().as_slice(),
                    [StateTransitionExecutionResult::InternalError(message)]
                        if message.contains("missing epoch info for epochs 5..=5")
                );
                assert_eq!(
                    fixture.owner_token_balance(platform_version),
                    Some(INITIAL_BALANCE)
                );
            } else {
                assert_matches!(
                    processing_result.execution_results().as_slice(),
                    [StateTransitionExecutionResult::SuccessfulExecution { .. }]
                );
                assert_eq!(
                    fixture.owner_token_balance(platform_version),
                    Some(
                        INITIAL_BALANCE
                            + participation_share(
                                &linear(),
                                0,
                                1,
                                (0, 10),
                                |epoch| if epoch == 5 {
                                    0
                                } else {
                                    blocks_proposed_by_claimant(epoch)
                                },
                                platform_version,
                            )
                    )
                );
            }
        }
    }

    /// The previous epoch is finalized by the fee distribution of the first block of the next
    /// one, after that block's transitions. A claim in that block leaves the epoch to a later
    /// claim from protocol version 14; up to 13 a function other than a fixed amount failed as
    /// an internal error and a fixed amount paid the epoch at the share of the epochs before
    /// it.
    #[tokio::test]
    async fn should_leave_an_epoch_not_finalized_yet_to_a_later_claim_from_protocol_version_14() {
        for platform_version in [
            PlatformVersion::get(13).expect("expected protocol version 13"),
            PlatformVersion::latest(),
        ] {
            for function in [linear(), fixed_amount()] {
                let fixture = setup(
                    platform_version.protocol_version,
                    0,
                    1,
                    function.clone(),
                    TokenDistributionRecipient::EvonodesByParticipation,
                );
                // Epoch 11 has begun; epoch 10 is not finalized yet. The claimant proposes
                // none of epoch 10's blocks.
                fixture.seed_finalized_epochs_proposing(
                    0..=9,
                    BLOCKS_PER_EPOCH,
                    blocks_proposed_by_claimant,
                    platform_version,
                );
                let share = |claimed: (EpochIndex, EpochIndex)| {
                    participation_share(
                        &function,
                        0,
                        1,
                        claimed,
                        |epoch| {
                            if epoch == 10 {
                                0
                            } else {
                                blocks_proposed_by_claimant(epoch)
                            }
                        },
                        platform_version,
                    )
                };

                let processing_result = fixture.claim_in_epoch(2, 11, platform_version).await;

                match (platform_version.protocol_version < 14, &function) {
                    (true, DistributionFunction::FixedAmount { .. }) => {
                        assert_matches!(
                            processing_result.execution_results().as_slice(),
                            [StateTransitionExecutionResult::SuccessfulExecution { .. }]
                        );
                        // Ten epochs at the share of the first nine, a fifth: 2,000, where
                        // the claimant's blocks earned 1,800.
                        assert_eq!(
                            fixture.owner_token_balance(platform_version),
                            Some(INITIAL_BALANCE + 2_000)
                        );
                        assert_eq!(share((0, 10)), 1_800);
                    }
                    (true, _) => {
                        assert_matches!(
                            processing_result.execution_results().as_slice(),
                            [StateTransitionExecutionResult::InternalError(message)]
                                if message.contains("missing epoch info for epochs 10..=10")
                        );
                        assert_eq!(
                            fixture.owner_token_balance(platform_version),
                            Some(INITIAL_BALANCE)
                        );
                    }
                    (false, _) => {
                        assert_matches!(
                            processing_result.execution_results().as_slice(),
                            [StateTransitionExecutionResult::SuccessfulExecution { .. }],
                            "{function}"
                        );
                        assert_eq!(
                            fixture.owner_token_balance(platform_version),
                            Some(INITIAL_BALANCE + share((0, 9))),
                            "{function}"
                        );

                        // Nothing else is known yet: a refusal, paid.
                        let processing_result =
                            fixture.claim_in_epoch(3, 11, platform_version).await;
                        assert_matches!(
                            processing_result.execution_results().as_slice(),
                            [StateTransitionExecutionResult::PaidConsensusError {
                                error: ConsensusError::StateError(
                                    StateError::InvalidTokenClaimNoCurrentRewards(_)
                                ),
                                ..
                            }],
                            "{function}"
                        );

                        // Epoch 10 is finalized, with no block of the claimant's, and epoch
                        // 11 after it: the next claim pays epoch 10 at that share, nothing,
                        // and moves on with epoch 11.
                        fixture.seed_finalized_epochs_proposing(
                            [10],
                            BLOCKS_PER_EPOCH,
                            |_| 0,
                            platform_version,
                        );
                        fixture.seed_finalized_epochs_proposing(
                            [11],
                            BLOCKS_PER_EPOCH,
                            blocks_proposed_by_claimant,
                            platform_version,
                        );
                        let processing_result =
                            fixture.claim_in_epoch(4, 12, platform_version).await;
                        assert_matches!(
                            processing_result.execution_results().as_slice(),
                            [StateTransitionExecutionResult::SuccessfulExecution { .. }],
                            "{function}"
                        );
                        assert_eq!(
                            fixture.owner_token_balance(platform_version),
                            Some(INITIAL_BALANCE + share((0, 11))),
                            "{function}"
                        );
                    }
                }
            }
        }
    }
}
