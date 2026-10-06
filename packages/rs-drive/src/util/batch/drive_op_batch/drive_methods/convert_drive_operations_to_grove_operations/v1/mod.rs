use crate::drive::Drive;
use crate::error::Error;
use crate::util::batch::{DriveOperation, GroveDbOpBatch};
use dpp::block::block_info::BlockInfo;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

impl Drive {
    /// Version 1 of [`Drive::convert_drive_operations_to_grove_operations`]:
    /// version 0, after refusing a batch that moves one document type's
    /// `summableOffCountIndex` counters for more than one document
    /// ([`Drive::refuse_repeated_counter_moves`]).
    #[inline(always)]
    pub(super) fn convert_drive_operations_to_grove_operations_v1(
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
        self.convert_drive_operations_to_grove_operations_v0(
            drive_batch_operations,
            block_info,
            transaction,
            platform_version,
        )
    }
}
