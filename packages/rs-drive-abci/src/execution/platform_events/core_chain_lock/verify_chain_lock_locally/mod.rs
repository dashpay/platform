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
    /// Returning None here means we were unable to verify the chain lock because of an absence of
    /// the quorum
    pub fn verify_chain_lock_locally(&self, chain_lock: &ChainLock, platform_version: &PlatformVersion) -> Result<Option<bool>, Error> {
        match platform_version
            .drive_abci
            .methods
            .core_chain_lock
            .verify_chain_lock_locally
        {
            0 => {
                self.verify_chain_lock_locally_v0(chain_lock, platform_version)
            }
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "verify_chain_lock_locally".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
