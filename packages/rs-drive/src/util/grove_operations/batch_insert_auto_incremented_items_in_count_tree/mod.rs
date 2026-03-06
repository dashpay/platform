mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::version::drive_versions::DriveVersion;
use grovedb::{Element, TransactionArg};

impl Drive {
    /// Inserts multiple items into a count tree with auto-incremented u64 big-endian keys.
    ///
    /// Reads the current count from the count tree, then generates sequential insert
    /// operations starting from that count.
    pub fn batch_insert_auto_incremented_items_in_count_tree(
        &self,
        parent_path: Vec<Vec<u8>>,
        count_tree_key: &[u8],
        items: Vec<Element>,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        drive_version: &DriveVersion,
    ) -> Result<(), Error> {
        match drive_version
            .grove_methods
            .batch
            .batch_insert_auto_incremented_items_in_count_tree
        {
            0 => self.batch_insert_auto_incremented_items_in_count_tree_v0(
                parent_path,
                count_tree_key,
                items,
                transaction,
                drive_operations,
                drive_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "batch_insert_auto_incremented_items_in_count_tree".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
