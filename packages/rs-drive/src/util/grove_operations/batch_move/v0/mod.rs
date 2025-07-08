use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::fees::op::LowLevelDriveOperation::GroveOperation;
use crate::util::grove_operations::{push_drive_operation_result, BatchMoveApplyType, QueryType};
use grovedb::batch::key_info::KeyInfo;
use grovedb::batch::{KeyInfoPath, QualifiedGroveDbOp};
use grovedb::operations::delete::DeleteOptions;
use grovedb::{Element, GroveDb, TransactionArg};
use grovedb_epoch_based_storage_flags::StorageFlags;
use grovedb_path::SubtreePath;
use grovedb_storage::rocksdb_storage::RocksDbStorage;
use platform_version::version::drive_versions::DriveVersion;

impl Drive {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn batch_move_v0<B: AsRef<[u8]>>(
        &self,
        from_path: SubtreePath<'_, B>,
        key: &[u8],
        to_path: Vec<Vec<u8>>,
        apply_type: BatchMoveApplyType,
        alter_flags_to_new_flags: Option<Option<StorageFlags>>,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        drive_version: &DriveVersion,
    ) -> Result<(), Error> {
        // ── 1. Fetch and validate the element ───────────────────────────────
        let mut element = match apply_type {
            BatchMoveApplyType::StatelessBatchMove {
                estimated_value_size,
                flags_len,
                ..
            } => {
                let value = vec![0u8; estimated_value_size as usize]; // if you want to simulate size
                let flags = vec![0u8; flags_len as usize];
                Element::new_item_with_flags(value, Some(flags))
            }
            BatchMoveApplyType::StatefulBatchMove { .. } => self
                .grove_get(
                    from_path.clone(),
                    key,
                    QueryType::StatefulQuery,
                    transaction,
                    drive_operations,
                    drive_version,
                )?
                .ok_or_else(|| {
                    Error::Drive(DriveError::ElementNotFound("element to move not found"))
                })?,
        };

        if element.is_any_tree() {
            return Err(Error::Drive(DriveError::NotSupported(
                "batch_move does not support moving trees",
            )));
        }

        // ── 2. Build the delete op ──────────────────────────────────────────
        let current_batch = LowLevelDriveOperation::grovedb_operations_batch(drive_operations);
        let delete_opts = DeleteOptions {
            allow_deleting_non_empty_trees: false,
            deleting_non_empty_trees_returns_error: true,
            base_root_storage_is_free: true,
            validate_tree_at_path_exists: false,
        };

        let delete_op = match apply_type {
            BatchMoveApplyType::StatelessBatchMove {
                in_tree_type,
                estimated_key_size,
                estimated_value_size,
                ..
            } => GroveDb::average_case_delete_operation_for_delete::<RocksDbStorage>(
                &KeyInfoPath::from_known_owned_path(from_path.to_vec()),
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
                from_path,
                key,
                &delete_opts,
                is_known_to_be_subtree_with_sum,
                &current_batch.operations,
                transaction,
                &drive_version.grove_version,
            ),
        };

        // ── 3. Push delete + insert into the batch ──────────────────────────
        if let Some(delete_op) = push_drive_operation_result(delete_op, drive_operations)? {
            if let Some(flags) = alter_flags_to_new_flags.as_ref() {
                element.set_flags(StorageFlags::map_to_some_element_flags(flags.as_ref()));
            }

            drive_operations.push(GroveOperation(delete_op));
            drive_operations.push(GroveOperation(QualifiedGroveDbOp::insert_or_replace_op(
                to_path,
                key.to_vec(),
                element,
            )));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        error::{drive::DriveError, Error},
        util::{
            grove_operations::{BatchMoveApplyType, QueryType},
            test_helpers::setup::setup_drive,
        },
    };
    use assert_matches::assert_matches;
    use grovedb::{Element, MaybeTree, TreeType};
    use grovedb_path::SubtreePath;
    use platform_version::version::PlatformVersion;

    /// Successfully move a single non‑tree item from `root` to `new_root`.
    #[test]
    fn test_batch_move_single_item_success() {
        let drive = setup_drive(None);
        let platform_version = PlatformVersion::latest();
        let tx = drive.grove.start_transaction();

        // prepare trees
        drive
            .grove_insert_empty_tree(
                SubtreePath::empty(),
                b"root",
                Some(&tx),
                None,
                &mut vec![],
                &platform_version.drive,
            )
            .unwrap();
        drive
            .grove_insert_empty_tree(
                SubtreePath::empty(),
                b"new_root",
                Some(&tx),
                None,
                &mut vec![],
                &platform_version.drive,
            )
            .unwrap();

        // insert the element to move
        let key = b"key1".to_vec();
        let value = Element::new_item(b"value1".to_vec());
        drive
            .grove
            .insert(
                &[b"root".as_slice()],
                &key,
                value.clone(),
                None,
                Some(&tx),
                &platform_version.drive.grove_version,
            )
            .unwrap()
            .unwrap();

        // batch‑move setup
        let apply_type = BatchMoveApplyType::StatefulBatchMove {
            is_known_to_be_subtree_with_sum: Some(MaybeTree::NotTree),
        };
        let mut ops = Vec::new();

        // call batch_move
        drive
            .batch_move_v0(
                [b"root".as_slice()].as_slice().into(),
                &key,
                vec![b"new_root".to_vec()],
                apply_type,
                None,
                Some(&tx),
                &mut ops,
                &platform_version.drive,
            )
            .expect("move should succeed");

        // apply & commit
        drive
            .apply_batch_low_level_drive_operations(
                None,
                Some(&tx),
                ops,
                &mut vec![],
                &platform_version.drive,
            )
            .unwrap();
        drive.grove.commit_transaction(tx).unwrap().unwrap();

        // verify new location
        let new_val = drive
            .grove_get(
                [b"new_root".as_slice()].as_slice().into(),
                &key,
                QueryType::StatefulQuery,
                None,
                &mut vec![],
                &platform_version.drive,
            )
            .expect("query")
            .expect("value");
        assert_eq!(new_val, value);

        // verify old location empty
        let old_res = drive.grove_get(
            [b"root".as_slice()].as_slice().into(),
            &key,
            QueryType::StatefulQuery,
            None,
            &mut vec![],
            &platform_version.drive,
        );
        assert_matches!(
            old_res,
            Err(Error::GroveDB(grovedb::Error::PathKeyNotFound(_)))
        );
    }

    /// Attempting to move a missing key should return a PathKeyNotFound error.
    #[test]
    fn test_batch_move_single_item_missing() {
        let drive = setup_drive(None);
        let platform_version = PlatformVersion::latest();
        let tx = drive.grove.start_transaction();

        drive
            .grove_insert_empty_tree(
                SubtreePath::empty(),
                b"root",
                Some(&tx),
                None,
                &mut vec![],
                &platform_version.drive,
            )
            .unwrap();
        drive
            .grove_insert_empty_tree(
                SubtreePath::empty(),
                b"new_root",
                Some(&tx),
                None,
                &mut vec![],
                &platform_version.drive,
            )
            .unwrap();

        let key = b"absent".to_vec();
        let apply_type = BatchMoveApplyType::StatefulBatchMove {
            is_known_to_be_subtree_with_sum: Some(MaybeTree::NotTree),
        };
        let mut ops = Vec::new();

        let res = drive.batch_move_v0(
            [b"root".as_slice()].as_slice().into(),
            &key,
            vec![b"new_root".to_vec()],
            apply_type,
            None,
            Some(&tx),
            &mut ops,
            &platform_version.drive,
        );

        assert_matches!(res, Err(Error::GroveDB(grovedb::Error::PathKeyNotFound(_))));
    }

    /// Moving a subtree (tree element) must fail with NotSupported.
    #[test]
    fn test_batch_move_single_item_tree_error() {
        let drive = setup_drive(None);
        let platform_version = PlatformVersion::latest();
        let tx = drive.grove.start_transaction();

        drive
            .grove_insert_empty_tree(
                SubtreePath::empty(),
                b"root",
                Some(&tx),
                None,
                &mut vec![],
                &platform_version.drive,
            )
            .unwrap();
        drive
            .grove_insert_empty_tree(
                SubtreePath::empty(),
                b"new_root",
                Some(&tx),
                None,
                &mut vec![],
                &platform_version.drive,
            )
            .unwrap();

        // insert a subtree under key "sub"
        drive
            .grove_insert_empty_tree(
                [b"root".as_slice()].as_slice().into(),
                b"sub",
                Some(&tx),
                None,
                &mut vec![],
                &platform_version.drive,
            )
            .unwrap();

        let apply_type = BatchMoveApplyType::StatefulBatchMove {
            is_known_to_be_subtree_with_sum: Some(MaybeTree::Tree(TreeType::NormalTree)),
        };
        let mut ops = Vec::new();

        let res = drive.batch_move_v0(
            [b"root".as_slice()].as_slice().into(),
            b"sub",
            vec![b"new_root".to_vec()],
            apply_type,
            None,
            Some(&tx),
            &mut ops,
            &platform_version.drive,
        );

        assert_matches!(res, Err(Error::Drive(DriveError::NotSupported(_))));
    }
}
