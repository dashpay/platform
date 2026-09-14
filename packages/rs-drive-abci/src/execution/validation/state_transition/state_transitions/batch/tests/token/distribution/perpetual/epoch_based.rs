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
    use crate::platform_types::state_transitions_processing_result::StateTransitionsProcessingResult;
    use dpp::block::epoch::{Epoch, EpochIndex};
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

    fn setup(protocol_version: ProtocolVersion) -> EpochTokenFixture {
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
                                function: DistributionFunction::FixedAmount {
                                    amount: AMOUNT_PER_CYCLE,
                                },
                            },
                            distribution_recipient: TokenDistributionRecipient::ContractOwner,
                        },
                    )));
            }),
            None,
            None,
            None,
            Some(CONTRACT_CREATION_EPOCH),
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
        let fixture = setup(platform_version.protocol_version);

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

    /// Protocol version 13 is frozen with the wrapping cap: the same claim is refused as having
    /// no rewards. The test profile checks arithmetic, so this also proves the frozen path
    /// reaches that refusal without an overflow panic, and replaying such a block cannot halt a
    /// node.
    #[tokio::test]
    async fn should_keep_refusing_epoch_rewards_with_a_nonzero_start_at_protocol_version_13() {
        let platform_version = PlatformVersion::get(13).expect("expected protocol version 13");
        let fixture = setup(platform_version.protocol_version);

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
