//! Implements in Drive functions relating to asset locks.

use crate::drive::RootTree;

mod add_asset_lock_outpoint_operations;
mod estimation_costs;
mod fetch_asset_lock_outpoint_info;

/// The asset lock root storage path
pub(crate) fn asset_lock_storage_path() -> [&'static [u8]; 1] {
    [Into::<&[u8; 1]>::into(RootTree::SpentAssetLockTransactions)]
}
