use crate::drive::shielded::paths::SHIELDED_CREDIT_POOL_KEY;
use crate::drive::RootTree;

/// The subtree key for per-block nullifiers storage (CountSumTree)
pub const SHIELDED_RECENT_NULLIFIERS_KEY: &[u8; 1] = b"n";

/// The subtree key for per-block nullifiers storage as u8
pub const SHIELDED_RECENT_NULLIFIERS_KEY_U8: u8 = b'n';

/// The subtree key for compacted nullifiers storage
pub const SHIELDED_COMPACTED_NULLIFIERS_KEY: &[u8; 1] = b"o";

/// The subtree key for compacted nullifiers storage as u8
pub const SHIELDED_COMPACTED_NULLIFIERS_KEY_U8: u8 = b'o';

/// The subtree key for nullifiers expiration time storage
pub const SHIELDED_NULLIFIERS_EXPIRATION_TIME_KEY: &[u8; 1] = b"p";

/// The subtree key for nullifiers expiration time storage as u8
pub const SHIELDED_NULLIFIERS_EXPIRATION_TIME_KEY_U8: u8 = b'p';

/// Path to per-block nullifiers: [AddressBalances, "s", "n"]
pub fn shielded_recent_nullifiers_path() -> [&'static [u8]; 3] {
    [
        Into::<&[u8; 1]>::into(RootTree::AddressBalances),
        SHIELDED_CREDIT_POOL_KEY,
        SHIELDED_RECENT_NULLIFIERS_KEY,
    ]
}

/// Path to per-block nullifiers as vec: [AddressBalances, "s", "n"]
pub fn shielded_recent_nullifiers_path_vec() -> Vec<Vec<u8>> {
    vec![
        vec![RootTree::AddressBalances as u8],
        SHIELDED_CREDIT_POOL_KEY.to_vec(),
        vec![SHIELDED_RECENT_NULLIFIERS_KEY_U8],
    ]
}

/// Path to compacted nullifiers: [AddressBalances, "s", "o"]
pub fn shielded_compacted_nullifiers_path() -> [&'static [u8]; 3] {
    [
        Into::<&[u8; 1]>::into(RootTree::AddressBalances),
        SHIELDED_CREDIT_POOL_KEY,
        SHIELDED_COMPACTED_NULLIFIERS_KEY,
    ]
}

/// Path to compacted nullifiers as vec: [AddressBalances, "s", "o"]
pub fn shielded_compacted_nullifiers_path_vec() -> Vec<Vec<u8>> {
    vec![
        vec![RootTree::AddressBalances as u8],
        SHIELDED_CREDIT_POOL_KEY.to_vec(),
        vec![SHIELDED_COMPACTED_NULLIFIERS_KEY_U8],
    ]
}

/// Path to nullifiers expiration time: [AddressBalances, "s", "p"]
pub fn shielded_nullifiers_expiration_time_path() -> [&'static [u8]; 3] {
    [
        Into::<&[u8; 1]>::into(RootTree::AddressBalances),
        SHIELDED_CREDIT_POOL_KEY,
        SHIELDED_NULLIFIERS_EXPIRATION_TIME_KEY,
    ]
}

/// Path to nullifiers expiration time as vec: [AddressBalances, "s", "p"]
pub fn shielded_nullifiers_expiration_time_path_vec() -> Vec<Vec<u8>> {
    vec![
        vec![RootTree::AddressBalances as u8],
        SHIELDED_CREDIT_POOL_KEY.to_vec(),
        vec![SHIELDED_NULLIFIERS_EXPIRATION_TIME_KEY_U8],
    ]
}
