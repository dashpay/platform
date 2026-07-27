use crate::drive::RootTree;
use grovedb::{PathQuery, Query, SizedQuery};

/// The subtree key for the shielded credit pool under ShieldedBalances
pub const MAIN_SHIELDED_CREDIT_POOL_KEY: &[u8; 1] = b"M";

/// The subtree key for the shielded credit pool as a u8
pub const MAIN_SHIELDED_CREDIT_POOL_KEY_U8: u8 = b'M';

// The five subtree keys of the shielded credit pool are placed at evenly-spaced
// byte positions across [0, 255] so that GroveDB's AVL-balanced parent tree
// puts the highest-traffic subtree (`SHIELDED_NOTES_KEY`) at the root, with the
// next-most-queried subtrees one hop below it, and the cold ones at the leaves:
//
//                              [128] NOTES                  ← root, every wallet sync
//                              /          \
//                  [64] NULLIFIERS         [192] ANCHORS_IN_POOL
//                   /        \
//          [32] TOTAL    [96] BY_HEIGHT
//
// Within a depth tier (children of a given internal node), placement is by
// access frequency: the spend-path subtrees (`NULLIFIERS`, `ANCHORS_IN_POOL`)
// are at depth 1; the cold balance/anchor-index subtrees (`TOTAL`, `BY_HEIGHT`)
// sit at the leaves. Key 7 is the historical
// `SHIELDED_MOST_RECENT_ANCHOR_KEY` slot — see retired-key note below.

/// Key for the total balance sum item inside a shielded pool.
///
/// Depth 2 in the parent tree (left subtree of `SHIELDED_NULLIFIERS_KEY`).
pub const SHIELDED_TOTAL_BALANCE_KEY: u8 = 32;

/// Key for the nullifiers tree inside a shielded pool.
///
/// Depth 1 in the parent tree — checked on every spend for membership.
pub const SHIELDED_NULLIFIERS_KEY: u8 = 64;

// Key 7 was previously `SHIELDED_MOST_RECENT_ANCHOR_KEY`, a redundant
// `Item([u8;32])` slot mirroring the latest entry in
// `SHIELDED_ANCHORS_BY_HEIGHT_KEY`. It was removed because the duplicated
// state could (and did) drift out of sync with the anchors tree under prune,
// leaving the validator's lookup table empty while the pool was still live.
// The most-recent anchor is now derived from `SHIELDED_ANCHORS_BY_HEIGHT_KEY`
// (`[96]`) via a `limit 1` reverse query — see
// `Drive::query_most_recent_shielded_anchor`.

/// Key for the anchors-by-height tree inside a shielded pool (block_height_be → anchor_bytes).
/// Reverse index of `SHIELDED_ANCHORS_IN_POOL_KEY`, used both for pruning old
/// anchors by height range and as the canonical source of the most-recent
/// anchor (read via `limit 1` reverse query).
///
/// Depth 2 in the parent tree.
pub const SHIELDED_ANCHORS_BY_HEIGHT_KEY: u8 = 96;

/// Key for the notes tree (CommitmentTree) inside a shielded pool.
///
/// Placed at byte 128 — the median of the pool subtrees, putting it at
/// the root of the parent Merk tree because every wallet sync and every
/// shield/transfer/spend touches this subtree.
pub const SHIELDED_NOTES_KEY: u8 = 128;

/// Key for the anchors tree inside a shielded pool (anchor_bytes → block_height_be).
/// Used by `validate_anchor_exists` for O(1) membership checks at spend time.
///
/// Depth 1 in the parent tree — checked on every spend.
pub const SHIELDED_ANCHORS_IN_POOL_KEY: u8 = 192;

/// Chunk power for the notes CommitmentTree (2^11 = 2048 items per chunk)
pub const SHIELDED_NOTES_CHUNK_POWER: u8 = 11;

/// Path to the shielded credit pool: [ShieldedBalances, "M"]
pub fn shielded_credit_pool_path() -> [&'static [u8]; 2] {
    [
        Into::<&[u8; 1]>::into(RootTree::ShieldedBalances),
        MAIN_SHIELDED_CREDIT_POOL_KEY,
    ]
}

/// Path to the shielded credit pool as a vec
pub fn shielded_credit_pool_path_vec() -> Vec<Vec<u8>> {
    vec![
        vec![RootTree::ShieldedBalances as u8],
        MAIN_SHIELDED_CREDIT_POOL_KEY.to_vec(),
    ]
}

/// Path to the notes tree: [ShieldedBalances, "M", [128]]
pub fn shielded_credit_pool_notes_path() -> [&'static [u8]; 3] {
    [
        Into::<&[u8; 1]>::into(RootTree::ShieldedBalances),
        MAIN_SHIELDED_CREDIT_POOL_KEY,
        &[SHIELDED_NOTES_KEY],
    ]
}

