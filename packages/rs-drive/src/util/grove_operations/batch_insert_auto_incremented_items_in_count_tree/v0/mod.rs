use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::grove_operations::DirectQueryType;
use dpp::version::drive_versions::DriveVersion;
use grovedb::{Element, TransactionArg};

impl Drive {
    /// Inserts multiple items into a count tree with auto-incremented u64 big-endian keys.
    ///
    /// Reads the current count from the count tree, then generates sequential insert
    /// operations starting from that count.
    ///
    /// # Parameters
    /// * `parent_path` - Path to the parent tree containing the count tree
    /// * `count_tree_key` - Key of the count tree under the parent path
    /// * `items` - Items (Elements) to insert with auto-incremented keys
    /// * `transaction` - GroveDB transaction
    /// * `drive_operations` - Accumulator for low-level drive operations
    /// * `drive_version` - Drive version for method dispatch
    pub(crate) fn batch_insert_auto_incremented_items_in_count_tree_v0(
        &self,
        parent_path: Vec<Vec<u8>>,
        count_tree_key: &[u8],
        items: Vec<Element>,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        drive_version: &DriveVersion,
    ) -> Result<(), Error> {
        if items.is_empty() {
            return Ok(());
        }

        // Read the current count from the count tree element
        let current_count = self
            .grove_get_raw_optional(
                parent_path.as_slice().into(),
                count_tree_key,
                DirectQueryType::StatefulDirectQuery,
                transaction,
                drive_operations,
                drive_version,
            )?
            .map(|element| element.count_value_or_default())
            .unwrap_or(0);

        // Build the full path to the count tree (parent_path + count_tree_key)
        let mut insert_path = parent_path;
        insert_path.push(count_tree_key.to_vec());

        // Insert each item with a sequential key
        for (i, element) in items.into_iter().enumerate() {
            let index = current_count + i as u64;
            drive_operations.push(LowLevelDriveOperation::insert_for_known_path_key_element(
                insert_path.clone(),
                index.to_be_bytes().to_vec(),
                element,
            ));
        }

        Ok(())
    }
}
