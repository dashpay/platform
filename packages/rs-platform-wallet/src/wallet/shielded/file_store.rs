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

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error as StdError;
use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use dpp::serialization::{PlatformDeserializableTrusted, PlatformSerializable};
use dpp::state_transition::StateTransition;
use grovedb_commitment_tree::{ClientPersistentCommitmentTree, Position, Retention};

use super::balance::{ShieldedBalanceSource, ShieldedLocalAccountBalance};
use super::store::{
    PendingRedrive, PrivateContactPayment, PrivatePaymentRecord, ShieldedNote,
    ShieldedOutgoingNote, ShieldedStore, StalePendingSpend, SubwalletId, SubwalletState,
};
use crate::error::PlatformWalletError;
use crate::wallet::platform_wallet::WalletId;

/// The note-spend kind of a private document create from the shielded pool
/// (type 29), the transition a private contact request payment is.
const PRIVATE_DOCUMENT_CREATE_KIND: &str = "private document create from shielded pool";

/// A private payment journal row: the entry's commitment, which keys it, and
/// the record's bytes ([`PrivatePaymentRecord::to_bytes`]).
type JournalRow = ([u8; 32], Vec<u8>);

/// Error type for [`FileBackedShieldedStore`].
#[derive(Debug)]
pub enum FileShieldedStoreError {
    /// SQLite or commitment-tree operation failed.
    Storage(String),
    /// A durable recovery row cannot be interpreted safely. It is retained.
    RecoveryCorrupted {
        account_index: Option<u32>,
        reason: String,
    },
}

impl fmt::Display for FileShieldedStoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Storage(message) => f.write_str(message),
            Self::RecoveryCorrupted {
                account_index,
                reason,
            } => write!(
                f,
                "damaged shielded recovery record (account {account_index:?}): {reason}"
            ),
        }
    }
}

impl StdError for FileShieldedStoreError {}

impl From<FileShieldedStoreError> for PlatformWalletError {
    fn from(error: FileShieldedStoreError) -> Self {
        match error {
            FileShieldedStoreError::Storage(message) => Self::ShieldedStoreError(message),
            FileShieldedStoreError::RecoveryCorrupted {
                account_index,
                reason,
            } => Self::ShieldedRecoveryCorrupted {
                account_index,
                reason,
            },
        }
    }
}

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
    /// Second connection on the same SQLite file, owning the recovery
    /// tables: `shielded_pending_spends` (armed [`PendingRedrive`] records)
    /// and the private payment journal. Separate from `tree` because the
    /// commitment-tree wrapper takes its `Connection` by value; WAL mode
    /// makes the two-connection setup safe. It commits at
    /// `synchronous=FULL` ([`Self::open_recovery_connection`]): a recovery
    /// record saved before a broadcast must survive a power loss, unlike the
    /// rebuildable tree. `Mutex` for the same `Sync`-shim reason as `tree`.
    pending_conn: Mutex<rusqlite::Connection>,
    /// The private contact request each armed record pays for, by subwallet
    /// and activity id, as the `private_contact_payment` column keeps it.
    /// Read joined with the live records, so an entry whose record was
    /// dropped is passed over. A record with no association (SQL NULL, as
    /// one saved before associations were kept) has no entry.
    private_contact_payments: BTreeMap<(SubwalletId, [u8; 32]), SavedPrivateContactPayment>,
    /// The wallets of damaged records retained but not loaded that carry a
    /// private contact request association, `None` for one whose wallet does
    /// not read either: the payment they stand for cannot be restored, so
    /// restoring a covered wallet's reservations fails
    /// ([`ShieldedStore::private_contact_redrives_for_wallet`]).
    unloaded_private_contact_payments: BTreeSet<Option<WalletId>>,
    /// The private payment journal's records, by wallet and commitment, as
    /// the `shielded_private_payment_journal` table keeps them.
    private_payment_journal: BTreeMap<(WalletId, [u8; 32]), PrivatePaymentRecord>,
    /// The wallets whose private payment journal is open, as the
    /// `shielded_private_payment_journal_wallets` table keeps them.
    private_payment_journals: BTreeSet<WalletId>,
    /// The wallets of journal records that do not read back, `None` for one
    /// whose wallet does not read either. The row stays, and reading a
    /// covered wallet's journal fails, so its holds stay.
    unreadable_private_payment_journal: BTreeSet<Option<WalletId>>,
}

/// The private contact request association a saved record carries.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SavedPrivateContactPayment {
    /// The association, as it was saved.
    Readable(PrivateContactPayment),
    /// A non-null association that does not decode, or a column that does
    /// not read: restoring the record's wallet fails, so its holds stay.
    Unreadable,
}

impl FileBackedShieldedStore {
    /// Drop the private contact request associations of `id` whose record is
    /// gone, as its redrives are removed. The listing joins with the live
    /// records, so a left one restores nothing, but would stay in memory
    /// until the store is reopened.
    fn evict_private_contact_payments(&mut self, id: SubwalletId) {
        let live = self.subwallets.get(&id);
        self.private_contact_payments
            .retain(|(owner, activity_id), _| {
                *owner != id || live.is_some_and(|sw| sw.redrives.contains_key(activity_id))
            });
    }

    /// Persist `redrive`, with the private contact request it pays for when
    /// it does, and the journal `records` of the entries it appends, in one
    /// transaction committed at `synchronous=FULL`.
    fn arm_redrive_paying_for(
        &mut self,
        id: SubwalletId,
        redrive: PendingRedrive,
        payment: Option<PrivateContactPayment>,
        records: &[PrivatePaymentRecord],
    ) -> Result<(), FileShieldedStoreError> {
        let journal_rows = Self::journal_rows(records)?;
        {
            let mut conn = self
                .pending_conn
                .lock()
                .map_err(|e| FileShieldedStoreError::Storage(format!("pending_conn mutex: {e}")))?;
            let tx = conn
                .transaction()
                .map_err(|e| FileShieldedStoreError::Storage(format!("begin redrive: {e}")))?;
            let nullifier_blob: Vec<u8> = redrive.nullifiers.iter().flatten().copied().collect();
            tx.execute(
                "INSERT OR REPLACE INTO shielded_pending_spends \
                 (wallet_id, account_index, activity_id, anchor, nullifiers, st_bytes, attempts, identity_nonce_finalized, identity_user_abandoned, private_contact_payment, anchor_height) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
                rusqlite::params![
                    id.wallet_id.as_slice(),
                    id.account_index,
                    redrive.activity_id.as_slice(),
                    redrive.anchor.as_slice(),
                    nullifier_blob,
                    redrive.st_bytes,
                    redrive.attempts,
                    redrive.identity_nonce_finalized,
                    redrive.identity_user_abandoned,
                    payment.map(|payment| payment.to_bytes().to_vec()),
                    // A height past SQLite's integer range is kept unknown,
                    // which only holds the notes longer.
                    redrive
                        .anchor_height
                        .and_then(|height| i64::try_from(height).ok()),
                ],
            )
            .map_err(|e| FileShieldedStoreError::Storage(format!("persist redrive: {e}")))?;
            Self::insert_journal_rows(&tx, id.wallet_id, &journal_rows)?;
            tx.commit()
                .map_err(|e| FileShieldedStoreError::Storage(format!("commit redrive: {e}")))?;
        }
        if !records.is_empty() {
            self.private_payment_journals.insert(id.wallet_id);
        }
        for record in records {
            self.private_payment_journal
                .insert((id.wallet_id, record.commitment), record.clone());
        }
        match payment {
            Some(payment) => {
                self.private_contact_payments.insert(
                    (id, redrive.activity_id),
                    SavedPrivateContactPayment::Readable(payment),
                );
            }
            None => {
                self.private_contact_payments
                    .remove(&(id, redrive.activity_id));
            }
        }
        self.subwallets.entry(id).or_default().arm_redrive(redrive);
        Ok(())
    }

    /// The rows `records` are kept as: commitment and record bytes. Fails
    /// for a record [`PrivatePaymentRecord::to_bytes`] refuses.
    fn journal_rows(
        records: &[PrivatePaymentRecord],
    ) -> Result<Vec<JournalRow>, FileShieldedStoreError> {
        records
            .iter()
            .map(|record| {
                record
                    .to_bytes()
                    .map(|bytes| (record.commitment, bytes))
                    .ok_or_else(|| {
                        FileShieldedStoreError::Storage(
                            "a private payment's journal record spends no note, or too many"
                                .to_string(),
                        )
                    })
            })
            .collect()
    }

    /// Insert `rows` into `wallet_id`'s journal within `tx`, opening the
    /// journal too.
    fn insert_journal_rows(
        tx: &rusqlite::Transaction<'_>,
        wallet_id: WalletId,
        rows: &[JournalRow],
    ) -> Result<(), FileShieldedStoreError> {
        if rows.is_empty() {
            return Ok(());
        }
        tx.execute(
            "INSERT OR IGNORE INTO shielded_private_payment_journal_wallets (wallet_id) \
             VALUES (?1)",
            rusqlite::params![wallet_id.as_slice()],
        )
        .map_err(|e| FileShieldedStoreError::Storage(format!("open payment journal: {e}")))?;
        for (commitment, bytes) in rows {
            tx.execute(
                "INSERT OR REPLACE INTO shielded_private_payment_journal \
                 (wallet_id, commitment, record) VALUES (?1, ?2, ?3)",
                rusqlite::params![wallet_id.as_slice(), commitment.as_slice(), bytes],
            )
            .map_err(|e| FileShieldedStoreError::Storage(format!("journal payment: {e}")))?;
        }
        Ok(())
    }

    /// Load the private payment journal into memory. A row that does not
    /// read back stays, and flags its wallet
    /// (`unreadable_private_payment_journal`); the open does not fail.
    fn load_private_payment_journal(&mut self) -> Result<(), FileShieldedStoreError> {
        let conn = self
            .pending_conn
            .lock()
            .map_err(|e| FileShieldedStoreError::Storage(format!("pending_conn mutex: {e}")))?;
        let mut journals = conn
            .prepare("SELECT wallet_id FROM shielded_private_payment_journal_wallets")
            .map_err(|e| FileShieldedStoreError::Storage(format!("prepare journals: {e}")))?;
        let wallets = journals
            .query_map([], |row| row.get::<_, Vec<u8>>(0))
            .map_err(|e| FileShieldedStoreError::Storage(format!("query journals: {e}")))?;
        for wallet in wallets {
            // A journal whose wallet does not read is not open for any wallet.
            if let Some(wallet_id) = wallet
                .ok()
                .and_then(|bytes| <[u8; 32]>::try_from(bytes.as_slice()).ok())
            {
                self.private_payment_journals.insert(wallet_id);
            }
        }
        drop(journals);
        let mut statement = conn
            .prepare("SELECT wallet_id, commitment, record FROM shielded_private_payment_journal")
            .map_err(|e| FileShieldedStoreError::Storage(format!("prepare journal: {e}")))?;
        let rows = statement
            .query_map([], |row| {
                Ok((
                    row.get::<_, Vec<u8>>(0).ok(),
                    row.get::<_, Vec<u8>>(1).ok(),
                    row.get::<_, Vec<u8>>(2).ok(),
                ))
            })
            .map_err(|e| FileShieldedStoreError::Storage(format!("query journal: {e}")))?;
        for row in rows {
            let (wallet_id, commitment, record) =
                row.map_err(|e| FileShieldedStoreError::Storage(format!("read journal: {e}")))?;
            let wallet_id = wallet_id.and_then(|bytes| <[u8; 32]>::try_from(bytes.as_slice()).ok());
            let read = wallet_id.and_then(|wallet_id| {
                let commitment = <[u8; 32]>::try_from(commitment?.as_slice()).ok()?;
                let record = PrivatePaymentRecord::from_bytes(&record?)?;
                (record.commitment == commitment).then_some((wallet_id, record))
            });
            match read {
                Some((wallet_id, record)) => {
                    self.private_payment_journal
                        .insert((wallet_id, record.commitment), record);
                }
                None => {
                    tracing::warn!(
                        "retaining a private payment journal record that does not read back; \
                         its wallet's private contact request holds stay"
                    );
                    self.unreadable_private_payment_journal.insert(wallet_id);
                }
            }
        }
        Ok(())
    }

    /// Called under the coordinator's store read guard so amount and provenance
    /// cannot straddle a sync or a pending-spend reservation.
    pub(super) fn local_account_balance(
        &self,
        id: SubwalletId,
    ) -> Result<ShieldedLocalAccountBalance, FileShieldedStoreError> {
        let state = self.subwallets.get(&id);
        Ok(ShieldedLocalAccountBalance {
            spendable_credits: self.spendable_balance(id)?,
            last_scanned_index: state.and_then(|state| state.last_scanned_index),
            source: state.map(|state| state.balance_source).unwrap_or_default(),
        })
    }

