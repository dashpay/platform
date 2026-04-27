use crate::error::Error;
use crate::execution::types::block_state_info;
use dpp::version::PlatformVersion;

use crate::platform_types::block_proposal;
use crate::platform_types::epoch_info::v0::EpochInfoV0;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
use crate::platform_types::platform_state::PlatformStateV0Methods;
use drive::grovedb::Transaction;

impl<C> Platform<C> {
    /// Creates epoch info from the platform state and the block proposal
    pub(super) fn gather_epoch_info_v0(
        &self,
        block_proposal: &block_proposal::v0::BlockProposal,
        transaction: &Transaction,
        platform_state: &PlatformState,
        platform_version: &PlatformVersion,
    ) -> Result<EpochInfoV0, Error> {
        // Start by getting information from the state
        let last_block_time_ms = platform_state.last_committed_block_time_ms();

        // Init block execution context
        let block_state_info = block_state_info::v0::BlockStateInfoV0::from_block_proposal(
            block_proposal,
            last_block_time_ms,
        );

        // destructure the block proposal
        let block_proposal::v0::BlockProposal {
            height,
            block_time_ms,
            ..
        } = &block_proposal;

        let genesis_time_ms =
            self.get_genesis_time(*height, *block_time_ms, transaction, platform_version)?;

        EpochInfoV0::from_genesis_time_and_block_info(
            genesis_time_ms,
            &block_state_info,
            self.config.execution.epoch_time_length_s,
        )
    }
}

#[cfg(test)]
mod tests {
    use crate::error::execution::ExecutionError;
    use crate::error::Error;
    use crate::platform_types::block_proposal::v0::BlockProposal;
    use crate::test::helpers::setup::TestPlatformBuilder;
    use dpp::version::PlatformVersion;
    use tenderdash_abci::proto::version::Consensus;

    fn make_block_proposal(
        height: u64,
        block_time_ms: u64,
        raw: &Vec<Vec<u8>>,
    ) -> BlockProposal<'_> {
        BlockProposal {
            consensus_versions: Consensus { block: 1, app: 1 },
            block_hash: None,
            height,
            round: 0,
            core_chain_locked_height: 1,
            core_chain_lock_update: None,
            proposed_app_version: 1,
            proposer_pro_tx_hash: [0u8; 32],
            validator_set_quorum_hash: [0u8; 32],
            block_time_ms,
            raw_state_transitions: raw,
        }
    }

    /// At the configured genesis height, `gather_epoch_info_v0` must succeed and
    /// report `current_epoch_index = 0` — because `get_genesis_time` echoes the
    /// proposal's block_time_ms, the time since genesis is zero, which maps to
    /// epoch 0. Guards the genesis-path integration between `get_genesis_time`
    /// and `EpochInfoV0::from_genesis_time_and_block_info`.
    #[test]
    fn v0_at_genesis_height_yields_epoch_zero() {
        let platform_version = PlatformVersion::latest();
        let platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_initial_state_structure();

        let genesis_height = platform.platform.config.abci.genesis_height;
        let transaction = platform.drive.grove.start_transaction();
        let platform_state = platform.state.load();
        let raw: Vec<Vec<u8>> = Vec::new();
        let proposal = make_block_proposal(genesis_height, 1_000_000, &raw);

        use crate::platform_types::epoch_info::v0::EpochInfoV0Getters;
        let info = platform
            .platform
            .gather_epoch_info_v0(&proposal, &transaction, &platform_state, platform_version)
            .expect("gather_epoch_info_v0 must succeed at genesis height");

        assert_eq!(info.current_epoch_index(), 0);
        assert_eq!(info.previous_epoch_index(), None);
    }

    /// If drive has no cached genesis time and the height is NOT the genesis
    /// height, `gather_epoch_info_v0` must propagate the `DriveIncoherence`
    /// error from `get_genesis_time`. This pins the error-propagation contract.
    #[test]
    fn v0_above_genesis_without_stored_time_propagates_error() {
        let platform_version = PlatformVersion::latest();
        let platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_initial_state_structure();

        let genesis_height = platform.platform.config.abci.genesis_height;
        let transaction = platform.drive.grove.start_transaction();
        let platform_state = platform.state.load();
        let raw: Vec<Vec<u8>> = Vec::new();
        let proposal = make_block_proposal(genesis_height + 1, 2_000_000, &raw);

        let err = platform
            .platform
            .gather_epoch_info_v0(&proposal, &transaction, &platform_state, platform_version)
            .expect_err("must fail when genesis time is missing");

        match err {
            Error::Execution(ExecutionError::DriveIncoherence(msg)) => {
                assert!(
                    msg.to_lowercase().contains("genesis"),
                    "DriveIncoherence message must mention genesis time, got: {}",
                    msg
                );
            }
            other => panic!("expected DriveIncoherence, got: {:?}", other),
        }
    }

    /// When platform_state has NO `last_committed_block_time_ms`, the epoch info
    /// calculation takes the default branch and returns
    /// `EpochInfoV0::default()` — `current_epoch_index = 0`, `is_epoch_change = true`.
    /// This pins the "no previous block" short-circuit.
    #[test]
    fn v0_no_previous_block_time_returns_default_epoch_info() {
        let platform_version = PlatformVersion::latest();
        let platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_initial_state_structure();

        // Supply a cached genesis time so the drive lookup succeeds when we
        // pass a non-genesis height.
        const GENESIS_MS: u64 = 10_000_000;
        platform.drive.set_genesis_time(GENESIS_MS);

        let genesis_height = platform.platform.config.abci.genesis_height;
        let transaction = platform.drive.grove.start_transaction();
        let platform_state = platform.state.load();

        // We have no last_committed_block_info on the state, so
        // `last_committed_block_time_ms()` is None. The calculation returns the
        // default EpochInfoV0 regardless of how far past genesis we are.
        let block_time_ms = GENESIS_MS
            + 10 * platform
                .platform
                .config
                .execution
                .epoch_time_length_s
                .saturating_mul(1000);
        let raw: Vec<Vec<u8>> = Vec::new();
        let proposal = make_block_proposal(genesis_height + 10, block_time_ms, &raw);

        use crate::platform_types::epoch_info::v0::EpochInfoV0Getters;
        let info = platform
            .platform
            .gather_epoch_info_v0(&proposal, &transaction, &platform_state, platform_version)
            .expect("must succeed with cached genesis time");

        assert_eq!(
            info.current_epoch_index(),
            0,
            "default epoch info when no previous block"
        );
        assert!(info.is_epoch_change(), "default is_epoch_change is true");
        assert_eq!(info.previous_epoch_index(), None);
    }
}
