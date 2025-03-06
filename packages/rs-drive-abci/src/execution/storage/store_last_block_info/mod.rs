mod v0;

use dpp::block::block_info::BlockInfo;
use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use dpp::version::PlatformVersion;
use drive::query::TransactionArg;

impl<C> Platform<C> {
    /// Store the execution state in grovedb storage
    pub fn store_last_block_info(
        &self,
        block_info: &BlockInfo,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        match platform_version
            .drive_abci
            .methods
            .platform_state_storage
            .store_platform_state
        {
            0 => self.store_last_block_info_v0(block_info, transaction, platform_version),
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "store_last_block_info".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
