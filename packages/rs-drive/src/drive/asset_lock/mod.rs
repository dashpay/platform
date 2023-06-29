use crate::drive::RootTree;

mod estimation_costs;
mod add_asset_lock_outpoint_operations;
mod has_asset_lock_outpoint;

/// The asset lock root storage path
pub(crate) fn asset_lock_storage_path() -> [&'static [u8]; 1] {
    [Into::<&[u8; 1]>::into(RootTree::SpentAssetLockTransactions)]
}

