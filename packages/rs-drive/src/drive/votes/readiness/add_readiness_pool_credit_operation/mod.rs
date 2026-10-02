mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::block::block_info::BlockInfo;
use dpp::fee::Credits;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

impl Drive {
    /// The operation that adds `amount` to the block epoch's processing fee pool, reading
    /// the current pool value in the same transaction. In estimation mode the read is
    /// priced without state and the write is priced as an insert of the widest sum item
    /// (the estimator bills a plain element by its serialized size, the applied write
    /// bills the fixed sum item size); the caller describes the pool layers first.
    ///
    /// Readiness credits reach the pool this way (the cleanup reserve at retirement, the
    /// membership lookup fees of the block event) so that every credit leaving a readiness
    /// fund lands in a bucket conservation counts. The write is an absolute rewrite of the
    /// pool item, so one such operation per applied batch.
    ///
    /// # Parameters
    ///
    /// * `block_info` - The block whose epoch receives the credits.
    /// * `amount` - The credits to add.
    /// * `apply` - Whether to read state (`true`) or only estimate the cost (`false`).
    /// * `transaction` - The current transaction.
    /// * `drive_operations` - The accumulator the read cost is appended to.
    /// * `platform_version` - The platform version to use.
    ///
    /// # Returns
    ///
    /// * The operation that writes the pool item.
    /// * `Err(DriveError::VersionNotActive)` on a platform version without readiness.
    pub fn add_readiness_pool_credit_operation(
        &self,
        block_info: &BlockInfo,
        amount: Credits,
        apply: bool,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<LowLevelDriveOperation, Error> {
        match platform_version.drive.methods.vote.readiness.credit_pool {
            Some(0) => self.add_readiness_pool_credit_operation_v0(
                block_info,
                amount,
                apply,
                transaction,
                drive_operations,
                platform_version,
            ),
            None => Err(Error::Drive(DriveError::VersionNotActive {
                method: "add_readiness_pool_credit_operation".to_string(),
                known_versions: vec![0],
            })),
            Some(version) => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "add_readiness_pool_credit_operation".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
