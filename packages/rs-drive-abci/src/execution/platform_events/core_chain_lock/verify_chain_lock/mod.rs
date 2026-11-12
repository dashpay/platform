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
    /// Verify the chain lock against the chain lock quorums Platform holds, and leave it to
    /// `verify_chain_lock_through_core` when Platform cannot tell on its own.
    ///
    /// With `make_sure_core_is_synced`, a chain lock found valid locally is submitted to Core
    /// to find out whether Core is synced to it. A chain lock left to Core is submitted by
    /// `verify_chain_lock_through_core` 0 without a signature check of its own; from
    /// `verify_chain_lock_through_core` 1 (protocol version 15) it is submitted only after
    /// Core's `verifychainlock` accepts it.
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
