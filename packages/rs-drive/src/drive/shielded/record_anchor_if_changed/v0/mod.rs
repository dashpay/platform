use crate::drive::shielded::paths::{
    shielded_credit_pool_anchors_by_height_path, shielded_credit_pool_anchors_path,
    shielded_credit_pool_path, shielded_latest_recorded_anchor_path_query, SHIELDED_NOTES_KEY,
};
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use dpp::version::PlatformVersion;
use grovedb::query_result_type::QueryResultType;
use grovedb::{Element, Transaction};

impl Drive {
    /// Version 0 implementation of recording the shielded pool anchor.
    ///
    /// Reads the current Sinsemilla anchor from the CommitmentTree at
    /// `[AddressBalances, "s", [128]]`. If it differs from the most-recent
    /// anchor — derived as the latest entry in the anchors-by-height
    /// index `[..., "s", [96]]` (a `limit 1` reverse query) — inserts:
    /// - `anchor_bytes → block_height_be` into the anchors tree `[..., [192]]`
    /// - `block_height_be → anchor_bytes` into the anchors-by-height tree `[..., [96]]`
    ///
    /// There is intentionally no separate "most recent anchor" item:
    /// the anchors-by-height index is the canonical log, and the
    /// most-recent anchor is whatever sits at the highest block-height
    /// key. Eliminating the duplicate slot also eliminates the prune
    /// vs. record desync that previously left the anchors tree empty
    /// while the live anchor remained pinned in the redundant slot.
    pub(in crate::drive) fn record_shielded_pool_anchor_if_changed_v0(
        &self,
        block_height: u64,
        transaction: &Transaction,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        let drive_version = &platform_version.drive;
        let grove_version = &drive_version.grove_version;
        let pool_path = shielded_credit_pool_path();

        // 1. Read current anchor from the CommitmentTree.
        //
        // NOTE: `.unwrap()` below is `CostContext::unwrap()`, NOT
        // `Result::unwrap()`. `CostContext::unwrap()` simply discards
        // cost-tracking info and never panics. Standard pattern for
        // GroveDB operations across the Drive codebase when cost
        // tracking is not needed.
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

        // 2. Read the latest recorded anchor from `[8]` via a
        //    `limit 1` reverse query. This is the post-removal
        //    replacement for the old `most_recent_anchor` slot — same
        //    value, but derived from the canonical log so it cannot
        //    drift out of sync with the anchors tree under prune.
        //
        //    NOTE: there is intentionally no "skip when current is the
        //    Sinsemilla empty root" guard. The empty root is a
        //    well-defined value, recording it is harmless (it can't
        //    be spent against — no notes), and it ensures `[6]` is
        //    populated from the very first block-end event onward
        //    rather than only after the first shield op.
        let latest_recorded =
            self.read_latest_recorded_shielded_anchor_v0(Some(transaction), drive_version)?;

        // 3. Only insert if the anchor actually changed. Orchard's
        //    commitment tree only changes when a new note is
        //    appended, so over an idle pool this short-circuits every
        //    block and avoids the per-block insert cost.
        if latest_recorded == Some(current_anchor_bytes) {
            return Ok(());
        }

        // 4. Anchor changed — insert into both trees atomically with
        //    the rest of the block transaction.
        let anchors_path = shielded_credit_pool_anchors_path();
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

        Ok(())
    }

