mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

impl Drive {
    /// Store the execution state in grovedb storage
    pub fn store_platform_state_bytes(
        &self,
        state_bytes: &[u8],
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        match platform_version
            .drive
            .methods
            .platform_state
            .store_platform_state_bytes
        {
            0 => self.store_platform_state_bytes_v0(state_bytes, transaction),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "store_platform_state_bytes".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
