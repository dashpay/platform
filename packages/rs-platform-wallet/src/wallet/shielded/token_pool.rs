//! A wallet's notes in token shielded pools.
//!
//! Every token whose configuration has a shielded pool keeps its own Orchard
//! commitment tree, nullifier set and anchors, separate from the credit pool.
//! A wallet cannot scan every token's pool, so it scans the pools it watches:
//! a token is watched once the wallet buys it into its pool (type 28) or the
//! host asks for it ([`NetworkShieldedCoordinator::watch_token_pool`]).
//!
//! The state lives in the shielded store's SQLite file, in the tables
//! [`create_tables`] makes, and nowhere else: unlike the credit pool's notes
//! it is not handed to the host's persister.
//!
//! - `shielded_token_pool_watches`: which wallet watches which token. A
//!   Clear keeps them; everything else is scan state a sync rebuilds.
//! - `shielded_token_pool_scans`: per token, the pool's note count, the
//!   height of the last chunk read and the last scan error.
//! - `shielded_token_pool_watermarks`: per account and token, how many of
//!   the pool's notes, from position 0 without a gap, the account has
//!   trial-decrypted.
//! - `shielded_token_pool_leaves`: every note commitment of a watched pool
//!   by position, for the Merkle paths a spend of a token note needs.
//! - `shielded_token_pool_notes`: the notes an account decrypted, and
//!   whether the scan has seen their nullifiers.
//!
//! [`NetworkShieldedCoordinator::watch_token_pool`]:
//!     super::NetworkShieldedCoordinator::watch_token_pool

use std::collections::{BTreeMap, BTreeSet};

use async_trait::async_trait;
use dash_sdk::platform::shielded::try_decrypt_note;
use dash_sdk::platform::Fetch;
use dash_sdk::query_types::{
    ShieldedEncryptedNote, ShieldedEncryptedNotes, TokenShieldedEncryptedNotesQuery,
};
use dpp::data_contract::TokenContractPosition;
use dpp::prelude::Identifier;
use grovedb_commitment_tree::{
    Anchor, ClientMemoryCommitmentTree, MerklePath, Position, Retention,
};
use rusqlite::{params, Connection, OptionalExtension};
use tokio::sync::RwLock;
use tokio_util::sync::CancellationToken;
use tracing::{debug, info, warn};

use super::keys::AccountViewingKeys;
use super::store::SubwalletId;
use super::sync::{serialize_note, CHUNK_SIZE};
use crate::error::PlatformWalletError;
use crate::wallet::platform_wallet::WalletId;

/// A token pool a wallet watches: the token and where its contract defines it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShieldedTokenPoolWatch {
    /// The token's id, derived from the contract id and the position.
    pub token_id: Identifier,
    /// The contract that defines the token.
    pub data_contract_id: Identifier,
    /// The token's position in its contract.
    pub token_contract_position: TokenContractPosition,
}

/// One account's tokens in one watched token pool, as the local scan knows
/// them ([`NetworkShieldedCoordinator::token_pool_balances`]).
///
/// [`NetworkShieldedCoordinator::token_pool_balances`]:
///     super::NetworkShieldedCoordinator::token_pool_balances
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShieldedTokenPoolBalance {
    /// The watched token.
    pub watch: ShieldedTokenPoolWatch,
    /// The Orchard account the notes belong to.
    pub account_index: u32,
    /// The value of the account's notes whose nullifiers the scan has not
    /// seen and that no spend in flight holds, in the token's smallest unit:
    /// what a token pool spend can use now.
    pub balance: u64,
    /// How many notes make up [`Self::balance`].
    pub unspent_note_count: u32,
    /// The value of the account's unspent notes a spend in flight holds
    /// ([`TokenNoteReservation`]): they leave the balance when the scan sees
    /// them spent, or come back to it if the spend can no longer execute.
    pub reserved_balance: u64,
    /// How many of the pool's notes, from the first without a gap, were
    /// checked for this account; `None` before the first scan reached it.
    /// The balance covers exactly these.
    pub scanned_note_count: Option<u64>,
    /// The pool's note count at the last chunk read, `None` before any.
    pub pool_note_count: Option<u64>,
    /// The height the last chunk read was proven at, `None` before any.
    pub scanned_height: Option<u64>,
    /// Why the last scan of the pool stopped early, if it did. Cleared by the
    /// next chunk read.
    pub last_error: Option<String>,
}

/// A note an account received in a token pool.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TokenPoolNote {
    /// The account that can spend it.
    pub id: SubwalletId,
    /// The token whose pool holds it.
    pub token_id: Identifier,
    /// Its position in the token pool's commitment tree.
    pub position: u64,
    /// Its extracted note commitment.
    pub cmx: [u8; 32],
    /// The nullifier that spends it.
    pub nullifier: [u8; 32],
    /// Its value, in the token's smallest unit.
    pub value: u64,
    /// The serialized Orchard note (115 bytes, see `serialize_note`).
    pub note_data: Vec<u8>,
    /// The height of the chunk read that found it: an at-or-before bound,
    /// not the height it was written at.
    pub block_height: u64,
    /// Whether the scan has seen its nullifier.
    pub is_spent: bool,
    /// The spend in flight that holds it, if one does: a reservation marker
    /// or the activity id of the spend's saved record.
    pub reserved_by: Option<[u8; 32]>,
}

/// What one pass over the watched token pools did, for logs and tests.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct TokenPoolSyncSummary {
    /// The pools scanned to their end this pass.
    pub completed: BTreeSet<Identifier>,
    /// The pools whose scan stopped early, and why.
    pub failed: BTreeMap<Identifier, String>,
    /// Notes newly found, over every pool and account.
    pub new_notes: usize,
    /// Owned notes newly seen spent, over every pool.
    pub newly_spent: usize,
}

/// One proven chunk of a token pool's encrypted notes.
#[derive(Debug, Clone, Default)]
pub struct TokenPoolChunk {
    /// The notes from the requested start, in tree order.
    pub notes: Vec<ShieldedEncryptedNote>,
    /// The pool's note count the proof showed.
    pub total_count: u64,
    /// The height the answer was proven at.
    pub height: u64,
}

