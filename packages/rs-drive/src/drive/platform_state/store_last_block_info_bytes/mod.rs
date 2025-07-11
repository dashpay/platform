mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

impl Drive {
    /// Store the execution state in grovedb storage
    pub fn store_last_block_info_bytes(
        &self,
        last_block_info_bytes: &[u8],
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        match platform_version
            .drive
            .methods
            .last_block_info
            .store_last_block_info_bytes
        {
            0 => self.store_last_block_info_bytes_v0(
                last_block_info_bytes,
                transaction,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "store_last_block_info_bytes".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
