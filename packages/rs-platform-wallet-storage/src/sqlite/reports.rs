//! Outcome reports for the SQLite persister's `delete_wallet` and
//! `commit_writes` operations.
//!
//! These are SQLite-backend artifacts — the trait surface in
//! `platform-wallet` no longer carries `delete_wallet` / `commit_writes`,
//! so their reports live here next to the inherent methods that build
//! them.

use std::path::PathBuf;

use platform_wallet::changeset::PersistenceError;
use platform_wallet::wallet::platform_wallet::WalletId;

/// Outcome of a [`SqlitePersister::commit_writes`](crate::SqlitePersister::commit_writes) call.
///
/// Each dirty wallet's per-flush result lands in exactly one of the
/// three vectors so a single failed wallet doesn't hide its siblings'
/// success (or vice-versa). Callers can retry `still_pending` directly;
/// `failed` carries the classified `PersistenceError` per wallet so
/// transient-vs-fatal decisions stay local.
#[derive(Debug)]
pub struct CommitReport {
    /// Wallets that flushed successfully (durable on disk).
    pub succeeded: Vec<WalletId>,
    /// Wallets whose flush returned an error. The `PersistenceError`
    /// carries the classification and source per
    /// [`PersistenceErrorKind`](platform_wallet::changeset::PersistenceErrorKind).
    pub failed: Vec<(WalletId, PersistenceError)>,
    /// Wallets we never attempted because an earlier per-flush call
    /// short-circuited the loop (today: a `LockPoisoned` — the
    /// connection mutex is gone).
    pub still_pending: Vec<WalletId>,
}

impl CommitReport {
    /// `true` when every dirty wallet flushed cleanly.
    pub fn is_ok(&self) -> bool {
        self.failed.is_empty() && self.still_pending.is_empty()
    }
}

/// Outcome of a [`SqlitePersister::delete_wallet`](crate::SqlitePersister::delete_wallet) call.
///
/// The wallet's rows are removed by the native FK cascade plus the
/// `meta_*` soft-cascade triggers; the report carries only the deleted
/// id and the pre-delete backup path.
#[derive(Debug, Clone)]
pub struct DeleteWalletReport {
    /// The wallet that was deleted.
    pub wallet_id: WalletId,
    /// Absolute path of the pre-delete auto-backup taken before the
    /// cascade. `None` when the backup was skipped intentionally (e.g.
    /// the CLI's `--no-auto-backup`).
    pub backup_path: Option<PathBuf>,
}
