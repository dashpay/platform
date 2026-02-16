use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::rpc::core::CoreRPCLike;
use dpp::version::PlatformVersion;
use drive::drive::shielded::paths::{
    shielded_credit_pool_anchors_path, shielded_credit_pool_path, SHIELDED_NOTES_KEY,
};
use drive::grovedb::query_result_type::QueryResultType;
use drive::grovedb::{Element, PathQuery, Query, QueryItem, SizedQuery, Transaction};

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    /// Records the current shielded pool anchor if the commitment tree changed this block.
    ///
    /// After all state transitions are processed, reads the current Sinsemilla anchor
    /// from the CommitmentTree at [AddressBalances, "s", [1]]. If it differs from the
    /// most recently stored anchor (or no anchor exists yet), inserts
    /// `block_height.to_be_bytes() → anchor_bytes` into the anchors tree at
    /// [AddressBalances, "s", [6]].
    ///
    /// This ensures anchors are only recorded once per block (not per-transaction),
    /// and only when the commitment tree actually changed.
    pub(in crate::execution) fn record_shielded_pool_anchor_if_changed(
        &self,
        block_height: u64,
        transaction: &Transaction,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        let grove_version = &platform_version.drive.grove_version;
        let pool_path = shielded_credit_pool_path();

        // 1. Read current anchor from CommitmentTree
        let current_anchor = self
            .drive
            .grove
            .commitment_tree_anchor(
                &pool_path,
                &[SHIELDED_NOTES_KEY],
                Some(transaction),
                grove_version,
            )
            .unwrap()
            .map_err(|e| Error::Drive(drive::error::Error::from(e)))?;

        let current_anchor_bytes: [u8; 32] = current_anchor.to_bytes();

        // 2. Query latest stored anchor (descending, limit 1)
        let anchors_path = shielded_credit_pool_anchors_path();
        let mut query = Query::new();
        query.insert_item(QueryItem::RangeFull(..));
        let path_query = PathQuery {
            path: anchors_path.iter().map(|p| p.to_vec()).collect(),
            query: SizedQuery {
                query,
                limit: Some(1),
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

        let latest_stored_anchor: Option<[u8; 32]> = results
            .to_key_elements()
            .into_iter()
            .last()
            .and_then(|(_key, element)| {
                if let Element::Item(value, _) = element {
                    value.try_into().ok()
                } else {
                    None
                }
            });

        // 3. Only store if different (or none stored yet)
        let should_store = match latest_stored_anchor {
            None => {
                // No anchors stored yet — only store if the tree has notes
                // (an empty tree has a zero anchor which isn't useful)
                current_anchor_bytes != [0u8; 32]
            }
            Some(stored) => stored != current_anchor_bytes,
        };

        if should_store {
            self.drive
                .grove
                .insert(
                    &anchors_path,
                    &block_height.to_be_bytes(),
                    Element::new_item(current_anchor_bytes.to_vec()),
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
