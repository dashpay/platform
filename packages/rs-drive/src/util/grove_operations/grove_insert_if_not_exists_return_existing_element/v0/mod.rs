use crate::drive::Drive;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::grove_operations::push_drive_operation_result_optional;
use grovedb::{Element, TransactionArg};
use grovedb_path::SubtreePath;
use platform_version::version::drive_versions::DriveVersion;

impl Drive {
    /// Pushes the `OperationCost` of inserting an element in groveDB where the path key does not yet exist
    /// to `drive_operations`.
    pub(super) fn grove_insert_if_not_exists_return_existing_element_v0<B: AsRef<[u8]>>(
        &self,
        path: SubtreePath<'_, B>,
        key: &[u8],
        element: Element,
        transaction: TransactionArg,
        drive_operations: Option<&mut Vec<LowLevelDriveOperation>>,
        drive_version: &DriveVersion,
    ) -> Result<Option<Element>, Error> {
        let cost_context = self.grove.insert_if_not_exists_return_existing_element(
            path,
            key,
            element,
            transaction,
            &drive_version.grove_version,
        );
        push_drive_operation_result_optional(cost_context, drive_operations)
    }
}

#[cfg(test)]
mod tests {
    use crate::util::test_helpers::setup::setup_drive;
    use grovedb::{Element, TreeType};
    use grovedb_path::SubtreePath;
    use platform_version::version::PlatformVersion;

    #[test]
    fn test_grove_insert_if_not_exists_return_existing_new() {
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
        let result = drive
            .grove_insert_if_not_exists_return_existing_element_v0(
                [b"root".as_slice()].as_slice().into(),
                b"key",
                Element::new_item(b"val".to_vec()),
                Some(&tx),
                Some(&mut ops),
                &pv.drive,
            )
            .expect("expected operation to succeed");

        // Element did not exist, so None returned and it was inserted
        assert!(result.is_none());
    }

    #[test]
    fn test_grove_insert_if_not_exists_return_existing_element_already_exists() {
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
        let result = drive
            .grove_insert_if_not_exists_return_existing_element_v0(
                [b"root".as_slice()].as_slice().into(),
                b"key",
                Element::new_item(b"new".to_vec()),
                Some(&tx),
                Some(&mut ops),
                &pv.drive,
            )
            .expect("expected operation to succeed");

        // Existing element should be returned
        assert_eq!(result, Some(Element::new_item(b"existing".to_vec())));
    }
}
