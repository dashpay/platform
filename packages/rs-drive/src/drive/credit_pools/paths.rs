use crate::drive::RootTree;
use crate::fee_pools::epochs_root_tree_key_constants::KEY_STORAGE_FEE_POOL;

/// Returns the path to the Pools subtree.
pub fn pools_path() -> [&'static [u8]; 1] {
    [Into::<&[u8; 1]>::into(RootTree::Pools)]
}

/// Returns the path to the Pools subtree as a mutable vector.
pub fn pools_vec_path() -> Vec<Vec<u8>> {
    vec![vec![RootTree::Pools as u8]]
}

/// Returns the path to the aggregate storage fee distribution pool.
pub fn aggregate_storage_fees_distribution_pool_path() -> [&'static [u8]; 2] {
    [
        Into::<&[u8; 1]>::into(RootTree::Pools),
        KEY_STORAGE_FEE_POOL,
    ]
}

/// Returns the path to the aggregate storage fee distribution pool as a mutable vector.
pub fn aggregate_storage_fees_distribution_pool_vec_path() -> Vec<Vec<u8>> {
    vec![vec![RootTree::Pools as u8], KEY_STORAGE_FEE_POOL.to_vec()]
}
