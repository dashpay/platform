mod v0;
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::util::batch::{DriveOperation, GroveDbOpBatch};

use dpp::block::block_info::BlockInfo;

use dpp::version::PlatformVersion;

use grovedb::TransactionArg;

impl Drive {
    /// Convert a batch of drive operations to a batch of grove database operations.
    ///
    /// This function takes drive operations and converts them into grove database operations by
    /// processing each operation in the `drive_batch_operations` vector, transforming them to low-level
    /// drive operations and finally, into grove database operations. The resulting operations are
    /// returned as a `GroveDbOpBatch`.
    ///
    /// # Arguments
    ///
    /// * `drive_batch_operations` - A vector of high-level drive operations to be converted.
    /// * `block_info` - A reference to the block information associated with these operations.
    /// * `transaction` - A transaction argument to be used during processing.
    /// * `platform_version` - A `PlatformVersion` reference that dictates which version of the method to call.
    ///
    /// # Returns
    ///
    /// Returns a `Result` containing a `GroveDbOpBatch` with transformed grove database operations,
    /// or an error if any step in the conversion process fails.
    pub fn convert_drive_operations_to_grove_operations(
        &self,
        drive_batch_operations: Vec<DriveOperation>,
        block_info: &BlockInfo,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<GroveDbOpBatch, Error> {
        match platform_version
            .drive
            .methods
            .batch_operations
            .convert_drive_operations_to_grove_operations
        {
            0 => self.convert_drive_operations_to_grove_operations_v0(
                drive_batch_operations,
                block_info,
                transaction,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "convert_drive_operations_to_grove_operations".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
