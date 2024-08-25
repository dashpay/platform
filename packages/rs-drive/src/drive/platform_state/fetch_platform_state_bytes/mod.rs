use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

mod v0;

impl Drive {
    /// Fetches execution state from grovedb storage
    pub fn fetch_platform_state_bytes(
        &self,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Option<Vec<u8>>, Error> {
        match platform_version
            .drive
            .methods
            .platform_state
            .fetch_platform_state_bytes
        {
            0 => self.fetch_platform_state_bytes_v0(transaction),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "fetch_platform_state_bytes".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
