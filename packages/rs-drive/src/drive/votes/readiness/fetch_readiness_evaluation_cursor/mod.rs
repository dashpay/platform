mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

impl Drive {
    /// Fetches the block event's fairness cursor: the last contract id it visited.
    ///
    /// # Parameters
    ///
    /// * `transaction` - The current transaction.
    /// * `drive_operations` - The accumulator the read cost is appended to.
    /// * `platform_version` - The platform version to use.
    ///
    /// # Returns
    ///
    /// * `Ok(None)` before the first sweep.
    /// * `Err(DriveError::VersionNotActive)` on a platform version without readiness.
    pub fn fetch_readiness_evaluation_cursor_operations(
        &self,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<Option<[u8; 32]>, Error> {
        match platform_version
            .drive
            .methods
            .vote
            .readiness
            .fetch_evaluation_cursor
        {
            Some(0) => self.fetch_readiness_evaluation_cursor_operations_v0(
                transaction,
                drive_operations,
                platform_version,
            ),
            None => Err(Error::Drive(DriveError::VersionNotActive {
                method: "fetch_readiness_evaluation_cursor_operations".to_string(),
                known_versions: vec![0],
            })),
            Some(version) => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "fetch_readiness_evaluation_cursor_operations".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }

    /// Fetches the block event's fairness cursor.
    ///
    /// # Parameters
    ///
    /// * `transaction` - The current transaction.
    /// * `platform_version` - The platform version to use.
    ///
    /// # Returns
    ///
    /// * `Ok(None)` before the first sweep.
    pub fn fetch_readiness_evaluation_cursor(
        &self,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Option<[u8; 32]>, Error> {
        self.fetch_readiness_evaluation_cursor_operations(transaction, &mut vec![], platform_version)
    }
}
