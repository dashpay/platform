mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::fee::Credits;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

impl Drive {
    /// Fetches a readiness fund balance.
    ///
    /// # Parameters
    ///
    /// * `fund_id` - The fund, derived from the round id.
    /// * `transaction` - The current transaction.
    /// * `platform_version` - The platform version to use.
    ///
    /// # Returns
    ///
    /// * `Ok(None)` when no such fund exists.
    /// * `Err(DriveError::VersionNotActive)` on a platform version without readiness.
    pub fn fetch_readiness_fund(
        &self,
        fund_id: [u8; 32],
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Option<Credits>, Error> {
        let mut drive_operations = vec![];
        self.fetch_readiness_fund_operations(
            fund_id,
            true,
            transaction,
            &mut drive_operations,
            platform_version,
        )
    }

    /// Fetches a readiness fund balance, accumulating the cost of the read. With
    /// `apply = false` the read is priced without touching state and returns `Some(0)`.
    ///
    /// # Parameters
    ///
    /// * `fund_id` - The fund, derived from the round id.
    /// * `apply` - Whether to read state (`true`) or only estimate the cost (`false`).
    /// * `transaction` - The current transaction.
    /// * `drive_operations` - The accumulator the read cost is appended to.
    /// * `platform_version` - The platform version to use.
    ///
    /// # Returns
    ///
    /// * `Ok(None)` when no such fund exists.
    /// * `Err(DriveError::VersionNotActive)` on a platform version without readiness.
    pub fn fetch_readiness_fund_operations(
        &self,
        fund_id: [u8; 32],
        apply: bool,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<Option<Credits>, Error> {
        match platform_version.drive.methods.vote.readiness.fetch_fund {
            Some(0) => self.fetch_readiness_fund_operations_v0(
                fund_id,
                apply,
                transaction,
                drive_operations,
                platform_version,
            ),
            None => Err(Error::Drive(DriveError::VersionNotActive {
                method: "fetch_readiness_fund_operations".to_string(),
                known_versions: vec![0],
            })),
            Some(version) => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "fetch_readiness_fund_operations".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
