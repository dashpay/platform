use crate::drive::Drive;
use crate::error::Error;
use grovedb::operations::delete::ClearOptions;
use grovedb::TransactionArg;
use grovedb_path::SubtreePath;
use platform_version::version::drive_versions::DriveVersion;

impl Drive {
    /// Pushes the `OperationCost` of deleting an element in groveDB to `drive_operations`.
    pub(super) fn grove_clear_v0<B: AsRef<[u8]>>(
        &self,
        path: SubtreePath<'_, B>,
        transaction: TransactionArg,
        drive_version: &DriveVersion,
    ) -> Result<(), Error> {
        let options = ClearOptions {
            check_for_subtrees: false,
            allow_deleting_subtrees: false,
            trying_to_clear_with_subtrees_returns_error: false,
        };

        #[cfg(feature = "grovedb_operations_logging")]
        let maybe_params_for_logs = if tracing::event_enabled!(target: "drive_grovedb_operations", tracing::Level::TRACE)
        {
            let root_hash = self
                .grove
                .root_hash(transaction, &drive_version.grove_version)
                .unwrap()
                .map_err(Error::from)?;

            Some((path.clone(), root_hash))
        } else {
            None
        };

        // we will always return true if there is no error when we don't check for subtrees
        #[allow(clippy::let_and_return)] // due to feature below; we must allow this lint here
        let result = self
            .grove
            .clear_subtree(
                path,
                Some(options),
                transaction,
                &drive_version.grove_version,
            )
            .map_err(Error::from)
            .map(|_| ());

        #[cfg(feature = "grovedb_operations_logging")]
        if tracing::event_enabled!(target: "drive_grovedb_operations", tracing::Level::TRACE)
            && result.is_ok()
        {
            if let Some((path, previous_root_hash)) = maybe_params_for_logs {
                let root_hash = self
                    .grove
                    .root_hash(transaction, &drive_version.grove_version)
                    .unwrap()
                    .map_err(Error::from)?;

                tracing::trace!(
                    target: "drive_grovedb_operations",
                    path = ?path.to_vec(),
                    ?root_hash,
                    ?previous_root_hash,
                    is_transactional = transaction.is_some(),
                    "grovedb clear",
                );
            }
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use crate::util::test_helpers::setup::setup_drive;
    use grovedb::{Element, TreeType};
    use grovedb_path::SubtreePath;
    use platform_version::version::PlatformVersion;

    #[test]
    fn test_grove_clear_empty_subtree() {
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

        // Clear empty subtree should succeed
        drive
            .grove_clear_v0([b"root".as_slice()].as_slice().into(), Some(&tx), &pv.drive)
            .unwrap();
    }

    #[test]
    fn test_grove_clear_with_items() {
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

        // Insert some items
        drive
            .grove
            .insert(
                &[b"root".as_slice()],
                b"key1",
                Element::new_item(b"val1".to_vec()),
                None,
                Some(&tx),
                &pv.drive.grove_version,
            )
            .unwrap()
            .unwrap();

        drive
            .grove
            .insert(
                &[b"root".as_slice()],
                b"key2",
                Element::new_item(b"val2".to_vec()),
                None,
                Some(&tx),
                &pv.drive.grove_version,
            )
            .unwrap()
            .unwrap();

        // Clear subtree with items should succeed
        drive
            .grove_clear_v0([b"root".as_slice()].as_slice().into(), Some(&tx), &pv.drive)
            .unwrap();

        // Verify the items are actually gone
        let result1 = drive
            .grove
            .get_raw_optional(
                [b"root".as_slice()].as_slice().into(),
                b"key1",
                Some(&tx),
                &pv.drive.grove_version,
            )
            .unwrap()
            .unwrap();
        assert!(result1.is_none(), "key1 should be gone after clear");

        let result2 = drive
            .grove
            .get_raw_optional(
                [b"root".as_slice()].as_slice().into(),
                b"key2",
                Some(&tx),
                &pv.drive.grove_version,
            )
            .unwrap()
            .unwrap();
        assert!(result2.is_none(), "key2 should be gone after clear");
    }
}
