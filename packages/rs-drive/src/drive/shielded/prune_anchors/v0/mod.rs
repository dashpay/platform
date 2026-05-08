use crate::drive::shielded::paths::{
    shielded_credit_pool_anchors_by_height_path, shielded_credit_pool_anchors_by_height_path_vec,
    shielded_credit_pool_anchors_path,
};
use crate::drive::Drive;
use crate::error::Error;
use dpp::version::PlatformVersion;
use grovedb::query_result_type::QueryResultType;
use grovedb::{Element, PathQuery, Query, QueryItem, SizedQuery, Transaction};

impl Drive {
    /// Version 0 implementation of pruning shielded pool anchors.
    ///
    /// Deletes anchors-by-height entries with `block_height < cutoff_height`
    /// (and the matching primary anchors-tree entries), with one
    /// crucial exception: **at least one entry must always remain in
    /// the index**. Specifically, if every recorded anchor is below
    /// the cutoff (no shielded ops have happened in the retention
    /// window), the entry with the highest block_height is preserved.
    ///
    /// Why: `validate_anchor_exists` reads the primary anchors tree
    /// (`[..., "s", [192]]`) when checking spend bundles. The
    /// most-recent anchor — exposed via
    /// `query_most_recent_shielded_anchor` — is derived from the
    /// highest entry in the anchors-by-height index, so any chain
    /// state where that index is empty also has an empty primary
    /// anchors tree, and *every* spend would be rejected with
    /// `InvalidAnchorError` until a new shielded op refreshed the
    /// state. Preserving the highest entry keeps the live anchor in
    /// `[6]` indefinitely while the pool sits idle, at the cost of
    /// at most one stale entry — bounded and acceptable.
    pub(in crate::drive) fn prune_shielded_pool_anchors_v0(
        &self,
        cutoff_height: u64,
        transaction: &Transaction,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        let grove_version = &platform_version.drive.grove_version;
        let by_height_path = shielded_credit_pool_anchors_by_height_path();
        let by_height_path_vec = shielded_credit_pool_anchors_by_height_path_vec();

        // 1. Query for entries strictly below cutoff (`RangeTo` is
        //    exclusive). Anything in this set is a candidate for
        //    deletion subject to the "always keep one" rule below.
        let mut below_query = Query::new();
        below_query.insert_item(QueryItem::RangeTo(..cutoff_height.to_be_bytes().to_vec()));

        let below_path_query = PathQuery {
            path: by_height_path_vec.clone(),
            query: SizedQuery {
                query: below_query,
                limit: None,
                offset: None,
            },
        };

        let (below_results, _) = self.grove_get_raw_path_query(
            &below_path_query,
            Some(transaction),
            QueryResultType::QueryKeyElementPairResultType,
            &mut vec![],
            &platform_version.drive,
        )?;

        let entries_below: Vec<(Vec<u8>, Element)> = below_results.to_key_elements();
        if entries_below.is_empty() {
            return Ok(());
        }

        // 2. Probe for any entry at or above cutoff. Cheap — we only
        //    need to know whether one exists, hence `limit: Some(1)`.
        //    If at least one does, the live anchor is recent and
        //    every entry below cutoff can be pruned safely.
        //    Otherwise, the highest entry below cutoff *is* the live
        //    anchor; we exclude it from deletion.
        let mut above_query = Query::new();
        above_query.insert_item(QueryItem::RangeFrom(cutoff_height.to_be_bytes().to_vec()..));
        let above_path_query = PathQuery {
            path: by_height_path_vec,
            query: SizedQuery {
                query: above_query,
                limit: Some(1),
                offset: None,
            },
        };
        let (above_results, _) = self.grove_get_raw_path_query(
            &above_path_query,
            Some(transaction),
            QueryResultType::QueryKeyElementPairResultType,
            &mut vec![],
            &platform_version.drive,
        )?;
        let any_above_cutoff = !above_results.to_key_elements().is_empty();

        let to_delete: Vec<(Vec<u8>, Element)> = if any_above_cutoff {
            entries_below
        } else {
            // Exclude the entry with the highest block_height key.
            // `entries_below` came from a `Query::new()` (default
            // `left_to_right = true`) over a `RangeTo` against
            // `SHIELDED_ANCHORS_BY_HEIGHT_KEY`, whose keys are
            // big-endian u64 — so GroveDB returns the entries in
            // ascending numeric order and the live anchor is
            // always the last element. Pop it off and discard;
            // the remaining vec is exactly the prunable set, no
            // extra scan or per-key clone.
            let mut candidates = entries_below;
            candidates
                .pop()
                .expect("entries_below non-empty (guarded above)");
            candidates
        };

        // 3. Delete from both trees. Order doesn't matter for
        //    correctness — both writes occur atomically as part of
        //    the block transaction.
        let anchors_path = shielded_credit_pool_anchors_path();
        for (height_key, element) in to_delete {
            if let Element::Item(anchor_bytes, _) = element {
                // NOTE: `.unwrap()` is `CostContext::unwrap()`, NOT
                // `Result::unwrap()`. Discards cost-tracking info,
                // never panics — standard pattern across Drive.
                self.grove
                    .delete(
                        &anchors_path,
                        &anchor_bytes,
                        None,
                        Some(transaction),
                        grove_version,
                    )
                    .unwrap()
                    .map_err(Error::from)?;
            }
            self.grove
                .delete(
                    &by_height_path,
                    &height_key,
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
        shielded_credit_pool_anchors_by_height_path, shielded_credit_pool_anchors_path,
    };
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use dpp::version::PlatformVersion;
    use grovedb::Element;

    /// Inserts (anchor_bytes → height) and (height_be → anchor_bytes) at a given height.
    fn seed_anchor(
        drive: &crate::drive::Drive,
        transaction: &grovedb::Transaction,
        anchor: [u8; 32],
        height: u64,
        platform_version: &PlatformVersion,
    ) {
        let grove_version = &platform_version.drive.grove_version;

        drive
            .grove
            .insert(
                &shielded_credit_pool_anchors_path(),
                &anchor,
                Element::new_item(height.to_be_bytes().to_vec()),
                None,
                Some(transaction),
                grove_version,
            )
            .unwrap()
            .expect("insert anchor");

        drive
            .grove
            .insert(
                &shielded_credit_pool_anchors_by_height_path(),
                &height.to_be_bytes(),
                Element::new_item(anchor.to_vec()),
                None,
                Some(transaction),
                grove_version,
            )
            .unwrap()
            .expect("insert by height");
    }

    #[test]
    fn prune_on_empty_tree_is_ok_noop() {
        // Empty anchors-by-height tree -> empty entries -> early return Ok(()).
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();
        let transaction = drive.grove.start_transaction();

        drive
            .prune_shielded_pool_anchors_v0(100, &transaction, platform_version)
            .expect("pruning an empty tree should be a noop");
    }

    #[test]
    fn prune_cutoff_excludes_anchors_at_cutoff_height() {
        // Cutoff is exclusive (`RangeTo ..cutoff`). An anchor at exactly `cutoff`
        // must not be pruned. Sanity-checks that the at-or-above
        // probe succeeds (anchor 20 is the live one).
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();
        let transaction = drive.grove.start_transaction();

        seed_anchor(&drive, &transaction, [1u8; 32], 10, platform_version);
        seed_anchor(&drive, &transaction, [2u8; 32], 20, platform_version);

        drive
            .prune_shielded_pool_anchors_v0(20, &transaction, platform_version)
            .expect("prune below 20");

        let mut drive_ops = vec![];
        assert!(!drive
            .has_shielded_anchor(
                &[1u8; 32],
                Some(&transaction),
                &mut drive_ops,
                platform_version
            )
            .unwrap());
        assert!(drive
            .has_shielded_anchor(
                &[2u8; 32],
                Some(&transaction),
                &mut drive_ops,
                platform_version
            )
            .unwrap());
    }

    #[test]
    fn prune_removes_all_below_cutoff_when_a_recent_anchor_exists() {
        // Multiple old anchors below cutoff AND a newer anchor at
        // or above cutoff → every old anchor gets pruned, the new
        // one survives. (Distinguishes from
        // `prune_keeps_highest_when_all_below_cutoff` below.)
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();
        let transaction = drive.grove.start_transaction();

        for h in 1u64..=5 {
            seed_anchor(&drive, &transaction, [h as u8; 32], h, platform_version);
        }
        seed_anchor(&drive, &transaction, [0xAAu8; 32], 12, platform_version);

        drive
            .prune_shielded_pool_anchors_v0(10, &transaction, platform_version)
            .expect("prune below 10");

        let mut drive_ops = vec![];
        for h in 1u64..=5 {
            assert!(!drive
                .has_shielded_anchor(
                    &[h as u8; 32],
                    Some(&transaction),
                    &mut drive_ops,
                    platform_version
                )
                .unwrap());
        }
        assert!(drive
            .has_shielded_anchor(
                &[0xAAu8; 32],
                Some(&transaction),
                &mut drive_ops,
                platform_version
            )
            .unwrap());
    }

    #[test]
    fn prune_preserves_all_at_or_above_cutoff() {
        // Cutoff below all entries -> nothing pruned.
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();
        let transaction = drive.grove.start_transaction();

        seed_anchor(&drive, &transaction, [1u8; 32], 100, platform_version);
        seed_anchor(&drive, &transaction, [2u8; 32], 200, platform_version);

        drive
            .prune_shielded_pool_anchors_v0(50, &transaction, platform_version)
            .expect("prune below 50");

        let mut drive_ops = vec![];
        assert!(drive
            .has_shielded_anchor(
                &[1u8; 32],
                Some(&transaction),
                &mut drive_ops,
                platform_version
            )
            .unwrap());
        assert!(drive
            .has_shielded_anchor(
                &[2u8; 32],
                Some(&transaction),
                &mut drive_ops,
                platform_version
            )
            .unwrap());
    }

    #[test]
    fn prune_keeps_highest_when_all_below_cutoff() {
        // The desync regression test. After ≥ retention_blocks of
        // shielded inactivity, every anchor in the by-height index
        // is below the prune cutoff. Pruning naively would empty
        // both trees and freeze every spend with `InvalidAnchorError`.
        // The fix preserves the highest entry below cutoff so the
        // anchors tree always has the live anchor.
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();
        let transaction = drive.grove.start_transaction();

        // Three old anchors, no recent one.
        seed_anchor(&drive, &transaction, [0xA1u8; 32], 100, platform_version);
        seed_anchor(&drive, &transaction, [0xA2u8; 32], 200, platform_version);
        seed_anchor(&drive, &transaction, [0xA3u8; 32], 300, platform_version);

        // Cutoff above every entry -> all are pruning candidates.
        drive
            .prune_shielded_pool_anchors_v0(1000, &transaction, platform_version)
            .expect("prune below 1000");

        let mut drive_ops = vec![];
        // Old entries (100, 200) are gone.
        assert!(!drive
            .has_shielded_anchor(
                &[0xA1u8; 32],
                Some(&transaction),
                &mut drive_ops,
                platform_version
            )
            .unwrap());
        assert!(!drive
            .has_shielded_anchor(
                &[0xA2u8; 32],
                Some(&transaction),
                &mut drive_ops,
                platform_version
            )
            .unwrap());
        // Highest entry (300) survives — this is the live anchor
        // the validator must still find.
        assert!(drive
            .has_shielded_anchor(
                &[0xA3u8; 32],
                Some(&transaction),
                &mut drive_ops,
                platform_version
            )
            .unwrap());

        // And the most-recent reader still sees it.
        let latest = drive
            .read_latest_recorded_shielded_anchor_v0(Some(&transaction), &platform_version.drive)
            .expect("read latest");
        assert_eq!(latest, Some([0xA3u8; 32]));
    }

    #[test]
    fn prune_keeps_single_old_entry() {
        // Edge case of the previous test: only one entry, it's old.
        // Must survive pruning regardless of cutoff.
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();
        let transaction = drive.grove.start_transaction();

        seed_anchor(&drive, &transaction, [0xCCu8; 32], 50, platform_version);

        drive
            .prune_shielded_pool_anchors_v0(10_000, &transaction, platform_version)
            .expect("prune");

        let mut drive_ops = vec![];
        assert!(drive
            .has_shielded_anchor(
                &[0xCCu8; 32],
                Some(&transaction),
                &mut drive_ops,
                platform_version
            )
            .unwrap());
    }
}