/// Where a token pool scan reads its chunks from: the SDK, or a test double.
#[async_trait]
pub trait TokenPoolNoteSource: Send + Sync {
    /// The notes of `token_id`'s pool from `start_index` (a multiple of
    /// [`CHUNK_SIZE`]), at most `count`, with proofs verified.
    async fn token_pool_chunk(
        &self,
        token_id: Identifier,
        start_index: u64,
        count: u32,
    ) -> Result<TokenPoolChunk, PlatformWalletError>;

    /// How many notes one request may ask for.
    fn token_pool_chunk_capacity(&self) -> u32;
}

#[async_trait]
impl TokenPoolNoteSource for dash_sdk::Sdk {
    async fn token_pool_chunk(
        &self,
        token_id: Identifier,
        start_index: u64,
        count: u32,
    ) -> Result<TokenPoolChunk, PlatformWalletError> {
        let (notes, metadata) = ShieldedEncryptedNotes::fetch_with_metadata(
            self,
            TokenShieldedEncryptedNotesQuery {
                token_id,
                start_index,
                count,
            },
            None,
        )
        .await
        .map_err(|e| {
            PlatformWalletError::ShieldedSyncFailed(format!(
                "reading the shielded pool of token {token_id} from note {start_index} failed: {e}"
            ))
        })?;
        let (notes, total_count) = notes
            .map(|ShieldedEncryptedNotes { notes, total_count }| (notes, total_count))
            .unwrap_or_default();
        Ok(TokenPoolChunk {
            notes,
            total_count,
            height: metadata.height,
        })
    }

    fn token_pool_chunk_capacity(&self) -> u32 {
        let chunks = u64::from(
            self.version()
                .drive_abci
                .query
                .shielded_queries
                .max_query_chunks,
        )
        .max(1);
        // The query handler also caps one answer at `u16::MAX` notes.
        u32::try_from(
            CHUNK_SIZE
                .saturating_mul(chunks)
                .min(u64::from(u16::MAX) / CHUNK_SIZE * CHUNK_SIZE),
        )
        .unwrap_or(CHUNK_SIZE as u32)
    }
}

fn storage(context: &str) -> impl Fn(rusqlite::Error) -> PlatformWalletError + '_ {
    move |e| PlatformWalletError::ShieldedStoreError(format!("token pool {context}: {e}"))
}

fn to_sql_u64(value: u64, what: &str) -> Result<i64, PlatformWalletError> {
    i64::try_from(value).map_err(|_| {
        PlatformWalletError::ShieldedStoreError(format!(
            "token pool {what} {value} is past SQLite's integers"
        ))
    })
}

fn from_sql_u64(value: i64, what: &str) -> Result<u64, PlatformWalletError> {
    u64::try_from(value).map_err(|_| {
        PlatformWalletError::ShieldedStoreError(format!("token pool {what} {value} is negative"))
    })
}

fn bytes32(bytes: Vec<u8>, what: &str) -> Result<[u8; 32], PlatformWalletError> {
    <[u8; 32]>::try_from(bytes.as_slice()).map_err(|_| {
        PlatformWalletError::ShieldedStoreError(format!(
            "token pool {what} is {} bytes, not 32",
            bytes.len()
        ))
    })
}

/// Create the token pool tables if they are missing. Idempotent.
pub(super) fn create_tables(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS shielded_token_pool_watches (
            wallet_id               BLOB    NOT NULL,
            token_id                BLOB    NOT NULL,
            data_contract_id        BLOB    NOT NULL,
            token_contract_position INTEGER NOT NULL,
            PRIMARY KEY (wallet_id, token_id)
        );
        CREATE TABLE IF NOT EXISTS shielded_token_pool_scans (
            token_id        BLOB    NOT NULL PRIMARY KEY,
            pool_note_count INTEGER,
            scanned_height  INTEGER,
            last_error      TEXT
        );
        CREATE TABLE IF NOT EXISTS shielded_token_pool_watermarks (
            wallet_id       BLOB    NOT NULL,
            account_index   INTEGER NOT NULL,
            token_id        BLOB    NOT NULL,
            scanned_through INTEGER NOT NULL,
            PRIMARY KEY (wallet_id, account_index, token_id)
        );
        CREATE TABLE IF NOT EXISTS shielded_token_pool_leaves (
            token_id BLOB    NOT NULL,
            position INTEGER NOT NULL,
            cmx      BLOB    NOT NULL,
            PRIMARY KEY (token_id, position)
        );
        CREATE TABLE IF NOT EXISTS shielded_token_pool_notes (
            wallet_id     BLOB    NOT NULL,
            account_index INTEGER NOT NULL,
            token_id      BLOB    NOT NULL,
            position      INTEGER NOT NULL,
            cmx           BLOB    NOT NULL,
            nullifier     BLOB    NOT NULL,
            value         BLOB    NOT NULL,
            note_data     BLOB    NOT NULL,
            block_height  INTEGER NOT NULL,
            spent         INTEGER NOT NULL DEFAULT 0,
            PRIMARY KEY (wallet_id, account_index, token_id, nullifier)
        );
        CREATE INDEX IF NOT EXISTS shielded_token_pool_notes_by_nullifier
            ON shielded_token_pool_notes (token_id, nullifier);",
    )?;
    // The spend in flight holding a note ([`TokenNoteReservation`]); added to
    // a table made before reservations were kept.
    let has_reserved_by = conn
        .prepare("PRAGMA table_info(shielded_token_pool_notes)")?
        .query_map([], |row| row.get::<_, String>(1))?
        .collect::<rusqlite::Result<Vec<_>>>()?
        .iter()
        .any(|column| column == "reserved_by");
    if !has_reserved_by {
        conn.execute(
            "ALTER TABLE shielded_token_pool_notes ADD COLUMN reserved_by BLOB",
            [],
        )?;
    }
    Ok(())
}

/// Record that `wallet_id` watches `watch`'s pool. Idempotent.
pub(super) fn insert_watch(
    conn: &Connection,
    wallet_id: WalletId,
    watch: &ShieldedTokenPoolWatch,
) -> Result<(), PlatformWalletError> {
    conn.execute(
        "INSERT OR REPLACE INTO shielded_token_pool_watches \
         (wallet_id, token_id, data_contract_id, token_contract_position) \
         VALUES (?1, ?2, ?3, ?4)",
        params![
            wallet_id.as_slice(),
            watch.token_id.as_slice(),
            watch.data_contract_id.as_slice(),
            watch.token_contract_position,
        ],
    )
    .map_err(storage("watch"))?;
    Ok(())
}

