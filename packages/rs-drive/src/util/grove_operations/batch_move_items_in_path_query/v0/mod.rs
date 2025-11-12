use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::fees::op::LowLevelDriveOperation::GroveOperation;
use crate::util::grove_operations::{push_drive_operation_result, BatchMoveApplyType};
use grovedb::batch::key_info::KeyInfo;
use grovedb::batch::{KeyInfoPath, QualifiedGroveDbOp};
use grovedb::operations::delete::DeleteOptions;
use grovedb::query_result_type::QueryResultType;
use grovedb::{GroveDb, PathQuery, TransactionArg};
use grovedb_epoch_based_storage_flags::StorageFlags;
use grovedb_storage::rocksdb_storage::RocksDbStorage;
use platform_version::version::drive_versions::DriveVersion;

impl Drive {
    /// Version 0 implementation of the "delete multiple elements" and "insert multiple elements" operation based on a `PathQuery`.
    /// Deletes items in the specified path that match the given query.
    /// Then inserts those items at the given path.
    ///
    /// # Parameters
    /// * `path_query`: The path query specifying the items to delete within the path.
    /// * `error_if_intermediate_path_tree_not_present`: Tells the function to either error or do nothing if an intermediate tree is not present.
    /// * `apply_type`: The apply type for the move operations.
    /// * `alter_flags_to_new_flags`: Should we alter the flags to new storage flags.
    /// * `transaction`: The transaction argument.
    /// * `drive_operations`: The vector containing low-level drive operations.
    /// * `drive_version`: The drive version to select the correct function version to run.
    ///
    /// # Returns
    /// * `Ok(())` if the operation was successful.
    /// * `Err(DriveError::CorruptedCodeExecution)` if the operation is not supported.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn batch_move_items_in_path_query_v0(
        &self,
        path_query: &PathQuery,
        new_path: Vec<Vec<u8>>,
        error_if_intermediate_path_tree_not_present: bool,
        apply_type: BatchMoveApplyType,
        alter_flags_to_new_flags: Option<Option<StorageFlags>>,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        drive_version: &DriveVersion,
    ) -> Result<(), Error> {
        if path_query.query.limit.is_none() {
            return Err(Error::Drive(DriveError::NotSupported(
                "Limits are required for path_query",
            )));
        }
        let query_result = if path_query
            .query
            .query
            .items
            .iter()
            .all(|query_item| query_item.is_key())
        {
            // Fetch the elements that match the path query
            let query_result = self.grove_get_raw_path_query_with_optional(
                path_query,
                error_if_intermediate_path_tree_not_present,
                transaction,
                drive_operations,
                drive_version,
            )?;

            query_result
                .into_iter()
                .filter_map(|(path, key, maybe_element)| {
                    maybe_element.map(|element| (path, key, element))
                })
                .collect()
        } else {
            self.grove_get_raw_path_query(
                path_query,
                transaction,
                QueryResultType::QueryPathKeyElementTrioResultType,
                drive_operations,
                drive_version,
            )?
            .0
            .to_path_key_elements()
        };

        // Iterate over each element and add a delete operation for it
        for (path, key, mut element) in query_result {
            let current_batch_operations =
                LowLevelDriveOperation::grovedb_operations_batch(drive_operations);
            let options = DeleteOptions {
                allow_deleting_non_empty_trees: false,
                deleting_non_empty_trees_returns_error: true,
                base_root_storage_is_free: true,
                validate_tree_at_path_exists: false,
            };
            let delete_operation = match apply_type {
                BatchMoveApplyType::StatelessBatchMove {
                    in_tree_type,
                    estimated_key_size,
                    estimated_value_size,
                    ..
                } => GroveDb::average_case_delete_operation_for_delete::<RocksDbStorage>(
                    &KeyInfoPath::from_known_owned_path(path.to_vec()),
                    &KeyInfo::KnownKey(key.to_vec()),
                    in_tree_type,
                    false,
                    true,
                    0,
                    (estimated_key_size, estimated_value_size),
                    &drive_version.grove_version,
                )
                .map(|r| r.map(Some)),
                BatchMoveApplyType::StatefulBatchMove {
                    is_known_to_be_subtree_with_sum,
                } => self.grove.delete_operation_for_delete_internal(
                    path.as_slice().into(),
                    key.as_slice(),
                    &options,
                    is_known_to_be_subtree_with_sum,
                    &current_batch_operations.operations,
                    transaction,
                    &drive_version.grove_version,
                ),
            };

            if let Some(delete_operation) =
                push_drive_operation_result(delete_operation, drive_operations)?
            {
                if let Some(altered_flags) = alter_flags_to_new_flags.as_ref() {
                    element.set_flags(StorageFlags::map_to_some_element_flags(
                        altered_flags.as_ref(),
                    ))
                }
                // Add the delete operation to the batch of drive operations
                drive_operations.push(GroveOperation(delete_operation));
                // Adds the insert operation to the batch of drive operations
                drive_operations.push(GroveOperation(QualifiedGroveDbOp::insert_or_replace_op(
                    new_path.clone(),
                    key,
                    element,
                )));
            }
        }

        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use crate::util::grove_operations::QueryType;
    use crate::{
        error::Error, util::grove_operations::BatchMoveApplyType,
        util::test_helpers::setup::setup_drive,
    };
    use assert_matches::assert_matches;
    use grovedb::{Element, MaybeTree, PathQuery, Query, SizedQuery};
    use grovedb_path::SubtreePath;
    use platform_version::version::PlatformVersion;

    #[test]
    fn test_batch_move_items_in_path_query_success() {
        // Set up a test drive instance and transaction
        let drive = setup_drive(None);
        let platform_version = PlatformVersion::latest();
        let transaction = drive.grove.start_transaction();

        // Insert elements that will be moved
        let path = vec![b"root".to_vec()];
        let new_path = vec![b"new_root".to_vec()];
        let key1 = b"key1".to_vec();
        let key2 = b"key2".to_vec();
        let element1 = Element::new_item(b"value1".to_vec());
        let element2 = Element::new_item(b"value2".to_vec());

        // Insert the root tree and new root tree
        drive
            .grove_insert_empty_tree(
                SubtreePath::empty(),
                b"root",
                Some(&transaction),
                None,
                &mut vec![],
                &platform_version.drive,
            )
            .expect("expected to insert root tree");

        drive
            .grove_insert_empty_tree(
                SubtreePath::empty(),
                b"new_root",
                Some(&transaction),
                None,
                &mut vec![],
                &platform_version.drive,
            )
            .expect("expected to insert new root tree");

        drive
            .grove
            .insert(
                path.as_slice(),
                &key1,
                element1.clone(),
                None,
                Some(&transaction),
                &platform_version.drive.grove_version,
            )
            .unwrap()
            .expect("expected to insert element1");

        drive
            .grove
            .insert(
                path.as_slice(),
                &key2,
                element2.clone(),
                None,
                Some(&transaction),
                &platform_version.drive.grove_version,
            )
            .unwrap()
            .expect("expected to insert element2");

        // Create a path query that matches the inserted elements
        let mut query = Query::new();
        query.insert_key(key1.clone());
        query.insert_key(key2.clone());
        let path_query = PathQuery::new(path.clone(), SizedQuery::new(query, Some(100), None));

        // Set up the apply type and drive operations vector
        let apply_type = BatchMoveApplyType::StatefulBatchMove {
            is_known_to_be_subtree_with_sum: Some(MaybeTree::NotTree),
        };
        let mut drive_operations = Vec::new();

        // Call the function
        drive
            .batch_move_items_in_path_query_v0(
                &path_query,
                new_path.clone(),
                true,
                apply_type,
                None,
                Some(&transaction),
                &mut drive_operations,
                &platform_version.drive,
            )
            .expect("expected to move items");

        // Apply batch operations
        drive
            .apply_batch_low_level_drive_operations(
                None,
                Some(&transaction),
                drive_operations,
                &mut vec![],
                &platform_version.drive,
            )
            .expect("expected to apply operations");

        // Commit the transaction
        drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("expected to commit transaction");

        // Verify that keys have been moved to the new path
        let get_result_1 = drive.grove_get(
            new_path.as_slice().into(),
            &key1,
            QueryType::StatefulQuery,
            None,
            &mut vec![],
            &platform_version.drive,
        );
        assert_matches!(get_result_1, Ok(Some(Element::Item(value, _))) if value == b"value1".to_vec());

        let get_result_2 = drive.grove_get(
            new_path.as_slice().into(),
            &key2,
            QueryType::StatefulQuery,
            None,
            &mut vec![],
            &platform_version.drive,
        );
        assert_matches!(get_result_2, Ok(Some(Element::Item(value, _))) if value == b"value2".to_vec());

        // Verify that keys are no longer in the original path
        let get_result_1_old = drive.grove_get(
            path.as_slice().into(),
            &key1,
            QueryType::StatefulQuery,
            None,
            &mut vec![],
            &platform_version.drive,
        );
        assert_matches!(
            get_result_1_old,
            Err(Error::GroveDB(e)) if matches!(e.as_ref(), grovedb::Error::PathKeyNotFound(_))
        );

        let get_result_2_old = drive.grove_get(
            path.as_slice().into(),
            &key2,
            QueryType::StatefulQuery,
            None,
            &mut vec![],
            &platform_version.drive,
        );
        assert_matches!(
            get_result_2_old,
            Err(Error::GroveDB(e)) if matches!(e.as_ref(), grovedb::Error::PathKeyNotFound(_))
        );
    }

    #[test]
    fn test_batch_move_items_in_path_query_no_elements() {
        // Set up a test drive instance and transaction
        let drive = setup_drive(None);
        let platform_version = PlatformVersion::latest();
        let transaction = drive.grove.start_transaction();

        // Create the root tree to allow querying it
        drive
            .grove_insert_empty_tree(
                SubtreePath::empty(),
                b"root",
                Some(&transaction),
                None,
                &mut vec![],
                &platform_version.drive,
            )
            .expect("expected to insert root tree");

        // Create a path query that does not match any elements
        let path = vec![b"root".to_vec()];
        let new_path = vec![b"new_root".to_vec()];
        let mut query = Query::new();
        query.insert_key(b"non_existent_key".to_vec());
        let path_query = PathQuery::new(path.clone(), SizedQuery::new(query, Some(1), None));

        // Set up the apply type and drive operations vector
        let apply_type = BatchMoveApplyType::StatefulBatchMove {
            is_known_to_be_subtree_with_sum: Some(MaybeTree::NotTree),
        };
        let mut drive_operations = Vec::new();

        // Call the function
        let result = drive.batch_move_items_in_path_query_v0(
            &path_query,
            new_path.clone(),
            true,
            apply_type,
            None,
            Some(&transaction),
            &mut drive_operations,
            &platform_version.drive,
        );

        assert!(result.is_ok());
    }

    #[test]
    fn test_batch_move_items_in_path_query_range_query() {
        // Set up a test drive instance and transaction
        let drive = setup_drive(None);
        let platform_version = PlatformVersion::latest();
        let transaction = drive.grove.start_transaction();

        // Insert the root tree
        drive
            .grove_insert_empty_tree(
                SubtreePath::empty(),
                b"root",
                Some(&transaction),
                None,
                &mut vec![],
                &platform_version.drive,
            )
            .expect("expected to insert root tree");

        // Insert three elements with keys 1, 2, and 3
        let path = vec![b"root".to_vec()];
        let new_path = vec![b"new_root".to_vec()];
        let key1 = b"1".to_vec();
        let key2 = b"2".to_vec();
        let key3 = b"3".to_vec();
        let element = Element::new_item(b"value".to_vec());

        // Insert the new root tree
        drive
            .grove_insert_empty_tree(
                SubtreePath::empty(),
                b"new_root",
                Some(&transaction),
                None,
                &mut vec![],
                &platform_version.drive,
            )
            .expect("expected to insert new root tree");

        drive
            .grove
            .insert(
                path.as_slice(),
                &key1,
                element.clone(),
                None,
                Some(&transaction),
                &platform_version.drive.grove_version,
            )
            .unwrap()
            .expect("expected insert for key 1");

        drive
            .grove
            .insert(
                path.as_slice(),
                &key2,
                element.clone(),
                None,
                Some(&transaction),
                &platform_version.drive.grove_version,
            )
            .unwrap()
            .expect("expected insert for key 2");

        drive
            .grove
            .insert(
                path.as_slice(),
                &key3,
                element.clone(),
                None,
                Some(&transaction),
                &platform_version.drive.grove_version,
            )
            .unwrap()
            .expect("expected insert for key 3");

        // Create a range path query that matches keys less than 3
        let mut query = Query::new();
        query.insert_range_to(..b"3".to_vec());
        let path_query = PathQuery::new(path.clone(), SizedQuery::new(query, Some(100), None));

        // Set up the apply type and drive operations vector
        let apply_type = BatchMoveApplyType::StatefulBatchMove {
            is_known_to_be_subtree_with_sum: Some(MaybeTree::NotTree),
        };
        let mut drive_operations = Vec::new();

        // Call the function
        drive
            .batch_move_items_in_path_query_v0(
                &path_query,
                new_path.clone(),
                true,
                apply_type,
                None,
                Some(&transaction),
                &mut drive_operations,
                &platform_version.drive,
            )
            .expect("expected to batch move items");

        // Apply batch operations
        drive
            .apply_batch_low_level_drive_operations(
                None,
                Some(&transaction),
                drive_operations,
                &mut vec![],
                &platform_version.drive,
            )
            .expect("expected to apply operations");

        // Commit the transaction
        drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("expected to commit transaction");

        // Verify that keys 1 and 2 have been moved to the new path
        let get_result_1 = drive.grove_get(
            new_path.as_slice().into(),
            &key1,
            QueryType::StatefulQuery,
            None,
            &mut vec![],
            &platform_version.drive,
        );
        assert_matches!(get_result_1, Ok(Some(Element::Item(value, _))) if value == b"value".to_vec());

        let get_result_2 = drive.grove_get(
            new_path.as_slice().into(),
            &key2,
            QueryType::StatefulQuery,
            None,
            &mut vec![],
            &platform_version.drive,
        );
        assert_matches!(get_result_2, Ok(Some(Element::Item(value, _))) if value == b"value".to_vec());

        // Verify that keys 1 and 2 are no longer in the original path
        let get_result_1_old = drive.grove_get(
            path.as_slice().into(),
            &key1,
            QueryType::StatefulQuery,
            None,
            &mut vec![],
            &platform_version.drive,
        );
        assert_matches!(
            get_result_1_old,
            Err(Error::GroveDB(e)) if matches!(e.as_ref(), grovedb::Error::PathKeyNotFound(_))
        );

        let get_result_2_old = drive.grove_get(
            path.as_slice().into(),
            &key2,
            QueryType::StatefulQuery,
            None,
            &mut vec![],
            &platform_version.drive,
        );
        assert_matches!(
            get_result_2_old,
            Err(Error::GroveDB(e)) if matches!(e.as_ref(), grovedb::Error::PathKeyNotFound(_))
        );

        // Verify that key 3 is still in the original path
        let get_result_3 = drive.grove_get(
            path.as_slice().into(),
            &key3,
            QueryType::StatefulQuery,
            None,
            &mut vec![],
            &platform_version.drive,
        );
        assert_matches!(get_result_3, Ok(Some(Element::Item(..))));
    }
}
