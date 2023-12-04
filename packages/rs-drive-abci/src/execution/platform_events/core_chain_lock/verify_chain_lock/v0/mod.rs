
use dpp::dashcore::ChainLock;
use dpp::version::PlatformVersion;
use crate::error::Error;

use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
use crate::platform_types::platform_state::v0::PlatformStateV0Methods;

use crate::rpc::core::CoreRPCLike;

impl<C> Platform<C>
    where
        C: CoreRPCLike,
{
    pub(super) fn verify_chain_lock_v0(&self, platform_state: &PlatformState, chain_lock: &ChainLock, platform_version: &PlatformVersion) -> Result<bool, Error> {
        // we attempt to verify the chain lock locally
        // if the chain lock height is within the interval in which the quorums should not have changed
        let current_height = platform_state.core_height();

        //todo: is this correct? Also maybe this should be a parameter

        let end_window = current_height % 24 + 23;

        if chain_lock.block_height <= end_window {
            // first we verify the chain lock locally
            if let Some(valid) = self.verify_chain_lock_locally(platform_state, chain_lock, platform_version)? {
                Ok(valid)
            } else {
                // if we were not able to validate it locally then we should go to core
                self.verify_chain_lock_through_core(chain_lock, platform_version)
            }
        } else {
            self.verify_chain_lock_through_core(chain_lock, platform_version)
        }
    }
}
