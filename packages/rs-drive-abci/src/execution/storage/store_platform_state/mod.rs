mod v0;

use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
use dpp::version::PlatformVersion;
use drive::query::TransactionArg;

impl<C> Platform<C> {
    /// Store the execution state in grovedb storage
    pub fn store_platform_state(
        &self,
        state: &PlatformState,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        match platform_version
            .drive_abci
            .methods
            .execution_state_storage
            .store_execution_state
        {
            0 => self.store_execution_state_v0(state, transaction, platform_version),
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "fetch_execution_state".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
