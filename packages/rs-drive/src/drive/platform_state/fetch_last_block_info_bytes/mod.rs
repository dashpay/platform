use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

mod v0;

impl Drive {
    /// Fetches last block info from grove (needed for Platform reconstruction once state sync is completed)
    pub fn fetch_last_block_info_bytes(
        &self,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Option<Vec<u8>>, Error> {
        match platform_version
            .drive
            .methods
            .last_block_info
            .fetch_last_block_info_bytes
        {
            0 => self.fetch_last_block_info_bytes_v0(transaction, platform_version),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "fetch_last_block_info_bytes".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
