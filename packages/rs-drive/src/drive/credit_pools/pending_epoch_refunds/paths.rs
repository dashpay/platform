use crate::drive::RootTree;
use crate::fee_pools::epochs_root_tree_key_constants::KEY_PENDING_EPOCH_REFUNDS;

/// Returns the path to pending epoch refunds
pub fn pending_epoch_refunds_path_vec() -> Vec<Vec<u8>> {
    vec![
        vec![RootTree::Pools as u8],
        KEY_PENDING_EPOCH_REFUNDS.to_vec(),
    ]
}
