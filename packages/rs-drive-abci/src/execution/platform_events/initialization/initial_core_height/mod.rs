mod v0;

use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::rpc::core::CoreRPCLike;
use dpp::version::PlatformVersion;

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    /// Determine initial core height.
    ///
    /// Use core height received from Tenderdash (from genesis.json) by default,
    /// otherwise we go with height of v20 fork.
    ///
    /// Core height is verified to ensure that it is both at or after v20 fork, and
    /// before or at last chain lock.
    ///
    /// ## Error handling
    ///
    /// This function will fail if:
    ///
    /// * v20 fork is not yet active
    /// * `requested` core height is before v20 fork
    /// * `requested` core height is after current best chain lock
    ///
    pub(in crate::execution) fn initial_core_height(
        &self,
        requested: Option<u32>,
        platform_version: &PlatformVersion,
    ) -> Result<u32, Error> {
        match platform_version
            .drive_abci
            .methods
            .initialization
            .initial_core_height
        {
            0 => self.initial_core_height_v0(requested),
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "initial_core_height".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
