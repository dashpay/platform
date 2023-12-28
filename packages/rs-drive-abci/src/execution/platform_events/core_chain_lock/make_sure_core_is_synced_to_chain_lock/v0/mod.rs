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
    pub(super) fn make_sure_core_is_synced_to_chain_lock_v0(
        &self,
        chain_lock: &ChainLock,
    ) -> Result<(), Error> {
        // We need to make sure core is synced to the core height we see as valid for the state transitions

        match self.core_rpc.submit_chain_lock(chain_lock) {
            Ok(_) => {}
            Err(_) => {}
        }

    }
}