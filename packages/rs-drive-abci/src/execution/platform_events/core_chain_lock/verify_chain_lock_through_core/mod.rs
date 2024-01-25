mod v0;

use crate::error::execution::ExecutionError;
use crate::error::Error;
use dpp::dashcore::ChainLock;

use crate::platform_types::platform::Platform;

use crate::rpc::core::CoreRPCLike;

use crate::execution::platform_events::core_chain_lock::make_sure_core_is_synced_to_chain_lock::CoreSyncStatus;
use dpp::version::PlatformVersion;

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    /// Verify the chain lock through core
    pub fn verify_chain_lock_through_core(
        &self,
        chain_lock: &ChainLock,
        submit: bool,
        platform_version: &PlatformVersion,
    ) -> Result<(bool, Option<CoreSyncStatus>), Error> {
        match platform_version
            .drive_abci
            .methods
            .core_chain_lock
            .verify_chain_lock_through_core
        {
            0 => self.verify_chain_lock_through_core_v0(chain_lock, submit, platform_version),
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "verify_chain_lock_through_core".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
