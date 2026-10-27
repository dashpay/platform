use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::batch::grovedb_op_batch::GroveDbOpBatchV0Methods;
use crate::util::batch::{DriveOperation, GroveDbOpBatch};
use dpp::block::block_info::BlockInfo;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

impl Drive {
    /// Version 2 of [`Drive::convert_drive_operations_to_grove_operations`] (protocol version
    /// 15): version 1, with every operation built against the operations of the batch built
    /// before it, as `apply_drive_operations` 2 builds them
    /// ([`DriveOperation::append_low_level_drive_operations_after_ttl_drain`]). Two deletions
    /// that empty an index tree together remove it, and an insert into a tree an earlier deletion
    /// of the batch empties keeps the tree. Credits that repaid an identity's debt and ephemeral
    /// operations are refused as by version 0, for the same reasons. A batch writing one token
    /// balance, supply, pool total or escrow more than once is refused first, as
    /// `apply_drive_operations` refuses it
    /// ([`DriveOperation::refuse_repeated_token_balance_writes`]): each such write sets the value
    /// computed from state before the batch, so all but the last would be lost.
    #[inline(always)]
    pub(super) fn convert_drive_operations_to_grove_operations_v2(
        &self,
        drive_batch_operations: Vec<DriveOperation>,
        block_info: &BlockInfo,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<GroveDbOpBatch, Error> {
        DriveOperation::refuse_repeated_token_balance_writes(&drive_batch_operations)?;
        self.refuse_repeated_counter_moves(
            &drive_batch_operations,
            block_info,
            transaction,
            platform_version,
        )?;
        self.prepare_drive_operations_time_range_ttl(
            &drive_batch_operations,
            block_info,
            transaction,
            platform_version,
        )?;
        let mut batch_operations: Vec<LowLevelDriveOperation> = vec![];
        for drive_op in drive_batch_operations {
            drive_op.append_low_level_drive_operations_after_ttl_drain(
                self,
                &mut None,
                block_info,
                &mut batch_operations,
                transaction,
                platform_version,
            )?;
        }
        if LowLevelDriveOperation::holds_repaid_identity_debt(&batch_operations) {
            return Err(Error::Drive(DriveError::CorruptedCodeExecution(
                "convert_drive_operations_to_grove_operations cannot carry credits that repaid an \
                 identity's debt; apply them through apply_drive_operations",
            )));
        }
        if batch_operations.iter().any(|operation| {
            matches!(
                operation,
                LowLevelDriveOperation::EphemeralGroveOperation(..)
            )
        }) {
            return Err(Error::Drive(DriveError::NotSupported(
                "convert_drive_operations_to_grove_operations returns one plain batch and cannot \
                 carry ephemeral operations (a TTL'd subtree's, or a document's with a time to \
                 live), whose bytes are priced separately; apply them through \
                 apply_drive_operations",
            )));
        }
        Ok(GroveDbOpBatch::from_operations(
            LowLevelDriveOperation::grovedb_operations_consume(batch_operations),
        ))
    }
}
