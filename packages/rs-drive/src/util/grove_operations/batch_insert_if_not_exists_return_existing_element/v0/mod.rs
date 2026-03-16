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
    /// Version 0 implementation of the "insert element if the path key does not yet exist" operation.
    /// If the element already exists, it returns the existing element.
    pub(super) fn batch_insert_if_not_exists_return_existing_element_v0<const N: usize>(
        &self,
        path_key_element_info: PathKeyElementInfo<N>,
        apply_type: BatchInsertApplyType,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        drive_version: &DriveVersion,
    ) -> Result<Option<Element>, Error> {
        match path_key_element_info {
            PathKeyRefElement((path, key, element)) => {
                // Check if the element already exists
                let existing_element = self.grove_get_raw_optional(
                    path.as_slice().into(),
                    key,
                    apply_type.to_direct_query_type(),
                    transaction,
                    drive_operations,
                    drive_version,
                )?;
                if let Some(existing_element) = existing_element {
                    return Ok(Some(existing_element));
                }

                // Element does not exist, proceed with insertion
                drive_operations.push(LowLevelDriveOperation::insert_for_known_path_key_element(
                    path,
                    key.to_vec(),
                    element,
                ));
                Ok(None)
            }
            PathKeyElement((path, key, element)) => {
                // Check if the element already exists
                let existing_element = self.grove_get_raw_optional(
                    path.as_slice().into(),
                    key.as_slice(),
                    apply_type.to_direct_query_type(),
                    transaction,
                    drive_operations,
                    drive_version,
                )?;
                if let Some(existing_element) = existing_element {
                    return Ok(Some(existing_element));
                }

                // Element does not exist, proceed with insertion
                drive_operations.push(LowLevelDriveOperation::insert_for_known_path_key_element(
                    path, key, element,
                ));
                Ok(None)
            }
            PathFixedSizeKeyRefElement((path, key, element)) => {
                // Check if the element already exists
                let existing_element = self.grove_get_raw_optional(
                    path.as_slice().into(),
                    key,
                    apply_type.to_direct_query_type(),
                    transaction,
                    drive_operations,
                    drive_version,
                )?;
                if let Some(existing_element) = existing_element {
                    return Ok(Some(existing_element));
                }

                // Element does not exist, proceed with insertion
                let path_items: Vec<Vec<u8>> = path.into_iter().map(Vec::from).collect();
                drive_operations.push(LowLevelDriveOperation::insert_for_known_path_key_element(
                    path_items,
                    key.to_vec(),
                    element,
                ));
                Ok(None)
            }
            PathKeyElementSize((key_info_path, key_info, element)) => {
                match apply_type {
                    BatchInsertApplyType::StatelessBatchInsert {
                        in_tree_type: in_tree_using_sums,
                        ..
                    } => {
                        // Estimate if the element with the given size already exists
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
                        Ok(None)
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

    /// Insert new element via PathKeyRefElement - should return None (inserted).
    #[test]
    fn test_insert_if_not_exists_return_existing_new_ref() {
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
            .unwrap();

        let mut ops = vec![];
        let info = PathKeyElementInfo::<0>::PathKeyRefElement((
            vec![b"root".to_vec()],
            b"key",
            Element::new_item(b"val".to_vec()),
        ));

        let result = drive
            .batch_insert_if_not_exists_return_existing_element_v0(
                info,
                BatchInsertApplyType::StatefulBatchInsert,
                Some(&tx),
                &mut ops,
                &pv.drive,
            )
            .unwrap();

        assert!(result.is_none());
    }

    /// Insert when element exists - should return existing element via PathKeyRefElement.
    #[test]
    fn test_insert_if_not_exists_return_existing_ref_existing() {
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
            .unwrap();

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
            .unwrap();

        let mut ops = vec![];
        let info = PathKeyElementInfo::<0>::PathKeyRefElement((
            vec![b"root".to_vec()],
            b"key",
            Element::new_item(b"new".to_vec()),
        ));

        let result = drive
            .batch_insert_if_not_exists_return_existing_element_v0(
                info,
                BatchInsertApplyType::StatefulBatchInsert,
                Some(&tx),
                &mut ops,
                &pv.drive,
            )
            .unwrap();

        assert_eq!(result, Some(Element::new_item(b"existing".to_vec())));
    }

    /// Test PathKeyElement variant - new insert.
    #[test]
    fn test_insert_if_not_exists_return_existing_path_key_element_new() {
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
            .unwrap();

        let mut ops = vec![];
        let info = PathKeyElementInfo::<0>::PathKeyElement((
            vec![b"root".to_vec()],
            b"key".to_vec(),
            Element::new_item(b"val".to_vec()),
        ));

        let result = drive
            .batch_insert_if_not_exists_return_existing_element_v0(
                info,
                BatchInsertApplyType::StatefulBatchInsert,
                Some(&tx),
                &mut ops,
                &pv.drive,
            )
            .unwrap();

        assert!(result.is_none());
    }

    /// Test PathFixedSizeKeyRefElement variant - new insert.
    #[test]
    fn test_insert_if_not_exists_return_existing_fixed_key_new() {
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
            .unwrap();

        let mut ops = vec![];
        let path: [&[u8]; 1] = [b"root"];
        let info = PathKeyElementInfo::PathFixedSizeKeyRefElement((
            path,
            b"key",
            Element::new_item(b"val".to_vec()),
        ));

        let result = drive
            .batch_insert_if_not_exists_return_existing_element_v0(
                info,
                BatchInsertApplyType::StatefulBatchInsert,
                Some(&tx),
                &mut ops,
                &pv.drive,
            )
            .unwrap();

        assert!(result.is_none());
    }

    /// Test PathKeyElementSize stateless variant.
    #[test]
    fn test_insert_if_not_exists_return_existing_stateless_size() {
        let drive = setup_drive(None);
        let pv = PlatformVersion::latest();

        let mut ops = vec![];
        let info = PathKeyElementInfo::<0>::PathKeyElementSize((
            KeyInfoPath::from_known_owned_path(vec![b"root".to_vec()]),
            KeyInfo::KnownKey(b"key".to_vec()),
            Element::new_item(b"val".to_vec()),
        ));

        let result = drive
            .batch_insert_if_not_exists_return_existing_element_v0(
                info,
                BatchInsertApplyType::StatelessBatchInsert {
                    in_tree_type: TreeType::NormalTree,
                    target: QueryTarget::QueryTargetValue(100),
                },
                None,
                &mut ops,
                &pv.drive,
            )
            .unwrap();

        assert!(result.is_none());
        assert_eq!(ops.len(), 2); // cost + insert
    }

    /// Test PathKeyElementSize stateful variant returns error.
    #[test]
    fn test_insert_if_not_exists_return_existing_stateful_size_error() {
        let drive = setup_drive(None);
        let pv = PlatformVersion::latest();

        let mut ops = vec![];
        let info = PathKeyElementInfo::<0>::PathKeyElementSize((
            KeyInfoPath::from_known_owned_path(vec![b"root".to_vec()]),
            KeyInfo::KnownKey(b"key".to_vec()),
            Element::new_item(b"val".to_vec()),
        ));

        let result = drive.batch_insert_if_not_exists_return_existing_element_v0(
            info,
            BatchInsertApplyType::StatefulBatchInsert,
            None,
            &mut ops,
            &pv.drive,
        );

        assert!(result.is_err());
    }

    /// Test PathKeyUnknownElementSize returns error.
    #[test]
    fn test_insert_if_not_exists_return_existing_unknown_size_error() {
        let drive = setup_drive(None);
        let pv = PlatformVersion::latest();

        let mut ops = vec![];
        let info = PathKeyElementInfo::<0>::PathKeyUnknownElementSize((
            KeyInfoPath::from_known_owned_path(vec![b"root".to_vec()]),
            KeyInfo::KnownKey(b"key".to_vec()),
            8,
        ));

        let result = drive.batch_insert_if_not_exists_return_existing_element_v0(
            info,
            BatchInsertApplyType::StatefulBatchInsert,
            None,
            &mut ops,
            &pv.drive,
        );

        assert!(result.is_err());
    }
}
