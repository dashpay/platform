mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::version::PlatformVersion;
use dpp::voting::readiness::scan_cursor::ReadinessScanCursor;
use grovedb::TransactionArg;

impl Drive {
    /// Fetches the persisted position of a round's paged eligibility walk.
    ///
    /// # Parameters
    ///
    /// * `contract_id` - The contract.
    /// * `round_id` - The round.
    /// * `transaction` - The current transaction.
    /// * `drive_operations` - The accumulator the read cost is appended to.
    /// * `platform_version` - The platform version to use.
    ///
    /// # Returns
    ///
    /// * `Ok(None)` when no walk is open.
    /// * `Err(DriveError::VersionNotActive)` on a platform version without readiness.
    pub fn fetch_readiness_scan_cursor_operations(
        &self,
        contract_id: [u8; 32],
        round_id: [u8; 32],
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<Option<ReadinessScanCursor>, Error> {
        match platform_version
            .drive
            .methods
            .vote
            .readiness
            .fetch_scan_cursor
        {
            Some(0) => self.fetch_readiness_scan_cursor_operations_v0(
                contract_id,
                round_id,
                transaction,
                drive_operations,
                platform_version,
            ),
            None => Err(Error::Drive(DriveError::VersionNotActive {
                method: "fetch_readiness_scan_cursor_operations".to_string(),
                known_versions: vec![0],
            })),
            Some(version) => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "fetch_readiness_scan_cursor_operations".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }

    /// Fetches the persisted position of a round's paged eligibility walk.
    ///
    /// # Parameters
    ///
    /// * `contract_id` - The contract.
    /// * `round_id` - The round.
    /// * `transaction` - The current transaction.
    /// * `platform_version` - The platform version to use.
    ///
    /// # Returns
    ///
    /// * `Ok(None)` when no walk is open.
    pub fn fetch_readiness_scan_cursor(
        &self,
        contract_id: [u8; 32],
        round_id: [u8; 32],
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Option<ReadinessScanCursor>, Error> {
        self.fetch_readiness_scan_cursor_operations(
            contract_id,
            round_id,
            transaction,
            &mut vec![],
            platform_version,
        )
    }
}
