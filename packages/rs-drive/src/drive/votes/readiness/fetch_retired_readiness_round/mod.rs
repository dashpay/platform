mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

impl Drive {
    /// Fetches the first retired round awaiting cleanup: its round id and contract id.
    ///
    /// # Parameters
    ///
    /// * `transaction` - The current transaction.
    /// * `drive_operations` - The accumulator the read cost is appended to.
    /// * `platform_version` - The platform version to use.
    ///
    /// # Returns
    ///
    /// * `Ok(None)` when the queue is empty.
    /// * `Err(DriveError::VersionNotActive)` on a platform version without readiness.
    pub fn fetch_retired_readiness_round_operations(
        &self,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<Option<([u8; 32], [u8; 32])>, Error> {
        match platform_version
            .drive
            .methods
            .vote
            .readiness
            .fetch_retired_round
        {
            Some(0) => self.fetch_retired_readiness_round_operations_v0(
                transaction,
                drive_operations,
                platform_version,
            ),
            None => Err(Error::Drive(DriveError::VersionNotActive {
                method: "fetch_retired_readiness_round_operations".to_string(),
                known_versions: vec![0],
            })),
            Some(version) => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "fetch_retired_readiness_round_operations".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }

    /// Fetches the first retired round awaiting cleanup.
    ///
    /// # Parameters
    ///
    /// * `transaction` - The current transaction.
    /// * `platform_version` - The platform version to use.
    ///
    /// # Returns
    ///
    /// * `Ok(None)` when the queue is empty; otherwise the round id and its contract id.
    pub fn fetch_retired_readiness_round(
        &self,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Option<([u8; 32], [u8; 32])>, Error> {
        self.fetch_retired_readiness_round_operations(transaction, &mut vec![], platform_version)
    }
}
