//! File-backed `ShieldedStore` impl.
//!
//! The Orchard commitment tree is shared across every subwallet
//! that decrypts notes against the same network — the on-chain
//! commitment stream is identical for every consumer. This store
//! therefore persists the tree to a SQLite file (via
//! [`ClientPersistentCommitmentTree`]) and keeps per-subwallet
//! decrypted notes / nullifier bookkeeping in memory, scoped by
//! [`SubwalletId`]. Notes are rediscovered on cold start by
//! re-running [`ShieldedWallet::sync_notes`] against the cached
//! tree (or, when the host persister is wired up, restored from
//! SwiftData before sync runs).

use std::collections::BTreeMap;
use std::error::Error as StdError;
use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use grovedb_commitment_tree::{ClientPersistentCommitmentTree, Position, Retention};

use super::store::{
    ShieldedNote, ShieldedOutgoingNote, ShieldedStore, SubwalletId, SubwalletState,
};
use crate::wallet::platform_wallet::WalletId;

/// Error type for [`FileBackedShieldedStore`].
#[derive(Debug)]
pub struct FileShieldedStoreError(pub String);

impl fmt::Display for FileShieldedStoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl StdError for FileShieldedStoreError {}

/// File-backed shielded store: SQLite-persisted commitment tree
/// plus in-memory per-subwallet decrypted notes / nullifier
/// bookkeeping.
pub struct FileBackedShieldedStore {
    /// SQLite-backed commitment tree. Wrapped in a `Mutex` because
    /// the underlying SQLite store is not `Sync`; the
    /// [`ShieldedStore`] trait requires `Send + Sync`. Outer
    /// concurrency is still serialized through `ShieldedWallet`'s
    /// `RwLock<S>`; this inner mutex is just a `Sync`-restoring
    /// shim and is uncontended in practice.
    tree: Mutex<ClientPersistentCommitmentTree>,
    /// Backing SQLite path, retained so [`reset_commitment_tree`]
    /// can wipe the on-disk tree tables and rebuild a fresh
    /// `ClientPersistentCommitmentTree` over the same file. The
    /// wrapper takes its `Connection` by value and exposes no
    /// public truncate, so a full reset reopens the tree rather
    /// than mutating the live handle in place.
    ///
    /// [`reset_commitment_tree`]: ShieldedStore::reset_commitment_tree
    path: PathBuf,
    /// `max_checkpoints` passed at open time, retained so the
    /// rebuilt tree in [`reset_commitment_tree`] matches the
    /// original retention policy.
    ///
    /// [`reset_commitment_tree`]: ShieldedStore::reset_commitment_tree
    max_checkpoints: usize,
    /// Per-subwallet notes + sync state, keyed by `(wallet_id,
    /// account_index)`. Lazily populated on first use of an id.
    subwallets: BTreeMap<SubwalletId, SubwalletState>,
}

impl FileBackedShieldedStore {
    /// Open or create a shielded store at `path`.
    ///
    /// SQLite is opened with **WAL journal + synchronous=NORMAL + temp_store=MEMORY**
    /// rather than the rusqlite defaults (DELETE + sync=FULL). Rationale: every
    /// `append_commitment` invocation runs an implicit one-statement transaction
    /// that, under DELETE+FULL, forces a fsync per cmx. On hosts where fsync is
    /// strictly honored (macOS Mac/simulator filesystems), that turns into the
    /// dominant cost of cold sync — a 1M-leaf tree build was ~6 min, vs ~17 s
    /// with the PRAGMAs below, per
    /// `packages/rs-platform-wallet/tests/shielded_tree_append_bench.rs`.
    ///
    /// `synchronous=NORMAL` retains crash-safety for the WAL (the WAL itself is
    /// fsync'd at checkpoint); we don't need `FULL` because no row in the
    /// commitment-tree SQLite is "user money" — every commitment is chain-side
    /// authenticated and can be rebuilt by re-running sync from a recorded
    /// `last_synced_note_index`. A torn WAL on power loss would at worst
    /// require resync from the last checkpoint, which is the same cost the
    /// host already accepts on a fresh install.
    pub fn open_path(
        path: impl AsRef<Path>,
        max_checkpoints: usize,
    ) -> Result<Self, FileShieldedStoreError> {
        let path = path.as_ref().to_path_buf();
        let conn = Self::open_tuned_connection(&path)?;
        let tree = ClientPersistentCommitmentTree::open(conn, max_checkpoints)
            .map_err(|e| FileShieldedStoreError(format!("open commitment tree: {e}")))?;
        Ok(Self {
            tree: Mutex::new(tree),
            path,
            max_checkpoints,
            subwallets: BTreeMap::new(),
        })
    }

