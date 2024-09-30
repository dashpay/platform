mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::grove_operations::BatchMoveApplyType;

use dpp::version::drive_versions::DriveVersion;

use grovedb::{PathQuery, TransactionArg};

impl Drive {
    /// Pushes multiple "delete element" and "insert element operations for items in a given path based on a `PathQuery` to `drive_operations`.
    ///
    /// # Parameters
    /// * `path_query`: The path query specifying the items to delete within the path.
    /// * `error_if_intermediate_path_tree_not_present`: Tells the function to either error or do nothing if an intermediate tree is not present.
    /// * `apply_type`: The apply type for the move operations.
    /// * `transaction`: The transaction argument.
    /// * `drive_operations`: The vector containing low-level drive operations.
    /// * `drive_version`: The drive version to select the correct function version to run.
    ///
    /// # Returns
    /// * `Ok(())` if the operation was successful.
    /// * `Err(DriveError::UnknownVersionMismatch)` if the drive version does not match known versions.
    pub fn batch_move_items_in_path_query(
        &self,
        path_query: &PathQuery,
        new_path: Vec<Vec<u8>>,
        error_if_intermediate_path_tree_not_present: bool,
        apply_type: BatchMoveApplyType,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        drive_version: &DriveVersion,
    ) -> Result<(), Error> {
        match drive_version
            .grove_methods
            .batch
            .batch_move_items_in_path_query
        {
            0 => self.batch_move_items_in_path_query_v0(
                path_query,
                new_path,
                error_if_intermediate_path_tree_not_present,
                apply_type,
                transaction,
                drive_operations,
                drive_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "batch_move_items_in_path_query".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
