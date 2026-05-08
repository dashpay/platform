use crate::drive::shielded::paths::SHIELDED_CREDIT_POOL_KEY;
use crate::drive::RootTree;

// Byte positions chosen to balance the parent shielded-pool Merk tree —
// see the layout diagram at the top of `crate::drive::shielded::paths`.

/// The subtree key for per-block nullifiers storage (CountSumTree).
///
/// Depth 2 in the parent tree (right subtree of `SHIELDED_NOTES_KEY`).
pub const SHIELDED_RECENT_NULLIFIERS_KEY: &[u8; 1] = &[160];

/// The subtree key for per-block nullifiers storage as u8.
pub const SHIELDED_RECENT_NULLIFIERS_KEY_U8: u8 = 160;

/// The subtree key for compacted nullifiers storage.
///
/// Depth 2 in the parent tree.
pub const SHIELDED_COMPACTED_NULLIFIERS_KEY: &[u8; 1] = &[224];

/// The subtree key for compacted nullifiers storage as u8.
pub const SHIELDED_COMPACTED_NULLIFIERS_KEY_U8: u8 = 224;

/// The subtree key for nullifiers expiration time storage.
///
/// Deepest leaf in the parent tree — only touched by periodic expiry sweeps.
pub const SHIELDED_NULLIFIERS_EXPIRATION_TIME_KEY: &[u8; 1] = &[240];

/// The subtree key for nullifiers expiration time storage as u8.
pub const SHIELDED_NULLIFIERS_EXPIRATION_TIME_KEY_U8: u8 = 240;

/// Path to per-block nullifiers: [AddressBalances, "s", [160]]
pub fn shielded_recent_nullifiers_path() -> [&'static [u8]; 3] {
    [
        Into::<&[u8; 1]>::into(RootTree::AddressBalances),
        SHIELDED_CREDIT_POOL_KEY,
        SHIELDED_RECENT_NULLIFIERS_KEY,
    ]
}

/// Path to per-block nullifiers as vec: [AddressBalances, "s", [160]]
pub fn shielded_recent_nullifiers_path_vec() -> Vec<Vec<u8>> {
    vec![
        vec![RootTree::AddressBalances as u8],
        SHIELDED_CREDIT_POOL_KEY.to_vec(),
        vec![SHIELDED_RECENT_NULLIFIERS_KEY_U8],
    ]
}

/// Path to compacted nullifiers: [AddressBalances, "s", [224]]
pub fn shielded_compacted_nullifiers_path() -> [&'static [u8]; 3] {
    [
        Into::<&[u8; 1]>::into(RootTree::AddressBalances),
        SHIELDED_CREDIT_POOL_KEY,
        SHIELDED_COMPACTED_NULLIFIERS_KEY,
    ]
}

/// Path to compacted nullifiers as vec: [AddressBalances, "s", [224]]
pub fn shielded_compacted_nullifiers_path_vec() -> Vec<Vec<u8>> {
    vec![
        vec![RootTree::AddressBalances as u8],
        SHIELDED_CREDIT_POOL_KEY.to_vec(),
        vec![SHIELDED_COMPACTED_NULLIFIERS_KEY_U8],
    ]
}

/// Path to nullifiers expiration time: [AddressBalances, "s", [240]]
pub fn shielded_nullifiers_expiration_time_path() -> [&'static [u8]; 3] {
    [
        Into::<&[u8; 1]>::into(RootTree::AddressBalances),
        SHIELDED_CREDIT_POOL_KEY,
        SHIELDED_NULLIFIERS_EXPIRATION_TIME_KEY,
    ]
}

/// Path to nullifiers expiration time as vec: [AddressBalances, "s", [240]]
pub fn shielded_nullifiers_expiration_time_path_vec() -> Vec<Vec<u8>> {
    vec![
        vec![RootTree::AddressBalances as u8],
        SHIELDED_CREDIT_POOL_KEY.to_vec(),
        vec![SHIELDED_NULLIFIERS_EXPIRATION_TIME_KEY_U8],
    ]
}