    /// Add restore provenance without downgrading a live scan or rewinding it.
    /// A positive legacy watermark also proves a prior scan even when an older
    /// Rust persister did not supply the new row-presence metadata.
    pub(super) fn restore_balance_metadata(
        &mut self,
        id: SubwalletId,
        index: u64,
        has_sync_state: bool,
        has_notes: bool,
    ) {
        let state = self.subwallets.entry(id).or_default();
        state.last_synced_index = state.last_synced_index.max(index);
        if has_sync_state || index > 0 {
            state.last_scanned_index = Some(state.last_scanned_index.unwrap_or(0).max(index));
        }
        if state.balance_source == ShieldedBalanceSource::NoHistory
            && (has_notes || has_sync_state || index > 0)
        {
            state.balance_source = ShieldedBalanceSource::Restored;
        }
    }

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
            .map_err(|e| FileShieldedStoreError::Storage(format!("open commitment tree: {e}")))?;
        let pending_conn = Self::open_recovery_connection(&path)?;
        pending_conn
            .execute(
                "CREATE TABLE IF NOT EXISTS shielded_pending_spends (
                    wallet_id     BLOB    NOT NULL,
                    account_index INTEGER NOT NULL,
                    activity_id   BLOB    NOT NULL,
                    anchor        BLOB    NOT NULL,
                    nullifiers    BLOB    NOT NULL,
                    st_bytes      BLOB    NOT NULL,
                    attempts      INTEGER NOT NULL DEFAULT 0,
                    identity_nonce_finalized INTEGER NOT NULL DEFAULT 0,
                    identity_user_abandoned INTEGER NOT NULL DEFAULT 0,
                    PRIMARY KEY (wallet_id, account_index, activity_id)
                )",
                [],
            )
            .map_err(|e| {
                FileShieldedStoreError::Storage(format!("create pending_spends table: {e}"))
            })?;
        // Additive, idempotent migration: old guards remain active.
        let columns = {
            let mut statement = pending_conn
                .prepare("PRAGMA table_info(shielded_pending_spends)")
                .map_err(|e| {
                    FileShieldedStoreError::Storage(format!("inspect pending schema: {e}"))
                })?;
            let rows = statement
                .query_map([], |row| row.get::<_, String>(1))
                .map_err(|e| {
                    FileShieldedStoreError::Storage(format!("read pending schema: {e}"))
                })?;
            rows.collect::<Result<Vec<_>, _>>()
                .map_err(|e| FileShieldedStoreError::Storage(format!("read pending column: {e}")))?
        };
        for column in ["identity_nonce_finalized", "identity_user_abandoned"] {
            if !columns.iter().any(|existing| existing == column) {
                pending_conn.execute(&format!("ALTER TABLE shielded_pending_spends ADD COLUMN {column} INTEGER NOT NULL DEFAULT 0"), [])
                    .map_err(|e| FileShieldedStoreError::Storage(format!("upgrade pending schema: {e}")))?;
            }
        }
        // A proved height at which the record's anchor was recorded, when
        // known (`PendingRedrive::anchor_height`): NULL for a record saved
        // before it was kept.
        if !columns.iter().any(|existing| existing == "anchor_height") {
            pending_conn
                .execute(
                    "ALTER TABLE shielded_pending_spends ADD COLUMN anchor_height INTEGER",
                    [],
                )
                .map_err(|e| {
                    FileShieldedStoreError::Storage(format!("upgrade pending schema: {e}"))
                })?;
        }
        // The private contact request a record pays for, when it does.
        if !columns
            .iter()
            .any(|existing| existing == "private_contact_payment")
        {
            pending_conn
                .execute(
                    "ALTER TABLE shielded_pending_spends ADD COLUMN private_contact_payment BLOB",
                    [],
                )
                .map_err(|e| {
                    FileShieldedStoreError::Storage(format!("upgrade pending schema: {e}"))
                })?;
        }
        // The private payment journal: one record per private contact request
        // payment that may have reached the network, by wallet and entry
        // commitment, and the wallets whose journal is open. Never purged.
        pending_conn
            .execute_batch(
                "CREATE TABLE IF NOT EXISTS shielded_private_payment_journal (
                    wallet_id  BLOB NOT NULL,
                    commitment BLOB NOT NULL,
                    record     BLOB NOT NULL,
                    PRIMARY KEY (wallet_id, commitment)
                );
                CREATE TABLE IF NOT EXISTS shielded_private_payment_journal_wallets (
                    wallet_id BLOB NOT NULL PRIMARY KEY
                );",
            )
            .map_err(|e| {
                FileShieldedStoreError::Storage(format!("create private payment journal: {e}"))
            })?;
        let mut store = Self {
            tree: Mutex::new(tree),
            path,
            max_checkpoints,
            subwallets: BTreeMap::new(),
            pending_conn: Mutex::new(pending_conn),
            private_contact_payments: BTreeMap::new(),
            unloaded_private_contact_payments: BTreeSet::new(),
            private_payment_journal: BTreeMap::new(),
            private_payment_journals: BTreeSet::new(),
            unreadable_private_payment_journal: BTreeSet::new(),
        };
        store.rehydrate_pending_spends()?;
        store.load_private_payment_journal()?;
        Ok(store)
    }

    /// Reload every persisted [`PendingRedrive`] into the in-memory
    /// per-subwallet state, re-arming both the redrive record and the
    /// note reservations its nullifiers carry — an unconfirmed
    /// broadcast therefore keeps its notes reserved (and its re-drive
    /// alive) across restarts. Malformed metadata for a possible identity debit
    /// fails the open without deleting the row. A row whose exact transition
    /// bytes prove it spends shielded notes can be retained and skipped safely:
    /// Platform's nullifier set still prevents those notes from being spent twice.
    fn rehydrate_pending_spends(&mut self) -> Result<(), FileShieldedStoreError> {
        let conn = self.pending_conn.lock().expect("pending_conn mutex");
        let mut stmt = conn
            .prepare(
                "SELECT wallet_id, account_index, activity_id, anchor, nullifiers, st_bytes, \
                 attempts, identity_nonce_finalized, identity_user_abandoned, \
                 private_contact_payment, anchor_height FROM shielded_pending_spends",
            )
            .map_err(|e| FileShieldedStoreError::Storage(format!("prepare rehydrate: {e}")))?;
        let rows = stmt
            .query_map([], |row| {
                // Read a separate copy of the exact transition bytes first so
                // another malformed column cannot hide a provable note spend.
                // A malformed st_bytes value remains unclassifiable and must
                // fail closed.
                let classifiable_st_bytes = row.get::<_, Vec<u8>>(5).ok();
                // The wallet, read on its own too, for a damaged record that is
                // retained without being loaded.
                let readable_wallet_id = row
                    .get::<_, Vec<u8>>(0)
                    .ok()
                    .and_then(|bytes| <[u8; 32]>::try_from(bytes.as_slice()).ok());
                // SQL NULL is a record with no association. Anything else that
                // does not read back fails restoring the wallet's reservations
                // rather than losing the pair it holds.
                let private_contact_payment = match row.get::<_, Option<Vec<u8>>>(9) {
                    Ok(None) => None,
                    Ok(Some(bytes)) => Some(
                        PrivateContactPayment::from_bytes(&bytes)
                            .map_or(SavedPrivateContactPayment::Unreadable, |payment| {
                                SavedPrivateContactPayment::Readable(payment)
                            }),
                    ),
                    Err(_) => Some(SavedPrivateContactPayment::Unreadable),
                };
                // A height that does not read is not known: the anchor is then
                // not proven pruned until a set is seen holding it.
                let anchor_height = row
                    .get::<_, Option<i64>>(10)
                    .ok()
                    .flatten()
                    .and_then(|height| u64::try_from(height).ok());
                let decoded: rusqlite::Result<_> = (|| {
                    Ok((
                        row.get::<_, Vec<u8>>(0)?,
                        row.get::<_, u32>(1)?,
                        row.get::<_, Vec<u8>>(2)?,
                        row.get::<_, Vec<u8>>(3)?,
                        row.get::<_, Vec<u8>>(4)?,
                        row.get::<_, Vec<u8>>(5)?,
                        row.get::<_, u32>(6)?,
                        row.get::<_, i64>(7)?,
                        row.get::<_, i64>(8)?,
                    ))
                })();
                Ok((
                    classifiable_st_bytes,
                    readable_wallet_id,
                    private_contact_payment,
                    anchor_height,
                    decoded,
                ))
            })
            .map_err(|e| FileShieldedStoreError::Storage(format!("query rehydrate: {e}")))?;
        for row in rows {
            let (
                classifiable_st_bytes,
                readable_wallet_id,
                private_contact_payment,
                anchor_height,
                decoded,
            ) = row.map_err(|e| FileShieldedStoreError::RecoveryCorrupted {
                account_index: None,
                reason: format!("cannot decode recovery row: {e}"),
            })?;
            let note_spend_kind = classifiable_st_bytes
                .as_deref()
                .and_then(Self::non_identity_note_spend_kind);
            let (
                wallet_id,
                account_index,
                activity_id,
                anchor,
                nullifiers,
                st_bytes,
                attempts,
                identity_nonce_finalized,
                identity_user_abandoned,
            ) = match decoded {
                Ok(decoded) => decoded,
                Err(error) => {
                    if let Some(kind) = note_spend_kind {
                        tracing::warn!(
                            transition = kind,
                            error = %error,
                            "retaining but skipping a damaged shielded note-spend recovery row"
                        );
                        if private_contact_payment.is_some() || kind == PRIVATE_DOCUMENT_CREATE_KIND
                        {
                            self.unloaded_private_contact_payments
                                .insert(readable_wallet_id);
                        }
                        continue;
                    }
                    return Err(FileShieldedStoreError::RecoveryCorrupted {
                        account_index: None,
                        reason: format!("cannot decode recovery row: {error}"),
                    });
                }
            };
            let decoded = (|| {
                let corrupted = |reason: &str| FileShieldedStoreError::RecoveryCorrupted {
                    account_index: Some(account_index),
                    reason: reason.to_owned(),
                };
                if !matches!(identity_nonce_finalized, 0 | 1)
                    || !matches!(identity_user_abandoned, 0 | 1)
                {
                    return Err(corrupted("invalid recovery-state flag"));
                }
                let identity_nonce_finalized = identity_nonce_finalized == 1;
                let identity_user_abandoned = identity_user_abandoned == 1;
                if !nullifiers.is_empty() && (identity_nonce_finalized || identity_user_abandoned) {
                    return Err(corrupted("identity recovery flags on a note-spend record"));
                }
                // Routing and account purge use the nullifier metadata. It must not
                // disguise a still-executable identity debit as a note spend merely
                // because its active recovery flags are both false.
                if !nullifiers.is_empty()
                    && matches!(
                        StateTransition::deserialize_from_bytes_trusted(&st_bytes),
                        Ok(StateTransition::ShieldFromIdentity(_))
                    )
                {
                    return Err(corrupted("identity debit contains note-spend nullifiers"));
                }
                let (Ok(wallet_id), Ok(activity_id), Ok(anchor)) = (
                    <[u8; 32]>::try_from(wallet_id.as_slice()),
                    <[u8; 32]>::try_from(activity_id.as_slice()),
                    <[u8; 32]>::try_from(anchor.as_slice()),
                ) else {
                    return Err(corrupted(
                        "invalid wallet, activity, or anchor identifier width",
                    ));
                };
                // Empty nullifiers identify a ShieldFromIdentity retry guard.
                // Its exact transition bytes, rather than note reservations,
                // preserve idempotency across restarts.
                if nullifiers.len() % 32 != 0 {
                    return Err(corrupted("invalid nullifier width"));
                }
                let nullifiers: Vec<[u8; 32]> = nullifiers.as_chunks::<32>().0.to_vec();
                Ok((
                    SubwalletId::new(wallet_id, account_index),
                    PendingRedrive {
                        activity_id,
                        anchor,
                        nullifiers,
                        st_bytes,
                        attempts,
                        anchor_height,
                        identity_nonce_finalized,
                        identity_user_abandoned,
                    },
                ))
            })();
            let (id, redrive) = match decoded {
                Ok(decoded) => decoded,
                Err(error) => {
                    if let Some(kind) = note_spend_kind {
                        tracing::warn!(
                            transition = kind,
                            account_index,
                            reason = %error,
                            "retaining but skipping a damaged shielded note-spend recovery row"
                        );
                        if private_contact_payment.is_some() || kind == PRIVATE_DOCUMENT_CREATE_KIND
                        {
                            self.unloaded_private_contact_payments
                                .insert(readable_wallet_id);
                        }
                        continue;
                    }
                    return Err(error);
                }
            };
            if let Some(payment) = private_contact_payment {
                self.private_contact_payments
                    .insert((id, redrive.activity_id), payment);
            }
            let sw = self.subwallets.entry(id).or_default();
            for n in &redrive.nullifiers {
                sw.mark_pending(n);
                sw.set_pending_spend(
                    n,
                    redrive.anchor,
                    redrive.activity_id,
                    redrive.anchor_height,
                );
            }
            sw.arm_redrive(redrive);
        }
        Ok(())
    }

    /// Identify only transition families whose funding comes from shielded
    /// note nullifiers. A complete outer transition decode is required so
    /// corrupt, unknown, or identity-funded bytes remain fail-closed.
    fn non_identity_note_spend_kind(st_bytes: &[u8]) -> Option<&'static str> {
        let state_transition = StateTransition::deserialize_from_bytes_trusted(st_bytes).ok()?;
        if state_transition.serialize_to_bytes().ok()?.as_slice() != st_bytes {
            return None;
        }
        match state_transition {
            StateTransition::ShieldedTransfer(_) => Some("shielded transfer"),
            StateTransition::Unshield(_) => Some("unshield"),
            StateTransition::ShieldedWithdrawal(_) => Some("shielded withdrawal"),
            StateTransition::IdentityCreateFromShieldedPool(_) => {
                Some("identity create from shielded pool")
            }
            StateTransition::IdentityTopUpFromShieldedPool(_) => {
                Some("identity top up from shielded pool")
            }
            StateTransition::PrivateDocumentCreateFromShieldedPool(_) => {
                Some(PRIVATE_DOCUMENT_CREATE_KIND)
            }
            _ => None,
        }
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
    /// [`Self::open_tuned_connection`] committing at `synchronous=FULL`, for
    /// the recovery tables: in WAL mode that syncs the WAL on every commit,
    /// so a recovery record saved before a broadcast survives a crash or
    /// power loss. The commitment tree's connection keeps `NORMAL`.
    fn open_recovery_connection(
        path: &Path,
    ) -> Result<rusqlite::Connection, FileShieldedStoreError> {
        let conn = Self::open_tuned_connection(path)?;
        conn.pragma_update(None, "synchronous", "FULL")
            .map_err(|e| {
                FileShieldedStoreError::Storage(format!("PRAGMA synchronous=FULL: {e}"))
            })?;
        Ok(conn)
    }

    fn open_tuned_connection(path: &Path) -> Result<rusqlite::Connection, FileShieldedStoreError> {
        let conn = rusqlite::Connection::open(path)
            .map_err(|e| FileShieldedStoreError::Storage(format!("open sqlite: {e}")))?;
        // Pragmas must be applied before the schema is touched. They survive
        // for the lifetime of the connection; WAL also persists for any
        // subsequent reopen on the same file until explicitly changed.
        for (k, v) in [
            ("journal_mode", "WAL"),
            ("synchronous", "NORMAL"),
            ("temp_store", "MEMORY"),
        ] {
            conn.pragma_update(None, k, v)
                .map_err(|e| FileShieldedStoreError::Storage(format!("PRAGMA {k}={v}: {e}")))?;
        }
        // Two writer connections share this file (the commitment tree's and
        // `pending_conn`). WAL allows one writer at a time; without a busy
        // timeout a write colliding with the other connection's write txn
        // fails immediately with SQLITE_BUSY instead of briefly waiting.
        conn.busy_timeout(std::time::Duration::from_secs(5))
            .map_err(|e| FileShieldedStoreError::Storage(format!("busy_timeout: {e}")))?;
        Ok(conn)
    }

    /// Delete the single persisted redrive row for `id` keyed by
    /// `activity_id`. Used to mirror the exact in-memory drops
    /// [`SubwalletState::mark_spent`] reports, avoiding the
    /// scan-every-row cost of [`Self::delete_redrive_rows_containing`]
    /// on the common path where the resolved note had no armed redrive.
    fn delete_redrive_row(
        &self,
        id: SubwalletId,
        activity_id: &[u8; 32],
    ) -> Result<(), FileShieldedStoreError> {
        let conn = self.pending_conn.lock().expect("pending_conn mutex");
        conn.execute(
            "DELETE FROM shielded_pending_spends \
             WHERE wallet_id = ?1 AND account_index = ?2 AND activity_id = ?3",
            rusqlite::params![
                id.wallet_id.as_slice(),
                id.account_index,
                activity_id.as_slice(),
            ],
        )
        .map_err(|e| {
            FileShieldedStoreError::Storage(format!("delete redrive row by activity: {e}"))
        })?;
        Ok(())
    }

    /// Mirror to SQLite the redrive deletions [`SubwalletState`] performs
    /// in memory when a nullifier resolves via `clear_pending`: delete
    /// every persisted row for `id` whose nullifier blob contains
    /// `nullifier`.
    fn delete_redrive_rows_containing(
        &self,
        id: SubwalletId,
        nullifier: &[u8; 32],
    ) -> Result<(), FileShieldedStoreError> {
        let conn = self.pending_conn.lock().expect("pending_conn mutex");
        let mut stmt = conn
            .prepare(
                "SELECT activity_id, nullifiers FROM shielded_pending_spends \
                 WHERE wallet_id = ?1 AND account_index = ?2",
            )
            .map_err(|e| FileShieldedStoreError::Storage(format!("prepare redrive lookup: {e}")))?;
        let rows: Vec<(Vec<u8>, Vec<u8>)> = stmt
            .query_map(
                rusqlite::params![id.wallet_id.as_slice(), id.account_index],
                |row| Ok((row.get::<_, Vec<u8>>(0)?, row.get::<_, Vec<u8>>(1)?)),
            )
            .map_err(|e| FileShieldedStoreError::Storage(format!("query redrive lookup: {e}")))?
            .collect::<Result<_, _>>()
            .map_err(|e| FileShieldedStoreError::Storage(format!("read redrive lookup: {e}")))?;
        drop(stmt);
        for (activity_id, nullifiers) in rows {
            if nullifiers
                .as_chunks::<32>()
                .0
                .iter()
                .any(|c| c.as_slice() == nullifier.as_slice())
            {
                conn.execute(
                    "DELETE FROM shielded_pending_spends \
                     WHERE wallet_id = ?1 AND account_index = ?2 AND activity_id = ?3",
                    rusqlite::params![id.wallet_id.as_slice(), id.account_index, activity_id],
                )
                .map_err(|e| FileShieldedStoreError::Storage(format!("delete redrive row: {e}")))?;
            }
        }
        Ok(())
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

    fn spendable_balance(&self, id: SubwalletId) -> Result<u64, Self::Error> {
        // Borrow notes under the store guard: balance reads need only values,
        // not an allocated/deep-copied selection vector. Corrupt restored sums
        // still fail closed before sync or local snapshots can publish them.
        self.subwallets
            .get(&id)
            .map_or(Some(0), SubwalletState::spendable_balance)
            .ok_or_else(|| {
                FileShieldedStoreError::Storage("spendable shielded balance exceeds u64".into())
            })
    }

    fn get_all_notes(&self, id: SubwalletId) -> Result<Vec<ShieldedNote>, Self::Error> {
        Ok(self
            .subwallets
            .get(&id)
            .map(SubwalletState::all_notes)
            .unwrap_or_default())
    }

    fn mark_spent(&mut self, id: SubwalletId, nullifier: &[u8; 32]) -> Result<bool, Self::Error> {
        let Some(sw) = self.subwallets.get_mut(&id) else {
            return Ok(false);
        };
        let outcome = sw.mark_spent(nullifier);
        // Mirror the durable deletion whenever the in-memory drop
        // happened — keyed on the returned activity ids, NOT on
        // `newly_spent`. A note restored already-spent still resolves a
        // rehydrated redrive here (`newly_spent == false`), and leaving
        // the SQLite row would resurrect the reservation on the next
        // open. Targeting the exact activity ids means the common case
        // (no armed redrive for this note) issues zero SQLite work. The
        // in-memory transition already happened, so a SQLite failure
        // only warns — a surviving row rehydrates and self-heals via the
        // reconcile / prune passes.
        for activity_id in &outcome.dropped_redrives {
            if let Err(e) = self.delete_redrive_row(id, activity_id) {
                tracing::warn!(
                    error = %e,
                    "redrive row deletion failed after mark_spent; a stale row may \
                     rehydrate on the next open (self-heals via reconcile/prune)"
                );
            }
            self.private_contact_payments.remove(&(id, *activity_id));
        }
        Ok(outcome.newly_spent)
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
        let Some(sw) = self.subwallets.get_mut(&id) else {
            return Ok(false);
        };
        let removed = sw.clear_pending(nullifier);
        if removed {
            // Same log-don't-abort rationale as `mark_spent` above.
            if let Err(e) = self.delete_redrive_rows_containing(id, nullifier) {
                tracing::warn!(
                    error = %e,
                    "redrive row deletion failed after clear_pending; a stale row may \
                     rehydrate on the next open (self-heals via reconcile/prune)"
                );
            }
            self.evict_private_contact_payments(id);
        }
        Ok(removed)
    }

    fn mark_pending_by(
        &mut self,
        id: SubwalletId,
        nullifier: &[u8; 32],
        reservation: u64,
    ) -> Result<bool, Self::Error> {
        Ok(self
            .subwallets
            .entry(id)
            .or_default()
            .mark_pending_by(nullifier, reservation))
    }

    fn clear_pending_reserved_by(
        &mut self,
        id: SubwalletId,
        nullifier: &[u8; 32],
        reservation: u64,
    ) -> Result<bool, Self::Error> {
        let Some(sw) = self.subwallets.get_mut(&id) else {
            return Ok(false);
        };
        let released = sw.clear_pending_reserved_by(nullifier, reservation);
        if released {
            // Same log-don't-abort rationale as `clear_pending`.
            if let Err(e) = self.delete_redrive_rows_containing(id, nullifier) {
                tracing::warn!(
                    error = %e,
                    "redrive row deletion failed after a released reservation; a stale row \
                     may rehydrate on the next open (self-heals via reconcile/prune)"
                );
            }
            self.evict_private_contact_payments(id);
        }
        Ok(released)
    }

    fn set_pending_spend(
        &mut self,
        id: SubwalletId,
        nullifier: &[u8; 32],
        anchor: [u8; 32],
        activity_id: [u8; 32],
        anchor_height: Option<u64>,
    ) -> Result<(), Self::Error> {
        if let Some(sw) = self.subwallets.get_mut(&id) {
            sw.set_pending_spend(nullifier, anchor, activity_id, anchor_height);
        }
        Ok(())
    }

    fn note_anchor_recorded(
        &mut self,
        id: SubwalletId,
        anchor: &[u8; 32],
        height: u64,
    ) -> Result<(), Self::Error> {
        let Some(sw) = self.subwallets.get_mut(&id) else {
            return Ok(());
        };
        // Durable first: a failure gives nothing a height, which holds. So
        // does a height past SQLite's integer range.
        let Ok(durable_height) = i64::try_from(height) else {
            return Ok(());
        };
        let records = sw.redrives_without_anchor_height(anchor);
        if !records.is_empty() {
            let mut conn = self
                .pending_conn
                .lock()
                .map_err(|e| FileShieldedStoreError::Storage(format!("pending_conn mutex: {e}")))?;
            let tx = conn.transaction().map_err(|e| {
                FileShieldedStoreError::Storage(format!("begin anchor height: {e}"))
            })?;
            for activity_id in &records {
                tx.execute(
                    "UPDATE shielded_pending_spends SET anchor_height = ?4 \
                     WHERE wallet_id = ?1 AND account_index = ?2 AND activity_id = ?3 \
                     AND anchor_height IS NULL",
                    rusqlite::params![
                        id.wallet_id.as_slice(),
                        id.account_index,
                        activity_id.as_slice(),
                        durable_height,
                    ],
                )
                .map_err(|e| FileShieldedStoreError::Storage(format!("note anchor height: {e}")))?;
            }
            tx.commit().map_err(|e| {
                FileShieldedStoreError::Storage(format!("commit anchor height: {e}"))
            })?;
        }
        sw.note_anchor_recorded(anchor, height);
        Ok(())
    }

    fn stale_pending_spends(&self, id: SubwalletId) -> Result<Vec<StalePendingSpend>, Self::Error> {
        Ok(self
            .subwallets
            .get(&id)
            .map(SubwalletState::stale_pending_spends)
            .unwrap_or_default())
    }

    fn arm_redrive(&mut self, id: SubwalletId, redrive: PendingRedrive) -> Result<(), Self::Error> {
        self.arm_redrive_paying_for(id, redrive, None, &[])
    }

    fn arm_private_contact_payment(
        &mut self,
        id: SubwalletId,
        redrive: PendingRedrive,
        payment: PrivateContactPayment,
        commitments: &[[u8; 32]],
        anchor_height: u64,
    ) -> Result<(), Self::Error> {
        let records: Vec<PrivatePaymentRecord> = commitments
            .iter()
            .map(|commitment| PrivatePaymentRecord {
                account_index: id.account_index,
                activity_id: redrive.activity_id,
                anchor: redrive.anchor,
                anchor_height: Some(anchor_height),
                commitment: *commitment,
                payment,
                nullifiers: redrive.nullifiers.clone(),
            })
            .collect();
        self.arm_redrive_paying_for(id, redrive, Some(payment), &records)
    }

    fn private_payment_journal_open(&self, wallet_id: WalletId) -> Result<bool, Self::Error> {
        Ok(self.private_payment_journals.contains(&wallet_id))
    }

    fn open_private_payment_journal(&mut self, wallet_id: WalletId) -> Result<(), Self::Error> {
        if self.private_payment_journals.contains(&wallet_id) {
            return Ok(());
        }
        {
            let conn = self
                .pending_conn
                .lock()
                .map_err(|e| FileShieldedStoreError::Storage(format!("pending_conn mutex: {e}")))?;
            conn.execute(
                "INSERT OR IGNORE INTO shielded_private_payment_journal_wallets (wallet_id) \
                 VALUES (?1)",
                rusqlite::params![wallet_id.as_slice()],
            )
            .map_err(|e| FileShieldedStoreError::Storage(format!("open payment journal: {e}")))?;
        }
        self.private_payment_journals.insert(wallet_id);
        Ok(())
    }

    /// Fails with [`FileShieldedStoreError::RecoveryCorrupted`] when a
    /// journal record of the wallet, or one whose wallet does not read, does
    /// not read back: the pair it holds could not be restored.
    fn private_payment_journal(
        &self,
        wallet_id: WalletId,
    ) -> Result<Vec<PrivatePaymentRecord>, Self::Error> {
        if self
            .unreadable_private_payment_journal
            .contains(&Some(wallet_id))
            || self.unreadable_private_payment_journal.contains(&None)
        {
            return Err(FileShieldedStoreError::RecoveryCorrupted {
                account_index: None,
                reason: "a private payment journal record of the wallet does not read back"
                    .to_string(),
            });
        }
        Ok(self
            .private_payment_journal
            .range((wallet_id, [0u8; 32])..=(wallet_id, [u8::MAX; 32]))
            .map(|(_, record)| record.clone())
            .collect())
    }

    fn journal_private_payments(
        &mut self,
        wallet_id: WalletId,
        records: &[PrivatePaymentRecord],
    ) -> Result<(), Self::Error> {
        let rows = Self::journal_rows(records)?;
        {
            let mut conn = self
                .pending_conn
                .lock()
                .map_err(|e| FileShieldedStoreError::Storage(format!("pending_conn mutex: {e}")))?;
            let tx = conn
                .transaction()
                .map_err(|e| FileShieldedStoreError::Storage(format!("begin journal: {e}")))?;
            Self::insert_journal_rows(&tx, wallet_id, &rows)?;
            tx.commit()
                .map_err(|e| FileShieldedStoreError::Storage(format!("commit journal: {e}")))?;
        }
        if !records.is_empty() {
            self.private_payment_journals.insert(wallet_id);
        }
        for record in records {
            self.private_payment_journal
                .insert((wallet_id, record.commitment), record.clone());
        }
        Ok(())
    }

    fn retire_undelivered_spend(
        &mut self,
        id: SubwalletId,
        activity_id: &[u8; 32],
        commitments: &[[u8; 32]],
    ) -> Result<bool, Self::Error> {
        if self
            .subwallets
            .get(&id)
            .and_then(|subwallet| subwallet.redrive_attempts(activity_id))
            != Some(0)
        {
            return Ok(false);
        }
        // Durable first, in one transaction: a failure removes nothing, so the
        // record and the journal records stay together, which over-holds.
        {
            let mut conn = self
                .pending_conn
                .lock()
                .map_err(|e| FileShieldedStoreError::Storage(format!("pending_conn mutex: {e}")))?;
            let tx = conn
                .transaction()
                .map_err(|e| FileShieldedStoreError::Storage(format!("begin retirement: {e}")))?;
            // Only a record no re-broadcast was claimed for: the durable
            // counter is written before memory's.
            let retired = tx
                .execute(
                    "DELETE FROM shielded_pending_spends \
                     WHERE wallet_id = ?1 AND account_index = ?2 AND activity_id = ?3 \
                     AND attempts = 0",
                    rusqlite::params![
                        id.wallet_id.as_slice(),
                        id.account_index,
                        activity_id.as_slice(),
                    ],
                )
                .map_err(|e| FileShieldedStoreError::Storage(format!("retire redrive: {e}")))?;
            if retired != 1 {
                // Dropped uncommitted: nothing is removed.
                return Ok(false);
            }
            for commitment in commitments {
                tx.execute(
                    "DELETE FROM shielded_private_payment_journal \
                     WHERE wallet_id = ?1 AND commitment = ?2",
                    rusqlite::params![id.wallet_id.as_slice(), commitment.as_slice()],
                )
                .map_err(|e| {
                    FileShieldedStoreError::Storage(format!("discharge retired payment: {e}"))
                })?;
            }
            tx.commit()
                .map_err(|e| FileShieldedStoreError::Storage(format!("commit retirement: {e}")))?;
        }
        if let Some(subwallet) = self.subwallets.get_mut(&id) {
            subwallet.clear_redrive(activity_id);
        }
        self.private_contact_payments.remove(&(id, *activity_id));
        for commitment in commitments {
            self.private_payment_journal
                .remove(&(id.wallet_id, *commitment));
        }
        Ok(true)
    }

    fn discharge_private_payment(
        &mut self,
        wallet_id: WalletId,
        commitment: &[u8; 32],
    ) -> Result<bool, Self::Error> {
        // Durable first: a failure leaves the record, which over-holds.
        {
            let conn = self
                .pending_conn
                .lock()
                .map_err(|e| FileShieldedStoreError::Storage(format!("pending_conn mutex: {e}")))?;
            conn.execute(
                "DELETE FROM shielded_private_payment_journal \
                 WHERE wallet_id = ?1 AND commitment = ?2",
                rusqlite::params![wallet_id.as_slice(), commitment.as_slice()],
            )
            .map_err(|e| FileShieldedStoreError::Storage(format!("discharge payment: {e}")))?;
        }
        Ok(self
            .private_payment_journal
            .remove(&(wallet_id, *commitment))
            .is_some())
    }

    /// Fails with [`FileShieldedStoreError::RecoveryCorrupted`] when a live
    /// record of the wallet carries an association that does not read back,
    /// or a damaged record of it retained without being loaded carries one:
    /// the pair that payment holds could not be restored, so the restoration
    /// fails and the wallet's holds stay.
    fn private_contact_redrives_for_wallet(
        &self,
        wallet_id: WalletId,
    ) -> Result<Vec<(PendingRedrive, PrivateContactPayment)>, Self::Error> {
        if self
            .unloaded_private_contact_payments
            .contains(&Some(wallet_id))
            || self.unloaded_private_contact_payments.contains(&None)
        {
            return Err(FileShieldedStoreError::RecoveryCorrupted {
                account_index: None,
                reason: "a damaged saved payment of the wallet carries a private contact request \
                         association"
                    .to_string(),
            });
        }
        let mut payments = Vec::new();
        for (id, redrive) in self.pending_redrives_for_wallet(wallet_id)? {
            match self
                .private_contact_payments
                .get(&(id, redrive.activity_id))
            {
                None => {}
                Some(SavedPrivateContactPayment::Readable(payment)) => {
                    payments.push((redrive, *payment));
                }
                Some(SavedPrivateContactPayment::Unreadable) => {
                    return Err(FileShieldedStoreError::RecoveryCorrupted {
                        account_index: Some(id.account_index),
                        reason: "the private contact request a saved payment pays for does not \
                                 decode"
                            .to_string(),
                    });
                }
            }
        }
        Ok(payments)
    }

    fn pending_redrives(&self, id: SubwalletId) -> Result<Vec<PendingRedrive>, Self::Error> {
        Ok(self
            .subwallets
            .get(&id)
            .map(SubwalletState::pending_redrives)
            .unwrap_or_default())
    }

    fn pending_redrives_for_wallet(
        &self,
        wallet_id: WalletId,
    ) -> Result<Vec<(SubwalletId, PendingRedrive)>, Self::Error> {
        Ok(self
            .subwallets
            .iter()
            .filter(|(id, _)| id.wallet_id == wallet_id)
            .flat_map(|(id, subwallet)| {
                subwallet
                    .pending_redrives()
                    .into_iter()
                    .map(|redrive| (*id, redrive))
            })
            .collect())
    }

    fn bump_redrive_attempts(
        &mut self,
        id: SubwalletId,
        activity_id: &[u8; 32],
    ) -> Result<u32, Self::Error> {
        // Persist FIRST, mutate memory only on success: the reverse order
        // would leave the in-memory counter ahead of the durable row on a
        // SQLite failure, and a restart would rewind the attempt budget.
        let Some(next) = self
            .subwallets
            .get(&id)
            .and_then(|sw| sw.redrive_attempts(activity_id))
            .map(|attempts| attempts.saturating_add(1))
        else {
            return Ok(0);
        };
        {
            let conn = self.pending_conn.lock().expect("pending_conn mutex");
            conn.execute(
                "UPDATE shielded_pending_spends SET attempts = ?4 \
                 WHERE wallet_id = ?1 AND account_index = ?2 AND activity_id = ?3",
                rusqlite::params![
                    id.wallet_id.as_slice(),
                    id.account_index,
                    activity_id.as_slice(),
                    next,
                ],
            )
            .map_err(|e| FileShieldedStoreError::Storage(format!("bump redrive attempts: {e}")))?;
        }
        let attempts = self
            .subwallets
            .get_mut(&id)
            .map(|sw| sw.bump_redrive_attempts(activity_id))
            .unwrap_or(0);
        Ok(attempts)
    }

    fn clear_redrive(
        &mut self,
        id: SubwalletId,
        activity_id: &[u8; 32],
    ) -> Result<(), Self::Error> {
        // Persist first so an I/O failure leaves the active process guarded in
        // exactly the same way as a restart that rehydrates the surviving row.
        self.delete_redrive_row(id, activity_id)?;
        if let Some(sw) = self.subwallets.get_mut(&id) {
            sw.clear_redrive(activity_id);
        }
        self.private_contact_payments.remove(&(id, *activity_id));
        Ok(())
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
            .map_err(|e| FileShieldedStoreError::Storage(format!("tree mutex poisoned: {e}")))?;
        tree.append(*cmx, retention)
            .map_err(|e| FileShieldedStoreError::Storage(format!("append commitment: {e}")))
    }

    fn checkpoint_tree(&mut self, checkpoint_id: u32) -> Result<(), Self::Error> {
        let mut tree = self
            .tree
            .lock()
            .map_err(|e| FileShieldedStoreError::Storage(format!("tree mutex poisoned: {e}")))?;
        tree.checkpoint(checkpoint_id)
            .map(|_| ())
            .map_err(|e| FileShieldedStoreError::Storage(format!("checkpoint tree: {e}")))
    }

    fn tree_anchor(&self) -> Result<[u8; 32], Self::Error> {
        let tree = self
            .tree
            .lock()
            .map_err(|e| FileShieldedStoreError::Storage(format!("tree mutex poisoned: {e}")))?;
        tree.anchor()
            .map(|a| a.to_bytes())
            .map_err(|e| FileShieldedStoreError::Storage(format!("read tree anchor: {e}")))
    }

    fn witness_at_depth(
        &self,
        position: u64,
        depth: usize,
    ) -> Result<Option<grovedb_commitment_tree::MerklePath>, Self::Error> {
        let tree = self
            .tree
            .lock()
            .map_err(|e| FileShieldedStoreError::Storage(format!("tree mutex poisoned: {e}")))?;
        // `checkpoint_depth = 0` is the current tree state; deeper values
        // reach older checkpoints so a spend can be built against a root
        // Platform actually recorded (it records one anchor per block, while
        // an index-chunk sync routinely leaves the tree mid-block). The proof
        // uses whichever anchor this witness produces via `MerklePath::root`,
        // so the anchor and the authentication path always agree.
        tree.witness(Position::from(position), depth).map_err(|e| {
            FileShieldedStoreError::Storage(format!("witness({position}, depth {depth}): {e}"))
        })
    }

    fn tree_size(&self) -> Result<u64, Self::Error> {
        let tree = self
            .tree
            .lock()
            .map_err(|e| FileShieldedStoreError::Storage(format!("tree mutex poisoned: {e}")))?;
        let size = tree
            .max_leaf_position()
            .map_err(|e| FileShieldedStoreError::Storage(format!("read tree size: {e}")))?
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
        let state = self.subwallets.entry(id).or_default();
        state.last_synced_index = index;
        state.last_scanned_index = Some(index);
        state.balance_source = ShieldedBalanceSource::ScannedThisSession;
        Ok(())
    }

    fn purge_wallet(&mut self, wallet_id: WalletId) -> Result<(), Self::Error> {
        // The redrive table IS durable (unlike the rest of subwallet
        // state), so purging the in-memory map alone would leave this
        // wallet's rows to rehydrate stale reservations / rebroadcast
        // state on the next open. Delete them first, scoped by
        // wallet_id — SQL before memory, so an Err return means neither
        // store was touched (fail-atomic) rather than a memory purge
        // the caller can't distinguish from a no-op.
        {
            let conn = self.pending_conn.lock().expect("pending_conn mutex");
            conn.execute(
                "DELETE FROM shielded_pending_spends WHERE wallet_id = ?1",
                rusqlite::params![wallet_id.as_slice()],
            )
            .map_err(|e| {
                FileShieldedStoreError::Storage(format!("purge pending spends for wallet: {e}"))
            })?;
        }
        // Per-subwallet note / watermark / checkpoint state is
        // in-memory only (`subwallets`); the commitment tree in
        // SQLite is chain-wide and intentionally left intact.
        self.subwallets.retain(|id, _| id.wallet_id != wallet_id);
        self.private_contact_payments
            .retain(|(id, _), _| id.wallet_id != wallet_id);
        self.unloaded_private_contact_payments
            .remove(&Some(wallet_id));
        Ok(())
    }

    fn purge_subwallet(&mut self, id: SubwalletId) -> Result<(), Self::Error> {
        // Note-spend redrives are account-scoped and go with the account.
        // Empty-nullifier ShieldFromIdentity guards are wallet-wide: the signed
        // debit remains executable after its recipient account is re-bound, so
        // retain those rows and their minimal in-memory state.
        {
            let conn = self.pending_conn.lock().expect("pending_conn mutex");
            conn.execute(
                "DELETE FROM shielded_pending_spends \
                 WHERE wallet_id = ?1 AND account_index = ?2 \
                   AND length(nullifiers) > 0",
                rusqlite::params![id.wallet_id.as_slice(), id.account_index],
            )
            .map_err(|e| {
                FileShieldedStoreError::Storage(format!("purge pending spends for subwallet: {e}"))
            })?;
        }
        let identity_redrives = self
            .subwallets
            .get(&id)
            .map(SubwalletState::identity_redrives)
            .unwrap_or_default();
        self.subwallets.remove(&id);
        if !identity_redrives.is_empty() {
            let retained = self.subwallets.entry(id).or_default();
            for redrive in identity_redrives {
                retained.arm_redrive(redrive);
            }
        }
        self.evict_private_contact_payments(id);
        Ok(())
    }

    fn purge_all_subwallets(&mut self) -> Result<(), Self::Error> {
        // Durable redrive rows for every wallet go with the in-memory
        // purge; SQL first for the same fail-atomic reason as
        // `purge_wallet`.
        {
            let conn = self.pending_conn.lock().expect("pending_conn mutex");
            conn.execute("DELETE FROM shielded_pending_spends", [])
                .map_err(|e| {
                    FileShieldedStoreError::Storage(format!("purge all pending spends: {e}"))
                })?;
        }
        self.subwallets.clear();
        self.private_contact_payments.clear();
        self.unloaded_private_contact_payments.clear();
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
            .map_err(|e| FileShieldedStoreError::Storage(format!("tree mutex poisoned: {e}")))?;

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
            .map_err(|e| {
                FileShieldedStoreError::Storage(format!("reset commitment tree tables: {e}"))
            })?;

            // Durably flush the DELETEs into the main database file with a
            // TRUNCATE checkpoint before this connection is dropped.
            //
            // Without this, Clear is a no-op across a hard kill. The store
            // runs `synchronous=NORMAL` in WAL mode (see `open_tuned_connection`),
            // so a committed transaction lands in the `-wal` file but is NOT
            // fsync'd until a checkpoint. Two other connections (`tree` and
            // `pending_conn`) stay open on the same file, so SQLite's
            // last-connection-close auto-checkpoint never fires when this
            // transient connection drops — the emptied tables live only in the
            // WAL. On Android the "Clear" button is routinely followed by a
            // force-stop (non-graceful SIGKILL, no checkpoint), so the WAL
            // frames are discarded and the next launch reopens the OLD full
            // tree (the 771/771 "Clear did nothing" symptom). A TRUNCATE
            // checkpoint rewrites the main db and resets the WAL, making the
            // emptied state durable regardless of how the process later dies.
            // The prior graceful `drop(store)` in the unit test masked this —
            // a clean close checkpoints, a SIGKILL does not.
            conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);")
                .map_err(|e| {
                    FileShieldedStoreError::Storage(format!("checkpoint after tree reset: {e}"))
                })?;
        }

        let conn = Self::open_tuned_connection(&self.path)?;
        *tree = ClientPersistentCommitmentTree::open(conn, self.max_checkpoints)
            .map_err(|e| FileShieldedStoreError::Storage(format!("reopen commitment tree: {e}")))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dpp::state_transition::shielded_transfer_transition::v0::ShieldedTransferTransitionV0;

    /// Unique temp path for a test tree (no `tempfile` dev-dep).
    fn temp_tree_path(tag: &str) -> std::path::PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("shielded_tree_test_{tag}_{nanos}.sqlite"))
    }

    /// A [`PendingRedrive`] survives a store reopen — record, attempt
    /// counter, AND the note reservations its nullifiers carry — and is
    /// deleted (durably) when one of its nullifiers resolves.
    #[test]
    fn redrive_roundtrip_rehydration_and_resolution() {
        let path = temp_tree_path("redrive_roundtrip");
        let id = SubwalletId::new([7u8; 32], 0);
        let redrive = PendingRedrive {
            activity_id: [1u8; 32],
            anchor: [2u8; 32],
            nullifiers: vec![[3u8; 32], [4u8; 32]],
            st_bytes: vec![0xAB; 96],
            attempts: 0,
            identity_nonce_finalized: false,
            identity_user_abandoned: false,
            anchor_height: None,
        };
        {
            let mut store = FileBackedShieldedStore::open_path(&path, 100).expect("open");
            store.arm_redrive(id, redrive.clone()).expect("arm");
            assert_eq!(
                store
                    .bump_redrive_attempts(id, &redrive.activity_id)
                    .expect("bump"),
                1
            );
        }
        {
            // Reopen: record + attempts + reservations all rehydrated.
            let store = FileBackedShieldedStore::open_path(&path, 100).expect("reopen");
            let got = store.pending_redrives(id).expect("pending_redrives");
            assert_eq!(got.len(), 1, "record survives reopen");
            assert_eq!(got[0].attempts, 1, "attempt counter persists");
            assert_eq!(got[0].st_bytes, redrive.st_bytes, "transition bytes intact");
            assert_eq!(
                store.stale_pending_spends(id).expect("stale").len(),
                2,
                "both nullifier reservations rehydrated from the record"
            );
        }
        {
            // Resolving one nullifier (release path) durably drops the row.
            let mut store = FileBackedShieldedStore::open_path(&path, 100).expect("reopen 2");
            assert!(store.clear_pending(id, &[3u8; 32]).expect("clear"));
            assert!(store.pending_redrives(id).expect("redrives").is_empty());
        }
        {
            let store = FileBackedShieldedStore::open_path(&path, 100).expect("reopen 3");
            assert!(
                store.pending_redrives(id).expect("redrives").is_empty(),
                "deletion persisted across reopen"
            );
            assert!(
                store.stale_pending_spends(id).expect("stale").is_empty(),
                "no reservations rehydrate once the record is gone"
            );
        }
        let _ = std::fs::remove_file(&path);
    }

    /// The private contact request a record pays for is saved with it: it
    /// survives a reopen and an attempt bump, is listed only joined with its
    /// live record, and goes with the record. A record armed without one is
    /// not listed.
    #[test]
    fn should_keep_a_private_contact_payment_with_its_record_across_reopen() {
        let path = temp_tree_path("private_contact_payment");
        let wallet_id = [0x5A; 32];
        let id = SubwalletId::new(wallet_id, 0);
        let record = |activity: u8, nullifier: u8| PendingRedrive {
            activity_id: [activity; 32],
            anchor: [2u8; 32],
            nullifiers: vec![[nullifier; 32]],
            st_bytes: vec![0xAB; 96],
            attempts: 0,
            identity_nonce_finalized: false,
            identity_user_abandoned: false,
            anchor_height: None,
        };
        let payment = PrivateContactPayment {
            sender: [0xB1; 32],
            recipient: [0xA1; 32],
            account_reference: 0x1000_0005,
        };
        {
            let mut store = FileBackedShieldedStore::open_path(&path, 100).expect("open");
            store
                .arm_private_contact_payment(id, record(1, 3), payment, &[[0xC1; 32]], 0)
                .expect("arm the payment");
            store.arm_redrive(id, record(4, 6)).expect("arm a spend");
            store.bump_redrive_attempts(id, &[1u8; 32]).expect("bump");
        }
        {
            let mut store = FileBackedShieldedStore::open_path(&path, 100).expect("reopen");
            let listed = store
                .private_contact_redrives_for_wallet(wallet_id)
                .expect("listed");
            assert_eq!(listed.len(), 1, "only the payment: {listed:?}");
            assert_eq!(listed[0].0.activity_id, [1u8; 32]);
            assert_eq!(listed[0].0.attempts, 1);
            assert_eq!(listed[0].1, payment);
            assert!(store
                .private_contact_redrives_for_wallet([0x5B; 32])
                .expect("another wallet")
                .is_empty());
            assert!(store.clear_pending(id, &[3u8; 32]).expect("clear"));
            assert!(store
                .private_contact_redrives_for_wallet(wallet_id)
                .expect("listed")
                .is_empty());
            assert!(
                store.private_contact_payments.is_empty(),
                "the association goes with its record"
            );
        }
        {
            let store = FileBackedShieldedStore::open_path(&path, 100).expect("reopen 2");
            assert!(store
                .private_contact_redrives_for_wallet(wallet_id)
                .expect("listed")
                .is_empty());
            assert_eq!(store.pending_redrives(id).expect("redrives").len(), 1);
        }
        let _ = std::fs::remove_file(&path);
    }

    /// A private contact request association goes with its record on every
    /// path that drops the record: a resolved spend, a cleared redrive, and
    /// the purges.
    #[test]
    fn should_drop_a_private_contact_payment_with_its_record() {
        let path = temp_tree_path("private_contact_payment_eviction");
        let wallet_id = [0x5C; 32];
        let id = SubwalletId::new(wallet_id, 0);
        let other = SubwalletId::new([0x5D; 32], 0);
        let record = |activity: u8, nullifier: u8| PendingRedrive {
            activity_id: [activity; 32],
            anchor: [2u8; 32],
            nullifiers: vec![[nullifier; 32]],
            st_bytes: vec![0xAB; 96],
            attempts: 0,
            identity_nonce_finalized: false,
            identity_user_abandoned: false,
            anchor_height: None,
        };
        let payment = PrivateContactPayment {
            sender: [0xB1; 32],
            recipient: [0xA1; 32],
            account_reference: 5,
        };
        let mut store = FileBackedShieldedStore::open_path(&path, 100).expect("open");
        let arm = |store: &mut FileBackedShieldedStore, id, activity, nullifier| {
            store
                .arm_private_contact_payment(
                    id,
                    record(activity, nullifier),
                    payment,
                    &[[activity; 32]],
                    0,
                )
                .expect("armed");
        };

        let note = ShieldedNote {
            position: 0,
            cmx: [1; 32],
            nullifier: [11; 32],
            block_height: 1,
            is_spent: false,
            value: 1,
            note_data: vec![],
        };
        store.save_note(id, &note).expect("a note");
        arm(&mut store, id, 1, 11);
        store.mark_spent(id, &[11; 32]).expect("spent");
        assert!(store.private_contact_payments.is_empty(), "resolved spend");

        arm(&mut store, id, 2, 12);
        store.clear_redrive(id, &[2; 32]).expect("cleared");
        assert!(store.private_contact_payments.is_empty(), "cleared redrive");

        arm(&mut store, id, 3, 13);
        arm(&mut store, other, 4, 14);
        store.purge_subwallet(id).expect("purged");
        assert_eq!(
            store
                .private_contact_payments
                .keys()
                .copied()
                .collect::<Vec<_>>(),
            vec![(other, [4; 32])],
            "subwallet purge"
        );
        arm(&mut store, id, 5, 15);
        store.purge_wallet(wallet_id).expect("purged");
        assert_eq!(
            store
                .private_contact_payments
                .keys()
                .copied()
                .collect::<Vec<_>>(),
            vec![(other, [4; 32])],
            "wallet purge"
        );
        store.purge_all_subwallets().expect("purged");
        assert!(store.private_contact_payments.is_empty(), "every subwallet");
        drop(store);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn identity_debit_guard_with_no_nullifiers_survives_reopen() {
        let path = temp_tree_path("identity_debit_guard");
        let id = SubwalletId::new([9u8; 32], 4);
        let redrive = PendingRedrive {
            activity_id: [1u8; 32],
            anchor: [2u8; 32],
            nullifiers: vec![],
            st_bytes: vec![0xAB; 96],
            attempts: 0,
            identity_nonce_finalized: false,
            identity_user_abandoned: false,
            anchor_height: None,
        };
        {
            let mut store = FileBackedShieldedStore::open_path(&path, 100).expect("open");
            store.arm_redrive(id, redrive.clone()).expect("arm");
        }
        {
            let store = FileBackedShieldedStore::open_path(&path, 100).expect("reopen");
            assert_eq!(
                store.pending_redrives(id).expect("pending redrives"),
                vec![redrive],
                "identity debit guard must survive process restart"
            );
        }
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn should_upgrade_old_redrive_schema_and_preserve_finalized_guard_on_reopen() {
        let path = temp_tree_path("redrive_schema_upgrade");
        let id = SubwalletId::new([0x91; 32], 2);
        {
            let conn = rusqlite::Connection::open(&path).unwrap();
            conn.execute_batch(
                "CREATE TABLE shielded_pending_spends (
                wallet_id BLOB NOT NULL, account_index INTEGER NOT NULL,
                activity_id BLOB NOT NULL, anchor BLOB NOT NULL, nullifiers BLOB NOT NULL,
                st_bytes BLOB NOT NULL, attempts INTEGER NOT NULL DEFAULT 0,
                PRIMARY KEY(wallet_id, account_index, activity_id));",
            )
            .unwrap();
            conn.execute(
                "INSERT INTO shielded_pending_spends VALUES (?1, 2, ?2, ?3, ?4, ?5, 0)",
                rusqlite::params![
                    id.wallet_id.as_slice(),
                    [0x12u8; 32].as_slice(),
                    [0x23u8; 32].as_slice(),
                    Vec::<u8>::new(),
                    vec![0x34u8; 24]
                ],
            )
            .unwrap();
        }
        {
            let mut store = FileBackedShieldedStore::open_path(&path, 100).unwrap();
            let mut record = store.pending_redrives(id).unwrap().remove(0);
            assert!(
                !record.identity_nonce_finalized,
                "legacy records must remain retryable"
            );
            assert!(
                !record.identity_user_abandoned,
                "legacy guards require explicit recovery"
            );
            record.identity_nonce_finalized = true;
            record.identity_user_abandoned = true;
            store.arm_redrive(id, record).unwrap();
        }
        {
            let store = FileBackedShieldedStore::open_path(&path, 100).unwrap();
            let records = store.pending_redrives(id).unwrap();
            assert_eq!(
                records.len(),
                1,
                "parked payment must retain its durable guard"
            );
            assert!(records[0].identity_nonce_finalized);
            assert!(records[0].identity_user_abandoned);
        }
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn should_reject_corrupt_recovery_metadata_without_discarding_durable_rows() {
        for corruption in [
            "wallet_id = x'01'",
            "activity_id = x'01'",
            "anchor = x'01'",
            "nullifiers = x'01'",
            "account_index = -1",
            "st_bytes = 'not a blob'",
            "identity_nonce_finalized = 2",
            "identity_user_abandoned = -1",
            "identity_user_abandoned = 1, nullifiers = zeroblob(32)",
        ] {
            let path = temp_tree_path("corrupt_recovery_metadata");
            let mut store = FileBackedShieldedStore::open_path(&path, 100).unwrap();
            store
                .arm_redrive(
                    SubwalletId::new([1; 32], 2),
                    PendingRedrive {
                        activity_id: [2; 32],
                        anchor: [3; 32],
                        nullifiers: vec![],
                        st_bytes: vec![4; 32],
                        attempts: 0,
                        identity_nonce_finalized: false,
                        identity_user_abandoned: false,
                        anchor_height: None,
                    },
                )
                .unwrap();
            drop(store);
            let conn = rusqlite::Connection::open(&path).unwrap();
            conn.execute(
                &format!("UPDATE shielded_pending_spends SET {corruption}"),
                [],
            )
            .unwrap();
            let error = match FileBackedShieldedStore::open_path(&path, 100) {
                Ok(_) => panic!("must reject {corruption}"),
                Err(error) => error,
            };
            assert!(matches!(
                crate::error::PlatformWalletError::from(error),
                crate::error::PlatformWalletError::ShieldedRecoveryCorrupted { .. }
            ));
            let count: u32 = conn
                .query_row("SELECT COUNT(*) FROM shielded_pending_spends", [], |row| {
                    row.get(0)
                })
                .unwrap();
            assert_eq!(count, 1, "failed open must preserve damaged row");
            drop(conn);
            let _ = std::fs::remove_file(path);
        }
    }

    /// A saved payment's private contact request association that does not
    /// read back fails restoring its wallet's reservations, rather than losing
    /// the pair it holds: a non-null value that does not decode on a record
    /// that loads, which is kept, and one carried by a damaged note-spend
    /// record retained without being loaded. SQL NULL stays a record with no
    /// association, and the wallet restores again once the record goes.
    #[test]
    fn should_fail_restoring_a_wallet_whose_saved_association_does_not_read() {
        let path = temp_tree_path("unreadable_private_contact_payment");
        let wallet_id = [0x6A; 32];
        let id = SubwalletId::new(wallet_id, 0);
        let note_transition: StateTransition = ShieldedTransferTransitionV0 {
            actions: vec![],
            value_balance: 1_000,
            anchor: [0x52; 32],
            proof: vec![],
            binding_signature: [0; 64],
        }
        .into();
        let record = PendingRedrive {
            activity_id: [0x11; 32],
            anchor: [0x52; 32],
            nullifiers: vec![[0x13; 32]],
            st_bytes: note_transition.serialize_to_bytes().unwrap(),
            attempts: 0,
            identity_nonce_finalized: false,
            identity_user_abandoned: false,
            anchor_height: None,
        };
        let payment = PrivateContactPayment {
            sender: [0xB1; 32],
            recipient: [0xA1; 32],
            account_reference: 5,
        };
        let arm = || {
            let mut store = FileBackedShieldedStore::open_path(&path, 100).unwrap();
            store
                .arm_private_contact_payment(id, record.clone(), payment, &[[0xC2; 32]], 0)
                .unwrap();
        };
        let set = |assignment: &str| {
            rusqlite::Connection::open(&path)
                .unwrap()
                .execute(
                    &format!("UPDATE shielded_pending_spends SET {assignment}"),
                    [],
                )
                .unwrap();
        };
        let unreadable = |store: &FileBackedShieldedStore| {
            matches!(
                store.private_contact_redrives_for_wallet(wallet_id),
                Err(FileShieldedStoreError::RecoveryCorrupted { .. })
            )
        };

        arm();
        set("private_contact_payment = x'01'");
        let mut store = FileBackedShieldedStore::open_path(&path, 100).unwrap();
        assert!(unreadable(&store), "an association that does not decode");
        assert_eq!(store.pending_redrives(id).unwrap(), vec![record.clone()]);
        assert!(store
            .private_contact_redrives_for_wallet([0x6B; 32])
            .unwrap()
            .is_empty());
        store.clear_redrive(id, &record.activity_id).unwrap();
        assert!(
            store
                .private_contact_redrives_for_wallet(wallet_id)
                .unwrap()
                .is_empty(),
            "the record went"
        );
        drop(store);

        arm();
        set("anchor = x'01'");
        let store = FileBackedShieldedStore::open_path(&path, 100).unwrap();
        assert!(store.pending_redrives(id).unwrap().is_empty(), "not loaded");
        assert!(unreadable(&store), "a damaged record carrying one");
        drop(store);

        set(
            "anchor = x'5252525252525252525252525252525252525252525252525252525252525252', \
             private_contact_payment = NULL",
        );
        let store = FileBackedShieldedStore::open_path(&path, 100).unwrap();
        assert_eq!(store.pending_redrives(id).unwrap(), vec![record]);
        assert!(
            store
                .private_contact_redrives_for_wallet(wallet_id)
                .unwrap()
                .is_empty(),
            "no association"
        );
        drop(store);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn should_skip_only_damaged_rows_proven_to_spend_shielded_notes() {
        use dpp::state_transition::shielded_transfer_transition::v0::ShieldedTransferTransitionV0;

        for corruption in [
            "wallet_id = x'01'",
            "activity_id = x'01'",
            "anchor = x'01'",
            "nullifiers = x'01'",
            "account_index = -1",
            "attempts = -1",
            "identity_nonce_finalized = 2",
            "identity_user_abandoned = -1",
            "identity_user_abandoned = 1, nullifiers = zeroblob(32)",
        ] {
            let path = temp_tree_path("damaged_note_spend_recovery");
            let note_id = SubwalletId::new([0x31; 32], 2);
            let guard_id = SubwalletId::new([0x41; 32], 3);
            let note_transition: StateTransition = ShieldedTransferTransitionV0 {
                actions: vec![],
                value_balance: 1_000,
                anchor: [0x51; 32],
                proof: vec![],
                binding_signature: [0; 64],
            }
            .into();
            let guard = PendingRedrive {
                activity_id: [0x61; 32],
                anchor: [0x71; 32],
                nullifiers: vec![],
                st_bytes: vec![0x81; 32],
                attempts: 0,
                identity_nonce_finalized: false,
                identity_user_abandoned: false,
                anchor_height: None,
            };
            {
                let mut store = FileBackedShieldedStore::open_path(&path, 100).unwrap();
                store
                    .arm_redrive(
                        note_id,
                        PendingRedrive {
                            activity_id: [0x21; 32],
                            anchor: [0x51; 32],
                            nullifiers: vec![[0x91; 32]],
                            st_bytes: note_transition.serialize_to_bytes().unwrap(),
                            attempts: 0,
                            identity_nonce_finalized: false,
                            identity_user_abandoned: false,
                            anchor_height: None,
                        },
                    )
                    .unwrap();
                store.arm_redrive(guard_id, guard.clone()).unwrap();
            }
            let conn = rusqlite::Connection::open(&path).unwrap();
            conn.execute(
                &format!("UPDATE shielded_pending_spends SET {corruption} WHERE wallet_id = ?1"),
                [note_id.wallet_id.as_slice()],
            )
            .unwrap();

            let store = FileBackedShieldedStore::open_path(&path, 100)
                .unwrap_or_else(|error| panic!("must tolerate {corruption}: {error}"));
            assert!(store.pending_redrives(note_id).unwrap().is_empty());
            assert!(store.stale_pending_spends(note_id).unwrap().is_empty());
            assert_eq!(store.pending_redrives(guard_id).unwrap(), vec![guard]);
            let count: u32 = conn
                .query_row("SELECT COUNT(*) FROM shielded_pending_spends", [], |row| {
                    row.get(0)
                })
                .unwrap();
            assert_eq!(
                count, 2,
                "damaged note row must remain available for repair"
            );
            drop(store);
            drop(conn);
            let _ = std::fs::remove_file(path);
        }
    }

    #[test]
    fn should_fail_closed_when_identity_debit_has_note_spend_metadata() {
        use dpp::platform_value::BinaryData;
        use dpp::state_transition::shield_from_identity_transition::v0::ShieldFromIdentityTransitionV0;

        let path = temp_tree_path("identity_debit_note_metadata");
        let id = SubwalletId::new([0xA1; 32], 4);
        let transition: StateTransition = ShieldFromIdentityTransitionV0 {
            identity_id: [0xB1; 32].into(),
            amount: 1_000,
            actions: vec![],
            anchor: [0xC1; 32],
            proof: vec![],
            binding_signature: [0; 64],
            nonce: 7,
            user_fee_increase: 0,
            signature_public_key_id: 1,
            signature: BinaryData::new(vec![]),
        }
        .into();
        {
            let mut store = FileBackedShieldedStore::open_path(&path, 100).unwrap();
            store
                .arm_redrive(
                    id,
                    PendingRedrive {
                        activity_id: [0xD1; 32],
                        anchor: [0xC1; 32],
                        nullifiers: vec![],
                        st_bytes: transition.serialize_to_bytes().unwrap(),
                        attempts: 0,
                        identity_nonce_finalized: false,
                        identity_user_abandoned: false,
                        anchor_height: None,
                    },
                )
                .unwrap();
        }
        let conn = rusqlite::Connection::open(&path).unwrap();
        conn.execute(
            "UPDATE shielded_pending_spends SET nullifiers = zeroblob(32)",
            [],
        )
        .unwrap();
        assert!(matches!(
            FileBackedShieldedStore::open_path(&path, 100),
            Err(FileShieldedStoreError::RecoveryCorrupted { .. })
        ));
        let count: u32 = conn
            .query_row("SELECT COUNT(*) FROM shielded_pending_spends", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(count, 1, "identity guard must remain durable");
        drop(conn);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn should_fail_closed_for_note_spend_bytes_with_trailing_data() {
        use dpp::state_transition::shielded_transfer_transition::v0::ShieldedTransferTransitionV0;

        let path = temp_tree_path("note_spend_trailing_data");
        let id = SubwalletId::new([0xE1; 32], 5);
        let transition: StateTransition = ShieldedTransferTransitionV0 {
            actions: vec![],
            value_balance: 1_000,
            anchor: [0xF1; 32],
            proof: vec![],
            binding_signature: [0; 64],
        }
        .into();
        let mut st_bytes = transition.serialize_to_bytes().unwrap();
        st_bytes.push(0xFF);
        {
            let mut store = FileBackedShieldedStore::open_path(&path, 100).unwrap();
            store
                .arm_redrive(
                    id,
                    PendingRedrive {
                        activity_id: [0xD2; 32],
                        anchor: [0xF1; 32],
                        nullifiers: vec![[0xC2; 32]],
                        st_bytes,
                        attempts: 0,
                        identity_nonce_finalized: false,
                        identity_user_abandoned: false,
                        anchor_height: None,
                    },
                )
                .unwrap();
        }
        let conn = rusqlite::Connection::open(&path).unwrap();
        conn.execute("UPDATE shielded_pending_spends SET wallet_id = x'01'", [])
            .unwrap();
        assert!(matches!(
            FileBackedShieldedStore::open_path(&path, 100),
            Err(FileShieldedStoreError::RecoveryCorrupted { .. })
        ));
        let count: u32 = conn
            .query_row("SELECT COUNT(*) FROM shielded_pending_spends", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(count, 1, "ambiguous row must remain durable");
        drop(conn);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn should_keep_retry_state_when_persisting_nonce_finality_fails() {
        let path = temp_tree_path("redrive_park_failure");
        let id = SubwalletId::new([0x91; 32], 2);
        let record = PendingRedrive {
            activity_id: [0x12; 32],
            anchor: [0x23; 32],
            nullifiers: vec![],
            st_bytes: vec![0x34; 24],
            attempts: 0,
            identity_nonce_finalized: false,
            identity_user_abandoned: false,
            anchor_height: None,
        };
        {
            let mut store = FileBackedShieldedStore::open_path(&path, 100).unwrap();
            store.arm_redrive(id, record.clone()).unwrap();
            store.pending_conn.lock().unwrap().execute_batch("CREATE TRIGGER reject_redrive_insert BEFORE INSERT ON shielded_pending_spends BEGIN SELECT RAISE(FAIL, 'injected insert failure'); END;").unwrap();
            let mut parked = record.clone();
            parked.identity_nonce_finalized = true;
            assert!(store.arm_redrive(id, parked).is_err());
            assert_eq!(store.pending_redrives(id).unwrap(), vec![record.clone()]);
        }
        let store = FileBackedShieldedStore::open_path(&path, 100).unwrap();
        assert_eq!(store.pending_redrives(id).unwrap(), vec![record]);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn clear_redrive_sql_failure_keeps_memory_and_durable_guard() {
        let path = temp_tree_path("clear_redrive_failure");
        let id = SubwalletId::new([0x91; 32], 2);
        let redrive = PendingRedrive {
            activity_id: [0x12; 32],
            anchor: [0x23; 32],
            nullifiers: vec![],
            st_bytes: vec![0x34; 24],
            attempts: 0,
            identity_nonce_finalized: false,
            identity_user_abandoned: false,
            anchor_height: None,
        };
        {
            let mut store = FileBackedShieldedStore::open_path(&path, 100).expect("open");
            store.arm_redrive(id, redrive.clone()).expect("arm");
            store
                .pending_conn
                .lock()
                .expect("pending_conn mutex")
                .execute_batch(
                    "CREATE TRIGGER reject_redrive_delete \
                     BEFORE DELETE ON shielded_pending_spends \
                     BEGIN SELECT RAISE(FAIL, 'injected delete failure'); END;",
                )
                .expect("install failure trigger");

            assert!(
                store.clear_redrive(id, &redrive.activity_id).is_err(),
                "the injected durable delete failure must surface"
            );
            assert_eq!(
                store.pending_redrives(id).expect("memory guard"),
                vec![redrive.clone()],
                "a failed durable delete must leave the live process guarded"
            );
        }
        {
            let store = FileBackedShieldedStore::open_path(&path, 100).expect("reopen");
            assert_eq!(
                store.pending_redrives(id).expect("durable guard"),
                vec![redrive],
                "the guard must also rehydrate after the failed delete"
            );
        }
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn account_purge_durably_retains_identity_guard_only() {
        let path = temp_tree_path("purge_subwallet_identity_guard");
        let wallet_id = [0xA4; 32];
        let id = SubwalletId::new(wallet_id, 8);
        let identity_guard = PendingRedrive {
            activity_id: [0x15; 32],
            anchor: [0x26; 32],
            nullifiers: vec![],
            st_bytes: vec![0x37; 24],
            attempts: 5,
            identity_nonce_finalized: false,
            identity_user_abandoned: false,
            anchor_height: None,
        };
        let note_spend = PendingRedrive {
            activity_id: [0x48; 32],
            anchor: [0x59; 32],
            nullifiers: vec![[0x6A; 32]],
            st_bytes: vec![0x7B; 24],
            attempts: 1,
            identity_nonce_finalized: false,
            identity_user_abandoned: false,
            anchor_height: None,
        };
        {
            let mut store = FileBackedShieldedStore::open_path(&path, 100).expect("open");
            store
                .arm_redrive(id, identity_guard.clone())
                .expect("arm identity");
            store.arm_redrive(id, note_spend).expect("arm spend");
            store.purge_subwallet(id).expect("purge account");
            assert_eq!(
                store.pending_redrives(id).expect("retained guard"),
                vec![identity_guard.clone()]
            );
        }
        {
            let store = FileBackedShieldedStore::open_path(&path, 100).expect("reopen");
            assert_eq!(
                store
                    .pending_redrives_for_wallet(wallet_id)
                    .expect("wallet guards"),
                vec![(id, identity_guard)],
                "only the wallet-wide identity guard may survive account removal"
            );
        }
        let _ = std::fs::remove_file(&path);
    }

    /// Purging a wallet (or all subwallets) must also delete its durable
    /// redrive rows — otherwise a Clear / unregister leaves stale rows
    /// that rehydrate ghost reservations on the next open. And
    /// `reset_commitment_tree` must NOT touch them: a redrive is
    /// broadcast state, not tree state, and a tree resync doesn't
    /// invalidate an in-flight transition.
    #[test]
    fn purge_clears_durable_redrive_rows_but_tree_reset_does_not() {
        let path = temp_tree_path("purge_redrive");
        let id_a = SubwalletId::new([0xA1; 32], 0);
        let id_b = SubwalletId::new([0xB2; 32], 0);
        let redrive = |activity: u8, nf: u8| PendingRedrive {
            activity_id: [activity; 32],
            anchor: [0x22; 32],
            nullifiers: vec![[nf; 32]],
            st_bytes: vec![0xCD; 32],
            attempts: 0,
            identity_nonce_finalized: false,
            identity_user_abandoned: false,
            anchor_height: None,
        };

        // purge_wallet is scoped: it drops A's rows, keeps B's.
        {
            let mut store = FileBackedShieldedStore::open_path(&path, 100).expect("open");
            store.arm_redrive(id_a, redrive(0x01, 0x0A)).expect("arm a");
            store.arm_redrive(id_b, redrive(0x02, 0x0B)).expect("arm b");

            // reset_commitment_tree leaves BOTH redrives intact.
            store.reset_commitment_tree().expect("reset tree");
            assert_eq!(store.pending_redrives(id_a).expect("a").len(), 1);
            assert_eq!(store.pending_redrives(id_b).expect("b").len(), 1);

            store.purge_wallet(id_a.wallet_id).expect("purge a");
            assert!(
                store.pending_redrives(id_a).expect("a").is_empty(),
                "purge_wallet dropped A's durable redrive rows"
            );
            assert_eq!(
                store.pending_redrives(id_b).expect("b").len(),
                1,
                "purge_wallet is scoped — B's rows survive"
            );
        }
        // The deletion is durable across reopen; B still rehydrates.
        {
            let store = FileBackedShieldedStore::open_path(&path, 100).expect("reopen");
            assert!(store.pending_redrives(id_a).expect("a").is_empty());
            assert_eq!(store.pending_redrives(id_b).expect("b").len(), 1);
        }
        // purge_all_subwallets drops everything, durably.
        {
            let mut store = FileBackedShieldedStore::open_path(&path, 100).expect("reopen 2");
            store.purge_all_subwallets().expect("purge all");
            assert!(store.pending_redrives(id_b).expect("b").is_empty());
        }
        {
            let store = FileBackedShieldedStore::open_path(&path, 100).expect("reopen 3");
            assert!(store.pending_redrives(id_b).expect("b").is_empty());
        }
        let _ = std::fs::remove_file(&path);
    }

    /// `mark_spent` must resolve a rehydrated redrive even for a note
    /// that is restored ALREADY spent (its transition landed in a prior
    /// session) — the durable SQLite row must be deleted too, not just
    /// the in-memory record, or it resurrects the reservation on the
    /// next open.
    #[test]
    fn mark_spent_on_restored_spent_note_clears_durable_redrive() {
        let path = temp_tree_path("mark_spent_idempotent");
        let id = SubwalletId::new([0x9; 32], 0);
        let nf = [0x3A; 32];
        let note = ShieldedNote {
            position: 0,
            cmx: [0x1; 32],
            nullifier: nf,
            block_height: 10,
            // Restored from disk ALREADY spent.
            is_spent: true,
            value: 500,
            note_data: vec![0u8; 115],
        };
        {
            let mut store = FileBackedShieldedStore::open_path(&path, 100).expect("open");
            store.save_note(id, &note).expect("save");
            store
                .arm_redrive(
                    id,
                    PendingRedrive {
                        activity_id: [0x7; 32],
                        anchor: [0x22; 32],
                        nullifiers: vec![nf],
                        st_bytes: vec![0xEF; 32],
                        attempts: 0,
                        identity_nonce_finalized: false,
                        identity_user_abandoned: false,
                        anchor_height: None,
                    },
                )
                .expect("arm");

            // Already-spent → returns false, but STILL resolves the redrive.
            let newly = store.mark_spent(id, &nf).expect("mark_spent");
            assert!(!newly, "note was already spent, so not newly spent");
            assert!(
                store.pending_redrives(id).expect("redrives").is_empty(),
                "the redrive must be dropped from memory even on the already-spent path"
            );
        }
        {
            // And the durable row was deleted — nothing rehydrates.
            let store = FileBackedShieldedStore::open_path(&path, 100).expect("reopen");
            assert!(
                store.pending_redrives(id).expect("redrives").is_empty(),
                "the SQLite redrive row was mirrored-deleted, not left to rehydrate"
            );
        }
        let _ = std::fs::remove_file(&path);
    }

    /// Guards against the "Shielded Merkle witness unavailable"
    /// spend failure on a multi-wallet shared tree.
    ///
    /// Invariant: the shared commitment tree marks EVERY position
    /// (`append_commitment(.., true)`); per-wallet ownership is
    /// tracked separately in the notes store. Appending a commitment
    /// as `Ephemeral` unless the owning wallet's IVK recognizes it in
    /// that very sync pass is wrong: with multiple wallets sharing
    /// one tree and binding at different times, a note appended
    /// before its owner binds stays Ephemeral forever — shardtree has
    /// no retroactive marking — so the balance shows but the spend
    /// fails to build a witness (on disk: every position
    /// un-witnessable, missing internal nodes at `Level(2) index 0` /
    /// `Level(1) index 2`).
    ///
    /// This test asserts
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

    /// Durability regression guard for the "Clear did nothing after a
    /// force-stop" bug: `reset_commitment_tree` must land the emptied
    /// tables in the MAIN database file, not merely in the `-wal` file of
    /// the store's own connections.
    ///
    /// The store runs `synchronous=NORMAL` in WAL mode and keeps multiple
    /// connections open, so SQLite's last-connection-close auto-checkpoint
    /// never fires on the transient reset connection — without an explicit
    /// checkpoint the DELETEs live only in the WAL. On Android the Clear
    /// button is routinely followed by a force-stop (SIGKILL, no graceful
    /// close, no checkpoint), which discards those WAL frames and reopens
    /// the OLD full tree (the 771/771 symptom).
    ///
    /// This test proves the fix WITHOUT dropping the store (a graceful drop
    /// would checkpoint and mask the bug — exactly what the prior test's
    /// `drop(store)` did): it opens an INDEPENDENT read-only connection that
    /// deliberately does NOT attach the `-wal` (`?immutable=1`), so it can
    /// only see rows already written to the main db file. If the reset left
    /// the shard rows in the WAL, this connection would still see the old
    /// leaves; seeing zero proves the checkpoint flushed them to the main db,
    /// where they survive any later process death.
    #[test]
    fn reset_commitment_tree_flushes_to_main_db_file_not_just_wal() {
        let path = temp_tree_path("reset_durable");
        let mut store = FileBackedShieldedStore::open_path(&path, 100).unwrap();

        // Build a non-trivial tree and checkpoint it (durably, via the
        // normal append path).
        const N: u64 = 6;
        for i in 0..N {
            let mut cmx = [0u8; 32];
            cmx[0] = (i as u8) + 1;
            store.append_commitment(&cmx, true).unwrap();
        }
        store.checkpoint_tree(N as u32).unwrap();
        assert_eq!(store.tree_size().unwrap(), N);

        // Reset. The store's connections stay open (mirroring a live
        // process that hasn't been force-stopped yet).
        store.reset_commitment_tree().unwrap();
        assert_eq!(store.tree_size().unwrap(), 0);

        // Independent immutable connection: reads ONLY the main .sqlite
        // file, ignoring any `-wal`. `immutable=1` tells SQLite the file
        // won't change and there is no live WAL to consult, so a shard row
        // visible here is one the checkpoint flushed into the main db.
        let uri = format!("file:{}?immutable=1", path.display());
        let main_only = rusqlite::Connection::open_with_flags(
            &uri,
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_URI,
        )
        .expect("open main-db-only connection");
        let shard_rows: i64 = main_only
            .query_row("SELECT COUNT(*) FROM commitment_tree_shards", [], |r| {
                r.get(0)
            })
            .expect("count shard rows in main db");
        drop(main_only);
        let _ = std::fs::remove_file(&path);

        assert_eq!(
            shard_rows, 0,
            "reset must checkpoint the emptied tables into the MAIN db file; \
             a non-zero count means the DELETEs sit only in the WAL and a \
             force-stop before checkpoint would resurrect the old tree"
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

        // The spend path uses exactly this anchor: `extract_spends_and_recorded_anchor`
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

    /// A type 29 transition paying for one entry whose commitment is
    /// `[commitment; 32]`, with placeholder actions, serialized.
    fn private_document_create_bytes(commitment: u8) -> Vec<u8> {
        use dpp::platform_value::BinaryData;
        use dpp::shielded::SerializedAction;
        use dpp::state_transition::private_document_create_from_shielded_pool_transition::methods::PrivateDocumentCreateFromShieldedPoolTransitionMethodsV0;
        use dpp::state_transition::private_document_create_from_shielded_pool_transition::PrivateDocumentCreateFromShieldedPoolTransition;
        use dpp::version::PlatformVersion;

        let action = SerializedAction {
            nullifier: [1; 32],
            rk: [2; 32],
            cmx: [3; 32],
            encrypted_note: vec![4; 216],
            cv_net: [5; 32],
            spend_auth_sig: [6; 64],
        };
        let mut entry = vec![commitment; 32];
        entry.extend_from_slice(&[0; 557]);
        PrivateDocumentCreateFromShieldedPoolTransition::try_from_bundle(
            dpp::prelude::Identifier::from([0x44; 32]),
            "privateContactRequest".to_string(),
            vec![BinaryData::new(entry)],
            vec![action.clone(), action],
            0,
            [7; 32],
            vec![8; 64],
            [9; 64],
            PlatformVersion::latest(),
        )
        .expect("a type 29 transition")
        .serialize_to_bytes()
        .expect("serializes")
    }

    /// The recovery tables commit at `synchronous=FULL`, so a record saved
    /// before a broadcast survives a power loss; the commitment tree's
    /// connection keeps `NORMAL`.
    #[test]
    fn should_commit_recovery_records_at_full_sync() {
        let path = temp_tree_path("recovery_full_sync");
        let store = FileBackedShieldedStore::open_path(&path, 100).expect("open");
        let synchronous = |conn: &rusqlite::Connection| {
            conn.pragma_query_value(None, "synchronous", |row| row.get::<_, i64>(0))
                .expect("synchronous")
        };
        let recovery = store.pending_conn.lock().expect("pending_conn mutex");
        assert_eq!(synchronous(&recovery), 2, "FULL");
        drop(recovery);
        let tree = FileBackedShieldedStore::open_tuned_connection(&path).expect("tree conn");
        assert_eq!(synchronous(&tree), 1, "NORMAL");
        let _ = std::fs::remove_file(&path);
    }

    /// A journal record is saved with its payment's redrive record, and
    /// outlives it: no note release, purge or reopen removes it, only a
    /// discharge does. The journal it was saved in reads as open.
    #[test]
    fn should_keep_a_journal_record_until_it_is_discharged() {
        let path = temp_tree_path("journal_lifecycle");
        let wallet_id = [0x5A; 32];
        let id = SubwalletId::new(wallet_id, 1);
        let payment = PrivateContactPayment {
            sender: [0xB1; 32],
            recipient: [0xA1; 32],
            account_reference: 7,
        };
        let redrive = PendingRedrive {
            activity_id: [0x21; 32],
            anchor: [0x22; 32],
            nullifiers: vec![[0x23; 32], [0x24; 32]],
            st_bytes: private_document_create_bytes(0xC3),
            attempts: 0,
            identity_nonce_finalized: false,
            identity_user_abandoned: false,
            anchor_height: None,
        };
        let expected = PrivatePaymentRecord {
            account_index: 1,
            activity_id: redrive.activity_id,
            anchor: redrive.anchor,
            anchor_height: Some(0),
            commitment: [0xC3; 32],
            payment,
            nullifiers: redrive.nullifiers.clone(),
        };
        let mut store = FileBackedShieldedStore::open_path(&path, 100).expect("open");
        assert!(!store
            .private_payment_journal_open(wallet_id)
            .expect("open?"));
        for nullifier in &redrive.nullifiers {
            store.mark_pending(id, nullifier).expect("reserved");
        }
        store
            .arm_private_contact_payment(id, redrive.clone(), payment, &[[0xC3; 32]], 0)
            .expect("armed");
        assert!(store
            .private_payment_journal_open(wallet_id)
            .expect("open?"));
        assert_eq!(
            store.private_payment_journal(wallet_id).expect("journal"),
            vec![expected.clone()]
        );
        // A note release drops the redrive record, not the journal record.
        assert!(store.clear_pending(id, &[0x23; 32]).expect("released"));
        assert!(store.pending_redrives(id).expect("records").is_empty());
        store.purge_subwallet(id).expect("account purge");
        store.purge_wallet(wallet_id).expect("wallet purge");
        store.purge_all_subwallets().expect("purge all");
        drop(store);
        let mut store = FileBackedShieldedStore::open_path(&path, 100).expect("reopened");
        assert!(store
            .private_payment_journal_open(wallet_id)
            .expect("open?"));
        assert_eq!(
            store.private_payment_journal(wallet_id).expect("journal"),
            vec![expected]
        );
        assert!(store
            .private_payment_journal([0x5B; 32])
            .expect("another wallet's")
            .is_empty());
        assert!(store
            .discharge_private_payment(wallet_id, &[0xC3; 32])
            .expect("discharged"));
        drop(store);
        let store = FileBackedShieldedStore::open_path(&path, 100).expect("reopened");
        assert!(store
            .private_payment_journal(wallet_id)
            .expect("journal")
            .is_empty());
        let _ = std::fs::remove_file(&path);
    }

    /// A never-delivered spend's saved record and its journal records are
    /// retired by one durable write: after it, a reopened store holds
    /// neither. A write that fails part way, here at the journal's delete,
    /// removes neither, and one whose re-broadcast was claimed is refused and
    /// keeps both; so after any reopen, the transition the redrive pass could
    /// broadcast and the journal record that holds its pair stay together.
    #[test]
    fn should_retire_an_undelivered_spend_with_its_journal_records_in_one_write() {
        let payment = PrivateContactPayment {
            sender: [0xB1; 32],
            recipient: [0xA1; 32],
            account_reference: 7,
        };
        let commitment = [0xC4; 32];
        for case in ["retired", "write fails", "re-broadcast claimed"] {
            let path = temp_tree_path("retire_undelivered");
            let wallet_id = [0x5C; 32];
            let id = SubwalletId::new(wallet_id, 0);
            let redrive = PendingRedrive {
                activity_id: [0x31; 32],
                anchor: [0x32; 32],
                nullifiers: vec![[0x33; 32]],
                st_bytes: private_document_create_bytes(0xC4),
                attempts: 0,
                identity_nonce_finalized: false,
                identity_user_abandoned: false,
                anchor_height: None,
            };
            let mut store = FileBackedShieldedStore::open_path(&path, 100).expect("open");
            store.mark_pending(id, &[0x33; 32]).expect("reserved");
            store
                .arm_private_contact_payment(id, redrive.clone(), payment, &[commitment], 0)
                .expect("armed");
            match case {
                "write fails" => store
                    .pending_conn
                    .lock()
                    .expect("pending_conn mutex")
                    .execute_batch(
                        "CREATE TRIGGER reject_journal_delete BEFORE DELETE ON \
                         shielded_private_payment_journal BEGIN SELECT RAISE(FAIL, \
                         'injected delete failure'); END;",
                    )
                    .expect("trigger"),
                "re-broadcast claimed" => {
                    assert_eq!(
                        store
                            .bump_redrive_attempts(id, &redrive.activity_id)
                            .expect("claimed"),
                        1
                    );
                }
                _ => {}
            }
            let retired = store.retire_undelivered_spend(id, &redrive.activity_id, &[commitment]);
            match case {
                "retired" => assert!(retired.expect("retired"), "{case}"),
                "write fails" => assert!(retired.is_err(), "{case}: {retired:?}"),
                _ => assert!(!retired.expect("refused"), "{case}"),
            }
            let kept = case != "retired";
            let both = |store: &FileBackedShieldedStore| {
                (
                    store.pending_redrives(id).expect("records").len(),
                    store
                        .private_payment_journal(wallet_id)
                        .expect("journal")
                        .len(),
                )
            };
            let expected = if kept { (1, 1) } else { (0, 0) };
            assert_eq!(both(&store), expected, "{case}: in memory");
            drop(store);
            let reopened = FileBackedShieldedStore::open_path(&path, 100).expect("reopened");
            assert_eq!(both(&reopened), expected, "{case}: reopened");
            drop(reopened);
            let _ = std::fs::remove_file(&path);
        }
    }

    /// A journal record that does not read back stays, and fails reading
    /// its wallet's journal, or every wallet's when its wallet does not read
    /// either; the store still opens.
    #[test]
    fn should_fail_reading_a_journal_whose_record_does_not_read() {
        for (damage, other_wallet_reads) in [
            (
                "UPDATE shielded_private_payment_journal SET record = x'01'",
                true,
            ),
            (
                "UPDATE shielded_private_payment_journal SET commitment = x'02'",
                true,
            ),
            (
                "UPDATE shielded_private_payment_journal SET wallet_id = x'03'",
                false,
            ),
        ] {
            let path = temp_tree_path("journal_damage");
            let wallet_id = [0x6A; 32];
            let id = SubwalletId::new(wallet_id, 0);
            let payment = PrivateContactPayment {
                sender: [0xB2; 32],
                recipient: [0xA2; 32],
                account_reference: 1,
            };
            {
                let mut store = FileBackedShieldedStore::open_path(&path, 100).expect("open");
                store
                    .arm_private_contact_payment(
                        id,
                        PendingRedrive {
                            activity_id: [0x31; 32],
                            anchor: [0x32; 32],
                            nullifiers: vec![[0x33; 32]],
                            st_bytes: private_document_create_bytes(0xD4),
                            attempts: 0,
                            identity_nonce_finalized: false,
                            identity_user_abandoned: false,
                            anchor_height: None,
                        },
                        payment,
                        &[[0xD4; 32]],
                        0,
                    )
                    .expect("armed");
                store
                    .pending_conn
                    .lock()
                    .expect("pending_conn mutex")
                    .execute(damage, [])
                    .expect("damaged");
            }
            let store = FileBackedShieldedStore::open_path(&path, 100).expect("still opens");
            assert!(
                matches!(
                    store.private_payment_journal(wallet_id),
                    Err(FileShieldedStoreError::RecoveryCorrupted { .. })
                ),
                "{damage}"
            );
            assert_eq!(
                store.private_payment_journal([0x6B; 32]).is_ok(),
                other_wallet_reads,
                "{damage}"
            );
            let rows: i64 = store
                .pending_conn
                .lock()
                .expect("pending_conn mutex")
                .query_row(
                    "SELECT count(*) FROM shielded_private_payment_journal",
                    [],
                    |row| row.get(0),
                )
                .expect("count");
            assert_eq!(rows, 1, "{damage}: the record stays");
            let _ = std::fs::remove_file(&path);
        }
    }

    /// A private document create (type 29) is a note spend: a recovery row
    /// for one with damaged metadata is retained and skipped, its wallet
    /// flagged so its legacy payments do not restore, and the shared store
    /// still opens for every wallet.
    #[test]
    fn should_retain_and_skip_a_damaged_private_document_payment_row() {
        let path = temp_tree_path("damaged_private_document_row");
        let wallet_id = [0x7A; 32];
        let id = SubwalletId::new(wallet_id, 0);
        {
            let mut store = FileBackedShieldedStore::open_path(&path, 100).expect("open");
            store
                .arm_redrive(
                    id,
                    PendingRedrive {
                        activity_id: [0x41; 32],
                        anchor: [0x42; 32],
                        nullifiers: vec![[0x43; 32]],
                        st_bytes: private_document_create_bytes(0xE5),
                        attempts: 0,
                        identity_nonce_finalized: false,
                        identity_user_abandoned: false,
                        anchor_height: None,
                    },
                )
                .expect("armed");
            store
                .pending_conn
                .lock()
                .expect("pending_conn mutex")
                .execute("UPDATE shielded_pending_spends SET anchor = x'01'", [])
                .expect("damaged");
        }
        let store = FileBackedShieldedStore::open_path(&path, 100).expect("still opens");
        assert!(store.pending_redrives(id).expect("records").is_empty());
        assert!(matches!(
            store.private_contact_redrives_for_wallet(wallet_id),
            Err(FileShieldedStoreError::RecoveryCorrupted { .. })
        ));
        assert!(store
            .private_contact_redrives_for_wallet([0x7B; 32])
            .is_ok());
        let _ = std::fs::remove_file(&path);
    }
}