/// Stop `wallet_id` watching `token_id`'s pool, dropping its notes and scan
/// state there, and the pool's own state once no wallet watches it.
pub(super) fn remove_watch(
    conn: &mut Connection,
    wallet_id: WalletId,
    token_id: Identifier,
) -> Result<(), PlatformWalletError> {
    let tx = conn.transaction().map_err(storage("unwatch"))?;
    for table in [
        "shielded_token_pool_watches",
        "shielded_token_pool_watermarks",
        "shielded_token_pool_notes",
    ] {
        tx.execute(
            &format!("DELETE FROM {table} WHERE wallet_id = ?1 AND token_id = ?2"),
            params![wallet_id.as_slice(), token_id.as_slice()],
        )
        .map_err(storage("unwatch"))?;
    }
    drop_unwatched_pools(&tx)?;
    tx.commit().map_err(storage("unwatch"))?;
    Ok(())
}

/// Drop the leaves and scan state of every pool no wallet watches.
fn drop_unwatched_pools(conn: &Connection) -> Result<(), PlatformWalletError> {
    for table in ["shielded_token_pool_scans", "shielded_token_pool_leaves"] {
        conn.execute(
            &format!(
                "DELETE FROM {table} WHERE token_id NOT IN \
                 (SELECT token_id FROM shielded_token_pool_watches)"
            ),
            [],
        )
        .map_err(storage("drop unwatched pools"))?;
    }
    Ok(())
}

/// Drop everything kept for `wallet_id`, as its removal does.
pub(super) fn purge_wallet(conn: &mut Connection, wallet_id: WalletId) -> rusqlite::Result<()> {
    let tx = conn.transaction()?;
    for table in [
        "shielded_token_pool_watches",
        "shielded_token_pool_watermarks",
        "shielded_token_pool_notes",
    ] {
        tx.execute(
            &format!("DELETE FROM {table} WHERE wallet_id = ?1"),
            params![wallet_id.as_slice()],
        )?;
    }
    for table in ["shielded_token_pool_scans", "shielded_token_pool_leaves"] {
        tx.execute(
            &format!(
                "DELETE FROM {table} WHERE token_id NOT IN \
                 (SELECT token_id FROM shielded_token_pool_watches)"
            ),
            [],
        )?;
    }
    tx.commit()
}

/// Drop one account's notes and scan state in every pool, as dropping or
/// re-keying the account does. The wallet keeps watching its pools.
pub(super) fn purge_subwallet(conn: &mut Connection, id: SubwalletId) -> rusqlite::Result<()> {
    let tx = conn.transaction()?;
    for table in [
        "shielded_token_pool_watermarks",
        "shielded_token_pool_notes",
    ] {
        tx.execute(
            &format!("DELETE FROM {table} WHERE wallet_id = ?1 AND account_index = ?2"),
            params![id.wallet_id.as_slice(), id.account_index],
        )?;
    }
    tx.commit()
}

/// Drop every token pool row but the watches, as clearing the shielded store
/// does: like the credit pool after a Clear, the next sync scans the watched
/// pools again from the start. The watches stay because nothing else would
/// bring them back, so the tokens they hold would stay unseen.
pub(super) fn purge_all(conn: &mut Connection) -> rusqlite::Result<()> {
    let tx = conn.transaction()?;
    for table in [
        "shielded_token_pool_scans",
        "shielded_token_pool_watermarks",
        "shielded_token_pool_leaves",
        "shielded_token_pool_notes",
    ] {
        tx.execute(&format!("DELETE FROM {table}"), [])?;
    }
    tx.commit()
}

/// Every watch, by wallet.
pub(super) fn watches(
    conn: &Connection,
) -> Result<BTreeMap<WalletId, Vec<ShieldedTokenPoolWatch>>, PlatformWalletError> {
    let mut statement = conn
        .prepare(
            "SELECT wallet_id, token_id, data_contract_id, token_contract_position \
             FROM shielded_token_pool_watches ORDER BY wallet_id, token_id",
        )
        .map_err(storage("read watches"))?;
    let rows = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, Vec<u8>>(0)?,
                row.get::<_, Vec<u8>>(1)?,
                row.get::<_, Vec<u8>>(2)?,
                row.get::<_, TokenContractPosition>(3)?,
            ))
        })
        .map_err(storage("read watches"))?;
    let mut out: BTreeMap<WalletId, Vec<ShieldedTokenPoolWatch>> = BTreeMap::new();
    for row in rows {
        let (wallet_id, token_id, data_contract_id, token_contract_position) =
            row.map_err(storage("read watches"))?;
        out.entry(bytes32(wallet_id, "wallet id")?)
            .or_default()
            .push(ShieldedTokenPoolWatch {
                token_id: Identifier::from(bytes32(token_id, "token id")?),
                data_contract_id: Identifier::from(bytes32(data_contract_id, "contract id")?),
                token_contract_position,
            });
    }
    Ok(out)
}

/// The number of the pool's notes `id` has checked, `None` before any scan.
pub(super) fn watermark(
    conn: &Connection,
    id: SubwalletId,
    token_id: Identifier,
) -> Result<Option<u64>, PlatformWalletError> {
    conn.query_row(
        "SELECT scanned_through FROM shielded_token_pool_watermarks \
         WHERE wallet_id = ?1 AND account_index = ?2 AND token_id = ?3",
        params![
            id.wallet_id.as_slice(),
            id.account_index,
            token_id.as_slice()
        ],
        |row| row.get::<_, i64>(0),
    )
    .optional()
    .map_err(storage("read watermark"))?
    .map(|value| from_sql_u64(value, "watermark"))
    .transpose()
}

/// Every note `id` holds in `token_id`'s pool, spent or not, by position.
#[cfg(test)]
pub(crate) fn notes(
    conn: &Connection,
    id: SubwalletId,
    token_id: Identifier,
) -> Result<Vec<TokenPoolNote>, PlatformWalletError> {
    notes_where(conn, id, token_id, "")
}

