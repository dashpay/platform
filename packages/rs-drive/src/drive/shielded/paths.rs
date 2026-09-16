use crate::drive::tokens::paths::TOKEN_SHIELDED_POOLS_KEY;
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

// ---------------------------------------------------------------------------
// Token shielded pools
//
// A token pool lives at `[Tokens, TOKEN_SHIELDED_POOLS_KEY, token_id]` and has the same five
// children, under the same keys, as the credit pool above. Everything that reads or writes a
// pool takes the pool's path, so the credit pool and every token pool share one implementation.
// ---------------------------------------------------------------------------

/// Path to a token's shielded pool: `[Tokens, 224, token_id]`
pub fn token_shielded_pool_path(token_id: &[u8; 32]) -> [&[u8]; 3] {
    [
        Into::<&[u8; 1]>::into(RootTree::Tokens),
        &[TOKEN_SHIELDED_POOLS_KEY],
        token_id,
    ]
}

/// Path to a token's shielded pool as a vec
pub fn token_shielded_pool_path_vec(token_id: [u8; 32]) -> Vec<Vec<u8>> {
    vec![
        vec![RootTree::Tokens as u8],
        vec![TOKEN_SHIELDED_POOLS_KEY],
        token_id.to_vec(),
    ]
}

/// Path to a token pool's notes tree: `[Tokens, 224, token_id, [128]]`
pub fn token_shielded_pool_notes_path(token_id: &[u8; 32]) -> [&[u8]; 4] {
    [
        Into::<&[u8; 1]>::into(RootTree::Tokens),
        &[TOKEN_SHIELDED_POOLS_KEY],
        token_id,
        &[SHIELDED_NOTES_KEY],
    ]
}

/// Path to a token pool's notes tree as a vec
pub fn token_shielded_pool_notes_path_vec(token_id: [u8; 32]) -> Vec<Vec<u8>> {
    vec![
        vec![RootTree::Tokens as u8],
        vec![TOKEN_SHIELDED_POOLS_KEY],
        token_id.to_vec(),
        vec![SHIELDED_NOTES_KEY],
    ]
}

/// Path to a token pool's nullifiers tree: `[Tokens, 224, token_id, [64]]`
pub fn token_shielded_pool_nullifiers_path(token_id: &[u8; 32]) -> [&[u8]; 4] {
    [
        Into::<&[u8; 1]>::into(RootTree::Tokens),
        &[TOKEN_SHIELDED_POOLS_KEY],
        token_id,
        &[SHIELDED_NULLIFIERS_KEY],
    ]
}

/// Path to a token pool's nullifiers tree as a vec
pub fn token_shielded_pool_nullifiers_path_vec(token_id: [u8; 32]) -> Vec<Vec<u8>> {
    vec![
        vec![RootTree::Tokens as u8],
        vec![TOKEN_SHIELDED_POOLS_KEY],
        token_id.to_vec(),
        vec![SHIELDED_NULLIFIERS_KEY],
    ]
}

/// Path to a token pool's anchors tree: `[Tokens, 224, token_id, [192]]`
pub fn token_shielded_pool_anchors_path(token_id: &[u8; 32]) -> [&[u8]; 4] {
    [
        Into::<&[u8; 1]>::into(RootTree::Tokens),
        &[TOKEN_SHIELDED_POOLS_KEY],
        token_id,
        &[SHIELDED_ANCHORS_IN_POOL_KEY],
    ]
}

/// Path to a token pool's anchors tree as a vec
pub fn token_shielded_pool_anchors_path_vec(token_id: [u8; 32]) -> Vec<Vec<u8>> {
    vec![
        vec![RootTree::Tokens as u8],
        vec![TOKEN_SHIELDED_POOLS_KEY],
        token_id.to_vec(),
        vec![SHIELDED_ANCHORS_IN_POOL_KEY],
    ]
}

/// Path to a token pool's anchors-by-height tree: `[Tokens, 224, token_id, [96]]`
pub fn token_shielded_pool_anchors_by_height_path(token_id: &[u8; 32]) -> [&[u8]; 4] {
    [
        Into::<&[u8; 1]>::into(RootTree::Tokens),
        &[TOKEN_SHIELDED_POOLS_KEY],
        token_id,
        &[SHIELDED_ANCHORS_BY_HEIGHT_KEY],
    ]
}

/// Path to a token pool's anchors-by-height tree as a vec
pub fn token_shielded_pool_anchors_by_height_path_vec(token_id: [u8; 32]) -> Vec<Vec<u8>> {
    vec![
        vec![RootTree::Tokens as u8],
        vec![TOKEN_SHIELDED_POOLS_KEY],
        token_id.to_vec(),
        vec![SHIELDED_ANCHORS_BY_HEIGHT_KEY],
    ]
}

/// The token-pool twin of [`shielded_latest_recorded_anchor_path_query`]: a `limit 1` reverse
/// scan over the pool's anchors-by-height tree. Shared by the anchor recorder, the proven RPC
/// handler and the SDK-side verifier for the same byte-for-byte reason.
pub fn token_shielded_pool_latest_recorded_anchor_path_query(token_id: [u8; 32]) -> PathQuery {
    let mut query = Query::new();
    query.insert_all();
    query.left_to_right = false;
    PathQuery {
        path: token_shielded_pool_anchors_by_height_path_vec(token_id),
        query: SizedQuery {
            query,
            limit: Some(1),
            offset: None,
        },
    }
}

