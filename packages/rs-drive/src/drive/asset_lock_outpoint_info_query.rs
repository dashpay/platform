use crate::drive::RootTree;
use dpp::platform_value::Bytes36;
use grovedb::PathQuery;

/// The query for a single stored asset-lock outpoint.
pub(crate) fn asset_lock_outpoint_info_query(outpoint: &Bytes36) -> PathQuery {
    let mut query = PathQuery::new_single_key(
        vec![vec![RootTree::SpentAssetLockTransactions as u8]],
        outpoint.0.to_vec(),
    );
    query.query.limit = Some(1);
    query
}