/// The notes `id` holds in `token_id`'s pool that a spend can take: unspent
/// and held by no spend in flight, by position. Filtered in the query, so a
/// long history of spent notes is never read.
fn spendable_notes(
    conn: &Connection,
    id: SubwalletId,
    token_id: Identifier,
) -> Result<Vec<TokenPoolNote>, PlatformWalletError> {
    notes_where(conn, id, token_id, " AND spent = 0 AND reserved_by IS NULL")
}

/// `id`'s notes in `token_id`'s pool matching `filter`, a fixed SQL condition
/// appended to the account and token match, by position.
fn notes_where(
    conn: &Connection,
    id: SubwalletId,
    token_id: Identifier,
    filter: &'static str,
) -> Result<Vec<TokenPoolNote>, PlatformWalletError> {
    let mut statement = conn
        .prepare(&format!(
            "SELECT position, cmx, nullifier, value, note_data, block_height, spent, \
             reserved_by FROM shielded_token_pool_notes \
             WHERE wallet_id = ?1 AND account_index = ?2 AND token_id = ?3{filter} \
             ORDER BY position"
        ))
        .map_err(storage("read notes"))?;
    let rows = statement
        .query_map(
            params![
                id.wallet_id.as_slice(),
                id.account_index,
                token_id.as_slice()
            ],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, Vec<u8>>(1)?,
                    row.get::<_, Vec<u8>>(2)?,
                    row.get::<_, Vec<u8>>(3)?,
                    row.get::<_, Vec<u8>>(4)?,
                    row.get::<_, i64>(5)?,
                    row.get::<_, i64>(6)?,
                    row.get::<_, Option<Vec<u8>>>(7)?,
                ))
            },
        )
        .map_err(storage("read notes"))?;
    let mut out = Vec::new();
    for row in rows {
        let (position, cmx, nullifier, value, note_data, block_height, spent, reserved_by) =
            row.map_err(storage("read notes"))?;
        let value = <[u8; 8]>::try_from(value.as_slice())
            .map(u64::from_le_bytes)
            .map_err(|_| {
                PlatformWalletError::ShieldedStoreError(
                    "token pool note value is not 8 bytes".to_string(),
                )
            })?;
        out.push(TokenPoolNote {
            id,
            token_id,
            position: from_sql_u64(position, "note position")?,
            cmx: bytes32(cmx, "note commitment")?,
            nullifier: bytes32(nullifier, "nullifier")?,
            value,
            note_data,
            block_height: from_sql_u64(block_height, "note height")?,
            is_spent: spent != 0,
            reserved_by: reserved_by
                .map(|marker| bytes32(marker, "reservation"))
                .transpose()?,
        });
    }
    Ok(out)
}

/// The values of `id`'s unspent notes in `token_id`'s pool, each with
/// whether a spend in flight holds it, read alone: a balance needs nothing
/// else of the notes.
fn unspent_values(
    conn: &Connection,
    id: SubwalletId,
    token_id: Identifier,
) -> Result<Vec<(u64, bool)>, PlatformWalletError> {
    let mut statement = conn
        .prepare(
            "SELECT value, reserved_by IS NOT NULL FROM shielded_token_pool_notes \
             WHERE wallet_id = ?1 AND account_index = ?2 AND token_id = ?3 AND spent = 0",
        )
        .map_err(storage("read note values"))?;
    let rows = statement
        .query_map(
            params![
                id.wallet_id.as_slice(),
                id.account_index,
                token_id.as_slice()
            ],
            |row| Ok((row.get::<_, Vec<u8>>(0)?, row.get::<_, bool>(1)?)),
        )
        .map_err(storage("read note values"))?;
    rows.map(|row| {
        let (value, reserved) = row.map_err(storage("read note values"))?;
        <[u8; 8]>::try_from(value.as_slice())
            .map(|value| (u64::from_le_bytes(value), reserved))
            .map_err(|_| {
                PlatformWalletError::ShieldedStoreError(
                    "token pool note value is not 8 bytes".to_string(),
                )
            })
    })
    .collect()
}

/// What is known of a pool's last scan.
#[derive(Default)]
struct ScanState {
    pool_note_count: Option<u64>,
    scanned_height: Option<u64>,
    last_error: Option<String>,
}

/// What is known of a pool's last scan, empty before any.
fn scan_state(conn: &Connection, token_id: Identifier) -> Result<ScanState, PlatformWalletError> {
    let row = conn
        .query_row(
            "SELECT pool_note_count, scanned_height, last_error \
             FROM shielded_token_pool_scans WHERE token_id = ?1",
            params![token_id.as_slice()],
            |row| {
                Ok((
                    row.get::<_, Option<i64>>(0)?,
                    row.get::<_, Option<i64>>(1)?,
                    row.get::<_, Option<String>>(2)?,
                ))
            },
        )
        .optional()
        .map_err(storage("read scan state"))?;
    let Some((count, height, last_error)) = row else {
        return Ok(ScanState::default());
    };
    Ok(ScanState {
        pool_note_count: count
            .map(|count| from_sql_u64(count, "pool note count"))
            .transpose()?,
        scanned_height: height
            .map(|height| from_sql_u64(height, "scan height"))
            .transpose()?,
        last_error,
    })
}

/// Record why a pool's scan stopped early.
fn record_scan_error(
    conn: &Connection,
    token_id: Identifier,
    error: &str,
) -> Result<(), PlatformWalletError> {
    conn.execute(
        "INSERT INTO shielded_token_pool_scans (token_id, last_error) VALUES (?1, ?2) \
         ON CONFLICT(token_id) DO UPDATE SET last_error = excluded.last_error",
        params![token_id.as_slice(), error],
    )
    .map_err(storage("record scan error"))?;
    Ok(())
}

