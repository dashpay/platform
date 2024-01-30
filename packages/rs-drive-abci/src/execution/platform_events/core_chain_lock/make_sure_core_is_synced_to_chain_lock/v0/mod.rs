use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::rpc::core::CoreRPCLike;
use dashcore_rpc::dashcore::ChainLock;
use dpp::version::PlatformVersion;
use crate::execution::platform_events::core_chain_lock::make_sure_core_is_synced_to_chain_lock::CoreSyncStatus;
use crate::execution::platform_events::core_chain_lock::make_sure_core_is_synced_to_chain_lock::CoreSyncStatus::{CoreIsSynced, CoreAlmostSynced, CoreNotSynced};

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    /// The point of this call is to make sure core is synced.
    /// Before this call we had previously validated that the chain lock is valid.
    pub(super) fn make_sure_core_is_synced_to_chain_lock_v0(
        &self,
        chain_lock: &ChainLock,
        platform_version: &PlatformVersion,
    ) -> Result<CoreSyncStatus, Error> {
        let given_chain_lock_height = chain_lock.block_height;
        // We need to make sure core is synced to the core height we see as valid for the state transitions
        let best_chain_locked_height = self.core_rpc.submit_chain_lock(chain_lock)?;
        Ok(if best_chain_locked_height >= given_chain_lock_height {
            CoreIsSynced
        } else if best_chain_locked_height - given_chain_lock_height
            <= platform_version
                .drive_abci
                .methods
                .core_chain_lock
                .recent_block_count_amount
        {
            CoreAlmostSynced
        } else {
            CoreNotSynced
        })
    }
}
