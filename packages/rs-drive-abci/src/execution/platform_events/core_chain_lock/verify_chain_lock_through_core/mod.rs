mod v0;

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
    /// Verify the chain lock through core
    pub fn verify_chain_lock_through_core(&self, chain_lock: &ChainLock, platform_version: &PlatformVersion) -> Result<bool, Error> {
        match platform_version
            .drive_abci
            .methods
            .core_chain_lock
            .verify_chain_lock_through_core
        {
            0 => {
                self.verify_chain_lock_through_core_v0(chain_lock)
            }
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "verify_chain_lock_through_core".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