/// The balances of `accounts` of `wallet_id` in every pool the wallet
/// watches, by token then account.
pub(super) fn balances(
    conn: &Connection,
    wallet_id: WalletId,
    accounts: &[u32],
) -> Result<Vec<ShieldedTokenPoolBalance>, PlatformWalletError> {
    let watches = watches(conn)?.remove(&wallet_id).unwrap_or_default();
    let mut out = Vec::with_capacity(watches.len() * accounts.len());
    for watch in watches {
        let ScanState {
            pool_note_count,
            scanned_height,
            last_error,
        } = scan_state(conn, watch.token_id)?;
        for &account_index in accounts {
            let id = SubwalletId::new(wallet_id, account_index);
            let mut balance = 0u64;
            let mut unspent_note_count = 0u32;
            let mut reserved_balance = 0u64;
            let overflow = || {
                PlatformWalletError::ShieldedStoreError(format!(
                    "the notes of account {account_index} in the pool of token {} add up past \
                     u64",
                    watch.token_id
                ))
            };
            for (value, reserved) in unspent_values(conn, id, watch.token_id)? {
                if reserved {
                    reserved_balance = reserved_balance.checked_add(value).ok_or_else(overflow)?;
                } else {
                    balance = balance.checked_add(value).ok_or_else(overflow)?;
                    unspent_note_count = unspent_note_count.saturating_add(1);
                }
            }
            out.push(ShieldedTokenPoolBalance {
                watch,
                account_index,
                balance,
                unspent_note_count,
                reserved_balance,
                scanned_note_count: watermark(conn, id, watch.token_id)?,
                pool_note_count,
                scanned_height,
                last_error: last_error.clone(),
            });
        }
    }
    Ok(out)
}

/// The markers of the token note reservations this process holds now: a
/// reservation marker from [`reserve_token_notes`] until the spend's activity
/// id replaces it, then that id until the spend has settled. Read with the
/// reservations under the connection's lock, so the sweep
/// ([`release_orphaned_reservations`]) never releases a spend still being
/// built or settled.
static IN_FLIGHT: std::sync::Mutex<BTreeSet<[u8; 32]>> = std::sync::Mutex::new(BTreeSet::new());

fn in_flight() -> std::sync::MutexGuard<'static, BTreeSet<[u8; 32]>> {
    IN_FLIGHT
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// The token notes one spend holds ([`reserve_token_notes`]).
///
/// The reservation is kept in the store, so it outlives the process; what
/// releases it is either the spend's own settlement ([`Self::release`], for a
/// spend that never reached the network) or the sweep, for a reservation
/// whose spend is no longer in flight here and has no saved record left: the
/// scan found its fee notes spent or their anchor pruned, or the process that
/// made it is gone. A note the scan sees spent stays spent whatever its
/// reservation says.
#[derive(Debug)]
pub(crate) struct TokenNoteReservation {
    marker: [u8; 32],
}

impl TokenNoteReservation {
    /// Hold the notes under `activity_id`, the id of the spend's saved
    /// record, from now on.
    pub(crate) fn rekey<S: TokenPoolConnection>(
        &mut self,
        store: &S,
        activity_id: [u8; 32],
    ) -> Result<(), PlatformWalletError> {
        store.with_token_pool_conn(|conn| {
            conn.execute(
                "UPDATE shielded_token_pool_notes SET reserved_by = ?1 WHERE reserved_by = ?2",
                params![activity_id.as_slice(), self.marker.as_slice()],
            )
            .map_err(storage("rekey reservation"))?;
            let mut held = in_flight();
            held.insert(activity_id);
            held.remove(&self.marker);
            Ok(())
        })?;
        self.marker = activity_id;
        Ok(())
    }

    /// Release the notes, for a spend that cannot execute.
    pub(crate) fn release<S: TokenPoolConnection>(self, store: &S) {
        let marker = self.marker;
        if let Err(e) = store.with_token_pool_conn(|conn| {
            conn.execute(
                "UPDATE shielded_token_pool_notes SET reserved_by = NULL WHERE reserved_by = ?1",
                params![marker.as_slice()],
            )
            .map_err(storage("release reservation"))
        }) {
            // Left reserved, the notes come back at the next sweep.
            warn!(error = %e, "releasing token notes failed; the next sync releases them");
        }
    }
}

impl Drop for TokenNoteReservation {
    fn drop(&mut self) {
        in_flight().remove(&self.marker);
    }
}

/// A reservation marker no other reservation of this or an earlier process
/// shares: a hash of the process id, the time and a counter.
fn new_reservation_marker() -> [u8; 32] {
    use sha2::{Digest, Sha256};
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let mut hasher = Sha256::new();
    hasher.update(b"token note reservation");
    hasher.update(std::process::id().to_le_bytes());
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|elapsed| elapsed.as_nanos())
        .unwrap_or_default();
    hasher.update(nanos.to_le_bytes());
    hasher.update(
        NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
            .to_le_bytes(),
    );
    hasher.finalize().into()
}

/// Reserve `id`'s unspent, unreserved notes in `token_id`'s pool covering
/// `amount`, largest first, at most `max_notes` of them. Refuses an amount the
/// notes do not cover ([`PlatformWalletError::TokenShieldedInsufficientBalance`])
/// and one needing more notes than allowed
/// ([`PlatformWalletError::ShieldedTooManyNotes`]) without reserving anything.
pub(crate) fn reserve_token_notes<S: TokenPoolConnection>(
    store: &S,
    id: SubwalletId,
    token_id: Identifier,
    amount: u64,
    max_notes: usize,
) -> Result<(Vec<TokenPoolNote>, TokenNoteReservation), PlatformWalletError> {
    store.with_token_pool_conn(|conn| {
        let selected = select_token_notes(conn, id, token_id, amount, max_notes)?;
        let marker = new_reservation_marker();
        let tx = conn.transaction().map_err(storage("reserve"))?;
        for note in &selected {
            tx.execute(
                "UPDATE shielded_token_pool_notes SET reserved_by = ?1 \
                 WHERE wallet_id = ?2 AND account_index = ?3 AND token_id = ?4 AND nullifier = ?5",
                params![
                    marker.as_slice(),
                    id.wallet_id.as_slice(),
                    id.account_index,
                    token_id.as_slice(),
                    note.nullifier.as_slice()
                ],
            )
            .map_err(storage("reserve"))?;
        }
        tx.commit().map_err(storage("reserve"))?;
        in_flight().insert(marker);
        let selected = selected
            .into_iter()
            .map(|note| TokenPoolNote {
                reserved_by: Some(marker),
                ..note
            })
            .collect();
        Ok((selected, TokenNoteReservation { marker }))
    })
}

