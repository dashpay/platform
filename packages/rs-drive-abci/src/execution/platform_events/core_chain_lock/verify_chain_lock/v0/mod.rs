
use dpp::dashcore::ChainLock;
use dpp::version::PlatformVersion;
use crate::error::Error;

use crate::platform_types::platform::Platform;

use crate::rpc::core::CoreRPCLike;

impl<C> Platform<C>
    where
        C: CoreRPCLike,
{
    pub(super) fn verify_chain_lock_v0(&self, chain_lock: &ChainLock, platform_version: &PlatformVersion) -> Result<bool, Error> {
        // first we verify the chain lock locally
        if let Some(valid) = self.verify_chain_lock_locally(chain_lock, platform_version)? {
            Ok(valid)
        } else {
            // if we were not able to validate it locally then we should go to core
            self.verify_chain_lock_through_core(chain_lock, platform_version)
        }
    }
}
