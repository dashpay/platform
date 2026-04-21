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

/// Key for the anchors tree inside a shielded pool (anchor_bytes → block_height_be)
pub const SHIELDED_ANCHORS_IN_POOL_KEY: u8 = 6;

/// Key for the most recent anchor item inside a shielded pool
pub const SHIELDED_MOST_RECENT_ANCHOR_KEY: u8 = 7;

/// Key for the anchors-by-height tree inside a shielded pool (block_height_be → anchor_bytes)
/// Reverse index of SHIELDED_ANCHORS_IN_POOL_KEY, used for pruning old anchors by height range.
pub const SHIELDED_ANCHORS_BY_HEIGHT_KEY: u8 = 8;

/// Chunk power for the notes CommitmentTree (2^11 = 2048 items per chunk)
pub const SHIELDED_NOTES_CHUNK_POWER: u8 = 11;

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

/// Path to the anchors-by-height tree: [AddressBalances, "s", [8]]
pub fn shielded_credit_pool_anchors_by_height_path() -> [&'static [u8]; 3] {
    [
        Into::<&[u8; 1]>::into(RootTree::AddressBalances),
        SHIELDED_CREDIT_POOL_KEY,
        &[SHIELDED_ANCHORS_BY_HEIGHT_KEY],
    ]
}

/// Path to the anchors-by-height tree as a vec: [AddressBalances, "s", [8]]
pub fn shielded_credit_pool_anchors_by_height_path_vec() -> Vec<Vec<u8>> {
    vec![
        vec![RootTree::AddressBalances as u8],
        SHIELDED_CREDIT_POOL_KEY.to_vec(),
        vec![SHIELDED_ANCHORS_BY_HEIGHT_KEY],
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::drive::DriveError;
    use crate::error::Error;

    #[test]
    fn pool_type_0_returns_credit_pool_nullifiers_path() {
        // pool_type = 0 maps to the main credit shielded pool.
        let path = nullifiers_path_for_pool(0, None).expect("credit pool path");
        assert_eq!(path, shielded_credit_pool_nullifiers_path_vec());
        // pool_identifier being Some(_) is ignored for pool_type 0.
        let path2 = nullifiers_path_for_pool(0, Some(&[0xABu8; 32])).expect("credit pool path");
        assert_eq!(path2, shielded_credit_pool_nullifiers_path_vec());
    }

    #[test]
    fn pool_type_1_and_2_return_not_supported() {
        // Pool types 1 and 2 hit the NotSupported error branch.
        let err1 =
            nullifiers_path_for_pool(1, None).expect_err("pool type 1 should return NotSupported");
        assert!(matches!(err1, Error::Drive(DriveError::NotSupported(_))));

        let err2 = nullifiers_path_for_pool(2, Some(&[0xFFu8; 32]))
            .expect_err("pool type 2 should return NotSupported");
        assert!(matches!(err2, Error::Drive(DriveError::NotSupported(_))));
    }

    #[test]
    fn unknown_pool_type_returns_invalid_input() {
        let err = nullifiers_path_for_pool(999, None)
            .expect_err("unknown pool type should return InvalidInput");
        match err {
            Error::Drive(DriveError::InvalidInput(msg)) => assert!(msg.contains("999")),
            other => panic!("expected InvalidInput, got: {:?}", other),
        }
        // Also exercise edge values (u32::MAX).
        let err_max =
            nullifiers_path_for_pool(u32::MAX, None).expect_err("u32::MAX should be invalid input");
        assert!(matches!(err_max, Error::Drive(DriveError::InvalidInput(_))));
    }

    #[test]
    fn shielded_pool_path_vec_matches_static_path() {
        // Cross-check: the vec and static-slice versions encode the same path bytes.
        let arr = shielded_credit_pool_path();
        let v = shielded_credit_pool_path_vec();
        assert_eq!(arr.len(), v.len());
        for (a, b) in arr.iter().zip(v.iter()) {
            assert_eq!(*a, b.as_slice());
        }
    }

    #[test]
    fn anchors_paths_by_height_vs_pool_tree_use_distinct_keys() {
        // Regression guard: SHIELDED_ANCHORS_IN_POOL_KEY != SHIELDED_ANCHORS_BY_HEIGHT_KEY.
        // A bug confusing these keys would silently break pruning.
        let pool_path = shielded_credit_pool_anchors_path_vec();
        let by_height = shielded_credit_pool_anchors_by_height_path_vec();
        assert_eq!(pool_path.len(), 3);
        assert_eq!(by_height.len(), 3);
        assert_ne!(pool_path[2], by_height[2]);
    }
}
