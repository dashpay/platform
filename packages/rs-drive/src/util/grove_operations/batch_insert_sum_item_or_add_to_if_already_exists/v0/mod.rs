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
use dpp::ProtocolError;
use grovedb::{Element, GroveDb, TransactionArg};

impl Drive {
    /// Version 0 implementation of the "insert sum item or add to it if the item already exists" operation.
    /// This operation either inserts a new sum item at the given path and key or adds the value to the existing sum item.
    ///
    /// # Parameters
    /// * `path_key_element_info`: Information about the path, key, and element.
    /// * `apply_type`: The apply type for the operation.
    /// * `transaction`: The transaction argument for the operation.
    /// * `drive_operations`: The list of drive operations to append to.
    /// * `drive_version`: The drive version to select the correct function version to run.
    ///
    /// # Returns
    /// * `Ok(())` if the operation was successful.
    /// * `Err(DriveError::CorruptedCodeExecution)` if the operation is not supported.
    pub(crate) fn batch_insert_sum_item_or_add_to_if_already_exists_v0<const N: usize>(
        &self,
        path_key_element_info: PathKeyElementInfo<N>,
        apply_type: BatchInsertApplyType,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        drive_version: &DriveVersion,
    ) -> Result<(), Error> {
        match path_key_element_info {
            PathKeyRefElement((path, key, element)) => {
                if let Element::SumItem(new_value, _) = element {
                    // Check if the sum item already exists
                    let existing_element = self.grove_get_raw_optional(
                        path.as_slice().into(),
                        key,
                        apply_type.to_direct_query_type(),
                        transaction,
                        drive_operations,
                        drive_version,
                    )?;

                    if let Some(Element::SumItem(existing_value, _)) = existing_element {
                        // Add to the existing sum item
                        let updated_value = existing_value
                            .checked_add(new_value)
                            .ok_or(ProtocolError::Overflow("overflow when adding to sum item"))?;
                        drive_operations.push(
                            LowLevelDriveOperation::insert_for_known_path_key_element(
                                path,
                                key.to_vec(),
                                Element::new_sum_item(updated_value),
                            ),
                        );
                    } else if existing_element.is_some() {
                        return Err(Error::Drive(DriveError::CorruptedElementType(
                            "expected sum item element type",
                        )));
                    } else {
                        // Insert as a new sum item
                        drive_operations.push(
                            LowLevelDriveOperation::insert_for_known_path_key_element(
                                path,
                                key.to_vec(),
                                Element::new_sum_item(new_value),
                            ),
                        );
                    }
                } else {
                    return Err(Error::Drive(DriveError::CorruptedCodeExecution(
                        "expected sum item element type",
                    )));
                }
                Ok(())
            }
            PathKeyElement((path, key, element)) => {
                if let Element::SumItem(new_value, _) = element {
                    // Check if the sum item already exists
                    let existing_element = self.grove_get_raw_optional(
                        path.as_slice().into(),
                        key.as_slice(),
                        apply_type.to_direct_query_type(),
                        transaction,
                        drive_operations,
                        drive_version,
                    )?;

                    if let Some(Element::SumItem(existing_value, _)) = existing_element {
                        // Add to the existing sum item
                        let updated_value = existing_value
                            .checked_add(new_value)
                            .ok_or(ProtocolError::Overflow("overflow when adding to sum item"))?;
                        drive_operations.push(
                            LowLevelDriveOperation::insert_for_known_path_key_element(
                                path,
                                key,
                                Element::new_sum_item(updated_value),
                            ),
                        );
                    } else if existing_element.is_some() {
                        return Err(Error::Drive(DriveError::CorruptedElementType(
                            "expected sum item element type",
                        )));
                    } else {
                        // Insert as a new sum item
                        drive_operations.push(
                            LowLevelDriveOperation::insert_for_known_path_key_element(
                                path,
                                key,
                                Element::new_sum_item(new_value),
                            ),
                        );
                    }
                } else {
                    return Err(Error::Drive(DriveError::CorruptedCodeExecution(
                        "expected sum item element type",
                    )));
                }
                Ok(())
            }
            PathFixedSizeKeyRefElement((path, key, element)) => {
                if let Element::SumItem(new_value, _) = element {
                    // Check if the sum item already exists
                    let existing_element = self.grove_get_raw_optional(
                        path.as_slice().into(),
                        key,
                        apply_type.to_direct_query_type(),
                        transaction,
                        drive_operations,
                        drive_version,
                    )?;

                    if let Some(Element::SumItem(existing_value, _)) = existing_element {
                        // Add to the existing sum item
                        let updated_value = existing_value
                            .checked_add(new_value)
                            .ok_or(ProtocolError::Overflow("overflow when adding to sum item"))?;
                        let path_items: Vec<Vec<u8>> = path.into_iter().map(Vec::from).collect();
                        drive_operations.push(
                            LowLevelDriveOperation::insert_for_known_path_key_element(
                                path_items,
                                key.to_vec(),
                                Element::new_sum_item(updated_value),
                            ),
                        );
                    } else if existing_element.is_some() {
                        return Err(Error::Drive(DriveError::CorruptedElementType(
                            "expected sum item element type",
                        )));
                    } else {
                        // Insert as a new sum item
                        let path_items: Vec<Vec<u8>> = path.into_iter().map(Vec::from).collect();
                        drive_operations.push(
                            LowLevelDriveOperation::insert_for_known_path_key_element(
                                path_items,
                                key.to_vec(),
                                Element::new_sum_item(new_value),
                            ),
                        );
                    }
                } else {
                    return Err(Error::Drive(DriveError::CorruptedCodeExecution(
                        "expected sum item element type",
                    )));
                }
                Ok(())
            }
            PathKeyElementSize((key_info_path, key_info, element)) => {
                if let Element::SumItem(new_value, _) = element {
                    match apply_type {
                        BatchInsertApplyType::StatelessBatchInsert {
                            in_tree_type, ..
                        } => {
                            // Estimate if the sum item with the given size already exists
                            drive_operations.push(CalculatedCostOperation(
                                GroveDb::average_case_for_has_raw(
                                    &key_info_path,
                                    &key_info,
                                    element.serialized_size(&drive_version.grove_version)? as u32,
                                    in_tree_type,
                                    &drive_version.grove_version,
                                )?,
                            ));

                            drive_operations.push(
                                LowLevelDriveOperation::insert_for_estimated_path_key_element(
                                    key_info_path,
                                    key_info,
                                    Element::new_sum_item(new_value),
                                ),
                            );
                            Ok(())
                        }
                        BatchInsertApplyType::StatefulBatchInsert => {
                            Err(Error::Drive(DriveError::NotSupportedPrivate(
                                "document sizes for stateful insert in batch operations not supported",
                            )))
                        }
                    }
                } else {
                    Err(Error::Drive(DriveError::CorruptedCodeExecution(
                        "expected sum item element type",
                    )))
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
    use crate::fees::op::LowLevelDriveOperation;
    use crate::util::grove_operations::{BatchInsertApplyType, QueryTarget};
    use crate::util::object_size_info::PathKeyElementInfo;
    use crate::util::test_helpers::setup::setup_drive;
    use grovedb::batch::key_info::KeyInfo;
    use grovedb::batch::{GroveOp, KeyInfoPath};
    use grovedb::{Element, TreeType};
    use grovedb_path::SubtreePath;
    use platform_version::version::PlatformVersion;

    /// Insert new sum item via PathKeyRefElement when nothing exists.
    #[test]
    fn test_sum_item_or_add_new_ref() {
        let drive = setup_drive(None);
        let pv = PlatformVersion::latest();
        let tx = drive.grove.start_transaction();

        drive
            .grove_insert_empty_tree(
                SubtreePath::empty(),
                b"root",
                TreeType::SumTree,
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
            Element::new_sum_item(42),
        ));

        drive
            .batch_insert_sum_item_or_add_to_if_already_exists_v0(
                info,
                BatchInsertApplyType::StatefulBatchInsert,
                Some(&tx),
                &mut ops,
                &pv.drive,
            )
            .expect("expected operation to succeed");

        // One cost op from grove_get_raw_optional + one insert op
        assert_eq!(ops.len(), 2);
    }

    /// Add to existing sum item via PathKeyRefElement.
    #[test]
    fn test_sum_item_or_add_existing_ref() {
        let drive = setup_drive(None);
        let pv = PlatformVersion::latest();
        let tx = drive.grove.start_transaction();

        drive
            .grove_insert_empty_tree(
                SubtreePath::empty(),
                b"root",
                TreeType::SumTree,
                Some(&tx),
                None,
                &mut vec![],
                &pv.drive,
            )
            .expect("expected to insert root tree");

        // Insert existing sum item
        drive
            .grove
            .insert(
                &[b"root".as_slice()],
                b"key",
                Element::new_sum_item(10),
                None,
                Some(&tx),
                &pv.drive.grove_version,
            )
            .unwrap()
            .expect("expected to insert element");

        let mut ops = vec![];
        let info = PathKeyElementInfo::<0>::PathKeyRefElement((
            vec![b"root".to_vec()],
            b"key",
            Element::new_sum_item(5),
        ));

        drive
            .batch_insert_sum_item_or_add_to_if_already_exists_v0(
                info,
                BatchInsertApplyType::StatefulBatchInsert,
                Some(&tx),
                &mut ops,
                &pv.drive,
            )
            .expect("expected operation to succeed");

        // One cost op from grove_get_raw_optional + one insert op with updated sum
        assert_eq!(ops.len(), 2);

        // Verify the insert operation contains the correct summed value (10 + 5 = 15)
        let insert_op = &ops[1];
        match insert_op {
            LowLevelDriveOperation::GroveOperation(grove_op) => match &grove_op.op {
                GroveOp::InsertOrReplace { element } => {
                    assert_eq!(
                        *element,
                        Element::new_sum_item(15),
                        "Expected sum item with value 15 (10 + 5)"
                    );
                }
                other => panic!("Expected InsertOrReplace op, got {:?}", other),
            },
            other => panic!("Expected GroveOperation, got {:?}", other),
        }
    }

    /// Error when existing element is not a sum item via PathKeyRefElement.
    #[test]
    fn test_sum_item_or_add_wrong_existing_type_ref() {
        let drive = setup_drive(None);
        let pv = PlatformVersion::latest();
        let tx = drive.grove.start_transaction();

        drive
            .grove_insert_empty_tree(
                SubtreePath::empty(),
                b"root",
                TreeType::SumTree,
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
                Element::new_item(b"not_sum".to_vec()),
                None,
                Some(&tx),
                &pv.drive.grove_version,
            )
            .unwrap()
            .expect("expected to insert element");

        let mut ops = vec![];
        let info = PathKeyElementInfo::<0>::PathKeyRefElement((
            vec![b"root".to_vec()],
            b"key",
            Element::new_sum_item(5),
        ));

        let result = drive.batch_insert_sum_item_or_add_to_if_already_exists_v0(
            info,
            BatchInsertApplyType::StatefulBatchInsert,
            Some(&tx),
            &mut ops,
            &pv.drive,
        );

        assert!(result.is_err());
    }

    /// Error when element passed is not a SumItem via PathKeyRefElement.
    #[test]
    fn test_sum_item_or_add_not_sum_item_element_ref() {
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
            Element::new_item(b"not_sum".to_vec()),
        ));

        let result = drive.batch_insert_sum_item_or_add_to_if_already_exists_v0(
            info,
            BatchInsertApplyType::StatefulBatchInsert,
            Some(&tx),
            &mut ops,
            &pv.drive,
        );

        assert!(result.is_err());
    }

    /// PathKeyElement variant - new insert.
    #[test]
    fn test_sum_item_or_add_new_path_key_element() {
        let drive = setup_drive(None);
        let pv = PlatformVersion::latest();
        let tx = drive.grove.start_transaction();

        drive
            .grove_insert_empty_tree(
                SubtreePath::empty(),
                b"root",
                TreeType::SumTree,
                Some(&tx),
                None,
                &mut vec![],
                &pv.drive,
            )
            .expect("expected to insert root tree");

        let mut ops = vec![];
        let info = PathKeyElementInfo::<0>::PathKeyElement((
            vec![b"root".to_vec()],
            b"key".to_vec(),
            Element::new_sum_item(42),
        ));

        drive
            .batch_insert_sum_item_or_add_to_if_already_exists_v0(
                info,
                BatchInsertApplyType::StatefulBatchInsert,
                Some(&tx),
                &mut ops,
                &pv.drive,
            )
            .expect("expected operation to succeed");
    }

    /// PathFixedSizeKeyRefElement variant - new insert.
    #[test]
    fn test_sum_item_or_add_new_fixed_key() {
        let drive = setup_drive(None);
        let pv = PlatformVersion::latest();
        let tx = drive.grove.start_transaction();

        drive
            .grove_insert_empty_tree(
                SubtreePath::empty(),
                b"root",
                TreeType::SumTree,
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
            Element::new_sum_item(42),
        ));

        drive
            .batch_insert_sum_item_or_add_to_if_already_exists_v0(
                info,
                BatchInsertApplyType::StatefulBatchInsert,
                Some(&tx),
                &mut ops,
                &pv.drive,
            )
            .expect("expected operation to succeed");
    }