    /// Open a `rusqlite::Connection` on `path` with the same WAL /
    /// `synchronous=NORMAL` / `temp_store=MEMORY` PRAGMAs the cold-sync
    /// append path depends on (see [`open_path`] for the rationale).
    ///
    /// Shared by [`open_path`] and [`reset_commitment_tree`] so any
    /// connection the store hands to `ClientPersistentCommitmentTree`
    /// — original or post-reset — is configured identically.
    ///
    /// [`open_path`]: Self::open_path
    /// [`reset_commitment_tree`]: ShieldedStore::reset_commitment_tree
    fn open_tuned_connection(path: &Path) -> Result<rusqlite::Connection, FileShieldedStoreError> {
        let conn = rusqlite::Connection::open(path)
            .map_err(|e| FileShieldedStoreError(format!("open sqlite: {e}")))?;
        // Pragmas must be applied before the schema is touched. They survive
        // for the lifetime of the connection; WAL also persists for any
        // subsequent reopen on the same file until explicitly changed.
        for (k, v) in [
            ("journal_mode", "WAL"),
            ("synchronous", "NORMAL"),
            ("temp_store", "MEMORY"),
        ] {
            conn.pragma_update(None, k, v)
                .map_err(|e| FileShieldedStoreError(format!("PRAGMA {k}={v}: {e}")))?;
        }
        Ok(conn)
    }
}

impl ShieldedStore for FileBackedShieldedStore {
    type Error = FileShieldedStoreError;

    fn save_note(&mut self, id: SubwalletId, note: &ShieldedNote) -> Result<(), Self::Error> {
        self.subwallets.entry(id).or_default().save_note(note);
        Ok(())
    }

    fn get_unspent_notes(&self, id: SubwalletId) -> Result<Vec<ShieldedNote>, Self::Error> {
        Ok(self
            .subwallets
            .get(&id)
            .map(SubwalletState::unspent_notes)
            .unwrap_or_default())
    }

    fn get_all_notes(&self, id: SubwalletId) -> Result<Vec<ShieldedNote>, Self::Error> {
        Ok(self
            .subwallets
            .get(&id)
            .map(SubwalletState::all_notes)
            .unwrap_or_default())
    }

    fn mark_spent(&mut self, id: SubwalletId, nullifier: &[u8; 32]) -> Result<bool, Self::Error> {
        Ok(self
            .subwallets
            .get_mut(&id)
            .map(|sw| sw.mark_spent(nullifier))
            .unwrap_or(false))
    }

    fn mark_pending(&mut self, id: SubwalletId, nullifier: &[u8; 32]) -> Result<bool, Self::Error> {
        Ok(self
            .subwallets
            .entry(id)
            .or_default()
            .mark_pending(nullifier))
    }

    fn clear_pending(
        &mut self,
        id: SubwalletId,
        nullifier: &[u8; 32],
    ) -> Result<bool, Self::Error> {
        Ok(self
            .subwallets
            .get_mut(&id)
            .map(|sw| sw.clear_pending(nullifier))
            .unwrap_or(false))
    }

    fn record_outgoing_note(
        &mut self,
        id: SubwalletId,
        note: &ShieldedOutgoingNote,
    ) -> Result<bool, Self::Error> {
        Ok(self
            .subwallets
            .entry(id)
            .or_default()
            .record_outgoing_note(note))
    }

    fn get_outgoing_notes(
        &self,
        id: SubwalletId,
    ) -> Result<Vec<ShieldedOutgoingNote>, Self::Error> {
        Ok(self
            .subwallets
            .get(&id)
            .map(SubwalletState::outgoing_notes)
            .unwrap_or_default())
    }

    fn save_activity(
        &mut self,
        id: SubwalletId,
        entry: &super::activity::ShieldedActivityEntry,
    ) -> Result<(), Self::Error> {
        self.subwallets.entry(id).or_default().save_activity(entry);
        Ok(())
    }

