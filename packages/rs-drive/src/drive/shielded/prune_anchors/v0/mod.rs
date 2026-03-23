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
