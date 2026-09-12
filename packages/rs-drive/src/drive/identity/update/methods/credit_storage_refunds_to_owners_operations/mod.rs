mod v0;

use crate::drive::identity::update::storage_refund_credit_outcome::StorageRefundCreditOutcome;
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::fee::fee_result::refunds::FeeRefunds;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

impl Drive {
    /// Credits storage refunds to their recorded owners and reports what could
    /// not be routed.
    ///
    /// For every owner in `fee_refunds` except `skip_owner`, the owner's
    /// per-epoch credits are summed with checked arithmetic. An owner with a
    /// balance element is credited through `add_to_identity_balance_operations`;
    /// no key, signature or permission is consulted, so a frozen but existing
    /// owner receives its bookkeeping refund. An owner without a balance
    /// element (the native proxy for a wiped owner) is not credited and its
    /// amount is reported as `routed_to_processing_pool`, for the caller to
    /// settle into the current epoch's processing pool with one pool write.
    /// The primitive records no pending refunds; the caller does, so the block
    /// keeps a single pending-refund and pool write per batch.
    ///
    /// # Parameters
    ///
    /// * `fee_refunds` - The refunds to settle, per owner and storage epoch.
    /// * `skip_owner` - An owner whose refund the caller settles itself (a
    ///   state transition payer, whose refund folds into its balance change);
    ///   `None` on lifecycle paths.
    /// * `transaction` - The current transaction.
    /// * `drive_operations` - The accumulator the balance operations are
    ///   appended to; the caller applies them.
    /// * `platform_version` - The platform version.
    ///
    /// # Returns
    ///
    /// * `Ok(StorageRefundCreditOutcome)` - The owners credited and the amount
    ///   routed to the processing pool.
    /// * `Err(Error)` - On overflow, a corrupted balance element, or when the
    ///   method is not active for the platform version.
    pub fn credit_storage_refunds_to_owners_operations(
        &self,
        fee_refunds: &FeeRefunds,
        skip_owner: Option<[u8; 32]>,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<StorageRefundCreditOutcome, Error> {
        match platform_version
            .drive
            .methods
            .identity
            .update
            .credit_storage_refunds_to_owners
        {
            Some(0) => self.credit_storage_refunds_to_owners_operations_v0(
                fee_refunds,
                skip_owner,
                transaction,
                drive_operations,
                platform_version,
            ),
            Some(version) => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "credit_storage_refunds_to_owners_operations".to_string(),
                known_versions: vec![0],
                received: version,
            })),
            None => Err(Error::Drive(DriveError::VersionNotActive {
                method: "credit_storage_refunds_to_owners_operations".to_string(),
                known_versions: vec![0],
            })),
        }
    }
}
