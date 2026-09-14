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
                                interval: EPOCH_INTERVAL,
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
            let transaction = self.platform.drive.grove.start_transaction();
            for epoch_index in epochs {
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
}
