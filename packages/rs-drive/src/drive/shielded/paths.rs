use crate::drive::RootTree;

/// The subtree key for the shielded credit pool under AddressBalances
pub const SHIELDED_CREDIT_POOL_KEY: &[u8; 1] = b"s";

/// The subtree key for the shielded credit pool as a u8
pub const SHIELDED_CREDIT_POOL_KEY_U8: u8 = b's';

/// The subtree key for the shielded anchors under AddressBalances
pub const SHIELDED_ANCHORS_KEY: &[u8; 1] = b"a";

/// The subtree key for the shielded anchors as a u8
pub const SHIELDED_ANCHORS_KEY_U8: u8 = b'a';

/// Key for the commitments tree inside a shielded pool
pub const SHIELDED_COMMITMENTS_KEY: u8 = 1;

/// Key for the nullifiers tree inside a shielded pool
pub const SHIELDED_NULLIFIERS_KEY: u8 = 2;

/// Key for the encrypted notes tree inside a shielded pool
pub const SHIELDED_ENCRYPTED_NOTES_KEY: u8 = 3;

/// Key for the params item inside a shielded pool
pub const SHIELDED_PARAMS_KEY: u8 = 4;

/// Key for the total balance sum item inside a shielded pool
pub const SHIELDED_TOTAL_BALANCE_KEY: u8 = 5;

/// Path to the shielded credit pool: [AddressBalances, "s"]
pub fn shielded_credit_pool_path() -> [&'static [u8]; 2] {
    [
        Into::<&[u8; 1]>::into(RootTree::AddressBalances),
        SHIELDED_CREDIT_POOL_KEY,
    ]
}

/// Path to the shielded credit pool as a vec
pub fn shielded_credit_pool_path_vec() -> Vec<Vec<u8>> {
    vec![
        vec![RootTree::AddressBalances as u8],
        SHIELDED_CREDIT_POOL_KEY.to_vec(),
    ]
}

/// Path to the commitments tree: [AddressBalances, "s", [1]]
pub fn shielded_credit_pool_commitments_path() -> [&'static [u8]; 3] {
    [
        Into::<&[u8; 1]>::into(RootTree::AddressBalances),
        SHIELDED_CREDIT_POOL_KEY,
        &[SHIELDED_COMMITMENTS_KEY],
    ]
}

/// Path to the nullifiers tree: [AddressBalances, "s", [2]]
pub fn shielded_credit_pool_nullifiers_path() -> [&'static [u8]; 3] {
    [
        Into::<&[u8; 1]>::into(RootTree::AddressBalances),
        SHIELDED_CREDIT_POOL_KEY,
        &[SHIELDED_NULLIFIERS_KEY],
    ]
}

/// Path to the encrypted notes tree: [AddressBalances, "s", [3]]
pub fn shielded_credit_pool_encrypted_notes_path() -> [&'static [u8]; 3] {
    [
        Into::<&[u8; 1]>::into(RootTree::AddressBalances),
        SHIELDED_CREDIT_POOL_KEY,
        &[SHIELDED_ENCRYPTED_NOTES_KEY],
    ]
}

/// Path to the anchors tree: [AddressBalances, "a"]
pub fn shielded_anchors_path() -> [&'static [u8]; 2] {
    [
        Into::<&[u8; 1]>::into(RootTree::AddressBalances),
        SHIELDED_ANCHORS_KEY,
    ]
}

/// Path to the anchors tree as a vec
pub fn shielded_anchors_path_vec() -> Vec<Vec<u8>> {
    vec![
        vec![RootTree::AddressBalances as u8],
        SHIELDED_ANCHORS_KEY.to_vec(),
    ]
}

/// Path to the commitments tree as a vec: [AddressBalances, "s", [1]]
pub fn shielded_credit_pool_commitments_path_vec() -> Vec<Vec<u8>> {
    vec![
        vec![RootTree::AddressBalances as u8],
        SHIELDED_CREDIT_POOL_KEY.to_vec(),
        vec![SHIELDED_COMMITMENTS_KEY],
    ]
}

/// Path to the nullifiers tree as a vec: [AddressBalances, "s", [2]]
pub fn shielded_credit_pool_nullifiers_path_vec() -> Vec<Vec<u8>> {
    vec![
        vec![RootTree::AddressBalances as u8],
        SHIELDED_CREDIT_POOL_KEY.to_vec(),
        vec![SHIELDED_NULLIFIERS_KEY],
    ]
}

/// Path to the encrypted notes tree as a vec: [AddressBalances, "s", [3]]
pub fn shielded_credit_pool_encrypted_notes_path_vec() -> Vec<Vec<u8>> {
    vec![
        vec![RootTree::AddressBalances as u8],
        SHIELDED_CREDIT_POOL_KEY.to_vec(),
        vec![SHIELDED_ENCRYPTED_NOTES_KEY],
    ]
}

/// Path to the anchors credit pool as a vec: [AddressBalances, "a", "s"]
pub fn shielded_anchors_credit_pool_path_vec() -> Vec<Vec<u8>> {
    vec![
        vec![RootTree::AddressBalances as u8],
        SHIELDED_ANCHORS_KEY.to_vec(),
        SHIELDED_CREDIT_POOL_KEY.to_vec(),
    ]
}

/// Path to the credit pool anchors: [AddressBalances, "a", "s"]
pub fn shielded_anchors_credit_pool_path() -> [&'static [u8]; 3] {
    [
        Into::<&[u8; 1]>::into(RootTree::AddressBalances),
        SHIELDED_ANCHORS_KEY,
        SHIELDED_CREDIT_POOL_KEY,
    ]
}
