use crate::util::grove_operations::BatchInsertApplyType;
use crate::util::object_size_info::PathKeyElementInfo::{
    PathFixedSizeKeyRefElement, PathKeyElement, PathKeyElementSize, PathKeyRefElement,
    PathKeyUnknownElementSize,
};

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::fees::op::LowLevelDriveOperation::CalculatedCostOperation;
use crate::util::object_size_info::PathKeyElementInfo;
use dpp::version::drive_versions::DriveVersion;
use grovedb::{GroveDb, TransactionArg};

impl Drive {
    /// Pushes an "insert element if the path key does not yet exist" operation to `drive_operations`.
    /// Returns true if we inserted.
    pub(crate) fn batch_insert_if_not_exists_v0<const N: usize>(
        &self,
        path_key_element_info: PathKeyElementInfo<N>,
        apply_type: BatchInsertApplyType,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        drive_version: &DriveVersion,
    ) -> Result<bool, Error> {
        match path_key_element_info {
            PathKeyRefElement((path, key, element)) => {
                let has_raw = self.grove_has_raw(
                    path.as_slice().into(),
                    key,
                    apply_type.to_direct_query_type(),
                    transaction,
                    drive_operations,
                    drive_version,
                )?;
                if !has_raw {
                    drive_operations.push(
                        LowLevelDriveOperation::insert_for_known_path_key_element(
                            path,
                            key.to_vec(),
                            element,
                        ),
                    );
                }
                Ok(!has_raw)
            }
            PathKeyElement((path, key, element)) => {
                let has_raw = self.grove_has_raw(
                    path.as_slice().into(),
                    key.as_slice(),
                    apply_type.to_direct_query_type(),
                    transaction,
                    drive_operations,
                    drive_version,
                )?;
                if !has_raw {
                    drive_operations.push(
                        LowLevelDriveOperation::insert_for_known_path_key_element(
                            path, key, element,
                        ),
                    );
                }
                Ok(!has_raw)
            }
            PathFixedSizeKeyRefElement((path, key, element)) => {
                let has_raw = self.grove_has_raw(
                    path.as_slice().into(),
                    key,
                    apply_type.to_direct_query_type(),
                    transaction,
                    drive_operations,
                    drive_version,
                )?;
                if !has_raw {
                    let path_items: Vec<Vec<u8>> = path.into_iter().map(Vec::from).collect();
                    drive_operations.push(
                        LowLevelDriveOperation::insert_for_known_path_key_element(
                            path_items,
                            key.to_vec(),
                            element,
                        ),
                    );
                }
                Ok(!has_raw)
            }
            PathKeyElementSize((key_info_path, key_info, element)) => {
                match apply_type {
                    BatchInsertApplyType::StatelessBatchInsert {
                        in_tree_type: in_tree_using_sums,
                        ..
                    } => {
                        // we can estimate that the element was the same size
                        drive_operations.push(CalculatedCostOperation(
                            GroveDb::average_case_for_has_raw(
                                &key_info_path,
                                &key_info,
                                element.serialized_size(&drive_version.grove_version)? as u32,
                                in_tree_using_sums,
                                &drive_version.grove_version,
                            )?,
                        ));
                        drive_operations.push(
                            LowLevelDriveOperation::insert_for_estimated_path_key_element(
                                key_info_path,
                                key_info,
                                element,
                            ),
                        );
                        Ok(true)
                    }
                    BatchInsertApplyType::StatefulBatchInsert => {
                        Err(Error::Drive(DriveError::NotSupportedPrivate(
                            "document sizes for stateful insert in batch operations not supported",
                        )))
                    }
                }
            }
            PathKeyUnknownElementSize(_) => Err(Error::Drive(DriveError::NotSupportedPrivate(
                "document sizes in batch operations not supported",
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::util::grove_operations::{BatchInsertApplyType, QueryTarget};
    use crate::util::object_size_info::PathKeyElementInfo;
    use crate::util::test_helpers::setup::setup_drive;
    use grovedb::batch::key_info::KeyInfo;
    use grovedb::batch::KeyInfoPath;
    use grovedb::{Element, TreeType};
    use grovedb_path::SubtreePath;
    use platform_version::version::PlatformVersion;

    /// Insert new element via PathKeyRefElement should return true.
    #[test]
    fn test_batch_insert_if_not_exists_new_ref() {
        let drive = setup_drive(None);
        let pv = PlatformVersion::latest();
        let tx = drive.grove.start_transaction();

        drive
            .grove_insert_empty_tree(
                SubtreePath::empty(),
                b"root",
                TreeType::NormalTree,
                Some(&tx),
                None,
                &mut vec![],
                &pv.drive,
            )
            .expect("expected to insert root tree");

        let mut ops = vec![];
        let info = PathKeyElementInfo::<0>::PathKeyRefElement((
            vec![b"root".to_vec()],
            b"key",
            Element::new_item(b"val".to_vec()),
        ));

        let inserted = drive
            .batch_insert_if_not_exists_v0(
                info,
                BatchInsertApplyType::StatefulBatchInsert,
                Some(&tx),
                &mut ops,
                &pv.drive,
            )
            .expect("expected operation to succeed");

        assert!(inserted);
    }

    /// Insert when element exists should return false.
    #[test]
    fn test_batch_insert_if_not_exists_already_exists() {
        let drive = setup_drive(None);
        let pv = PlatformVersion::latest();
        let tx = drive.grove.start_transaction();

        drive
            .grove_insert_empty_tree(
                SubtreePath::empty(),
                b"root",
                TreeType::NormalTree,
                Some(&tx),
                None,
                &mut vec![],
                &pv.drive,
            )
            .expect("expected to insert root tree");

        drive
            .grove
            .insert(
                &[b"root".as_slice()],
                b"key",
                Element::new_item(b"existing".to_vec()),
                None,
                Some(&tx),
                &pv.drive.grove_version,
            )
            .unwrap()
            .expect("expected to insert element");

        let mut ops = vec![];
        let info = PathKeyElementInfo::<0>::PathKeyElement((
            vec![b"root".to_vec()],
            b"key".to_vec(),
            Element::new_item(b"new".to_vec()),
        ));

        let inserted = drive
            .batch_insert_if_not_exists_v0(
                info,
                BatchInsertApplyType::StatefulBatchInsert,
                Some(&tx),
                &mut ops,
                &pv.drive,
            )
            .expect("expected operation to succeed");

        assert!(!inserted);
    }

    /// PathFixedSizeKeyRefElement variant.
    #[test]
    fn test_batch_insert_if_not_exists_fixed_key() {
        let drive = setup_drive(None);
        let pv = PlatformVersion::latest();
        let tx = drive.grove.start_transaction();

        drive
            .grove_insert_empty_tree(
                SubtreePath::empty(),
                b"root",
                TreeType::NormalTree,
                Some(&tx),
                None,
                &mut vec![],
                &pv.drive,
            )
            .expect("expected to insert root tree");

        let mut ops = vec![];
        let path: [&[u8]; 1] = [b"root"];
        let info = PathKeyElementInfo::PathFixedSizeKeyRefElement((
            path,
            b"key",
            Element::new_item(b"val".to_vec()),
        ));

        let inserted = drive
            .batch_insert_if_not_exists_v0(
                info,
                BatchInsertApplyType::StatefulBatchInsert,
                Some(&tx),
                &mut ops,
                &pv.drive,
            )
            .expect("expected operation to succeed");

        assert!(inserted);
    }

    /// PathKeyElementSize stateless variant.
    #[test]
    fn test_batch_insert_if_not_exists_stateless_size() {
        let drive = setup_drive(None);
        let pv = PlatformVersion::latest();

        let mut ops = vec![];
        let info = PathKeyElementInfo::<0>::PathKeyElementSize((
            KeyInfoPath::from_known_owned_path(vec![b"root".to_vec()]),
            KeyInfo::KnownKey(b"key".to_vec()),
            Element::new_item(b"val".to_vec()),
        ));

        let inserted = drive
            .batch_insert_if_not_exists_v0(
                info,
                BatchInsertApplyType::StatelessBatchInsert {
                    in_tree_type: TreeType::NormalTree,
                    target: QueryTarget::QueryTargetValue(100),
                },
                None,
                &mut ops,
                &pv.drive,
            )
            .expect("expected operation to succeed");

        assert!(inserted);
    }

    /// PathKeyUnknownElementSize returns error.
    #[test]
    fn test_batch_insert_if_not_exists_unknown_size_error() {
        let drive = setup_drive(None);
        let pv = PlatformVersion::latest();

        let mut ops = vec![];
        let info = PathKeyElementInfo::<0>::PathKeyUnknownElementSize((
            KeyInfoPath::from_known_owned_path(vec![b"root".to_vec()]),
            KeyInfo::KnownKey(b"key".to_vec()),
            8,
        ));

        let result = drive.batch_insert_if_not_exists_v0(
            info,
            BatchInsertApplyType::StatefulBatchInsert,
            None,
            &mut ops,
            &pv.drive,
        );

        assert!(result.is_err());
    }
}
