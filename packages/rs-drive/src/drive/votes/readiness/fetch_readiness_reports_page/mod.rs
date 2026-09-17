mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::version::PlatformVersion;
use dpp::voting::readiness::report_record::ReadinessReportRecord;
use grovedb::TransactionArg;

impl Drive {
    /// Fetches a page of a round's accepted reports in pro tx hash order, starting after
    /// `after` when given.
    ///
    /// # Parameters
    ///
    /// * `contract_id` - The contract.
    /// * `round_id` - The round.
    /// * `after` - The last key of the previous page; `None` for the first page.
    /// * `limit` - The maximum number of reports to return.
    /// * `transaction` - The current transaction.
    /// * `drive_operations` - The accumulator the read cost is appended to.
    /// * `platform_version` - The platform version to use.
    ///
    /// # Returns
    ///
    /// * The reports of the page, in key order.
    /// * `Err(DriveError::VersionNotActive)` on a platform version without readiness.
    #[allow(clippy::too_many_arguments)]
    pub fn fetch_readiness_reports_page_operations(
        &self,
        contract_id: [u8; 32],
        round_id: [u8; 32],
        after: Option<[u8; 32]>,
        limit: u16,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<([u8; 32], ReadinessReportRecord)>, Error> {
        match platform_version
            .drive
            .methods
            .vote
            .readiness
            .fetch_reports_page
        {
            Some(0) => self.fetch_readiness_reports_page_operations_v0(
                contract_id,
                round_id,
                after,
                limit,
                transaction,
                drive_operations,
                platform_version,
            ),
            None => Err(Error::Drive(DriveError::VersionNotActive {
                method: "fetch_readiness_reports_page_operations".to_string(),
                known_versions: vec![0],
            })),
            Some(version) => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "fetch_readiness_reports_page_operations".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }

    /// Fetches a page of a round's accepted reports in pro tx hash order.
    ///
    /// # Parameters
    ///
    /// * `contract_id` - The contract.
    /// * `round_id` - The round.
    /// * `after` - The last key of the previous page; `None` for the first page.
    /// * `limit` - The maximum number of reports to return.
    /// * `transaction` - The current transaction.
    /// * `platform_version` - The platform version to use.
    ///
    /// # Returns
    ///
    /// * The reports of the page, in key order.
    pub fn fetch_readiness_reports_page(
        &self,
        contract_id: [u8; 32],
        round_id: [u8; 32],
        after: Option<[u8; 32]>,
        limit: u16,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<([u8; 32], ReadinessReportRecord)>, Error> {
        self.fetch_readiness_reports_page_operations(
            contract_id,
            round_id,
            after,
            limit,
            transaction,
            &mut vec![],
            platform_version,
        )
    }
}
