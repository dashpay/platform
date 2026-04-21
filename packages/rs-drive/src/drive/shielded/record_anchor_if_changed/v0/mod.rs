use crate::drive::shielded::paths::{
    shielded_credit_pool_anchors_by_height_path, shielded_credit_pool_anchors_path,
    shielded_credit_pool_path, SHIELDED_MOST_RECENT_ANCHOR_KEY, SHIELDED_NOTES_KEY,
};
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use dpp::version::PlatformVersion;
use grovedb::{Element, Transaction};

impl Drive {
    /// Version 0 implementation of recording the shielded pool anchor.
    ///
    /// Reads the current Sinsemilla anchor from the CommitmentTree at
    /// `[AddressBalances, "s", [1]]`. If it differs from the most recent
    /// anchor (stored at `[AddressBalances, "s", [7]]`), inserts:
    /// - `anchor_bytes -> block_height.to_be_bytes()` into anchors tree `[..., [6]]`
    /// - `block_height.to_be_bytes() -> anchor_bytes` into anchors-by-height tree `[..., [8]]`
    /// - Updates the most recent anchor item
    pub(in crate::drive) fn record_shielded_pool_anchor_if_changed_v0(
        &self,
        block_height: u64,
        transaction: &Transaction,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        let grove_version = &platform_version.drive.grove_version;
        let pool_path = shielded_credit_pool_path();

        // 1. Read current anchor from CommitmentTree
        //
        // NOTE: .unwrap() below is CostContext::unwrap(), NOT Result::unwrap().
        // CostContext::unwrap() simply discards cost tracking info and never
        // panics. This is the standard pattern for GroveDB operations throughout
        // the Drive codebase when cost tracking is not needed.
        let current_anchor = self
            .grove
            .commitment_tree_anchor(
                &pool_path,
                &[SHIELDED_NOTES_KEY],
                Some(transaction),
                grove_version,
            )
            .unwrap()
            .map_err(Error::from)?;

        let current_anchor_bytes: [u8; 32] = current_anchor.to_bytes();

        // 2. Read most recent anchor from the dedicated element
        let most_recent_anchor: [u8; 32] = self
            .grove
            .get(
                &pool_path,
                &[SHIELDED_MOST_RECENT_ANCHOR_KEY],
                Some(transaction),
                grove_version,
            )
            .unwrap()
            .map_err(Error::from)
            .and_then(|element| {
                if let Element::Item(value, _) = element {
                    value.try_into().map_err(|_| {
                        Error::Drive(DriveError::CorruptedElementType(
                            "most recent anchor is not 32 bytes",
                        ))
                    })
                } else {
                    Err(Error::Drive(DriveError::CorruptedElementType(
                        "most recent anchor element is not an Item",
                    )))
                }
            })?;

        // 3. Only store if different (skip zero anchor from empty tree)
        let should_store =
            current_anchor_bytes != most_recent_anchor && current_anchor_bytes != [0u8; 32];

        if should_store {
            let anchors_path = shielded_credit_pool_anchors_path();

            // Insert anchor_bytes -> block_height into the anchors tree
            self.grove
                .insert(
                    &anchors_path,
                    &current_anchor_bytes,
                    Element::new_item(block_height.to_be_bytes().to_vec()),
                    None,
                    Some(transaction),
                    grove_version,
                )
                .unwrap()
                .map_err(Error::from)?;

            // Insert block_height -> anchor_bytes into the anchors-by-height tree (for pruning)
            let anchors_by_height_path = shielded_credit_pool_anchors_by_height_path();
            self.grove
                .insert(
                    &anchors_by_height_path,
                    &block_height.to_be_bytes(),
                    Element::new_item(current_anchor_bytes.to_vec()),
                    None,
                    Some(transaction),
                    grove_version,
                )
                .unwrap()
                .map_err(Error::from)?;

            // Update the most recent anchor
            self.grove
                .insert(
                    &pool_path,
                    &[SHIELDED_MOST_RECENT_ANCHOR_KEY],
                    Element::new_item(current_anchor_bytes.to_vec()),
                    None,
                    Some(transaction),
                    grove_version,
                )
                .unwrap()
                .map_err(Error::from)?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::drive::shielded::paths::{
        shielded_credit_pool_path, SHIELDED_MOST_RECENT_ANCHOR_KEY,
    };
    use crate::drive::Drive;
    use crate::error::drive::DriveError;
    use crate::error::Error;
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use dpp::version::PlatformVersion;
    use grovedb::Element;

    #[test]
    fn record_on_empty_pool_does_nothing() {
        // The commitment tree is empty - current_anchor_bytes is [0; 32]. The
        // should_store guard (`!= [0u8; 32]`) keeps us out of the write branch,
        // so has_shielded_anchor on any anchor still returns false.
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();
        let transaction = drive.grove.start_transaction();

        drive
            .record_shielded_pool_anchor_if_changed_v0(10, &transaction, platform_version)
            .expect("record on empty tree should succeed");

        let mut drive_ops = vec![];
        assert!(!drive
            .has_shielded_anchor(
                &[0u8; 32],
                Some(&transaction),
                &mut drive_ops,
                platform_version
            )
            .unwrap());
    }

    #[test]
    fn record_after_note_insert_stores_anchor() {
        // Insert a real note - the CommitmentTree advances and the current anchor
        // becomes non-zero. record_anchor_if_changed should then store an entry.
        // Note: cmx bytes must encode a valid Pallas field element; small values
        // like [0x01; 32] work because they're below the field modulus.
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();
        let transaction = drive.grove.start_transaction();

        let ops = Drive::insert_note_op(
            [0xAAu8; 32],
            [0x01u8; 32],
            vec![0x42; 216],
            platform_version,
        )
        .expect("build insert note op");
        let grove_ops =
            crate::fees::op::LowLevelDriveOperation::grovedb_operations_batch_consume(ops);
        drive
            .grove_apply_batch_with_add_costs(
                grove_ops,
                false,
                Some(&transaction),
                &mut vec![],
                &platform_version.drive,
            )
            .expect("apply note op");

        // Record the anchor.
        drive
            .record_shielded_pool_anchor_if_changed_v0(5, &transaction, platform_version)
            .expect("record anchor after insert");

        // The most recent anchor slot was updated to a non-zero value.
        let elem = drive
            .grove
            .get(
                &shielded_credit_pool_path(),
                &[SHIELDED_MOST_RECENT_ANCHOR_KEY],
                Some(&transaction),
                &platform_version.drive.grove_version,
            )
            .unwrap()
            .expect("most recent anchor");
        if let Element::Item(bytes, _) = elem {
            assert_ne!(bytes, vec![0u8; 32], "most recent anchor should be updated");
        } else {
            panic!("expected Element::Item for most recent anchor");
        }
    }

    #[test]
    fn corrupted_most_recent_anchor_returns_corrupted_element_type() {
        // Overwrite the most recent anchor key with an invalid length item (e.g. 10 bytes).
        // The try_into into [u8; 32] must fail with CorruptedElementType.
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();
        let transaction = drive.grove.start_transaction();
        let grove_version = &platform_version.drive.grove_version;

        // First: insert a note so current_anchor is non-zero (otherwise we'd skip past
        // the failing read via should_store=false). cmx must be a valid Pallas element.
        let ops = Drive::insert_note_op([1u8; 32], [0x02u8; 32], vec![3u8; 216], platform_version)
            .expect("build");
        let grove_ops =
            crate::fees::op::LowLevelDriveOperation::grovedb_operations_batch_consume(ops);
        drive
            .grove_apply_batch_with_add_costs(
                grove_ops,
                false,
                Some(&transaction),
                &mut vec![],
                &platform_version.drive,
            )
            .expect("apply");

        // Corrupt the most recent anchor item to a wrong length.
        drive
            .grove
            .insert(
                &shielded_credit_pool_path(),
                &[SHIELDED_MOST_RECENT_ANCHOR_KEY],
                Element::new_item(vec![0xEEu8; 10]),
                None,
                Some(&transaction),
                grove_version,
            )
            .unwrap()
            .expect("corrupt most recent anchor");

        let err = drive
            .record_shielded_pool_anchor_if_changed_v0(1, &transaction, platform_version)
            .expect_err("expected CorruptedElementType");
        assert!(
            matches!(err, Error::Drive(DriveError::CorruptedElementType(_))),
            "got: {:?}",
            err
        );
    }
}
