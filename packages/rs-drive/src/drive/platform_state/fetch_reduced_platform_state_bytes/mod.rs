mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

impl Drive {
    /// Fetch the reduced platform state from the replicated grovedb state (Misc tree).
    ///
    /// Returns `Ok(None)` when the key is absent (for example before the protocol
    /// version that introduced the reduced state activated).
    pub fn fetch_reduced_platform_state_bytes(
        &self,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Option<Vec<u8>>, Error> {
        match platform_version
            .drive
            .methods
            .platform_state
            .fetch_reduced_platform_state_bytes
        {
            0 => self.fetch_reduced_platform_state_bytes_v0(transaction, platform_version),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "fetch_reduced_platform_state_bytes".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
