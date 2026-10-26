//! Implements in Drive functions relating to asset locks.

use crate::drive::RootTree;
use grovedb::PathQuery;

/// The query for a single stored asset-lock outpoint.
pub fn asset_lock_outpoint_info_query(outpoint: &[u8; 36]) -> PathQuery {
    let mut query = PathQuery::new_single_key(
        asset_lock_storage_path()
            .iter()
            .map(|path| path.to_vec())
            .collect(),
        outpoint.to_vec(),
    );
    query.query.limit = Some(1);
    query
}

mod add_asset_lock_outpoint_operations;
mod estimation_costs;
mod fetch_asset_lock_outpoint_info;
mod prove_asset_lock_outpoint_info;
#[cfg(all(feature = "server", any(test, feature = "structure")))]
pub(crate) mod structure;

/// The asset lock root storage path
pub(crate) fn asset_lock_storage_path() -> [&'static [u8]; 1] {
    [Into::<&[u8; 1]>::into(RootTree::SpentAssetLockTransactions)]
}