/// The notes [`reserve_token_notes`] would reserve, refused as it refuses,
/// without reserving them.
pub(crate) fn select_token_notes(
    conn: &Connection,
    id: SubwalletId,
    token_id: Identifier,
    amount: u64,
    max_notes: usize,
) -> Result<Vec<TokenPoolNote>, PlatformWalletError> {
    let mut free = spendable_notes(conn, id, token_id)?;
    free.sort_by(|a, b| b.value.cmp(&a.value).then(a.position.cmp(&b.position)));
    let available = free
        .iter()
        .try_fold(0u64, |total, note| total.checked_add(note.value))
        .unwrap_or(u64::MAX);
    let mut selected = Vec::new();
    let mut covered = 0u64;
    for note in free {
        if covered >= amount {
            break;
        }
        covered = covered.saturating_add(note.value);
        selected.push(note);
    }
    if covered < amount || selected.is_empty() {
        return Err(PlatformWalletError::TokenShieldedInsufficientBalance {
            token_id,
            available,
            required: amount,
        });
    }
    if selected.len() > max_notes {
        return Err(PlatformWalletError::ShieldedTooManyNotes {
            required_notes: selected.len(),
            max_notes,
        });
    }
    Ok(selected)
}

/// One reservation row as read: wallet id, account, token id and marker bytes.
type HeldReservation = (Vec<u8>, u32, Vec<u8>, Vec<u8>);

/// Release the reservations in the pools of `scanned` (the pools this pass
/// scanned to their end) held by neither a spend in flight here nor a saved
/// record in `live_records`. Their spend can no longer execute (the scan
/// found its fee notes spent or pruned its anchor, which drops the record) or
/// its process is gone; a note it did spend is marked spent by the scan, which
/// ran first. Returns each account and marker released: the marker is the
/// spend's activity id once its record was saved, so the caller can settle
/// its activity row.
pub(crate) fn release_orphaned_reservations(
    conn: &Connection,
    scanned: &BTreeSet<Identifier>,
    live_records: &BTreeSet<[u8; 32]>,
) -> Result<BTreeSet<(SubwalletId, [u8; 32])>, PlatformWalletError> {
    let mut statement = conn
        .prepare(
            "SELECT DISTINCT wallet_id, account_index, token_id, reserved_by \
             FROM shielded_token_pool_notes WHERE reserved_by IS NOT NULL AND spent = 0",
        )
        .map_err(storage("read reservations"))?;
    let held: Vec<HeldReservation> = statement
        .query_map([], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
        })
        .map_err(storage("read reservations"))?
        .collect::<rusqlite::Result<_>>()
        .map_err(storage("read reservations"))?;
    let in_flight = in_flight();
    let mut released = BTreeSet::new();
    for (wallet_id, account_index, token_id, marker) in held {
        let id = SubwalletId::new(bytes32(wallet_id, "wallet id")?, account_index);
        let token_id = Identifier::from(bytes32(token_id, "token id")?);
        let marker = bytes32(marker, "reservation")?;
        if !scanned.contains(&token_id)
            || in_flight.contains(&marker)
            || live_records.contains(&marker)
        {
            continue;
        }
        conn.execute(
            "UPDATE shielded_token_pool_notes SET reserved_by = NULL \
             WHERE wallet_id = ?1 AND account_index = ?2 AND token_id = ?3 \
             AND reserved_by = ?4",
            params![
                id.wallet_id.as_slice(),
                id.account_index,
                token_id.as_slice(),
                marker.as_slice()
            ],
        )
        .map_err(storage("release reservation"))?;
        released.insert((id, marker));
    }
    Ok(released)
}

/// The Merkle paths of `selected`, notes of `token_id`'s pool, and the anchor
/// they share: the root of the pool's tree over the leaves the scan kept, from
/// position 0 to the first one missing. That is the pool's root when its last
/// scan ended, which is one of the anchors the pool records while it keeps
/// it; the caller checks that it is.
pub(crate) fn spend_paths(
    conn: &Connection,
    token_id: Identifier,
    selected: &[TokenPoolNote],
) -> Result<(Vec<(u64, MerklePath)>, Anchor), PlatformWalletError> {
    let positions: BTreeSet<u64> = selected.iter().map(|note| note.position).collect();
    let mut statement = conn
        .prepare(
            "SELECT position, cmx FROM shielded_token_pool_leaves \
             WHERE token_id = ?1 ORDER BY position",
        )
        .map_err(storage("read leaves"))?;
    let rows = statement
        .query_map(params![token_id.as_slice()], |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, Vec<u8>>(1)?))
        })
        .map_err(storage("read leaves"))?;
    let built = |e: grovedb_commitment_tree::CommitmentTreeError| {
        PlatformWalletError::ShieldedMerkleWitnessUnavailable(format!(
            "the tree of the shielded pool of token {token_id} does not build: {e}"
        ))
    };
    let mut tree = ClientMemoryCommitmentTree::new(1);
    let mut next = 0u64;
    for row in rows {
        let (position, cmx) = row.map_err(storage("read leaves"))?;
        if from_sql_u64(position, "leaf position")? != next {
            break;
        }
        let retention = if positions.contains(&next) {
            Retention::Marked
        } else {
            Retention::Ephemeral
        };
        tree.append(bytes32(cmx, "leaf")?, retention)
            .map_err(built)?;
        next += 1;
    }
    tree.checkpoint(1).map_err(built)?;
    let anchor = tree.anchor().map_err(built)?;
    let mut paths = Vec::with_capacity(selected.len());
    for note in selected {
        let path = (note.position < next)
            .then(|| tree.witness(Position::from(note.position), 0))
            .transpose()
            .map_err(built)?
            .flatten()
            .ok_or_else(|| {
                PlatformWalletError::ShieldedMerkleWitnessUnavailable(format!(
                    "note {} of the shielded pool of token {token_id} is past the leaves kept; \
                     sync the shielded pools first",
                    note.position
                ))
            })?;
        paths.push((note.position, path));
    }
    Ok((paths, anchor))
}