    /// Read the latest recorded shielded-pool anchor from
    /// `SHIELDED_ANCHORS_BY_HEIGHT_KEY` (`[..., "s", [96]]`) via a
    /// `limit 1` reverse query. Returns `None` if the index is empty
    /// (pool has never recorded an anchor — chain is at genesis or
    /// no shielded ops yet).
    ///
    /// Used by `record_shielded_pool_anchor_if_changed_v0` to decide
    /// whether the anchor changed this block. The drive-abci handler
    /// (`Platform::query_most_recent_shielded_anchor_v0`) and SDK
    /// verifier (`Drive::verify_most_recent_shielded_anchor_v0`)
    /// share the same `PathQuery` shape via
    /// `shielded_latest_recorded_anchor_path_query`, but operate on
    /// proofs rather than this raw helper.
    pub(in crate::drive) fn read_latest_recorded_shielded_anchor_v0(
        &self,
        transaction: grovedb::TransactionArg,
        drive_version: &dpp::version::drive_versions::DriveVersion,
    ) -> Result<Option<[u8; 32]>, Error> {
        let path_query = shielded_latest_recorded_anchor_path_query();

        let (results, _) = self.grove_get_raw_path_query(
            &path_query,
            transaction,
            QueryResultType::QueryKeyElementPairResultType,
            &mut vec![],
            drive_version,
        )?;

        let entries = results.to_key_elements();
        match entries.into_iter().next() {
            Some((_height_key, Element::Item(anchor_bytes, _))) => {
                let anchor: [u8; 32] = anchor_bytes.try_into().map_err(|_v: Vec<u8>| {
                    Error::Drive(DriveError::CorruptedElementType(
                        "anchors-by-height value is not 32 bytes",
                    ))
                })?;
                Ok(Some(anchor))
            }
            Some(_) => Err(Error::Drive(DriveError::CorruptedElementType(
                "anchors-by-height entry is not an Item",
            ))),
            None => Ok(None),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::drive::shielded::paths::{
        shielded_credit_pool_anchors_by_height_path, shielded_credit_pool_anchors_path,
    };
    use crate::drive::Drive;
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use dpp::version::PlatformVersion;
    use grovedb::Element;

    #[test]
    fn record_on_empty_pool_records_the_sinsemilla_empty_root() {
        // The commitment tree is empty, but its anchor is the
        // well-defined Sinsemilla empty root (a non-zero hash), not
        // `[0; 32]`. The new code records that anchor on the first
        // block-end after pool init; subsequent calls with the
        // unchanged anchor short-circuit (covered by
        // `record_idempotent_when_anchor_unchanged`). The wrong
        // assertion here is "no anchor was recorded" — it would
        // imply we silently dropped state on every empty pool — so
        // we instead assert that exactly one anchor lands in `[8]`,
        // and that the matching `[6]` membership succeeds for it.
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();
        let transaction = drive.grove.start_transaction();

        drive
            .record_shielded_pool_anchor_if_changed_v0(10, &transaction, platform_version)
            .expect("record on empty tree should succeed");

        let latest = drive
            .read_latest_recorded_shielded_anchor_v0(Some(&transaction), &platform_version.drive)
            .expect("read latest")
            .expect("empty-pool anchor should now be recorded");

        let mut drive_ops = vec![];
        assert!(drive
            .has_shielded_anchor(
                &latest,
                Some(&transaction),
                &mut drive_ops,
                platform_version
            )
            .unwrap());
        // The Sinsemilla empty root is not the zero hash; the old
        // code's `!= [0; 32]` guard was a stale defense against an
        // uninitialised slot, not a real "empty pool" gate.
        assert_ne!(latest, [0u8; 32]);
    }

    #[test]
    fn record_after_note_insert_stores_anchor() {
        // Insert a real note → CommitmentTree advances → current
        // anchor becomes non-zero → both `[6]` and `[8]` get an entry.
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();
        let transaction = drive.grove.start_transaction();

        let ops = Drive::insert_note_op(
            [0xAAu8; 32],
            [0x01u8; 32],
            [0xCCu8; 32],
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

        drive
            .record_shielded_pool_anchor_if_changed_v0(5, &transaction, platform_version)
            .expect("record anchor after insert");

        // `read_latest_recorded_shielded_anchor_v0` returns the same
        // anchor that's now in `[6]`. They write atomically, so a
        // membership check must succeed against the same key.
        let latest = drive
            .read_latest_recorded_shielded_anchor_v0(Some(&transaction), &platform_version.drive)
            .expect("read latest")
            .expect("anchor should be recorded");

        let mut drive_ops = vec![];
        assert!(drive
            .has_shielded_anchor(
                &latest,
                Some(&transaction),
                &mut drive_ops,
                platform_version
            )
            .unwrap());
    }

    #[test]
    fn record_idempotent_when_anchor_unchanged() {
        // Recording the same anchor twice in successive blocks must
        // not double-insert: the index would otherwise gain a stale
        // higher-height entry pointing at the live anchor and confuse
        // both prune and most-recent-anchor reads.
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();
        let transaction = drive.grove.start_transaction();

        let ops = Drive::insert_note_op(
            [0xAAu8; 32],
            [0x01u8; 32],
            [0xCCu8; 32],
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

        drive
            .record_shielded_pool_anchor_if_changed_v0(5, &transaction, platform_version)
            .expect("first record");
        drive
            .record_shielded_pool_anchor_if_changed_v0(6, &transaction, platform_version)
            .expect("second record (no-op)");
        drive
            .record_shielded_pool_anchor_if_changed_v0(7, &transaction, platform_version)
            .expect("third record (no-op)");

        // `[8]` should have exactly one entry — the original block 5.
        use crate::drive::shielded::paths::shielded_credit_pool_anchors_by_height_path_vec;
        use grovedb::query_result_type::QueryResultType;
        use grovedb::{PathQuery, Query, SizedQuery};
        let path_query = PathQuery {
            path: shielded_credit_pool_anchors_by_height_path_vec(),
            query: SizedQuery {
                query: Query::new_range_full(),
                limit: None,
                offset: None,
            },
        };
        let (results, _) = drive
            .grove_get_raw_path_query(
                &path_query,
                Some(&transaction),
                QueryResultType::QueryKeyElementPairResultType,
                &mut vec![],
                &platform_version.drive,
            )
            .expect("scan anchors-by-height");
        let entries = results.to_key_elements();
        assert_eq!(
            entries.len(),
            1,
            "expected single anchor entry at block 5, got {}",
            entries.len()
        );
        assert_eq!(entries[0].0, 5u64.to_be_bytes().to_vec());
    }

    #[test]
    fn read_latest_returns_highest_height_entry() {
        // With multiple anchors recorded, the helper must return the
        // one keyed at the highest block_height (the live root).
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();
        let transaction = drive.grove.start_transaction();
        let grove_version = &platform_version.drive.grove_version;

        let by_height_path = shielded_credit_pool_anchors_by_height_path();
        let anchors_path = shielded_credit_pool_anchors_path();
        for (h, anchor) in [
            (10u64, [0x11u8; 32]),
            (20u64, [0x22u8; 32]),
            (15u64, [0x33u8; 32]),
        ] {
            drive
                .grove
                .insert(
                    &anchors_path,
                    &anchor,
                    Element::new_item(h.to_be_bytes().to_vec()),
                    None,
                    Some(&transaction),
                    grove_version,
                )
                .unwrap()
                .expect("seed anchor");
            drive
                .grove
                .insert(
                    &by_height_path,
                    &h.to_be_bytes(),
                    Element::new_item(anchor.to_vec()),
                    None,
                    Some(&transaction),
                    grove_version,
                )
                .unwrap()
                .expect("seed by-height");
        }

        let latest = drive
            .read_latest_recorded_shielded_anchor_v0(Some(&transaction), &platform_version.drive)
            .expect("read latest")
            .expect("not empty");
        assert_eq!(latest, [0x22u8; 32], "highest height (20) should win");
    }
}