    fn get_activity(
        &self,
        id: SubwalletId,
        offset: usize,
        limit: usize,
    ) -> Result<Vec<super::activity::ShieldedActivityEntry>, Self::Error> {
        Ok(self
            .subwallets
            .get(&id)
            .map(|sw| sw.activity_page(offset, limit))
            .unwrap_or_default())
    }

    fn get_activity_by_entry_id(
        &self,
        id: SubwalletId,
        entry_id: &[u8; 32],
    ) -> Result<Option<super::activity::ShieldedActivityEntry>, Self::Error> {
        Ok(self
            .subwallets
            .get(&id)
            .and_then(|sw| sw.activity_by_id(entry_id)))
    }

    fn get_activity_ids(
        &self,
        id: SubwalletId,
    ) -> Result<std::collections::BTreeSet<[u8; 32]>, Self::Error> {
        Ok(self
            .subwallets
            .get(&id)
            .map(SubwalletState::activity_ids)
            .unwrap_or_default())
    }

    fn append_commitment(&mut self, cmx: &[u8; 32], marked: bool) -> Result<(), Self::Error> {
        let retention: Retention<u32> = if marked {
            Retention::Marked
        } else {
            Retention::Ephemeral
        };
        let mut tree = self
            .tree
            .lock()
            .map_err(|e| FileShieldedStoreError(format!("tree mutex poisoned: {e}")))?;
        tree.append(*cmx, retention)
            .map_err(|e| FileShieldedStoreError(format!("append commitment: {e}")))
    }

    fn checkpoint_tree(&mut self, checkpoint_id: u32) -> Result<(), Self::Error> {
        let mut tree = self
            .tree
            .lock()
            .map_err(|e| FileShieldedStoreError(format!("tree mutex poisoned: {e}")))?;
        tree.checkpoint(checkpoint_id)
            .map(|_| ())
            .map_err(|e| FileShieldedStoreError(format!("checkpoint tree: {e}")))
    }

    fn tree_anchor(&self) -> Result<[u8; 32], Self::Error> {
        let tree = self
            .tree
            .lock()
            .map_err(|e| FileShieldedStoreError(format!("tree mutex poisoned: {e}")))?;
        tree.anchor()
            .map(|a| a.to_bytes())
            .map_err(|e| FileShieldedStoreError(format!("read tree anchor: {e}")))
    }

    fn witness(
        &self,
        position: u64,
    ) -> Result<Option<grovedb_commitment_tree::MerklePath>, Self::Error> {
        let tree = self
            .tree
            .lock()
            .map_err(|e| FileShieldedStoreError(format!("tree mutex poisoned: {e}")))?;
        // `checkpoint_depth = 0` = current tree state. The Halo 2
        // proof we're about to build uses `tree_anchor()` — also
        // depth 0 — so the witness root must agree.
        tree.witness(Position::from(position), 0)
            .map_err(|e| FileShieldedStoreError(format!("witness({position}): {e}")))
    }

    fn tree_size(&self) -> Result<u64, Self::Error> {
        let tree = self
            .tree
            .lock()
            .map_err(|e| FileShieldedStoreError(format!("tree mutex poisoned: {e}")))?;
        let size = tree
            .max_leaf_position()
            .map_err(|e| FileShieldedStoreError(format!("read tree size: {e}")))?
            .map(|p| u64::from(p) + 1)
            .unwrap_or(0);
        Ok(size)
    }

    fn last_synced_note_index(&self, id: SubwalletId) -> Result<u64, Self::Error> {
        Ok(self
            .subwallets
            .get(&id)
            .map(|sw| sw.last_synced_index)
            .unwrap_or(0))
    }

    fn set_last_synced_note_index(
        &mut self,
        id: SubwalletId,
        index: u64,
    ) -> Result<(), Self::Error> {
        self.subwallets.entry(id).or_default().last_synced_index = index;
        Ok(())
    }

    fn purge_wallet(&mut self, wallet_id: WalletId) -> Result<(), Self::Error> {
        // Per-subwallet note / watermark / checkpoint state is
        // in-memory only (`subwallets`); the commitment tree in
        // SQLite is chain-wide and intentionally left intact.
        self.subwallets.retain(|id, _| id.wallet_id != wallet_id);
        Ok(())
    }

