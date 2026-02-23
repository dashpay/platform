use crate::drive::RootTree;

/// The subtree key for the shielded credit pool under AddressBalances
pub const SHIELDED_CREDIT_POOL_KEY: &[u8; 1] = b"s";

/// The subtree key for the shielded credit pool as a u8
pub const SHIELDED_CREDIT_POOL_KEY_U8: u8 = b's';

/// Key for the notes tree (CommitmentTree) inside a shielded pool
pub const SHIELDED_NOTES_KEY: u8 = 1;

/// Key for the nullifiers tree inside a shielded pool
pub const SHIELDED_NULLIFIERS_KEY: u8 = 2;

/// Key for the total balance sum item inside a shielded pool
pub const SHIELDED_TOTAL_BALANCE_KEY: u8 = 5;

/// Key for the anchors tree inside a shielded pool
pub const SHIELDED_ANCHORS_IN_POOL_KEY: u8 = 6;

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

/// Path to the notes tree: [AddressBalances, "s", [1]]
pub fn shielded_credit_pool_notes_path() -> [&'static [u8]; 3] {
    [
        Into::<&[u8; 1]>::into(RootTree::AddressBalances),
        SHIELDED_CREDIT_POOL_KEY,
        &[SHIELDED_NOTES_KEY],
    ]
}

/// Path to the notes tree as a vec: [AddressBalances, "s", [1]]
pub fn shielded_credit_pool_notes_path_vec() -> Vec<Vec<u8>> {
    vec![
        vec![RootTree::AddressBalances as u8],
        SHIELDED_CREDIT_POOL_KEY.to_vec(),
        vec![SHIELDED_NOTES_KEY],
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

/// Path to the nullifiers tree as a vec: [AddressBalances, "s", [2]]
pub fn shielded_credit_pool_nullifiers_path_vec() -> Vec<Vec<u8>> {
    vec![
        vec![RootTree::AddressBalances as u8],
        SHIELDED_CREDIT_POOL_KEY.to_vec(),
        vec![SHIELDED_NULLIFIERS_KEY],
    ]
}

/// Path to the anchors tree: [AddressBalances, "s", [6]]
pub fn shielded_credit_pool_anchors_path() -> [&'static [u8]; 3] {
    [
        Into::<&[u8; 1]>::into(RootTree::AddressBalances),
        SHIELDED_CREDIT_POOL_KEY,
        &[SHIELDED_ANCHORS_IN_POOL_KEY],
    ]
}

/// Path to the anchors tree as a vec: [AddressBalances, "s", [6]]
pub fn shielded_credit_pool_anchors_path_vec() -> Vec<Vec<u8>> {
    vec![
        vec![RootTree::AddressBalances as u8],
        SHIELDED_CREDIT_POOL_KEY.to_vec(),
        vec![SHIELDED_ANCHORS_IN_POOL_KEY],
    ]
}

/// Resolves the nullifiers path based on pool type.
///
/// Pool types:
/// - 0: Main credit shielded pool → `[AddressBalances, "s", [2]]`
/// - 1: Main token shielded pool (not yet implemented)
/// - 2: Individual token shielded pool (not yet implemented, requires pool_identifier)
pub fn nullifiers_path_for_pool(
    pool_type: u32,
    _pool_identifier: Option<&[u8]>,
) -> Result<Vec<Vec<u8>>, crate::error::Error> {
    use crate::error::drive::DriveError;
    use crate::error::Error;

    match pool_type {
        0 => Ok(shielded_credit_pool_nullifiers_path_vec()),
        1 | 2 => Err(Error::Drive(DriveError::NotSupported(
            "Token shielded pools not yet implemented",
        ))),
        _ => Err(Error::Drive(DriveError::InvalidInput(format!(
            "Unknown pool type: {}",
            pool_type
        )))),
    }
}
