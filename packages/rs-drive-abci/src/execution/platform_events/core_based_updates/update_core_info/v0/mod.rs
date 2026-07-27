use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
use crate::platform_types::platform_state::PlatformStateV0Methods;
use crate::rpc::core::CoreRPCLike;
use dpp::block::block_info::BlockInfo;
use dpp::version::PlatformVersion;
use drive::grovedb::Transaction;

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    /// Updates the core information in the platform state based on the given core block height.
    ///
    /// This function updates both the masternode list and the quorum information in the platform
    /// state. It calls the update_masternode_list and update_quorum_info functions to perform
    /// the respective updates.
    ///
    /// # Arguments
    ///
    /// * platform_state - A reference to the platform state before execution of current block.
    /// * block_platform_state - A mutable reference to the current platform state in the block
    ///   execution context to be updated.
    /// * core_block_height - The current block height in the Dash Core.
    /// * is_init_chain - A boolean indicating if the chain is being initialized.
    /// * block_info - A reference to the block information.
    /// * transaction - The current groveDB transaction.
    ///
    /// # Returns
    ///
    /// * Result<(), Error> - Returns Ok(()) if the update is successful. Returns an error if
    ///   there is a problem updating the masternode list, quorum information, or the state.
    #[inline(always)]
    #[allow(clippy::too_many_arguments)]
    pub(super) fn update_core_info_v0(
        &self,
        platform_state: Option<&PlatformState>,
        block_platform_state: &mut PlatformState,
        core_block_height: u32,
        is_init_chain: bool,
        block_info: &BlockInfo,
        transaction: &Transaction,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        // the core height of the block platform state is the last committed
        if !is_init_chain && block_platform_state.last_committed_core_height() == core_block_height
        {
            // if we get the same height that we know we do not need to update core info
            return Ok(());
        }
        self.update_masternode_list(
            platform_state,
            block_platform_state,
            core_block_height,
            is_init_chain,
            block_info,
            transaction,
            platform_version,
        )?;

        self.update_quorum_info(
            platform_state,
            block_platform_state,
            core_block_height,
            false,
            platform_version,
        )
    }
}

#[cfg(test)]
mod tests {
    use crate::platform_types::platform_state::PlatformStateV0Methods;
    use crate::test::helpers::setup::TestPlatformBuilder;
    use dpp::block::block_info::BlockInfo;
    use dpp::version::PlatformVersion;

    /// When `is_init_chain = false` and the current block state already has the
    /// same core height, `update_core_info_v0` must early-return `Ok(())` without
    /// calling Core RPC. This is critical because skipping this branch would
    /// fire RPC calls we haven't set up in the mock.
    #[test]
    fn v0_returns_ok_when_same_core_height_and_not_init() {
        let platform_version = PlatformVersion::latest();
        let platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_genesis_state();

        let state = platform.state.load();
        let mut block_platform_state = state.as_ref().clone();
        drop(state);

        // Read the already-committed core height off the block state — the v0
        // path early-returns only if we pass the SAME value as `core_block_height`.
        let same_core_height = block_platform_state.last_committed_core_height();

        let transaction = platform.drive.grove.start_transaction();
        let block_info = BlockInfo {
            time_ms: 0,
            height: 1,
            core_height: same_core_height,
            epoch: Default::default(),
        };

        platform
            .update_core_info_v0(
                None,
                &mut block_platform_state,
                same_core_height,
                /*is_init_chain=*/ false,
                &block_info,
                &transaction,
                platform_version,
            )
            .expect("same-height non-init must short-circuit OK");
    }

    /// The same-height early-return short-circuit must be idempotent: calling
    /// `update_core_info_v0` repeatedly with the same `core_block_height` and
    /// `is_init_chain = false` must succeed every time without ever reaching
    /// the downstream `update_masternode_list` / `update_quorum_info` paths
    /// (which would fail because the mock Core RPC has no expectations set).
    #[test]
    fn v0_short_circuit_is_idempotent_when_not_init_chain() {
        // Re-confirm that both conditions together are required by running
        // the short-circuit case twice; idempotency guards against flakiness.
        let platform_version = PlatformVersion::latest();
        let platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_genesis_state();

        let state = platform.state.load();
        let mut block_platform_state = state.as_ref().clone();
        drop(state);

        let same_core_height = block_platform_state.last_committed_core_height();
        let transaction = platform.drive.grove.start_transaction();
        let block_info = BlockInfo {
            time_ms: 0,
            height: 1,
            core_height: same_core_height,
            epoch: Default::default(),
        };

        for _ in 0..2 {
            platform
                .update_core_info_v0(
                    None,
                    &mut block_platform_state,
                    same_core_height,
                    false,
                    &block_info,
                    &transaction,
                    platform_version,
                )
                .expect("idempotent short-circuit");
        }
    }
}