/// Insert `note` as the scan would have, for tests that start from owned
/// token notes.
#[cfg(test)]
pub(crate) fn insert_note_for_test(
    conn: &Connection,
    note: &TokenPoolNote,
) -> Result<(), PlatformWalletError> {
    conn.execute(
        "INSERT INTO shielded_token_pool_notes \
         (wallet_id, account_index, token_id, position, cmx, nullifier, value, note_data, \
          block_height, spent, reserved_by) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
        params![
            note.id.wallet_id.as_slice(),
            note.id.account_index,
            note.token_id.as_slice(),
            to_sql_u64(note.position, "position")?,
            note.cmx.as_slice(),
            note.nullifier.as_slice(),
            note.value.to_le_bytes().as_slice(),
            note.note_data,
            to_sql_u64(note.block_height, "height")?,
            i64::from(note.is_spent),
            note.reserved_by.map(|marker| marker.to_vec()),
        ],
    )
    .map_err(storage("insert note"))?;
    Ok(())
}

/// The accounts that scan one pool and the viewing keys they decrypt with,
/// each with its watermark row there as this scan last read or wrote it
/// (`None`: no row).
struct PoolScanner<'a> {
    token_id: Identifier,
    accounts: Vec<(SubwalletId, &'a AccountViewingKeys, Option<u64>)>,
}

/// What one chunk did to the store.
#[derive(Default)]
struct ChunkOutcome {
    new_notes: usize,
    newly_spent: usize,
    /// No account scans the pool any more (see [`apply_chunk`]): the scan
    /// ends.
    stopped: bool,
}

/// Whether `wallet_id` watches `token_id`'s pool.
fn is_watched(
    conn: &Connection,
    wallet_id: WalletId,
    token_id: Identifier,
) -> Result<bool, PlatformWalletError> {
    conn.query_row(
        "SELECT 1 FROM shielded_token_pool_watches WHERE wallet_id = ?1 AND token_id = ?2",
        params![wallet_id.as_slice(), token_id.as_slice()],
        |_| Ok(()),
    )
    .optional()
    .map(|row| row.is_some())
    .map_err(storage("read watch"))
}

/// Apply one chunk read from `start_index` to the store in one transaction:
/// keep its commitments, save the notes each account decrypts at a position
/// it had not checked, mark the account notes whose nullifiers the chunk
/// reveals spent (after the saves, so a note received and spent within the
/// chunk ends spent), and raise every account's watermark to the chunk's end.
///
/// The chunk was read without the store, so a purge may have landed since the
/// scan last wrote (an unwatch, a wallet's removal, a Clear, an account a
/// rebind dropped). Inside the transaction, so no purge can land between the
/// check and the writes, it keeps only the accounts whose wallet still
/// watches the pool and whose watermark row is still what this scan last read
/// or wrote; the others are dropped from the scan. With none left, nothing is
/// written and the scan stops, so a purge is never undone.
fn apply_chunk(
    conn: &mut Connection,
    scanner: &mut PoolScanner<'_>,
    start_index: u64,
    chunk: &TokenPoolChunk,
) -> Result<ChunkOutcome, PlatformWalletError> {
    let token = scanner.token_id;
    let end = start_index
        .checked_add(chunk.notes.len() as u64)
        .ok_or_else(|| {
            PlatformWalletError::ShieldedSyncFailed(format!(
                "the shielded pool of token {token} runs past u64 positions"
            ))
        })?;
    let height = to_sql_u64(chunk.height, "scan height")?;
    let mut outcome = ChunkOutcome::default();
    let tx = conn.transaction().map_err(storage("begin chunk"))?;
    let mut still_scanning = Vec::with_capacity(scanner.accounts.len());
    for account in scanner.accounts.drain(..) {
        let (id, _, row) = account;
        if is_watched(&tx, id.wallet_id, token)? && watermark(&tx, id, token)? == row {
            still_scanning.push(account);
        }
    }
    scanner.accounts = still_scanning;
    if scanner.accounts.is_empty() {
        outcome.stopped = true;
        return Ok(outcome);
    }
    for (offset, raw) in chunk.notes.iter().enumerate() {
        let position = start_index + offset as u64;
        let cmx = <[u8; 32]>::try_from(raw.cmx.as_slice()).map_err(|_| {
            PlatformWalletError::ShieldedSyncFailed(format!(
                "note {position} of the shielded pool of token {token} has a {}-byte commitment",
                raw.cmx.len()
            ))
        })?;
        tx.execute(
            "INSERT OR IGNORE INTO shielded_token_pool_leaves (token_id, position, cmx) \
             VALUES (?1, ?2, ?3)",
            params![
                token.as_slice(),
                to_sql_u64(position, "leaf position")?,
                cmx.as_slice()
            ],
        )
        .map_err(storage("keep leaf"))?;
        for (id, views, watermark) in &scanner.accounts {
            if position < watermark.unwrap_or(0) {
                continue;
            }
            let Some((note, _address)) = try_decrypt_note(&views.prepared_ivk, raw) else {
                continue;
            };
            let nullifier = note.nullifier(&views.full_viewing_key).to_bytes();
            let inserted = tx
                .execute(
                    "INSERT OR IGNORE INTO shielded_token_pool_notes \
                     (wallet_id, account_index, token_id, position, cmx, nullifier, value, \
                      note_data, block_height) \
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                    params![
                        id.wallet_id.as_slice(),
                        id.account_index,
                        token.as_slice(),
                        to_sql_u64(position, "note position")?,
                        cmx.as_slice(),
                        nullifier.as_slice(),
                        note.value().inner().to_le_bytes().as_slice(),
                        serialize_note(&note),
                        height,
                    ],
                )
                .map_err(storage("save note"))?;
            if inserted > 0 {
                debug!(
                    wallet_id = %hex::encode(id.wallet_id),
                    account = id.account_index,
                    token = %token,
                    position,
                    "token pool note decrypted"
                );
                outcome.new_notes += 1;
            }
        }
    }
    for raw in &chunk.notes {
        outcome.newly_spent += tx
            .execute(
                "UPDATE shielded_token_pool_notes SET spent = 1 \
                 WHERE token_id = ?1 AND nullifier = ?2 AND spent = 0",
                params![token.as_slice(), raw.nullifier.as_slice()],
            )
            .map_err(storage("mark spent"))?;
    }
    let end_sql = to_sql_u64(end, "watermark")?;
    for (id, _, watermark) in &mut scanner.accounts {
        tx.execute(
            "INSERT INTO shielded_token_pool_watermarks \
             (wallet_id, account_index, token_id, scanned_through) VALUES (?1, ?2, ?3, ?4) \
             ON CONFLICT(wallet_id, account_index, token_id) \
             DO UPDATE SET scanned_through = MAX(scanned_through, excluded.scanned_through)",
            params![
                id.wallet_id.as_slice(),
                id.account_index,
                token.as_slice(),
                end_sql
            ],
        )
        .map_err(storage("raise watermark"))?;
        *watermark = Some(watermark.unwrap_or(0).max(end));
    }
    tx.execute(
        "INSERT INTO shielded_token_pool_scans \
         (token_id, pool_note_count, scanned_height, last_error) VALUES (?1, ?2, ?3, NULL) \
         ON CONFLICT(token_id) DO UPDATE SET pool_note_count = excluded.pool_note_count, \
         scanned_height = excluded.scanned_height, last_error = NULL",
        params![
            token.as_slice(),
            to_sql_u64(chunk.total_count, "pool note count")?,
            height
        ],
    )
    .map_err(storage("record scan"))?;
    tx.commit().map_err(storage("commit chunk"))?;
    Ok(outcome)
}

