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
use grovedb::{Element, GroveDb, TransactionArg};

impl Drive {
    /// Pushes an "insert element if element was changed or is new" operation to `drive_operations`.
    /// Returns true if the path key already exists without references.
    pub(crate) fn batch_insert_if_changed_value_v0<const N: usize>(
        &self,
        path_key_element_info: PathKeyElementInfo<N>,
        apply_type: BatchInsertApplyType,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        drive_version: &DriveVersion,
    ) -> Result<(bool, Option<Element>), Error> {
        match path_key_element_info {
            PathKeyRefElement((path, key, element)) => {
                let previous_element = self.grove_get_raw_optional(
                    path.as_slice().into(),
                    key,
                    apply_type.to_direct_query_type(),
                    transaction,
                    drive_operations,
                    drive_version,
                )?;
                let needs_insert = match &previous_element {
                    None => true,
                    Some(previous_element) => previous_element != &element,
                };
                if needs_insert {
                    drive_operations.push(
                        LowLevelDriveOperation::insert_for_known_path_key_element(
                            path,
                            key.to_vec(),
                            element,
                        ),
                    );
                }
                Ok((needs_insert, previous_element))
            }
            PathKeyElement((path, key, element)) => {
                let previous_element = self.grove_get_raw_optional(
                    path.as_slice().into(),
                    key.as_slice(),
                    apply_type.to_direct_query_type(),
                    transaction,
                    drive_operations,
                    drive_version,
                )?;
                let needs_insert = match &previous_element {
                    None => true,
                    Some(previous_element) => previous_element != &element,
                };
                if needs_insert {
                    drive_operations.push(
                        LowLevelDriveOperation::insert_for_known_path_key_element(
                            path, key, element,
                        ),
                    );
                }
                Ok((needs_insert, previous_element))
            }
            PathFixedSizeKeyRefElement((path, key, element)) => {
                let previous_element = self.grove_get_raw_optional(
                    (&path).into(),
                    key,
                    apply_type.to_direct_query_type(),
                    transaction,
                    drive_operations,
                    drive_version,
                )?;
                let needs_insert = match &previous_element {
                    None => true,
                    Some(previous_element) => previous_element != &element,
                };
                if needs_insert {
                    let path_items: Vec<Vec<u8>> = path.into_iter().map(Vec::from).collect();
                    drive_operations.push(
                        LowLevelDriveOperation::insert_for_known_path_key_element(
                            path_items,
                            key.to_vec(),
                            element,
                        ),
                    );
                }
                Ok((needs_insert, previous_element))
            }
            PathKeyElementSize((key_info_path, key_info, element)) => {
                match apply_type {
                    BatchInsertApplyType::StatelessBatchInsert {
                        in_tree_type: in_tree_using_sums,
                        ..
                    } => {
                        // we can estimate that the element was the same size
                        drive_operations.push(CalculatedCostOperation(
                            GroveDb::average_case_for_get_raw(
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
                        Ok((true, None))
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

    /// Test inserting a new element via PathKeyRefElement (no previous element exists).
    #[test]
    fn test_batch_insert_if_changed_value_new_element_path_key_ref() {
        let drive = setup_drive(None);
        let platform_version = PlatformVersion::latest();
        let tx = drive.grove.start_transaction();

        drive
            .grove_insert_empty_tree(
                SubtreePath::empty(),
                b"root",
                TreeType::NormalTree,
                Some(&tx),
                None,
                &mut vec![],
                &platform_version.drive,
            )
            .unwrap();

        let mut ops = vec![];
        let path = vec![b"root".to_vec()];
        let element = Element::new_item(b"value".to_vec());
        let info = PathKeyElementInfo::<0>::PathKeyRefElement((path, b"key", element));

        let (needs_insert, previous) = drive
            .batch_insert_if_changed_value_v0(
                info,
                BatchInsertApplyType::StatefulBatchInsert,
                Some(&tx),
                &mut ops,
                &platform_version.drive,
            )
            .unwrap();

        assert!(needs_insert);
        assert!(previous.is_none());
        // One cost op from grove_get_raw_optional + one insert op
        assert_eq!(ops.len(), 2);
    }

    /// Test that inserting the same element value does not push an insert op.
    #[test]
    fn test_batch_insert_if_changed_value_same_element_no_insert() {
        let drive = setup_drive(None);
        let platform_version = PlatformVersion::latest();
        let tx = drive.grove.start_transaction();

        drive
            .grove_insert_empty_tree(
                SubtreePath::empty(),
                b"root",
                TreeType::NormalTree,
                Some(&tx),
                None,
                &mut vec![],
                &platform_version.drive,
            )
            .unwrap();

        // Insert the item first
        drive
            .grove
            .insert(
                &[b"root".as_slice()],
                b"key",
                Element::new_item(b"value".to_vec()),
                None,
                Some(&tx),
                &platform_version.drive.grove_version,
            )
            .unwrap()
            .unwrap();

        let mut ops = vec![];
        let path = vec![b"root".to_vec()];
        let element = Element::new_item(b"value".to_vec());
        let info = PathKeyElementInfo::<0>::PathKeyRefElement((path, b"key", element));

        let (needs_insert, previous) = drive
            .batch_insert_if_changed_value_v0(
                info,
                BatchInsertApplyType::StatefulBatchInsert,
                Some(&tx),
                &mut ops,
                &platform_version.drive,
            )
            .unwrap();

        assert!(!needs_insert);
        assert!(previous.is_some());
    }

    /// Test that inserting a different value does push an insert op.
    #[test]
    fn test_batch_insert_if_changed_value_different_element() {
        let drive = setup_drive(None);
        let platform_version = PlatformVersion::latest();
        let tx = drive.grove.start_transaction();

        drive
            .grove_insert_empty_tree(
                SubtreePath::empty(),
                b"root",
                TreeType::NormalTree,
                Some(&tx),
                None,
                &mut vec![],
                &platform_version.drive,
            )
            .unwrap();

        drive
            .grove
            .insert(
                &[b"root".as_slice()],
                b"key",
                Element::new_item(b"old_value".to_vec()),
                None,
                Some(&tx),
                &platform_version.drive.grove_version,
            )
            .unwrap()
            .unwrap();

        let mut ops = vec![];
        let path = vec![b"root".to_vec()];
        let element = Element::new_item(b"new_value".to_vec());
        let info = PathKeyElementInfo::<0>::PathKeyElement((path, b"key".to_vec(), element));

        let (needs_insert, previous) = drive
            .batch_insert_if_changed_value_v0(
                info,
                BatchInsertApplyType::StatefulBatchInsert,
                Some(&tx),
                &mut ops,
                &platform_version.drive,
            )
            .unwrap();

        assert!(needs_insert);
        assert_eq!(previous, Some(Element::new_item(b"old_value".to_vec())));
    }

    /// Test PathFixedSizeKeyRefElement variant with new element.
    #[test]
    fn test_batch_insert_if_changed_value_fixed_size_key_ref() {
        let drive = setup_drive(None);
        let platform_version = PlatformVersion::latest();
        let tx = drive.grove.start_transaction();

        drive
            .grove_insert_empty_tree(
                SubtreePath::empty(),
                b"root",
                TreeType::NormalTree,
                Some(&tx),
                None,
                &mut vec![],
                &platform_version.drive,
            )
            .unwrap();

        let mut ops = vec![];
        let path: [&[u8]; 1] = [b"root"];
        let element = Element::new_item(b"value".to_vec());
        let info = PathKeyElementInfo::PathFixedSizeKeyRefElement((path, b"key", element));

        let (needs_insert, previous) = drive
            .batch_insert_if_changed_value_v0(
                info,
                BatchInsertApplyType::StatefulBatchInsert,
                Some(&tx),
                &mut ops,
                &platform_version.drive,
            )
            .unwrap();

        assert!(needs_insert);
        assert!(previous.is_none());
    }

    /// Test stateless batch insert with PathKeyElementSize.
    #[test]
    fn test_batch_insert_if_changed_value_stateless_element_size() {
        let drive = setup_drive(None);
        let platform_version = PlatformVersion::latest();

        let mut ops = vec![];
        let key_info_path = KeyInfoPath::from_known_owned_path(vec![b"root".to_vec()]);
        let key_info = KeyInfo::KnownKey(b"key".to_vec());
        let element = Element::new_item(b"value".to_vec());
        let info = PathKeyElementInfo::<0>::PathKeyElementSize((key_info_path, key_info, element));

        let (needs_insert, previous) = drive
            .batch_insert_if_changed_value_v0(
                info,
                BatchInsertApplyType::StatelessBatchInsert {
                    in_tree_type: TreeType::NormalTree,
                    target: QueryTarget::QueryTargetValue(100),
                },
                None,
                &mut ops,
                &platform_version.drive,
            )
            .unwrap();

        assert!(needs_insert);
        assert!(previous.is_none());
        assert_eq!(ops.len(), 2); // cost + insert
    }

    /// Test stateful batch insert with PathKeyElementSize returns error.
    #[test]
    fn test_batch_insert_if_changed_value_stateful_element_size_error() {
        let drive = setup_drive(None);
        let platform_version = PlatformVersion::latest();

        let mut ops = vec![];
        let key_info_path = KeyInfoPath::from_known_owned_path(vec![b"root".to_vec()]);
        let key_info = KeyInfo::KnownKey(b"key".to_vec());
        let element = Element::new_item(b"value".to_vec());
        let info = PathKeyElementInfo::<0>::PathKeyElementSize((key_info_path, key_info, element));

        let result = drive.batch_insert_if_changed_value_v0(
            info,
            BatchInsertApplyType::StatefulBatchInsert,
            None,
            &mut ops,
            &platform_version.drive,
        );

        assert!(result.is_err());
    }

    /// Test PathKeyUnknownElementSize returns error.
    #[test]
    fn test_batch_insert_if_changed_value_unknown_size_error() {
        let drive = setup_drive(None);
        let platform_version = PlatformVersion::latest();

        let mut ops = vec![];
        let key_info_path = KeyInfoPath::from_known_owned_path(vec![b"root".to_vec()]);
        let key_info = KeyInfo::KnownKey(b"key".to_vec());
        let info = PathKeyElementInfo::<0>::PathKeyUnknownElementSize((key_info_path, key_info, 8));

        let result = drive.batch_insert_if_changed_value_v0(
            info,
            BatchInsertApplyType::StatefulBatchInsert,
            None,
            &mut ops,
            &platform_version.drive,
        );

        assert!(result.is_err());
    }
}
