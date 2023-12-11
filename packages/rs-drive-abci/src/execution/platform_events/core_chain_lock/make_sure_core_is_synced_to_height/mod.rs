use dashcore_rpc::dashcore::ChainLock;
use dpp::version::PlatformVersion;
use crate::error::Error;
use crate::error::execution::ExecutionError;
use crate::platform_types::platform::Platform;
use crate::rpc::core::CoreRPCLike;

/// Version 0
pub mod v0;

impl<C> Platform<C>
    where
        C: CoreRPCLike,
{
    /// The point of this call is to make sure core is synced.
    /// Before this call we had previously validated that the chain lock is valid.
    /// The core height passed here is the core height that we need to be able to validate all asset lock proofs.
    /// It should be chosen by taking the highest height of all state transitions that require core.
    /// State transitions that require core are:
    ///     *Identity Create State transition
    ///     *Identity Top up State transition
    pub fn make_sure_core_is_synced_to_height(
        &self,
        core_height: u32,
        chain_lock: &ChainLock,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        match platform_version
            .drive_abci
            .methods
            .core_chain_lock
            .make_sure_core_is_synced_to_height
        {
            0 => Ok(self.make_sure_core_is_synced_to_height_v0(core_height, chain_lock, platform_version)),
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "make_sure_core_is_synced_to_height".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}