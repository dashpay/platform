use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::rpc::core::CoreRPCLike;
use dpp::version::PlatformVersion;
use drive::drive::shielded::paths::{
    shielded_credit_pool_anchors_by_height_path, shielded_credit_pool_anchors_path,
};
use drive::grovedb::query_result_type::QueryResultType;
use drive::grovedb::{PathQuery, Query, QueryItem, SizedQuery, Transaction};

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    /// Prunes anchors older than `shielded_anchor_retention_blocks` from the current height.
    ///
    /// Queries the anchors-by-height tree for all entries with block_height < cutoff,
    /// then deletes the corresponding entries from both the anchors-by-height tree
    /// (block_height → anchor_bytes) and the primary anchors tree (anchor_bytes → block_height).
    pub(super) fn prune_shielded_pool_anchors_v0(
        &self,
        block_height: u64,
        transaction: &Transaction,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        let retention_blocks = platform_version
            .drive_abci
            .validation_and_processing
            .event_constants
            .shielded_anchor_retention_blocks;

        // Only prune every 100 blocks to avoid unnecessary work
        if block_height % 100 != 0 {
            return Ok(());
        }

        // Nothing to prune if we haven't reached the retention depth yet
        if block_height <= retention_blocks {
            return Ok(());
        }

        let cutoff_height = block_height - retention_blocks;
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

        let (results, _) = self.drive.grove_get_raw_path_query(
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
            if let drive::grovedb::Element::Item(anchor_bytes, _) = element {
                // Delete from anchors tree (anchor_bytes → block_height)
                self.drive
                    .grove
                    .delete(
                        &anchors_path,
                        &anchor_bytes,
                        None,
                        Some(transaction),
                        grove_version,
                    )
                    .unwrap()
                    .map_err(|e| Error::Drive(drive::error::Error::from(e)))?;
            }

            // Delete from anchors-by-height tree (block_height → anchor_bytes)
            self.drive
                .grove
                .delete(
                    &by_height_path,
                    &height_key,
                    None,
                    Some(transaction),
                    grove_version,
                )
                .unwrap()
                .map_err(|e| Error::Drive(drive::error::Error::from(e)))?;
        }

        Ok(())
    }
}