/// The store's SQLite connection the token pool tables live on.
pub trait TokenPoolConnection: Send + Sync {
    /// Run `f` on the connection, holding whatever the store needs held.
    fn with_token_pool_conn<T>(
        &self,
        f: impl FnOnce(&mut Connection) -> Result<T, PlatformWalletError>,
    ) -> Result<T, PlatformWalletError>;
}

/// Scan one pool for `scanner`'s accounts, chunk by chunk from the lowest
/// watermark rounded down to a chunk boundary, until a chunk comes back
/// short. Chunks are read one after another and each is applied whole
/// before the next is asked for, so a watermark only ever covers positions
/// that were all read: a chunk proven at an earlier height that comes back
/// short ends the pass there, and the next pass reads on from it.
async fn scan_pool<F, S>(
    source: &F,
    store: &RwLock<S>,
    mut scanner: PoolScanner<'_>,
    cancel: &CancellationToken,
) -> Result<ChunkOutcome, PlatformWalletError>
where
    F: TokenPoolNoteSource + ?Sized,
    S: TokenPoolConnection,
{
    let capacity = source.token_pool_chunk_capacity().max(1);
    let lowest = scanner
        .accounts
        .iter()
        .map(|(_, _, watermark)| watermark.unwrap_or(0))
        .min()
        .unwrap_or(0);
    let mut start = lowest / CHUNK_SIZE * CHUNK_SIZE;
    let mut total = ChunkOutcome::default();
    loop {
        if cancel.is_cancelled() {
            return Ok(total);
        }
        let chunk = source
            .token_pool_chunk(scanner.token_id, start, capacity)
            .await?;
        let read = chunk.notes.len() as u64;
        let outcome = store
            .read()
            .await
            .with_token_pool_conn(|conn| apply_chunk(conn, &mut scanner, start, &chunk))?;
        total.new_notes += outcome.new_notes;
        total.newly_spent += outcome.newly_spent;
        if outcome.stopped || read < u64::from(capacity) {
            return Ok(total);
        }
        start += read;
    }
}

/// Scan every watched pool for the registered `accounts` of the wallets that
/// watch it. A pool whose scan fails keeps its error for the balances to
/// show, and the others are still scanned. The store lock is taken once per
/// chunk, never across a network read.
pub(crate) async fn sync_token_pools<F, S>(
    source: &F,
    store: &RwLock<S>,
    accounts: &[(SubwalletId, AccountViewingKeys)],
    cancel: &CancellationToken,
) -> TokenPoolSyncSummary
where
    F: TokenPoolNoteSource + ?Sized,
    S: TokenPoolConnection,
{
    let mut summary = TokenPoolSyncSummary::default();
    let planned = store.read().await.with_token_pool_conn(|conn| {
        let mut by_token: BTreeMap<Identifier, Vec<(SubwalletId, Option<u64>)>> = BTreeMap::new();
        for (wallet_id, watched) in watches(conn)? {
            for watch in watched {
                for (id, _) in accounts.iter().filter(|(id, _)| id.wallet_id == wallet_id) {
                    let watermark = watermark(conn, *id, watch.token_id)?;
                    by_token
                        .entry(watch.token_id)
                        .or_default()
                        .push((*id, watermark));
                }
            }
        }
        Ok(by_token)
    });
    let plan = match planned {
        Ok(plan) => plan,
        Err(e) => {
            warn!(error = %e, "token pool scan: reading the watched pools failed");
            return summary;
        }
    };
    for (token_id, scanning) in plan {
        if cancel.is_cancelled() {
            break;
        }
        let scanner = PoolScanner {
            token_id,
            accounts: scanning
                .into_iter()
                .filter_map(|(id, watermark)| {
                    accounts
                        .iter()
                        .find(|(registered, _)| *registered == id)
                        .map(|(_, views)| (id, views, watermark))
                })
                .collect(),
        };
        match scan_pool(source, store, scanner, cancel).await {
            Ok(outcome) => {
                summary.new_notes += outcome.new_notes;
                summary.newly_spent += outcome.newly_spent;
                if !cancel.is_cancelled() {
                    summary.completed.insert(token_id);
                }
            }
            Err(e) => {
                warn!(token = %token_id, error = %e, "token pool scan stopped early");
                let message = e.to_string();
                if let Err(record) = store
                    .read()
                    .await
                    .with_token_pool_conn(|conn| record_scan_error(conn, token_id, &message))
                {
                    warn!(token = %token_id, error = %record, "recording the token pool scan error failed");
                }
                summary.failed.insert(token_id, message);
            }
        }
    }
    if summary.new_notes > 0 || summary.newly_spent > 0 {
        info!(
            new_notes = summary.new_notes,
            newly_spent = summary.newly_spent,
            pools = summary.completed.len(),
            "token pool scan found changes"
        );
    }
    summary
}

#[cfg(test)]
#[path = "token_pool_tests.rs"]
mod tests;
