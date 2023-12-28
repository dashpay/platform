use crate::error::Error;
use dpp::dashcore::ChainLock;

use crate::platform_types::platform::Platform;

use crate::rpc::core::CoreRPCLike;

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    /// Verify the chain lock through core v0
    pub fn verify_chain_lock_through_core_v0(&self, chain_lock: &ChainLock, submit: bool) -> Result<bool, Error> {
        if submit {
            Ok(self.core_rpc.submit_chain_lock(chain_lock)?)
        } else {
            Ok(self.core_rpc.verify_chain_lock(chain_lock)?)
        }
    }
}
