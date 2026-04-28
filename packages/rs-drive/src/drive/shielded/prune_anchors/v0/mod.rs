use crate::drive::shielded::paths::{
    shielded_credit_pool_anchors_by_height_path, shielded_credit_pool_anchors_path,
};
use crate::drive::Drive;
use crate::error::Error;
use dpp::version::PlatformVersion;
use grovedb::query_result_type::QueryResultType;
use grovedb::{Element, PathQuery, Query, QueryItem, SizedQuery, Transaction};

impl Drive {
    /// Version 0 implementation of pruning shielded pool anchors.
    ///
    /// Queries the anchors-by-height tree for all entries with
    /// `block_height < cutoff_height`, then deletes those entries from both
    /// the anchors-by-height tree and the primary anchors tree.
    pub(in crate::drive) fn prune_shielded_pool_anchors_v0(
        &self,
        cutoff_height: u64,
        transaction: &Transaction,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        let grove_version = &platform_version.drive.grove_version;

        // Query anchors-by-height for all entries with height < cutoff (exclusive)
        let by_height_path = shielded_credit_pool_anchors_by_height_path();
        let mut query = Query::new();
        query.insert_item(QueryItem::RangeTo(..cutoff_height.to_be_bytes().to_vec()));

        let path_query = PathQuery {
            path: by_height_path.iter().map(|p| p.to_vec()).collect(),
            query: SizedQuery {
                query,
                limit: None,
                offset: None,
            },
        };

        let (results, _) = self.grove_get_raw_path_query(
            &path_query,
            Some(transaction),
            QueryResultType::QueryKeyElementPairResultType,
            &mut vec![],
            &platform_version.drive,
        )?;

        let entries = results.to_key_elements();
        if entries.is_empty() {
            return Ok(());
        }

        let anchors_path = shielded_credit_pool_anchors_path();

        for (height_key, element) in entries {
            // Extract anchor_bytes from the element value
            if let Element::Item(anchor_bytes, _) = element {
                // Delete from anchors tree (anchor_bytes -> block_height)
                // NOTE: .unwrap() is CostContext::unwrap(), not Result::unwrap().
                // It discards cost tracking info and never panics.
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

            // Delete from anchors-by-height tree (block_height -> anchor_bytes)
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

    /// Inserts (anchor_bytes -> height) and (height_be -> anchor_bytes) at a given height.
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
        // must not be pruned.
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();
        let transaction = drive.grove.start_transaction();

        seed_anchor(&drive, &transaction, [1u8; 32], 10, platform_version);
        seed_anchor(&drive, &transaction, [2u8; 32], 20, platform_version);

        drive
            .prune_shielded_pool_anchors_v0(20, &transaction, platform_version)
            .expect("prune below 20");

        // Anchor at height 10 should be gone; anchor at height 20 should remain.
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
    fn prune_removes_all_below_cutoff() {
        // Multiple old anchors all below cutoff -> all pruned.
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();
        let transaction = drive.grove.start_transaction();

        for h in 1u64..=5 {
            seed_anchor(&drive, &transaction, [h as u8; 32], h, platform_version);
        }

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
}
