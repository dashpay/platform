use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
use dpp::version::PlatformVersion;
use drive::query::TransactionArg;

mod v0;

impl<C> Platform<C> {
    pub fn fetch_execution_state(
        &self,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Option<PlatformState>, Error> {
        match platform_version
            .drive_abci
            .methods
            .execution_state_storage
            .fetch_execution_state
        {
            0 => self.fetch_execution_state_v0(transaction, platform_version),
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "fetch_execution_state".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
