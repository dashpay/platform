mod v0;

use crate::error::execution::ExecutionError;
use crate::error::Error;
use dpp::dashcore::ChainLock;

use crate::platform_types::platform::Platform;

use crate::rpc::core::CoreRPCLike;

use crate::config::PlatformConfig;
use crate::platform_types::platform_state::PlatformState;
use dpp::version::PlatformVersion;

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    /// Returning None here means we were unable to verify the chain lock because of an absence of
    /// the quorum
    pub fn verify_chain_lock_locally(
        &self,
        round: u32,
        platform_state: &PlatformState,
        chain_lock: &ChainLock,
        platform_version: &PlatformVersion,
    ) -> Result<Option<bool>, Error> {
        match platform_version
            .drive_abci
            .methods
            .core_chain_lock
            .verify_chain_lock_locally
        {
            0 => self.verify_chain_lock_locally_v0(
                round,
                platform_state,
                chain_lock,
                platform_version,
            ),
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "verify_chain_lock_locally".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
