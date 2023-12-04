mod v0;

use dpp::dashcore::ChainLock;
use crate::error::execution::ExecutionError;
use crate::error::Error;

use crate::platform_types::platform::Platform;

use crate::rpc::core::CoreRPCLike;

use dpp::version::PlatformVersion;
use crate::platform_types::platform_state::PlatformState;

impl<C> Platform<C>
    where
        C: CoreRPCLike,
{
    /// Verify the chain lock
    pub fn verify_chain_lock(&self, platform_state: &PlatformState, chain_lock: &ChainLock, platform_version: &PlatformVersion) -> Result<bool, Error> {
        match platform_version
            .drive_abci
            .methods
            .core_chain_lock
            .verify_chain_lock
        {
            0 => {
                self.verify_chain_lock_v0( platform_state, chain_lock, platform_version)
            }
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "verify_chain_lock".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
