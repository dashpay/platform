mod create_key_tree_with_keys;
mod create_new_identity_key_query_trees;
mod insert_key_searchable_references;
mod insert_key_to_storage;
mod insert_new_non_unique_key;
mod insert_new_unique_key;
mod replace_key_in_storage;

use dpp::identity::IdentityPublicKey;

/// The contract apply info
#[allow(dead_code)]
pub enum DataContractApplyInfo {
    /// Keys of the contract apply info
    Keys(Vec<IdentityPublicKey>),
}
