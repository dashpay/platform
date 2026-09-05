mod v0;

use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use dpp::reduced_platform_state::ReducedPlatformState;
use dpp::version::PlatformVersion;
use drive::query::TransactionArg;

impl<C> Platform<C> {
    /// Fetch the reduced platform state from the replicated grovedb state.
    ///
    /// Returns `Ok(None)` when the reduced state is absent (a snapshot taken before the
    /// protocol version that introduced it).
    pub fn fetch_reduced_platform_state(
        &self,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Option<ReducedPlatformState>, Error> {
        match platform_version
            .drive_abci
            .methods
            .platform_state_storage
            .fetch_reduced_platform_state
        {
            0 => self.fetch_reduced_platform_state_v0(transaction, platform_version),
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "fetch_reduced_platform_state".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
