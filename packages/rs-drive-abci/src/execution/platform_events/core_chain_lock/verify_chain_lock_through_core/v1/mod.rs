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
    /// Verify the chain lock through core v1: Core's `verifychainlock` checks the signature
    /// first, and only a chain lock it accepts is submitted. `submitchainlock` returns Core's
    /// best chain lock height without checking the signature when that height is at or above the
    /// chain lock's, so submitting alone does not authenticate a chain lock Platform left to
    /// Core.
    #[inline(always)]
    pub(super) fn verify_chain_lock_through_core_v1(
        &self,
        chain_lock: &ChainLock,
        submit: bool,
        platform_version: &PlatformVersion,
    ) -> Result<(bool, Option<CoreSyncStatus>), Error> {
        // `verifychainlock` also fails a valid chain lock while Core does not have the block at
        // the signing height, where `submitchainlock` fails too.
        if !self.core_rpc.verify_chain_lock(chain_lock)? {
            tracing::warn!(
                ?chain_lock,
                "Core's verifychainlock rejected the chain lock at core height {}: its signature is invalid, or Core does not have the block it was signed at yet",
                chain_lock.block_height,
            );
            return Ok((false, None));
        }

        if !submit {
            return Ok((true, None));
        }

        let sync_status =
            self.make_sure_core_is_synced_to_chain_lock(chain_lock, platform_version)?;
        Ok((true, Some(sync_status)))
    }
}
