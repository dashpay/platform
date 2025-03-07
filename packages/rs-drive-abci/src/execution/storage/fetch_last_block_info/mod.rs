use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use dpp::block::block_info::BlockInfo;
use dpp::reduced_platform_state::ReducedPlatformStateForSaving;
use dpp::version::PlatformVersion;
use drive::drive::Drive;
use drive::query::TransactionArg;

mod v0;

impl<C> Platform<C> {
    /// Fetches execution state from grovedb storage
    pub fn fetch_last_block_info(
        drive: &Drive,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Option<BlockInfo>, Error> {
        match platform_version
            .drive_abci
            .methods
            .last_block_info_storage
            .fetch_last_block_info
        {
            0 => Platform::<C>::fetch_last_block_info_v0(drive, transaction, platform_version),
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "fetch_last_block_info".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
