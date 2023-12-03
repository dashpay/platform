
use dpp::dashcore::ChainLock;
use crate::error::execution::ExecutionError;
use crate::error::Error;

use crate::platform_types::platform::Platform;

use crate::rpc::core::CoreRPCLike;

use dpp::version::PlatformVersion;

impl<C> Platform<C>
    where
        C: CoreRPCLike,
{
    /// Returning None here means we were unable to verify the chain lock because of an absence of
    /// the quorum
    pub fn verify_chain_lock_locally_v0(&self, chain_lock: &ChainLock, platform_version: &PlatformVersion) -> Result<Option<bool>, Error> {
        //todo()
        return Ok(Some(true))
    }
}
