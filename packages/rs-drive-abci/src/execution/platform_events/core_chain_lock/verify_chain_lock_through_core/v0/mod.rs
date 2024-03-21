use crate::error::Error;
use crate::execution::platform_events::core_chain_lock::make_sure_core_is_synced_to_chain_lock::CoreSyncStatus;
use dpp::dashcore::ChainLock;
use dpp::version::PlatformVersion;

use crate::platform_types::platform::Platform;

use crate::rpc::core::CoreRPCLike;

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    /// Verify the chain lock through core v0
    #[inline(always)]
    pub(super) fn verify_chain_lock_through_core_v0(
        &self,
        chain_lock: &ChainLock,
        submit: bool,
        platform_version: &PlatformVersion,
    ) -> Result<(bool, Option<CoreSyncStatus>), Error> {
        if submit {
            let given_chain_lock_height = chain_lock.block_height;

            let best_chain_locked_height = self.core_rpc.submit_chain_lock(chain_lock)?;
            Ok(if best_chain_locked_height >= given_chain_lock_height {
                (true, Some(CoreSyncStatus::Done))
            } else if best_chain_locked_height - given_chain_lock_height
                <= platform_version
                    .drive_abci
                    .methods
                    .core_chain_lock
                    .recent_block_count_amount
            {
                (true, Some(CoreSyncStatus::Almost))
            } else {
                (true, Some(CoreSyncStatus::Not))
            })
        } else {
            Ok((self.core_rpc.verify_chain_lock(chain_lock)?, None))
        }
    }
}
