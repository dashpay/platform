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
    /// Converts a plain batch while keeping sibling operations available to history deletes.
    /// Each type's lazy lifecycle container is therefore inserted only once in the batch.
    #[inline(always)]
    pub(super) fn convert_drive_operations_to_grove_operations_v2(
        &self,
        drive_batch_operations: Vec<DriveOperation>,
        block_info: &BlockInfo,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<GroveDbOpBatch, Error> {
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
        let mut operations = vec![];
        for drive_op in drive_batch_operations {
            let mut converted = drive_op.into_low_level_drive_operations_after_ttl_drain(
                self,
                &mut None,
                Some(&mut operations),
                block_info,
                transaction,
                platform_version,
            )?;
            if LowLevelDriveOperation::holds_repaid_identity_debt(&converted) {
                return Err(Error::Drive(DriveError::CorruptedCodeExecution(
                    "convert_drive_operations_to_grove_operations cannot carry credits that \
                     repaid an identity's debt; apply them through apply_drive_operations",
                )));
            }
            if converted.iter().any(|operation| {
                matches!(
                    operation,
                    LowLevelDriveOperation::EphemeralGroveOperation(..)
                )
            }) {
                return Err(Error::Drive(DriveError::NotSupported(
                    "convert_drive_operations_to_grove_operations returns one plain batch \
                     and cannot carry ephemeral operations (a TTL'd subtree's, or a \
                     document's with a time to live), whose bytes are priced separately; \
                     apply them through apply_drive_operations",
                )));
            }
            operations.append(&mut converted);
        }
        Ok(GroveDbOpBatch::from_operations(
            LowLevelDriveOperation::grovedb_operations_consume(operations),
        ))
    }
}
