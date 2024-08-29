use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
use dpp::version::PlatformVersion;
use drive::drive::Drive;
use drive::query::TransactionArg;

mod v0;

impl<C> Platform<C> {
    /// Fetches execution state from grovedb storage
    pub fn fetch_platform_state(
        drive: &Drive,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Option<PlatformState>, Error> {
        match platform_version
            .drive_abci
            .methods
            .platform_state_storage
            .fetch_platform_state
        {
            0 => Platform::<C>::fetch_platform_state_v0(drive, transaction, platform_version),
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "fetch_platform_state".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