    fn purge_all_subwallets(&mut self) -> Result<(), Self::Error> {
        self.subwallets.clear();
        Ok(())
    }

    fn reset_commitment_tree(&mut self) -> Result<(), Self::Error> {
        // The `ClientPersistentCommitmentTree` wrapper owns its
        // `Connection` and exposes no public truncate (only the inner
        // `SqliteShardStore` has `truncate_shards`). A full reset
        // therefore (1) wipes the four `commitment_tree_*` tables on a
        // fresh connection, then (2) rebuilds the wrapper over the now
        // empty DB so the in-memory shardtree frontier/cap reflect the
        // empty state. Reopening — rather than mutating the live tree —
        // is what guarantees `tree_size()` reads back 0: the wrapper
        // caches frontier nodes that a bare `DELETE` wouldn't clear.
        let mut tree = self
            .tree
            .lock()
            .map_err(|e| FileShieldedStoreError(format!("tree mutex poisoned: {e}")))?;

        {
            let conn = Self::open_tuned_connection(&self.path)?;
            // `commitment_tree_cap` is included alongside the three
            // shard/checkpoint tables: it caches upper-level tree nodes,
            // so leaving it populated while the shards are empty would
            // reopen into an inconsistent (non-empty) tree state.
            conn.execute_batch(
                "DELETE FROM commitment_tree_checkpoint_marks_removed;
                 DELETE FROM commitment_tree_checkpoints;
                 DELETE FROM commitment_tree_shards;
                 DELETE FROM commitment_tree_cap;",
            )
            .map_err(|e| FileShieldedStoreError(format!("reset commitment tree tables: {e}")))?;
        }

        let conn = Self::open_tuned_connection(&self.path)?;
        *tree = ClientPersistentCommitmentTree::open(conn, self.max_checkpoints)
            .map_err(|e| FileShieldedStoreError(format!("reopen commitment tree: {e}")))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Unique temp path for a test tree (no `tempfile` dev-dep).
    fn temp_tree_path(tag: &str) -> std::path::PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("shielded_tree_test_{tag}_{nanos}.sqlite"))
    }

    /// Regression test for the "Shielded Merkle witness
    /// unavailable" spend failure (multi-wallet shared-tree bug).
    ///
    /// Root cause: the shared commitment tree previously appended
    /// commitments as `Ephemeral` unless the owning wallet's IVK
    /// recognized them in that very sync pass. With multiple
    /// wallets sharing one tree and binding at different times, a
    /// note appended before its owner bound stayed Ephemeral
    /// forever — shardtree has no retroactive marking — so the
    /// balance showed but the spend failed to build a witness.
    /// Observed on-disk symptom: every position un-witnessable
    /// (missing internal nodes at `Level(2) index 0` /
    /// `Level(1) index 2`).
    ///
    /// The fix: the shared tree marks EVERY position
    /// (`append_commitment(.., true)`); per-wallet ownership is
    /// tracked separately in the notes store. This test asserts
    /// that a fully-marked tree witnesses every position —
    /// including the rightmost (frontier) leaf whose sibling
    /// doesn't exist yet — across a persist + reload cycle (the
    /// cross-session round-trip a real wallet does between sync
    /// and spend).
    #[test]
    fn all_marked_tree_witnesses_every_position_after_reload() {
        let path = temp_tree_path("all_marked");
        let mut store = FileBackedShieldedStore::open_path(&path, 100).unwrap();

        // Mirror the real failing wallet's tree shape: 6
        // commitments, single checkpoint at the tip. The fix
        // marks ALL of them regardless of ownership.
        const N: u64 = 6;
        for i in 0..N {
            let mut cmx = [0u8; 32];
            cmx[0] = (i as u8) + 1; // distinct non-zero leaves
            store.append_commitment(&cmx, true).unwrap();
        }
        store.checkpoint_tree(N as u32).unwrap();

        // Persist to SQLite and reopen — the wallet builds the
        // tree in one app session and witnesses it (at spend
        // time) in a later one.
        drop(store);
        let store = FileBackedShieldedStore::open_path(&path, 100).unwrap();

        let mut failures = Vec::new();
        for pos in 0..N {
            match store.witness(pos) {
                Ok(Some(_)) => {}
                Ok(None) => failures.push(format!("position {pos}: witness returned None")),
                Err(e) => failures.push(format!("position {pos}: {e}")),
            }
        }

        let _ = std::fs::remove_file(&path);

        assert!(
            failures.is_empty(),
            "every position in a fully-marked tree must be witnessable, but: {failures:?}"
        );
    }

    /// `tree_size()` is the append gate the multi-subwallet sync
    /// relies on to stay idempotent (it appends only positions
    /// `>= tree_size`). If the count were wrong — or didn't survive
    /// the persist + reload the wallet does between sessions — a
    /// re-fetch from a chunk boundary would double-append and
    /// corrupt the tree ("Anchor not found in the recorded anchors
    /// tree" on the next spend). This asserts the count is exact
    /// from empty, after appends, and across a reopen.
    #[test]
    fn tree_size_tracks_leaf_count_across_reload() {
        let path = temp_tree_path("tree_size");
        let mut store = FileBackedShieldedStore::open_path(&path, 100).unwrap();

        assert_eq!(store.tree_size().unwrap(), 0, "empty tree has size 0");

        const N: u64 = 6;
        for i in 0..N {
            let mut cmx = [0u8; 32];
            cmx[0] = (i as u8) + 1;
            store.append_commitment(&cmx, true).unwrap();
            assert_eq!(
                store.tree_size().unwrap(),
                i + 1,
                "size must equal leaves appended so far"
            );
        }
        store.checkpoint_tree(N as u32).unwrap();

        drop(store);
        let store = FileBackedShieldedStore::open_path(&path, 100).unwrap();

        let size = store.tree_size().unwrap();
        let _ = std::fs::remove_file(&path);

        assert_eq!(
            size, N,
            "tree size must survive persist + reload — the append gate \
             reads it on cold start to avoid re-appending existing leaves"
        );
    }

    /// `reset_commitment_tree()` must empty the shared tree back to
    /// zero leaves so the host's "Clear" action becomes a true cold
    /// rebuild: after a reset, `tree_size()` is 0, a fresh append
    /// starts at position 0, and the emptied state survives a
    /// persist + reload (the on-disk tables are genuinely wiped, not
    /// just the in-memory frontier). Without this, Clear rewinds the
    /// per-subwallet watermark to 0 but leaves the tree at its full
    /// size, so every re-downloaded position is gate-skipped and the
    /// "Checked" progress bar stalls.
    #[test]
    fn reset_commitment_tree_empties_and_allows_reappend_from_zero() {
        let path = temp_tree_path("reset");
        let mut store = FileBackedShieldedStore::open_path(&path, 100).unwrap();

        // Build a non-trivial tree.
        const N: u64 = 6;
        for i in 0..N {
            let mut cmx = [0u8; 32];
            cmx[0] = (i as u8) + 1;
            store.append_commitment(&cmx, true).unwrap();
        }
        store.checkpoint_tree(N as u32).unwrap();
        assert_eq!(
            store.tree_size().unwrap(),
            N,
            "precondition: tree holds N leaves before reset"
        );

        // Reset wipes it back to empty.
        store.reset_commitment_tree().unwrap();
        assert_eq!(
            store.tree_size().unwrap(),
            0,
            "tree_size must be 0 immediately after reset"
        );

        // A fresh append starts at position 0 again and the count
        // climbs from there — the cold-rebuild contract Clear relies on.
        let mut cmx = [0u8; 32];
        cmx[0] = 42;
        store.append_commitment(&cmx, true).unwrap();
        assert_eq!(
            store.tree_size().unwrap(),
            1,
            "first post-reset append must land at position 0 (size 1)"
        );
        store.checkpoint_tree(1).unwrap();

        // The emptied + re-appended state must survive persist +
        // reload, proving the reset wiped the on-disk tables rather
        // than only the in-memory frontier.
        drop(store);
        let store = FileBackedShieldedStore::open_path(&path, 100).unwrap();
        let size = store.tree_size().unwrap();
        let _ = std::fs::remove_file(&path);

        assert_eq!(
            size, 1,
            "post-reset tree state (1 leaf) must survive persist + reload, \
             confirming reset cleared the SQLite tree tables"
        );
    }

    /// Reproduces the shielded **withdrawal-never-lands** root cause (TestFlight
    /// report B): the wallet builds a spend against its depth-0 (current) tree
    /// root, but that root is a *Platform-recorded* anchor only when the tree
    /// sits exactly on a block boundary.
    ///
    /// - The spend anchor is `witness(pos, 0).root(cmx)`, which equals
    ///   `tree_anchor()` (both depth-0; see the comment on `witness`). This test
    ///   asserts that equality directly.
    /// - The wallet syncs commitments by index-chunk (`CHUNK_SIZE = 2048` in
    ///   `sync.rs`), **not** by block, so its tree routinely stops mid-block.
    /// - drive records **one anchor per block** (`record_anchor_if_changed` at
    ///   block-processing-end) and `validate_anchor_exists` rejects any anchor
    ///   it never recorded (`InvalidAnchorError`).
    ///
    /// So a mid-block depth-0 anchor is rejected every attempt — repeatable,
    /// never lands, funds untouched. The team already names this failure at the
    /// `tree_size` test above ("Anchor not found in the recorded anchors").
    #[test]
    fn depth0_spend_anchor_mid_block_is_not_a_recorded_block_boundary_anchor() {
        use grovedb_commitment_tree::ExtractedNoteCommitment;

        let path = temp_tree_path("anchor_midblock");
        let mut store = FileBackedShieldedStore::open_path(&path, 100).unwrap();

        let cmx = |b: u8| {
            let mut c = [0u8; 32];
            c[0] = b;
            c
        };

        // Two blocks of commitments. drive records ONE anchor per block, at
        // block-processing-end (after ALL of that block's commitments):
        //   block 1 = commitments 1..=3  -> recorded anchor at tree size 3
        //   block 2 = commitments 4..=6  -> recorded anchor at tree size 6
        for b in 1..=3u8 {
            store.append_commitment(&cmx(b), true).unwrap();
        }
        store.checkpoint_tree(3).unwrap();
        let recorded_after_block1 = store.tree_anchor().unwrap();

        // The index-chunk sync appends block 2's commitments incrementally; a
        // chunk/stream boundary that lands mid-block (the common case — a
        // 2048-leaf chunk rarely ends on a block boundary) leaves the wallet at
        // tree size 4, and it checkpoints there (sync.rs checkpoints at the
        // post-append leaf count). Its depth-0 anchor is now the root at size 4
        // — a state drive never recorded.
        store.append_commitment(&cmx(4), true).unwrap();
        store.checkpoint_tree(4).unwrap();
        let wallet_depth0_mid_block = store.tree_anchor().unwrap();

        // The spend path uses exactly this anchor: `extract_spends_and_anchor`
        // builds it as `witness(pos, 0).root(cmx)`. Pin that it equals the
        // mid-block `tree_anchor()`.
        let cmx0 = ExtractedNoteCommitment::from_bytes(&cmx(1))
            .into_option()
            .expect("valid cmx");
        let spend_anchor = store
            .witness(0)
            .unwrap()
            .expect("witness for marked position 0")
            .root(cmx0)
            .to_bytes();
        assert_eq!(
            spend_anchor, wallet_depth0_mid_block,
            "the spend anchor (depth-0 witness root) must equal the mid-block tree_anchor"
        );

        // Finish block 2. drive records the anchor at tree size 6.
        store.append_commitment(&cmx(5), true).unwrap();
        store.append_commitment(&cmx(6), true).unwrap();
        store.checkpoint_tree(6).unwrap();
        let recorded_after_block2 = store.tree_anchor().unwrap();

        let _ = std::fs::remove_file(&path);

        // drive's recorded anchor set is {block1, block2}. The wallet's mid-block
        // spend anchor is neither -> `validate_anchor_exists` rejects it with
        // InvalidAnchorError, and the withdrawal never lands.
        assert_ne!(
            wallet_depth0_mid_block, recorded_after_block1,
            "mid-block spend anchor must differ from block 1's recorded anchor"
        );
        assert_ne!(
            wallet_depth0_mid_block, recorded_after_block2,
            "mid-block spend anchor must differ from block 2's recorded anchor"
        );
        assert_ne!(
            recorded_after_block1, recorded_after_block2,
            "the two block-boundary anchors differ (the tree grew), so drive's \
             recorded set is exactly these two and the mid-block anchor is outside it"
        );
    }
}