/// Path to the notes tree as a vec: [ShieldedBalances, "M", [128]]
pub fn shielded_credit_pool_notes_path_vec() -> Vec<Vec<u8>> {
    vec![
        vec![RootTree::ShieldedBalances as u8],
        MAIN_SHIELDED_CREDIT_POOL_KEY.to_vec(),
        vec![SHIELDED_NOTES_KEY],
    ]
}

/// Path to the nullifiers tree: [ShieldedBalances, "M", [64]]
pub fn shielded_credit_pool_nullifiers_path() -> [&'static [u8]; 3] {
    [
        Into::<&[u8; 1]>::into(RootTree::ShieldedBalances),
        MAIN_SHIELDED_CREDIT_POOL_KEY,
        &[SHIELDED_NULLIFIERS_KEY],
    ]
}

/// Path to the nullifiers tree as a vec: [ShieldedBalances, "M", [64]]
pub fn shielded_credit_pool_nullifiers_path_vec() -> Vec<Vec<u8>> {
    vec![
        vec![RootTree::ShieldedBalances as u8],
        MAIN_SHIELDED_CREDIT_POOL_KEY.to_vec(),
        vec![SHIELDED_NULLIFIERS_KEY],
    ]
}

/// Path to the anchors tree: [ShieldedBalances, "M", [192]]
pub fn shielded_credit_pool_anchors_path() -> [&'static [u8]; 3] {
    [
        Into::<&[u8; 1]>::into(RootTree::ShieldedBalances),
        MAIN_SHIELDED_CREDIT_POOL_KEY,
        &[SHIELDED_ANCHORS_IN_POOL_KEY],
    ]
}

/// Path to the anchors tree as a vec: [ShieldedBalances, "M", [192]]
pub fn shielded_credit_pool_anchors_path_vec() -> Vec<Vec<u8>> {
    vec![
        vec![RootTree::ShieldedBalances as u8],
        MAIN_SHIELDED_CREDIT_POOL_KEY.to_vec(),
        vec![SHIELDED_ANCHORS_IN_POOL_KEY],
    ]
}

/// Path to the anchors-by-height tree: [ShieldedBalances, "M", [96]]
pub fn shielded_credit_pool_anchors_by_height_path() -> [&'static [u8]; 3] {
    [
        Into::<&[u8; 1]>::into(RootTree::ShieldedBalances),
        MAIN_SHIELDED_CREDIT_POOL_KEY,
        &[SHIELDED_ANCHORS_BY_HEIGHT_KEY],
    ]
}

/// Path to the anchors-by-height tree as a vec: [ShieldedBalances, "M", [96]]
pub fn shielded_credit_pool_anchors_by_height_path_vec() -> Vec<Vec<u8>> {
    vec![
        vec![RootTree::ShieldedBalances as u8],
        MAIN_SHIELDED_CREDIT_POOL_KEY.to_vec(),
        vec![SHIELDED_ANCHORS_BY_HEIGHT_KEY],
    ]
}

/// Canonical `PathQuery` used to read the most-recent recorded
/// shielded-pool anchor: a `limit 1` reverse scan over
/// `SHIELDED_ANCHORS_BY_HEIGHT_KEY`, returning the entry with the
/// highest `block_height_be` key.
///
/// Shared between three call sites that must agree byte-for-byte:
/// - `Drive::read_latest_recorded_shielded_anchor_v0` (raw read used
///   by `record_shielded_pool_anchor_if_changed_v0` to decide whether
///   the anchor changed this block);
/// - `Platform::query_most_recent_shielded_anchor_v0` (proven RPC
///   handler);
/// - `Drive::verify_most_recent_shielded_anchor_v0` (SDK-side proof
///   verifier — replays the same `PathQuery`).
///
/// Keep these three in sync via this helper rather than open-coding
/// the `PathQuery` at each site; subtle differences (e.g. swapping
/// `left_to_right` or the `limit`) would silently produce
/// non-matching proofs.
pub fn shielded_latest_recorded_anchor_path_query() -> PathQuery {
    let mut query = Query::new();
    query.insert_all();
    query.left_to_right = false;
    PathQuery {
        path: shielded_credit_pool_anchors_by_height_path_vec(),
        query: SizedQuery {
            query,
            limit: Some(1),
            offset: None,
        },
    }
}

/// Resolves the nullifiers path based on pool type.
///
/// Pool types:
/// - 0: Main credit shielded pool → `[ShieldedBalances, "M", [64]]`
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