    /// PathKeyElementSize stateless variant.
    #[test]
    fn test_sum_item_or_add_stateless_size() {
        let drive = setup_drive(None);
        let pv = PlatformVersion::latest();

        let mut ops = vec![];
        let info = PathKeyElementInfo::<0>::PathKeyElementSize((
            KeyInfoPath::from_known_owned_path(vec![b"root".to_vec()]),
            KeyInfo::KnownKey(b"key".to_vec()),
            Element::new_sum_item(42),
        ));

        drive
            .batch_insert_sum_item_or_add_to_if_already_exists_v0(
                info,
                BatchInsertApplyType::StatelessBatchInsert {
                    in_tree_type: TreeType::SumTree,
                    target: QueryTarget::QueryTargetValue(100),
                },
                None,
                &mut ops,
                &pv.drive,
            )
            .expect("expected operation to succeed");

        assert_eq!(ops.len(), 2); // cost + insert
    }

    /// PathKeyElementSize stateful variant returns error.
    #[test]
    fn test_sum_item_or_add_stateful_size_error() {
        let drive = setup_drive(None);
        let pv = PlatformVersion::latest();

        let mut ops = vec![];
        let info = PathKeyElementInfo::<0>::PathKeyElementSize((
            KeyInfoPath::from_known_owned_path(vec![b"root".to_vec()]),
            KeyInfo::KnownKey(b"key".to_vec()),
            Element::new_sum_item(42),
        ));

        let result = drive.batch_insert_sum_item_or_add_to_if_already_exists_v0(
            info,
            BatchInsertApplyType::StatefulBatchInsert,
            None,
            &mut ops,
            &pv.drive,
        );

        assert!(result.is_err());
    }