/// The `PathQuery` proving the spent status of `nullifiers` in a token pool: one key per
/// nullifier under the pool's nullifiers tree, limited to the number asked. Shared by the
/// state transition proof (`TokenShieldedTransfer`) and `Drive::verify_token_shielded_pool_nullifiers`
/// so prover and verifier agree byte-for-byte.
pub fn token_shielded_pool_nullifiers_path_query(
    token_id: [u8; 32],
    nullifiers: &[[u8; 32]],
) -> PathQuery {
    let mut query = Query::new();
    query.insert_keys(
        nullifiers
            .iter()
            .map(|nullifier| nullifier.to_vec())
            .collect(),
    );
    PathQuery {
        path: token_shielded_pool_nullifiers_path_vec(token_id),
        query: SizedQuery {
            query,
            limit: Some(nullifiers.len().min(u16::MAX as usize) as u16),
            offset: None,
        },
    }
}

/// Resolves the nullifiers path based on pool type.
///
/// Pool types:
/// - 0: Main credit shielded pool → `[ShieldedBalances, "M", [64]]`
/// - 1: Main token shielded pool (not supported: the Orchard note carries no asset identifier,
///   so tokens cannot share one pool)
/// - 2: Individual token shielded pool → `[Tokens, 224, token_id, [64]]`; `pool_identifier` is
///   the 32-byte token id
pub fn nullifiers_path_for_pool(
    pool_type: u32,
    pool_identifier: Option<&[u8]>,
) -> Result<Vec<Vec<u8>>, crate::error::Error> {
    use crate::error::drive::DriveError;
    use crate::error::Error;

    match pool_type {
        0 => Ok(shielded_credit_pool_nullifiers_path_vec()),
        1 => Err(Error::Drive(DriveError::NotSupported(
            "a shared token shielded pool is not supported: each token has its own pool",
        ))),
        2 => {
            let token_id: [u8; 32] = pool_identifier
                .ok_or(Error::Drive(DriveError::InvalidInput(
                    "an individual token shielded pool requires the token id as pool identifier"
                        .to_string(),
                )))?
                .try_into()
                .map_err(|_| {
                    Error::Drive(DriveError::InvalidInput(
                        "token shielded pool identifier must be a 32-byte token id".to_string(),
                    ))
                })?;
            Ok(token_shielded_pool_nullifiers_path_vec(token_id))
        }
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
    fn pool_type_1_returns_not_supported() {
        // A shared token pool does not exist: the Orchard note has no asset identifier.
        let err1 =
            nullifiers_path_for_pool(1, None).expect_err("pool type 1 should return NotSupported");
        assert!(matches!(err1, Error::Drive(DriveError::NotSupported(_))));
    }

    #[test]
    fn pool_type_2_resolves_the_token_pool_nullifiers_path() {
        let token_id = [0xFFu8; 32];
        let path = nullifiers_path_for_pool(2, Some(&token_id)).expect("token pool path");
        assert_eq!(path, token_shielded_pool_nullifiers_path_vec(token_id));
        assert_eq!(
            path,
            vec![
                vec![RootTree::Tokens as u8],
                vec![TOKEN_SHIELDED_POOLS_KEY],
                token_id.to_vec(),
                vec![SHIELDED_NULLIFIERS_KEY],
            ]
        );
        // The identifier is mandatory and must be a token id.
        assert!(matches!(
            nullifiers_path_for_pool(2, None),
            Err(Error::Drive(DriveError::InvalidInput(_)))
        ));
        assert!(matches!(
            nullifiers_path_for_pool(2, Some(&[1u8; 20])),
            Err(Error::Drive(DriveError::InvalidInput(_)))
        ));
    }

    #[test]
    fn token_pool_paths_share_the_credit_pool_child_keys() {
        // The token pool is the credit pool layout re-rooted under the token: every child key
        // (notes, nullifiers, anchors, anchors-by-height) is identical, so one implementation
        // serves both.
        let token_id = [7u8; 32];
        let pool = token_shielded_pool_path_vec(token_id);
        assert_eq!(pool.len(), 3);
        for (path, key) in [
            (
                token_shielded_pool_notes_path_vec(token_id),
                SHIELDED_NOTES_KEY,
            ),
            (
                token_shielded_pool_nullifiers_path_vec(token_id),
                SHIELDED_NULLIFIERS_KEY,
            ),
            (
                token_shielded_pool_anchors_path_vec(token_id),
                SHIELDED_ANCHORS_IN_POOL_KEY,
            ),
            (
                token_shielded_pool_anchors_by_height_path_vec(token_id),
                SHIELDED_ANCHORS_BY_HEIGHT_KEY,
            ),
        ] {
            assert_eq!(&path[..3], &pool[..]);
            assert_eq!(path[3], vec![key]);
        }
        // Static and vec forms agree.
        let arr = token_shielded_pool_notes_path(&token_id);
        let v = token_shielded_pool_notes_path_vec(token_id);
        for (a, b) in arr.iter().zip(v.iter()) {
            assert_eq!(*a, b.as_slice());
        }
        assert_eq!(
            token_shielded_pool_latest_recorded_anchor_path_query(token_id).path,
            token_shielded_pool_anchors_by_height_path_vec(token_id)
        );
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

/// The proof query for a token pool's total balance: the `TOTAL_BALANCE` item of the pool.
pub fn token_shielded_pool_state_path_query(token_id: [u8; 32]) -> PathQuery {
    PathQuery {
        path: token_shielded_pool_path_vec(token_id),
        query: SizedQuery {
            query: Query::new_single_key(vec![SHIELDED_TOTAL_BALANCE_KEY]),
            limit: Some(1),
            offset: None,
        },
    }
}
