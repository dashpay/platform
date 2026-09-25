use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::batch::grovedb_op_batch::GroveDbOpBatchV0Methods;
use crate::util::batch::{DriveOperation, GroveDbOpBatch};
use dpp::block::block_info::BlockInfo;

use dpp::version::PlatformVersion;
use grovedb::batch::QualifiedGroveDbOp;
use grovedb::TransactionArg;
use itertools::Itertools;

impl Drive {
    /// Convert a batch of drive operations to a batch of grove database operations.
    ///
    /// This function takes drive operations and converts them into grove database operations by
    /// processing each operation in the `drive_batch_operations` vector, transforming them to low-level
    /// drive operations and finally, into grove database operations. The resulting operations are
    /// returned as a `GroveDbOpBatch`.
    ///
    /// The result is ONE plain batch, so operations on a TTL'd (ephemeral) subtree are refused:
    /// their bytes are priced separately and travel in their own batch, which only
    /// `apply_drive_operations` keeps apart. TTL preparation drains expired buckets directly,
    /// inside `transaction`, before the conversion, so a caller must pass the transaction it will
    /// apply the returned batch in (preparation refuses to run without one) and roll it back when
    /// the conversion fails.
    ///
    /// # Arguments
    ///
    /// * `drive_batch_operations` - A vector of high-level drive operations to be converted.
    /// * `block_info` - A reference to the block information associated with these operations.
    /// * `transaction` - A transaction argument to be used during processing.
    ///
    /// # Returns
    ///
    /// Returns a `Result` containing a `GroveDbOpBatch` with transformed grove database operations,
    /// or an error if any step in the conversion process fails.
    #[inline(always)]
    pub(super) fn convert_drive_operations_to_grove_operations_v0(
        &self,
        drive_batch_operations: Vec<DriveOperation>,
        block_info: &BlockInfo,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<GroveDbOpBatch, Error> {
        self.prepare_drive_operations_time_range_ttl(
            &drive_batch_operations,
            block_info,
            transaction,
            platform_version,
        )?;
        let ops = drive_batch_operations
            .into_iter()
            .map(|drive_op| {
                let inner_drive_operations = drive_op
                    .into_low_level_drive_operations_after_ttl_drain(
                        self,
                        &mut None,
                        block_info,
                        transaction,
                        platform_version,
                    )?;
                // Credits that repaid an identity's debt are owed to a fee pool, which a plain
                // batch cannot carry: dropping them would leave them in no balance the credit
                // sum counts. In place: only `add_to_identity_balance_operations` 1 (protocol
                // version 14) produces one, and at that version the only caller converting an
                // identity credit here, the epoch payout, gives its credits to
                // `apply_drive_operations` instead
                if LowLevelDriveOperation::holds_repaid_identity_debt(&inner_drive_operations) {
                    return Err(Error::Drive(DriveError::CorruptedCodeExecution(
                        "convert_drive_operations_to_grove_operations cannot carry credits that \
                         repaid an identity's debt; apply them through apply_drive_operations",
                    )));
                }
                if inner_drive_operations.iter().any(|operation| {
                    matches!(
                        operation,
                        LowLevelDriveOperation::EphemeralGroveOperation(_)
                    )
                }) {
                    return Err(Error::Drive(DriveError::NotSupported(
                        "convert_drive_operations_to_grove_operations returns one plain batch \
                         and cannot carry a TTL'd subtree's ephemeral operations, whose bytes \
                         are priced separately; apply them through apply_drive_operations",
                    )));
                }
                Ok(LowLevelDriveOperation::grovedb_operations_consume(
                    inner_drive_operations,
                ))
            })
            .flatten_ok()
            .collect::<Result<Vec<QualifiedGroveDbOp>, Error>>()?;
        Ok(GroveDbOpBatch::from_operations(ops))
    }
}
