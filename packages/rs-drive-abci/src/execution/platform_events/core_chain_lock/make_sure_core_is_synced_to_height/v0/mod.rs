use dashcore_rpc::dashcore::ChainLock;
use dpp::version::PlatformVersion;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::rpc::core::CoreRPCLike;

impl<C> Platform<C>
    where
        C: CoreRPCLike,
{
    /// The point of this call is to make sure core is synced.
    /// Before this call we had previously validated that the chain lock is valid.
    pub(super) fn make_sure_core_is_synced_to_height_v0(
        &self,
        chain_lock: &ChainLock,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        // We need to make sure core is synced to the core height we see as valid for the state transitions

        // First we must ask core for the locked core height

        let best_core_chain_lock = self.core_rpc.get_best_chain_lock()?;

        // If the best core chain lock height is higher than that current chain lock
    }
}