mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

impl Drive {
    /// Fetches the per-block part of the platform state, if one was ever written.
    ///
    /// # Parameters
    ///
    /// * `transaction`: The GroveDB transaction.
    /// * `platform_version`: The platform version.
    ///
    /// # Returns
    ///
    /// * `Ok(Some(Vec<u8>))` with the stored bytes, `Ok(None)` when none were ever written.
    /// * `Err(Error)` when the method version is unknown or the auxiliary read fails.
    pub fn fetch_platform_state_recent_bytes(
        &self,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Option<Vec<u8>>, Error> {
        match platform_version
            .drive
            .methods
            .platform_state
            .fetch_platform_state_recent_bytes
        {
            0 => self.fetch_platform_state_recent_bytes_v0(transaction),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "fetch_platform_state_recent_bytes".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
