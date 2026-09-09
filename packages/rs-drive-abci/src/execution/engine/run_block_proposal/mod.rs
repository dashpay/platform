use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::execution::types::block_state_info;
use crate::execution::types::block_state_info::v0::BlockStateInfoV0Methods;
use crate::metrics::HistogramTiming;
use crate::platform_types::epoch_info::v0::{EpochInfoV0Getters, EpochInfoV0Methods};
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
use crate::platform_types::platform_state::PlatformStateV0Methods;
use crate::platform_types::{block_execution_outcome, block_proposal};
use crate::rpc::core::CoreRPCLike;
use dpp::block::epoch::Epoch;
use dpp::validation::ValidationResult;
use dpp::version::PlatformVersion;
use drive::grovedb::Transaction;

mod v0;

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    /// Runs a block proposal, either from process proposal or prepare proposal.
    ///
    /// This function takes a `BlockProposal` and a `Transaction` as input and processes the block
    /// proposal. It first validates the block proposal and then processes raw state transitions,
    /// withdrawal transactions, and block fees. It also updates the validator set.
    ///
    /// # Arguments
    ///
    /// * `block_proposal` - The block proposal to be processed.
    /// * `known_from_us` - Do we know that we made this block proposal?
    /// * `transaction` - The transaction associated with the block proposal.
    ///
    /// # Returns
    ///
    /// * `Result<ValidationResult<BlockExecutionOutcome, Error>, Error>` - If the block proposal is
    ///   successfully processed, it returns a `ValidationResult` containing the `BlockExecutionOutcome`.
    ///   If the block proposal processing fails, it returns an `Error`. Consensus errors are returned
    ///   in the `ValidationResult`, while critical system errors are returned in the `Result`.
    ///
    /// # Errors
    ///
    /// This function may return an `Error` variant if there is a problem with processing the block
    /// proposal, updating the core info, processing raw state transitions, or processing block fees.
    ///
    pub fn run_block_proposal(
        &self,
        block_proposal: block_proposal::v0::BlockProposal,
        known_from_us: bool,
        platform_state: &PlatformState,
        transaction: &Transaction,
        timer: Option<&HistogramTiming>,
    ) -> Result<ValidationResult<block_execution_outcome::v0::BlockExecutionOutcome, Error>, Error>
    {
        // Epoch information is always calculated with the last committed platform version
        // even if we are switching to a new version in this block.
        let last_committed_platform_version = platform_state.current_platform_version()?;

        // !!!! This EpochInfo is based on the last committed platform version
        // !!!! and will be used for the first block of the epoch.
        let epoch_info = self.gather_epoch_info(
            &block_proposal,
            transaction,
            platform_state,
            last_committed_platform_version,
        )?;

        // Cleanup block cache before we execute a new proposal.
        //
        // This has to happen before `perform_events_on_first_block_of_protocol_change` below:
        // that refreshes the block cache with the contract definitions the protocol change
        // rewrites, and clearing afterwards would wipe them before any state transition read
        // them, leaving those reads to fall back to pre-change global cache entries.
        self.clear_drive_block_cache(last_committed_platform_version)?;

        // Create a bock state from previous committed state
        let mut block_platform_state = platform_state.clone();

        // Determine a platform version for this block
        let block_platform_version = if epoch_info.is_epoch_change_but_not_genesis()
            && platform_state.next_epoch_protocol_version()
                != platform_state.current_protocol_version_in_consensus()
        {
            // Switch to next proposed platform version if we are on the first block of the new epoch
            // and the next protocol version (locked in the previous epoch) is different from the
            // current protocol version.
            // This version will be set to the block state, and we decide on next version for next epoch
            // during block processing
            let next_protocol_version = platform_state.next_epoch_protocol_version();

            // We should panic if this node is not supported a new protocol version
            let Ok(next_platform_version) = PlatformVersion::get(next_protocol_version) else {
                panic!(
                    r#"Failed to upgrade the network protocol version {next_protocol_version}.

Please update your software to the latest version: https://docs.dash.org/platform-protocol-upgrade

Your software version: {}, latest supported protocol version: {}."#,
                    env!("CARGO_PKG_VERSION"),
                    PlatformVersion::latest().protocol_version
                );
            };

            let old_protocol_version = block_platform_state.current_protocol_version_in_consensus();

            if old_protocol_version != next_protocol_version {
                // Set current protocol version to the block platform state
                block_platform_state
                    .set_current_protocol_version_in_consensus(next_protocol_version);

                let last_block_time_ms = platform_state.last_committed_block_time_ms();

                // Init block execution context
                let block_state_info = block_state_info::v0::BlockStateInfoV0::from_block_proposal(
                    &block_proposal,
                    last_block_time_ms,
                );

                let block_info = block_state_info.to_block_info(
                    Epoch::new(epoch_info.current_epoch_index())
                        .expect("current epoch index should be in range"),
                );

                // This is for events like adding stuff to the root tree, or making structural changes/fixes
                self.perform_events_on_first_block_of_protocol_change(
                    platform_state,
                    &block_info,
                    transaction,
                    old_protocol_version,
                    next_platform_version,
                )?;
            }

            next_platform_version
        } else {
            // Stay on the last committed platform version
            last_committed_platform_version
        };

        match block_platform_version
            .drive_abci
            .methods
            .engine
            .run_block_proposal
        {
            0 => self.run_block_proposal_v0(
                block_proposal,
                known_from_us,
                epoch_info,
                transaction,
                platform_state,
                block_platform_state,
                block_platform_version,
                timer,
            ),
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "run_block_proposal".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::platform_types::block_proposal::v0::BlockProposal;
    use crate::platform_types::platform_state::PlatformStateV0Methods;
    use crate::test::helpers::fast_forward_to_block::fast_forward_to_block;
    use crate::test::helpers::setup::TestPlatformBuilder;
    use dpp::data_contract::accessors::v0::DataContractV0Getters;
    use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
    use dpp::system_data_contracts::SystemDataContract;
    use dpp::version::PlatformVersion;
    use std::sync::Arc;
    use tenderdash_abci::proto::version::Consensus;

    /// The DPNS `domain` document type gains its history flags at protocol version 13, which is
    /// how the migrated definition is told apart from the pre-activation one.
    fn dpns_keeps_transfer_history(
        platform: &crate::test::helpers::setup::TempPlatform<crate::rpc::core::MockCoreRPCLike>,
        transaction: &drive::grovedb::Transaction,
        platform_version: &PlatformVersion,
    ) -> bool {
        platform
            .drive
            .get_contract_with_fetch_info(
                SystemDataContract::DPNS.id().to_buffer(),
                false,
                Some(transaction),
                platform_version,
            )
            .expect("expected to resolve DPNS")
            .expect("DPNS must be present")
            .contract
            .document_type_for_name("domain")
            .expect("DPNS must contain its domain document type")
            .documents_keep_transfer_history()
    }

    fn proposal_at(
        height: u64,
        block_time_ms: u64,
        core_chain_locked_height: u32,
        app_version: u64,
        raw: &Vec<Vec<u8>>,
    ) -> BlockProposal<'_> {
        BlockProposal {
            consensus_versions: Consensus {
                block: 1,
                app: app_version,
            },
            block_hash: None,
            height,
            round: 0,
            block_time_ms,
            core_chain_locked_height,
            core_chain_lock_update: None,
            proposed_app_version: app_version,
            proposer_pro_tx_hash: [0u8; 32],
            validator_set_quorum_hash: [0u8; 32],
            raw_state_transitions: raw,
        }
    }

    /// CONSENSUS-CRITICAL ORDERING PIN.
    ///
    /// `run_block_proposal` must clear the block cache *before* running the protocol change
    /// events, because those events seed that cache with the definitions the migration just
    /// wrote. Clearing afterwards — which is where the clear used to live, inside
    /// `run_block_proposal_v0` — wipes the seed before any state transition reads it, and the
    /// block's transactional lookups then fall back to global entries a concurrent
    /// committed-state query can repopulate with the pre-activation definition.
    ///
    /// This drives the production entry point rather than calling the migration or the clear
    /// directly, so that moving either one relative to the other fails here.
    #[test]
    fn should_seed_migrated_dpns_into_the_block_cache_through_run_block_proposal() {
        let platform = TestPlatformBuilder::new()
            .with_initial_protocol_version(12)
            .build_with_mock_rpc()
            .set_genesis_state_with_activation_info(1_000_000, 1);

        // A committed block one epoch back, so the next proposal is an epoch change and not
        // genesis — the condition that triggers the protocol change events.
        // The genesis time is normally persisted at finalize block, which these tests do not
        // reach;  needs it to resolve the epoch.
        platform.drive.set_genesis_time(1_000_000);
        fast_forward_to_block(&platform, 1_100_000, 100, 42, 0, false);

        let mut state = platform.state.load().as_ref().clone();
        state.set_current_protocol_version_in_consensus(12);
        state.set_next_epoch_protocol_version(13);
        platform.state.store(Arc::new(state));

        // A long-lived node: DPNS read once with no transaction, which is the path every
        // read-only DAPI query takes and what leaves the pre-activation definition in the
        // global cache. Without this the block-cache seed is unobservable, because a miss
        // would fall through to a grovedb read inside the block transaction and find the
        // migrated contract anyway.
        platform
            .drive
            .get_contract_with_fetch_info(
                SystemDataContract::DPNS.id().to_buffer(),
                true,
                None,
                PlatformVersion::get(12).expect("v12"),
            )
            .expect("expected to warm the global cache")
            .expect("DPNS must be present");

        let platform_state = platform.state.load();
        let transaction = platform.drive.grove.start_transaction();
        let raw = vec![];

        assert!(
            !dpns_keeps_transfer_history(
                &platform,
                &transaction,
                PlatformVersion::get(12).expect("v12")
            ),
            "precondition: the pre-activation DPNS is what state holds"
        );

        let proposal = proposal_at(101, 789_410_000, 42, 13, &raw);

        // The outcome of the proposal itself is not what is under test — the seeding happens
        // in the protocol change events, which run before any later validation can reject it.
        let _ = platform.run_block_proposal(proposal, false, &platform_state, &transaction, None);

        // Assert on the block cache directly rather than on what a read resolves to: the
        // migration also evicts the global entry, so a read would fall through to a grovedb
        // fetch inside the block transaction and find the migrated contract even if the seed
        // had been wiped. The seed is what stops a concurrent committed-state query from
        // diverting the rest of the block, so the seed itself is what must be pinned.
        let seeded = platform
            .drive
            .cache
            .data_contracts
            .get(SystemDataContract::DPNS.id().to_buffer(), true)
            .expect(
                "the activation must leave the migrated DPNS in the block cache; if this is empty the block cache was cleared after the migration seeded it",
            );
        assert!(
            seeded
                .contract
                .document_type_for_name("domain")
                .expect("DPNS must contain its domain document type")
                .documents_keep_transfer_history(),
            "the seeded definition must be the migrated one"
        );
        assert!(
            platform
                .drive
                .cache
                .data_contracts
                .get(SystemDataContract::DPNS.id().to_buffer(), false)
                .is_none(),
            "the uncommitted definition must not be visible to committed-state readers"
        );
    }

    /// The clear is deliberately performed before the proposal is validated, so a proposal
    /// rejected for height or app version still leaves no stale scratch behind for the next
    /// one. Pins that the clear sits above those checks rather than below them.
    #[test]
    fn should_clear_the_block_cache_before_rejecting_a_wrong_height_proposal() {
        let platform = TestPlatformBuilder::new()
            .with_initial_protocol_version(13)
            .build_with_mock_rpc()
            .set_genesis_state_with_activation_info(1_000_000, 1);
        let platform_version = PlatformVersion::get(13).expect("v13");

        // The genesis time is normally persisted at finalize block, which these tests do not
        // reach;  needs it to resolve the epoch.
        platform.drive.set_genesis_time(1_000_000);
        fast_forward_to_block(&platform, 1_100_000, 100, 42, 0, false);

        let platform_state = platform.state.load();
        let transaction = platform.drive.grove.start_transaction();
        let raw = vec![];

        // Seed the block cache the way a previous proposal would have.
        platform
            .drive
            .get_contract_with_fetch_info(
                SystemDataContract::DPNS.id().to_buffer(),
                true,
                Some(&transaction),
                platform_version,
            )
            .expect("expected to seed the block cache")
            .expect("DPNS must be present");
        assert!(
            platform
                .drive
                .cache
                .data_contracts
                .get(SystemDataContract::DPNS.id().to_buffer(), true)
                .is_some(),
            "precondition: the block cache holds the seeded contract"
        );

        // Wrong height: `next_block_to` requires previous_height + 1.
        let proposal = proposal_at(999, 1_200_000, 42, 13, &raw);
        let outcome = platform
            .run_block_proposal(proposal, false, &platform_state, &transaction, None)
            .expect("expected the proposal to be processed");

        assert!(
            !outcome.is_valid(),
            "precondition: a wrong-height proposal must be rejected"
        );
        assert!(
            platform
                .drive
                .cache
                .data_contracts
                .get(SystemDataContract::DPNS.id().to_buffer(), true)
                .is_none(),
            "the block cache must be cleared before the proposal is validated"
        );
    }

    /// An ordinary proposal — no protocol change — must still clear the block cache exactly as
    /// it did when the clear lived inside `run_block_proposal_v0`.
    #[test]
    fn should_clear_the_block_cache_on_an_ordinary_proposal() {
        let platform = TestPlatformBuilder::new()
            .with_initial_protocol_version(13)
            .build_with_mock_rpc()
            .set_genesis_state_with_activation_info(1_000_000, 1);
        let platform_version = PlatformVersion::get(13).expect("v13");

        // The genesis time is normally persisted at finalize block, which these tests do not
        // reach;  needs it to resolve the epoch.
        platform.drive.set_genesis_time(1_000_000);
        fast_forward_to_block(&platform, 1_100_000, 100, 42, 0, false);

        let platform_state = platform.state.load();
        let transaction = platform.drive.grove.start_transaction();
        let raw = vec![];

        platform
            .drive
            .get_contract_with_fetch_info(
                SystemDataContract::DPNS.id().to_buffer(),
                true,
                Some(&transaction),
                platform_version,
            )
            .expect("expected to seed the block cache")
            .expect("DPNS must be present");

        let proposal = proposal_at(101, 1_200_000, 42, 13, &raw);
        let _ = platform.run_block_proposal(proposal, false, &platform_state, &transaction, None);

        assert!(
            platform
                .drive
                .cache
                .data_contracts
                .get(SystemDataContract::DPNS.id().to_buffer(), true)
                .is_none(),
            "an ordinary proposal must still clear the block cache it inherited"
        );
    }
}