    /// PathKeyElementSize with non-sum-item returns error.
    #[test]
    fn test_sum_item_or_add_size_not_sum_item_error() {
        let drive = setup_drive(None);
        let pv = PlatformVersion::latest();

        let mut ops = vec![];
        let info = PathKeyElementInfo::<0>::PathKeyElementSize((
            KeyInfoPath::from_known_owned_path(vec![b"root".to_vec()]),
            KeyInfo::KnownKey(b"key".to_vec()),
            Element::new_item(b"not_sum".to_vec()),
        ));

        let result = drive.batch_insert_sum_item_or_add_to_if_already_exists_v0(
            info,
            BatchInsertApplyType::StatelessBatchInsert {
                in_tree_type: TreeType::SumTree,
                target: QueryTarget::QueryTargetValue(100),
            },
            None,
            &mut ops,
            &pv.drive,
        );

        assert!(result.is_err());
    }

    /// PathKeyUnknownElementSize returns error.
    #[test]
    fn test_sum_item_or_add_unknown_size_error() {
        let drive = setup_drive(None);
        let pv = PlatformVersion::latest();

        let mut ops = vec![];
        let info = PathKeyElementInfo::<0>::PathKeyUnknownElementSize((
            KeyInfoPath::from_known_owned_path(vec![b"root".to_vec()]),
            KeyInfo::KnownKey(b"key".to_vec()),
            8,
        ));

        let result = drive.batch_insert_sum_item_or_add_to_if_already_exists_v0(
            info,
            BatchInsertApplyType::StatefulBatchInsert,
            None,
            &mut ops,
            &pv.drive,
        );

        assert!(result.is_err());
    }
}
