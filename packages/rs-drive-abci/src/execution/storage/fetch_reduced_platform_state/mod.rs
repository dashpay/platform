use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use dpp::reduced_platform_state::ReducedPlatformStateForSaving;
use dpp::version::PlatformVersion;
use drive::drive::Drive;
use drive::query::TransactionArg;

mod v0;

impl<C> Platform<C> {
    /// Fetches execution state from grovedb storage
    pub fn fetch_reduced_platform_state(
        drive: &Drive,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Option<ReducedPlatformStateForSaving>, Error> {
        match platform_version
            .drive_abci
            .methods
            .platform_reduced_state_storage
            .fetch_reduced_platform_state
        {
            0 => {
                Platform::<C>::fetch_reduced_platform_state_v0(drive, transaction, platform_version)
            }
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "fetch_reduced_platform_state".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
