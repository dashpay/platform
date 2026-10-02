mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

impl Drive {
    /// Fetches a page of the contracts holding a readiness tree, in contract id order,
    /// starting after `after` when given. The block event walks rounds with it, wrapping to
    /// the start when a page comes back short.
    ///
    /// # Parameters
    ///
    /// * `after` - The last contract id of the previous page; `None` for the first page.
    /// * `limit` - The maximum number of contract ids to return.
    /// * `transaction` - The current transaction.
    /// * `drive_operations` - The accumulator the read cost is appended to.
    /// * `platform_version` - The platform version to use.
    ///
    /// # Returns
    ///
    /// * The contract ids of the page, in key order.
    /// * `Err(DriveError::VersionNotActive)` on a platform version without readiness.
    pub fn fetch_readiness_rounds_page_operations(
        &self,
        after: Option<[u8; 32]>,
        limit: u16,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<[u8; 32]>, Error> {
        match platform_version
            .drive
            .methods
            .vote
            .readiness
            .fetch_rounds_page
        {
            Some(0) => self.fetch_readiness_rounds_page_operations_v0(
                after,
                limit,
                transaction,
                drive_operations,
                platform_version,
            ),
            None => Err(Error::Drive(DriveError::VersionNotActive {
                method: "fetch_readiness_rounds_page_operations".to_string(),
                known_versions: vec![0],
            })),
            Some(version) => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "fetch_readiness_rounds_page_operations".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }

    /// Fetches a page of the contracts holding a readiness tree, in contract id order.
    ///
    /// # Parameters
    ///
    /// * `after` - The last contract id of the previous page; `None` for the first page.
    /// * `limit` - The maximum number of contract ids to return.
    /// * `transaction` - The current transaction.
    /// * `platform_version` - The platform version to use.
    ///
    /// # Returns
    ///
    /// * The contract ids of the page, in key order.
    pub fn fetch_readiness_rounds_page(
        &self,
        after: Option<[u8; 32]>,
        limit: u16,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<[u8; 32]>, Error> {
        self.fetch_readiness_rounds_page_operations(
            after,
            limit,
            transaction,
            &mut vec![],
            platform_version,
        )
    }
}
