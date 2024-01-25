use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::rpc::core::CoreRPCLike;
use dashcore_rpc::dashcore::ChainLock;
use dpp::version::PlatformVersion;

/// Version 0
pub mod v0;

/// As we ask to make sure that core is synced to the chain lock, we get back one of 3
pub enum CoreSyncStatus {
    /// Core is synced
    CoreIsSynced,
    /// Core is 1 or 2 blocks off, we should retry shortly
    CoreAlmostSynced,
    /// Core is more than 2 blocks off
    CoreNotSynced,
}

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    /// The point of this call is to make sure core is synced.
    /// Before this call we had previously validated that the chain lock is valid.
    /// Right now the core height should be the same as the chain lock height.
    ///
    /// Todo: In the future: The core height passed here is the core height that we need to be able to validate all
    /// asset lock proofs.
    /// It should be chosen by taking the highest height of all state transitions that require core.
    /// State transitions that require core are:
    ///     *Identity Create State transition
    ///     *Identity Top up State transition
    pub fn make_sure_core_is_synced_to_chain_lock(
        &self,
        chain_lock: &ChainLock,
        platform_version: &PlatformVersion,
    ) -> Result<CoreSyncStatus, Error> {
        match platform_version
            .drive_abci
            .methods
            .core_chain_lock
            .make_sure_core_is_synced_to_chain_lock
        {
            0 => self.make_sure_core_is_synced_to_chain_lock_v0(chain_lock, platform_version),
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "make_sure_core_is_synced_to_chain_lock".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
