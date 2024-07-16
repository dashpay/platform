use crate::drive::credit_pools::epochs::epochs_root_tree_key_constants::KEY_PENDING_EPOCH_REFUNDS;
use crate::drive::RootTree;

/// Returns the path to pending epoch refunds
pub fn pending_epoch_refunds_path_vec() -> Vec<Vec<u8>> {
    vec![
        vec![RootTree::Pools as u8],
        KEY_PENDING_EPOCH_REFUNDS.to_vec(),
    ]
}
