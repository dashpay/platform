mod v0;

use crate::error::execution::ExecutionError;
use crate::error::Error;
use dpp::dashcore::ChainLock;

use crate::platform_types::platform::Platform;

use crate::rpc::core::CoreRPCLike;

use crate::platform_types::platform_state::PlatformState;
use crate::platform_types::verify_chain_lock_result::v0::VerifyChainLockResult;
use dpp::version::PlatformVersion;

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    /// Verify the chain lock
    /// If submit is true, we try to submit the chain lock if it is considered valid or we can not check to see if it is
    /// valid on platform.
    pub fn verify_chain_lock(
        &self,
        round: u32,
        platform_state: &PlatformState,
        chain_lock: &ChainLock,
        make_sure_core_is_synced: bool,
        platform_version: &PlatformVersion,
    ) -> Result<VerifyChainLockResult, Error> {
        match platform_version
            .drive_abci
            .methods
            .core_chain_lock
            .verify_chain_lock
        {
            0 => self.verify_chain_lock_v0(
                round,
                platform_state,
                chain_lock,
                make_sure_core_is_synced,
                platform_version,
            ),
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "verify_chain_lock".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
