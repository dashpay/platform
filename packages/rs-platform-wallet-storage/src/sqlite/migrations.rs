//! Schema migration plumbing.
//!
//! Embeds every Rust migration under `migrations/` at compile time
//! (see `refinery::embed_migrations!`). The `run` function applies any
//! pending migrations to the supplied connection.

use rusqlite::OptionalExtension;

use crate::sqlite::error::WalletStorageError;
use refinery_core::error::WrapMigrationError;

mod legacy_v008;

// Generates a `migrations` module with `runner()`; path is relative to
// the crate root.
refinery::embed_migrations!("./migrations");

/// Apply every pending migration to `conn`.
pub fn run(conn: &mut rusqlite::Connection) -> Result<refinery::Report, refinery::Error> {
    run_with_runner(conn, migrations::runner())
}

/// Keep refinery's history validation, target selection and SQL/history writes
/// inside the same writer exclusion as the typed legacy conversion. Its native
/// rusqlite driver starts its own transactions, so this adapter implements the
/// supported driver traits over our outer transaction instead.
fn run_with_runner(
    conn: &mut rusqlite::Connection,
    runner: refinery::Runner,
) -> Result<refinery::Report, refinery::Error> {
    let tx = conn
        .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
        .migration_err("begin wallet schema migration", None)?;
    let embedded = migrations::runner();
    let hook_sql = |version| {
        embedded
            .get_migrations()
            .iter()
            .find(|m| m.version() == version)
            .and_then(|m| m.sql())
            .expect("embedded typed migration must exist")
            .to_owned()
    };
    let mut driver = MigrationTransaction {
        tx,
        registration_sql: hook_sql(8),
        pool_sql: hook_sql(11),
        history_sql: hook_sql(19),
    };
    // Grouped reports never claim that rolled-back migrations were applied.
    let report = runner.set_grouped(true).run(&mut driver)?;
    driver
        .tx
        .commit()
        .migration_err("commit wallet schema migration", None)?;
    Ok(report)
}

struct MigrationTransaction<'conn> {
    tx: rusqlite::Transaction<'conn>,
    registration_sql: String,
    pool_sql: String,
    history_sql: String,
}

impl refinery_core::traits::sync::Transaction for MigrationTransaction<'_> {
    type Error = WalletStorageError;

    fn execute<'a, T: Iterator<Item = &'a str>>(
        &mut self,
        queries: T,
    ) -> Result<usize, Self::Error> {
        let mut count = 0;
        for query in queries {
            self.tx.execute_batch(query)?;
            if query == self.registration_sql {
                legacy_v008::backfill_registrations(&self.tx)?;
            } else if query == self.pool_sql {
                legacy_v008::convert_pools(&self.tx)?;
            } else if query == self.history_sql {
                history_repair_v019::repair_history(&self.tx)?;
            }
            count += 1;
        }
        Ok(count)
    }
}

impl refinery_core::traits::sync::Query<Vec<refinery::Migration>> for MigrationTransaction<'_> {
    fn query(&mut self, query: &str) -> Result<Vec<refinery::Migration>, Self::Error> {
        let mut stmt = self.tx.prepare(query)?;
        let mut rows = stmt.query([])?;
        let mut applied = Vec::new();
        while let Some(row) = rows.next()? {
            let timestamp: String = row.get(2)?;
            let applied_on = time::OffsetDateTime::parse(
                &timestamp,
                &time::format_description::well_known::Rfc3339,
            )
            .map_err(|_| WalletStorageError::SchemaHistoryMalformed {
                reason: "applied_on is not a valid RFC3339 timestamp",
            })?;
            let checksum: String = row.get(3)?;
            let checksum =
                checksum
                    .parse()
                    .map_err(|_| WalletStorageError::SchemaHistoryMalformed {
                        reason: "checksum is not a valid u64",
                    })?;
            applied.push(refinery::Migration::applied(
                row.get(0)?,
                row.get(1)?,
                applied_on,
                checksum,
            ));
        }
        Ok(applied)
    }
}

impl refinery_core::traits::sync::Migrate for MigrationTransaction<'_> {}

/// Apply migrations on behalf of [`crate::sqlite::persister::SqlitePersister::open`].
///
/// A typed-error chokepoint: a single entry point for any future
/// migration that needs to re-classify a refinery error.
pub(crate) fn run_for_open(
    conn: &mut rusqlite::Connection,
) -> Result<refinery::Report, WalletStorageError> {
    run(conn).map_err(WalletStorageError::Migration)
}

/// Return a fresh atomic migration runner seeded with the embedded list.
/// Tests can apply a subset via [`MigrationRunner::set_target`].
#[cfg(any(test, feature = "__test-helpers"))]
pub fn runner() -> MigrationRunner {
    MigrationRunner(migrations::runner())
}

/// Test runner using the same atomic conversion path as production open.
#[cfg(any(test, feature = "__test-helpers"))]
pub struct MigrationRunner(refinery::Runner);

#[cfg(any(test, feature = "__test-helpers"))]
impl MigrationRunner {
    /// Stop after the selected migration, retaining staged legacy rows.
    pub fn set_target(self, target: refinery::Target) -> Self {
        Self(self.0.set_target(target))
    }

    /// Apply the selected migration set and typed conversions atomically.
    pub fn run(self, conn: &mut rusqlite::Connection) -> Result<refinery::Report, refinery::Error> {
        run_with_runner(conn, self.0)
    }
}

/// Highest migration version this binary knows how to apply. Used by
/// both `SqlitePersister::open` and `backup::restore_from` to refuse
/// forward-version databases.
pub fn max_supported_version() -> i64 {
    embedded_migrations()
        .iter()
        .map(|(v, _)| *v as i64)
        .max()
        .unwrap_or(0)
}

/// Returns true if the `refinery_schema_history` table exists on this
/// connection. Used by `open`, `restore_from`, and `count_pending` to
/// distinguish "fresh DB" (no migrations applied yet) from
/// "pre-existing DB" (carries refinery history).
pub(crate) fn has_schema_history(conn: &rusqlite::Connection) -> Result<bool, WalletStorageError> {
    let exists = conn
        .query_row(
            "SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = 'refinery_schema_history'",
            [],
            |_| Ok(()),
        )
        .optional()?
        .is_some();
    Ok(exists)
}

/// Returns true if the database already carries any schema object (table,
/// index, view, or trigger). `open` uses it to tell a brand-new empty DB
/// apart from a pre-existing NON-wallet SQLite file that lacks
/// `refinery_schema_history`: the former is migrated, the latter rejected
/// rather than silently grafting wallet tables onto a foreign schema.
pub(crate) fn db_has_objects(conn: &rusqlite::Connection) -> Result<bool, WalletStorageError> {
    let exists = conn
        .query_row("SELECT 1 FROM sqlite_master LIMIT 1", [], |_| Ok(()))
        .optional()?
        .is_some();
    Ok(exists)
}

/// Refuse to operate on a DB whose `refinery_schema_history` MAX(version)
/// exceeds [`max_supported_version`], returning
/// [`WalletStorageError::SchemaVersionUnsupported`]. This is a forward-only
/// gate — it refuses a newer DB but never migrates it down (SQLite
/// migrations are one-directional).
///
/// Quietly succeeds when the table is absent; the caller decides what a
/// missing schema-history means (`restore_from` rejects it, `open` treats
/// it as a brand-new DB).
pub fn assert_schema_version_supported(
    conn: &rusqlite::Connection,
) -> Result<(), WalletStorageError> {
    if !has_schema_history(conn)? {
        return Ok(());
    }
    let source_version: Option<i64> = conn
        .query_row(
            "SELECT MAX(version) FROM refinery_schema_history",
            [],
            |row| row.get(0),
        )
        .optional()?
        .flatten();
    let max_supported = max_supported_version();
    if let Some(v) = source_version {
        if v > max_supported {
            return Err(WalletStorageError::SchemaVersionUnsupported {
                found: v,
                max_supported,
            });
        }
    }
    Ok(())
}

fn refinery_timestamp_is_valid(value: &str) -> bool {
    let bytes = value.as_bytes();
    let digits = |range: std::ops::Range<usize>| bytes[range].iter().all(u8::is_ascii_digit);
    let shape_is_valid = bytes.len() >= 20
        && bytes.len() <= 30
        && digits(0..4)
        && bytes[4] == b'-'
        && digits(5..7)
        && bytes[7] == b'-'
        && digits(8..10)
        && bytes[10] == b'T'
        && digits(11..13)
        && bytes[13] == b':'
        && digits(14..16)
        && bytes[16] == b':'
        && digits(17..19)
        && bytes[17] <= b'5'
        && bytes[bytes.len() - 1] == b'Z'
        && (bytes.len() == 20 || (bytes[19] == b'.' && digits(20..bytes.len().saturating_sub(1))));

    shape_is_valid && chrono::DateTime::parse_from_rfc3339(value).is_ok()
}

/// Probe `refinery_schema_history` rows BEFORE handing the connection to
/// refinery, which parses `applied_on` (RFC3339) and `checksum` (`u64`)
/// with `unwrap()` — a malformed value would abort the process. Surfaces
/// a typed [`WalletStorageError::SchemaHistoryMalformed`] instead.
/// Quietly succeeds when the table is absent.
pub(crate) fn assert_schema_history_well_formed(
    conn: &rusqlite::Connection,
) -> Result<(), WalletStorageError> {
    if !has_schema_history(conn)? {
        return Ok(());
    }
    let mut stmt = conn.prepare("SELECT applied_on, checksum FROM refinery_schema_history")?;
    let rows = stmt.query_map([], |row| {
        let applied_on: String = row.get(0)?;
        let checksum: String = row.get(1)?;
        Ok((applied_on, checksum))
    })?;
    for row in rows {
        let (applied_on, checksum) = row?;
        // Deliberately NARROWER than refinery's own reader, which accepts any
        // RFC3339 value (`drivers/rusqlite.rs` parses with `time::Rfc3339`).
        // We can be narrower because refinery only ever WRITES the canonical
        // UTC shape: `runner.rs` stamps `OffsetDateTime::now_utc()` and
        // `traits/mod.rs` formats it with `time::Rfc3339`, which emits a
        // trailing `Z` for a UTC offset rather than `+00:00`. So every value
        // refinery produced passes this gate, while a hand-written `+00:00`
        // is refused — a typed error, which is the point: the panic this
        // guard replaces is unrecoverable.
        if !refinery_timestamp_is_valid(&applied_on) {
            return Err(WalletStorageError::SchemaHistoryMalformed {
                reason: "applied_on is not a valid RFC3339 timestamp",
            });
        }
        if checksum.parse::<u64>().is_err() {
            return Err(WalletStorageError::SchemaHistoryMalformed {
                reason: "checksum is not a valid u64",
            });
        }
    }
    Ok(())
}

/// True when `refinery_schema_history` is non-empty and every applied
/// `(version, name)` pair names a migration this binary embeds at that same
/// version.
///
/// This is the positive identification behind
/// [`crate::sqlite::conn::assert_wallet_application_id_or_legacy`]: a foreign
/// SQLite database does not carry our migration names, so an unstamped file
/// that passes here is a wallet database created before the header stamp
/// existed. Deliberately name-only — a checksum comparison is refinery's job
/// moments later, and doing it here would report a divergent body as
/// "not a wallet database", which is the wrong diagnosis.
pub(crate) fn applied_history_matches_embedded(
    conn: &rusqlite::Connection,
) -> Result<bool, WalletStorageError> {
    let embedded: std::collections::HashMap<i32, String> =
        embedded_migrations().into_iter().collect();
    let mut stmt = conn.prepare("SELECT version, name FROM refinery_schema_history")?;
    let rows = stmt.query_map([], |row| {
        Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
    })?;
    let mut saw_row = false;
    for row in rows {
        let (version, name) = row?;
        saw_row = true;
        let Ok(version) = i32::try_from(version) else {
            return Ok(false);
        };
        if embedded.get(&version) != Some(&name) {
            return Ok(false);
        }
    }
    Ok(saw_row)
}

/// List `(version, name)` of every embedded migration. Used by tests and
/// the migration-drift hash check.
pub fn embedded_migrations() -> Vec<(i32, String)> {
    migrations::runner()
        .get_migrations()
        .iter()
        .map(|m| (m.version(), m.name().to_string()))
        .collect()
}

/// SHA-256 over `(version, name)` of every embedded migration in version
/// order. Deliberately content-blind: it hashes the migration set's
/// identity, not the SQL bodies, so it catches an added/removed/renamed
/// migration but ignores in-place DDL edits (a content-pinning guard
/// belongs with the schema freeze at release).
#[cfg(any(test, feature = "__test-helpers"))]
pub fn embedded_migrations_fingerprint() -> [u8; 32] {
    use sha2::{Digest, Sha256};
    let mut entries = embedded_migrations();
    entries.sort_by_key(|(v, _)| *v);
    let mut hasher = Sha256::new();
    for (v, name) in entries {
        hasher.update(v.to_be_bytes());
        hasher.update([0u8]);
        hasher.update(name.as_bytes());
        hasher.update([0u8]);
    }
    hasher.finalize().into()
}

/// SHA-256 over `(version, name, rendered SQL)` of every embedded migration
/// in version order. Unlike [`embedded_migrations_fingerprint`] this is
/// content-level: it pins each migration's SQL body, so an in-place DDL edit
/// (e.g. renaming a table inside a same-named file) breaks the golden test.
/// This is the guard the D0 schema freeze relies on; the identity-only
/// fingerprint cannot catch a same-name body edit.
///
/// The SQL *text* is deterministic even where a value is generated at run
/// time (`randomblob(16)`): the literal string is hashed, not the runtime
/// bytes.
#[cfg(any(test, feature = "__test-helpers"))]
pub fn embedded_migrations_sql_fingerprint() -> [u8; 32] {
    use sha2::{Digest, Sha256};
    let mut migrations = migrations::runner().get_migrations().clone();
    migrations.sort_by_key(|m| m.version());
    let mut hasher = Sha256::new();
    for m in &migrations {
        hasher.update((m.version() as u32).to_be_bytes());
        hasher.update([0u8]);
        hasher.update(m.name().as_bytes());
        hasher.update([0u8]);
        let sql = m
            .sql()
            .expect("embedded migrations always carry rendered SQL");
        hasher.update(sql.as_bytes());
        hasher.update([0u8]);
    }
    hasher.finalize().into()
}

/// Rendered SQL of every embedded migration paired with its version, in
/// version order. The retired-name guard needs the version to skip the
/// pre-rename history, where a retired table name is correct.
#[cfg(any(test, feature = "__test-helpers"))]
pub fn embedded_migrations_sql_by_version() -> Vec<(i32, String)> {
    let mut migrations = migrations::runner().get_migrations().clone();
    migrations.sort_by_key(|m| m.version());
    migrations
        .iter()
        .map(|m| {
            (
                m.version(),
                m.sql()
                    .expect("embedded migrations always carry rendered SQL")
                    .to_string(),
            )
        })
        .collect()
}

/// Rendered SQL of every embedded migration, in version order. Used by the
/// schema-freeze grep guard to scan for retired table names.
#[cfg(any(test, feature = "__test-helpers"))]
pub fn embedded_migrations_sql() -> Vec<String> {
    let mut migrations = migrations::runner().get_migrations().clone();
    migrations.sort_by_key(|m| m.version());
    migrations
        .iter()
        .map(|m| {
            m.sql()
                .expect("embedded migrations always carry rendered SQL")
                .to_string()
        })
        .collect()
}

/// Undo V019 so the next [`run`] replays it over the current rows.
/// The V019 history repair: backfills the raw-input index and corrects
/// stored accounting for databases migrating from V018 or earlier. Only the
/// migration-specific handling lives here (strict record reads, the
/// undecodable-record resync policy, the entry point); the repair itself is
/// `core_history`'s frozen `_v019` functions, which the live per-round
/// repair shares. Beyond those, this relies only on helpers that must stay
/// behaviour-stable: the blob codec and its size/width gates (`blob`),
/// `id32`, `i64_to_u32` and the `WalletStorageError` variants they return.
/// Do not change its behaviour.
mod history_repair_v019 {
    use dashcore::hashes::Hash;
    use dashcore::Txid;
    use key_wallet::managed_account::transaction_record::TransactionRecord;
    use platform_wallet::wallet::platform_wallet::WalletId;
    use rusqlite::{params, OptionalExtension, Transaction};

    use crate::sqlite::error::WalletStorageError;
    use crate::sqlite::schema::core_history::{
        index_record_v019, network_v019, repair_record_v019,
    };
    use crate::sqlite::schema::{blob, id32};
    use crate::sqlite::util::safe_cast::i64_to_u32;

    /// Read a stored record; undecodable bytes are an error the caller classifies.
    fn read_record(
        tx: &Transaction<'_>,
        wallet_id: &WalletId,
        txid: &Txid,
    ) -> Result<Option<TransactionRecord>, WalletStorageError> {
        let key = params![wallet_id.as_slice(), txid.as_byte_array().as_slice()];
        // Gate the stored length in its own statement: SQLite evaluates every
        // result column before a row is returned, so selecting the payload
        // alongside its length would load an oversize blob before the check.
        let len: Option<Option<i64>> = tx
            .prepare_cached(
                "SELECT length(record_blob) FROM core_transactions \
                 WHERE wallet_id = ?1 AND txid = ?2",
            )?
            .query_row(key, |row| row.get(0))
            .optional()?;
        let Some(Some(len)) = len else {
            return Ok(None);
        };
        blob::check_size(len)?;
        let payload: Vec<u8> = tx
            .prepare_cached(
                "SELECT record_blob FROM core_transactions \
                 WHERE wallet_id = ?1 AND txid = ?2",
            )?
            .query_row(key, |row| row.get(0))?;
        let record: TransactionRecord = blob::decode(&payload)?;
        if record.txid != *txid {
            return Err(WalletStorageError::blob_decode(
                "transaction record names another transaction",
            ));
        }
        Ok(Some(record))
    }

    /// Whether `error` reports stored bytes that cannot be decoded, not a database failure.
    fn is_unreadable(error: &WalletStorageError) -> bool {
        matches!(
            error,
            WalletStorageError::BincodeDecode { .. }
                | WalletStorageError::BlobDecode { .. }
                | WalletStorageError::BlobTooLarge { .. }
                | WalletStorageError::HashDecode { .. }
                | WalletStorageError::IntegerOverflow { .. }
        )
    }

    /// Drop an undecodable record that a Core resync re-delivers, and force that resync.
    ///
    /// Only a block-confirmed record (typed `height` set) is re-delivered by a
    /// filter rescan; an unconfirmed one may never be seen again, so it keeps the
    /// migration failing rather than losing it. Lowering `synced_height` to just
    /// below the wallet's birth height makes the next SPV start rescan the wallet
    /// from its birth, which re-records the transaction and re-applies its spends.
    /// The spent marks and outputs it already produced stay as they are:
    /// conservative until the rescan confirms them.
    fn drop_for_resync(
        tx: &Transaction<'_>,
        wallet_id: &WalletId,
        txid: &Txid,
        error: WalletStorageError,
    ) -> Result<(), WalletStorageError> {
        let height: Option<i64> = tx.query_row(
            "SELECT height FROM core_transactions WHERE wallet_id = ?1 AND txid = ?2",
            params![wallet_id.as_slice(), txid.as_byte_array().as_slice()],
            |row| row.get(0),
        )?;
        if height.is_none() {
            return Err(error);
        }
        let birth_height: i64 = tx.query_row(
            "SELECT birth_height FROM wallets WHERE wallet_id = ?1",
            params![wallet_id.as_slice()],
            |row| row.get(0),
        )?;
        // A corrupt birth height fails the migration instead of choosing a rescan height.
        let rescan_from = i64_to_u32("wallets.birth_height", birth_height)?.saturating_sub(1);
        tx.execute(
            "DELETE FROM core_transaction_inputs WHERE wallet_id = ?1 AND txid = ?2",
            params![wallet_id.as_slice(), txid.as_byte_array().as_slice()],
        )?;
        tx.execute(
            "DELETE FROM core_transactions WHERE wallet_id = ?1 AND txid = ?2",
            params![wallet_id.as_slice(), txid.as_byte_array().as_slice()],
        )?;
        tx.execute(
            "UPDATE core_sync_state SET synced_height = MIN(COALESCE(synced_height, ?2), ?2) \
             WHERE wallet_id = ?1",
            params![wallet_id.as_slice(), rescan_from],
        )?;
        tracing::warn!(
            wallet_id = %hex::encode(wallet_id),
            %txid,
            %error,
            rescan_from,
            "dropped an undecodable confirmed transaction record; Core history rescans from birth"
        );
        Ok(())
    }

    /// Backfill the raw-input index and repair every stored record once.
    pub(super) fn repair_history(tx: &Transaction<'_>) -> Result<(), WalletStorageError> {
        // Keys are collected first so drops never race the open cursor.
        let mut keys = Vec::new();
        {
            let mut stmt = tx.prepare_cached(
                "SELECT length(wallet_id), wallet_id, length(txid), txid \
                 FROM core_transactions WHERE record_blob IS NOT NULL",
            )?;
            let mut rows = stmt.query([])?;
            while let Some(row) = rows.next()? {
                blob::check_fixed_width(row.get(0)?, 32, "core_transactions.wallet_id")?;
                let wallet_id: Vec<u8> = row.get(1)?;
                let wallet_id = id32("core_transactions.wallet_id", &wallet_id)?;
                blob::check_fixed_width(row.get(2)?, 32, "core_transactions.txid")?;
                let txid: Vec<u8> = row.get(3)?;
                keys.push((wallet_id, Txid::from_slice(&txid)?));
            }
        }
        let mut unrepaired = 0_usize;
        for (wallet_id, txid) in keys {
            match read_record(tx, &wallet_id, &txid) {
                Ok(Some(record)) => {
                    index_record_v019(tx, &wallet_id, &record)?;
                    if !repair_record_v019(tx, &wallet_id, record, network_v019(tx, &wallet_id)?)? {
                        unrepaired += 1;
                    }
                }
                Ok(None) => {}
                Err(error) if is_unreadable(&error) => {
                    drop_for_resync(tx, &wallet_id, &txid, error)?
                }
                Err(error) => return Err(error),
            }
        }
        if unrepaired > 0 {
            // Count only: the txids are peer-supplied.
            tracing::warn!(
                unrepaired,
                "unconfirmed transaction records with amounts beyond the accounting range kept as stored by the history repair"
            );
        }
        Ok(())
    }

    #[cfg(test)]
    mod tests {
        use dashcore::{Address, BlockHash, OutPoint, Transaction as CoreTransaction, TxOut};
        use key_wallet::account::{AccountType, StandardAccountType};
        use key_wallet::managed_account::transaction_record::{
            InputDetail, OutputDetail, OutputRole, TransactionDirection,
        };
        use key_wallet::transaction_checking::{BlockInfo, TransactionContext, TransactionType};
        use key_wallet::Utxo;
        use platform_wallet::changeset::CoreChangeSet;
        use rusqlite::Connection;

        use platform_wallet::test_support::core_fixtures::{
            foreign_address as address, outside, pay, spend_of,
        };

        use super::*;
        use crate::sqlite::migrations::{self, insert_test_wallet, rewind_to_v018, wallet_db};
        use crate::sqlite::schema::core_state;

        fn utxo(outpoint: OutPoint, value: u64, address: Address) -> Utxo {
            Utxo {
                outpoint,
                txout: TxOut {
                    value,
                    script_pubkey: address.script_pubkey(),
                },
                address,
                height: 100,
                is_coinbase: false,
                is_confirmed: true,
                is_instantlocked: false,
                is_locked: false,
                is_trusted: false,
            }
        }

        fn bip44_record(
            body: CoreTransaction,
            context: TransactionContext,
            outputs: Vec<OutputDetail>,
            net: i64,
        ) -> TransactionRecord {
            let account = AccountType::Standard {
                index: 0,
                standard_account_type: StandardAccountType::BIP44Account,
            };
            let (kind, direction) = (TransactionType::Standard, TransactionDirection::Incoming);
            TransactionRecord::new(
                body,
                account,
                context,
                kind,
                direction,
                vec![],
                outputs,
                net,
            )
        }

        /// Store `utxos` and then `record` verbatim, as a V018 build left them.
        fn stage_v018(
            conn: &mut Connection,
            wallet_id: &WalletId,
            utxos: Vec<Utxo>,
            record: &[u8],
            finalized: bool,
        ) {
            let decoded = blob::decode::<TransactionRecord>(record).unwrap();
            let height = decoded.block_info().map(|block| i64::from(block.height()));
            let tx = conn.transaction().unwrap();
            let new_utxos = CoreChangeSet {
                new_utxos: utxos,
                ..Default::default()
            };
            core_state::apply(&tx, wallet_id, &new_utxos).unwrap();
            tx.execute(
                "INSERT OR REPLACE INTO core_transactions (wallet_id, txid, height, finalized, record_blob) \
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    &wallet_id[..],
                    decoded.txid.as_byte_array().as_slice(),
                    height,
                    finalized,
                    record
                ],
            )
            .unwrap();
            rewind_to_v018(&tx);
            tx.commit().unwrap();
        }

        /// A V018 database holding one oversize confirmed record for a wallet born
        /// at `birth_height`, synced to 900.
        fn seed_oversize_confirmed_record(birth_height: i64) -> (Connection, WalletId) {
            let mut conn = Connection::open_in_memory().unwrap();
            migrations::run(&mut conn).unwrap();
            let wallet_id = [0xC2u8; 32];
            let txid = Txid::from_byte_array([0x72; 32]);
            insert_test_wallet(&conn, &wallet_id, birth_height);
            conn.execute(
                "INSERT INTO core_sync_state (wallet_id, last_processed_height, synced_height) \
                 VALUES (?1, 900, 900)",
                params![&wallet_id[..]],
            )
            .unwrap();
            conn.execute(
                "INSERT INTO core_transactions (wallet_id, txid, height, finalized, record_blob) \
                 VALUES (?1, ?2, 300, 1, zeroblob(?3))",
                params![
                    &wallet_id[..],
                    txid.as_byte_array().as_slice(),
                    i64::try_from(blob::BLOB_SIZE_LIMIT_BYTES + 1).unwrap()
                ],
            )
            .unwrap();
            rewind_to_v018(&conn);
            (conn, wallet_id)
        }

        /// `(record count, synced_height)` for `wallet_id`.
        fn records_and_synced_height(conn: &Connection, wallet_id: &WalletId) -> (i64, i64) {
            conn.query_row(
                "SELECT (SELECT count(*) FROM core_transactions WHERE wallet_id = ?1), \
                        (SELECT synced_height FROM core_sync_state WHERE wallet_id = ?1)",
                params![&wallet_id[..]],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap()
        }

        /// An oversize confirmed record fails its length gate before its payload
        /// is read, and is dropped with a rescan from just below the birth height,
        /// or from genesis for a wallet born at zero.
        #[test]
        fn should_drop_oversize_confirmed_record_for_resync() {
            for (birth_height, rescan_from) in [(50, 49), (0, 0)] {
                let (mut conn, wallet_id) = seed_oversize_confirmed_record(birth_height);

                migrations::run(&mut conn).unwrap();

                assert_eq!(
                    records_and_synced_height(&conn, &wallet_id),
                    (0, rescan_from),
                    "born at {birth_height}: the oversize record is dropped for a rescan"
                );
            }
        }

        /// A stored birth height outside `u32` fails the migration with a typed
        /// error, instead of overflowing, and leaves the record and sync state alone.
        #[test]
        fn should_reject_out_of_range_birth_height_before_scheduling_a_rescan() {
            for birth_height in [i64::MIN, -1, i64::from(u32::MAX) + 1, i64::MAX] {
                let (mut conn, wallet_id) = seed_oversize_confirmed_record(birth_height);

                let error = migrations::run(&mut conn).unwrap_err();

                assert!(
                    format!("{error:?}").contains("wallets.birth_height"),
                    "birth height {birth_height}: {error:?}"
                );
                assert_eq!(
                    records_and_synced_height(&conn, &wallet_id),
                    (1, 900),
                    "birth height {birth_height}: the failed upgrade must roll back"
                );
            }
        }

        /// A V018 database holding one record that pays the wallet twice, for a
        /// net past `i64`, in `context`. Returns the stored blob too.
        fn seed_unrepresentable_net(
            context: TransactionContext,
        ) -> (Connection, WalletId, Txid, Vec<u8>) {
            let wallet_id = [0xC3u8; 32];
            let mut conn = wallet_db(&wallet_id);
            let value = (1_u64 << 62) + 1;
            let outputs = [address(1), address(2)].map(|owner| pay(value, &owner));
            let body = spend_of(&[outside(0x73)], outputs.to_vec());
            let txid = body.txid();
            let original = bip44_record(
                body,
                context,
                [address(1), address(2)]
                    .into_iter()
                    .zip(0..)
                    .map(|(owner, index)| OutputDetail {
                        index,
                        role: OutputRole::Received,
                        address: Some(owner),
                        value,
                    })
                    .collect(),
                // What the old checker's unsigned-to-signed cast left behind.
                (2 * value) as i64,
            );
            let stored = blob::encode(&original).unwrap();
            let utxos = [address(1), address(2)]
                .into_iter()
                .zip(0..)
                .map(|(owner, vout)| utxo(OutPoint::new(txid, vout), value, owner))
                .collect();
            stage_v018(&mut conn, &wallet_id, utxos, &stored, false);
            (conn, wallet_id, txid, stored)
        }

        /// Nothing authenticates a mempool transaction, so one the wallet cannot
        /// account for must not keep the store from opening after an upgrade.
        #[test]
        fn should_leave_unrepresentable_mempool_record_unrepaired_when_migrating_from_v018() {
            let (mut conn, wallet_id, txid, stored) =
                seed_unrepresentable_net(TransactionContext::Mempool);

            migrations::run(&mut conn).unwrap();

            let key = params![&wallet_id[..], txid.as_byte_array().as_slice()];
            let (record, originals, indexed): (Vec<u8>, i64, i64) = conn
                .query_row(
                    "SELECT record_blob, \
                        (SELECT count(*) FROM core_transaction_record_originals WHERE wallet_id = ?1 AND txid = ?2), \
                        (SELECT count(*) FROM core_transaction_inputs WHERE wallet_id = ?1 AND txid = ?2) \
                     FROM core_transactions WHERE wallet_id = ?1 AND txid = ?2",
                    key,
                    |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
                )
                .unwrap();
            assert_eq!(record, stored, "the record stays exactly as stored");
            assert_eq!(
                originals, 0,
                "nothing was rewritten, so nothing is archived"
            );
            assert_eq!(
                indexed, 1,
                "its input is still indexed for conflict settlement"
            );
        }

        #[test]
        fn should_fail_migration_on_unrepresentable_confirmed_record() {
            let block = BlockInfo::new(101, BlockHash::all_zeros(), 7);
            let (mut conn, ..) = seed_unrepresentable_net(TransactionContext::InBlock(block));

            let error = migrations::run(&mut conn).unwrap_err();

            assert!(
                format!("{error:?}").contains("NetAmountOverflow"),
                "got {error:?}"
            );
        }

        /// Pins V019's observable result on a V018-shaped database: the repaired
        /// record, the preserved original, the input index and the spent marks.
        #[test]
        fn should_pin_v019_repair_of_a_v018_database() {
            let wallet_id = [0xC1u8; 32];
            let mut conn = wallet_db(&wallet_id);
            let (own, change, contact, external) = (address(1), address(2), address(3), address(4));
            conn.execute(
                "INSERT INTO core_address_pool (wallet_id, account_type, account_index, pool_type, address_index, script) \
                 VALUES (?1, 'dashpay_external', 0, 0, 0, ?2)",
                params![&wallet_id[..], contact.script_pubkey().as_bytes()],
            )
            .unwrap();
            let funding = OutPoint::new(Txid::from_byte_array([0x71; 32]), 0);
            let outputs = vec![
                pay(30_000, &change),
                pay(50_000, &contact),
                pay(15_000, &external),
            ];
            let body = spend_of(&[funding], outputs);
            let txid = body.txid();
            // What an old build stored: the input was not known to be ours and
            // the contact's output was credited as received.
            let original = bip44_record(
                body,
                TransactionContext::InBlock(BlockInfo::new(101, BlockHash::all_zeros(), 7)),
                vec![
                    OutputDetail {
                        index: 0,
                        role: OutputRole::Change,
                        address: Some(change.clone()),
                        value: 30_000,
                    },
                    OutputDetail {
                        index: 1,
                        role: OutputRole::Received,
                        address: Some(contact),
                        value: 50_000,
                    },
                ],
                80_000,
            );
            let utxos = vec![
                utxo(funding, 100_000, own.clone()),
                utxo(OutPoint::new(txid, 0), 30_000, change.clone()),
            ];
            let stored = blob::encode(&original).unwrap();
            stage_v018(&mut conn, &wallet_id, utxos, &stored, true);

            migrations::run(&mut conn).unwrap();

            let mut expected = original.clone();
            expected.input_details = vec![InputDetail {
                index: 0,
                value: 100_000,
                address: own,
            }];
            // The contact's output is no longer credited as ours.
            expected.output_details[1].role = OutputRole::Sent;
            expected.net_amount = -70_000;
            expected.direction = TransactionDirection::Outgoing;
            let read_blob = |sql: &str| -> Vec<u8> {
                conn.query_row(
                    sql,
                    params![&wallet_id[..], txid.as_byte_array().as_slice()],
                    |r| r.get(0),
                )
                .unwrap()
            };
            assert_eq!(
                read_blob(
                    "SELECT record_blob FROM core_transactions WHERE wallet_id = ?1 AND txid = ?2"
                ),
                blob::encode(&expected).unwrap()
            );
            assert_eq!(
                read_blob(
                    "SELECT record_blob FROM core_transaction_record_originals WHERE wallet_id = ?1 AND txid = ?2"
                ),
                blob::encode(&original).unwrap()
            );
            let (spent, spender): (bool, Option<Vec<u8>>) = conn
                .query_row(
                    "SELECT spent, spent_in_txid FROM core_utxos WHERE wallet_id = ?1 AND outpoint = ?2",
                    params![&wallet_id[..], blob::encode_outpoint(&funding).unwrap()],
                    |r| Ok((r.get(0)?, r.get(1)?)),
                )
                .unwrap();
            assert!(spent);
            assert_eq!(spender.as_deref(), Some(txid.as_byte_array().as_slice()));
            let indexed: i64 = conn
                .query_row(
                    "SELECT count(*) FROM core_transaction_inputs WHERE wallet_id = ?1 AND txid = ?2 AND outpoint = ?3",
                    params![
                        &wallet_id[..],
                        txid.as_byte_array().as_slice(),
                        blob::encode_outpoint(&funding).unwrap()
                    ],
                    |r| r.get(0),
                )
                .unwrap();
            assert_eq!(indexed, 1);
        }
    }
}

#[cfg(test)]
pub(crate) fn rewind_to_v018(conn: &rusqlite::Connection) {
    conn.execute_batch(
        "DROP TABLE core_transaction_inputs; DROP TABLE core_transaction_record_originals; \
         DELETE FROM refinery_schema_history WHERE version >= 19;",
    )
    .unwrap();
}

/// A migrated in-memory database holding one testnet wallet born at 0.
#[cfg(test)]
pub(crate) fn wallet_db(wallet_id: &[u8; 32]) -> rusqlite::Connection {
    let mut conn = rusqlite::Connection::open_in_memory().unwrap();
    run(&mut conn).unwrap();
    insert_test_wallet(&conn, wallet_id, 0);
    conn
}

/// A [`wallet_db`] whose one unspent row has a script over the connection's
/// length cap, so reading the column itself fails: only a length-only
/// pre-read reports it as oversize. Returns the row's outpoint.
#[cfg(test)]
pub(crate) fn oversize_script_db(
    wallet_id: &[u8; 32],
) -> (rusqlite::Connection, dashcore::OutPoint) {
    use dashcore::hashes::Hash;

    use crate::sqlite::conn::SQLITE_MAX_BLOB_BYTES;

    let conn = wallet_db(wallet_id);
    let outpoint = dashcore::OutPoint::new(dashcore::Txid::from_byte_array([0x42; 32]), 0);
    conn.execute(
        "INSERT INTO core_utxos (wallet_id, outpoint, value, script, spent) \
         VALUES (?1, ?2, 0, ?3, 0)",
        rusqlite::params![
            &wallet_id[..],
            crate::sqlite::schema::blob::encode_outpoint(&outpoint).unwrap(),
            vec![0u8; SQLITE_MAX_BLOB_BYTES as usize + 1]
        ],
    )
    .unwrap();
    conn.set_limit(
        rusqlite::limits::Limit::SQLITE_LIMIT_LENGTH,
        SQLITE_MAX_BLOB_BYTES,
    )
    .unwrap();
    (conn, outpoint)
}

#[cfg(test)]
pub(crate) fn insert_test_wallet(conn: &rusqlite::Connection, wallet_id: &[u8; 32], birth: i64) {
    conn.execute(
        "INSERT INTO wallets (wallet_id, network, birth_height) VALUES (?1, 'testnet', ?2)",
        rusqlite::params![&wallet_id[..], birth],
    )
    .unwrap();
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::{params, Connection};

    #[test]
    fn schema_history_guard_rejects_timestamps_refinery_rejects() {
        for applied_on in ["2024-01-01T00:00:00−00:00", "2024-01-01T00:00:60Z"] {
            let conn = Connection::open_in_memory().unwrap();
            conn.execute_batch(
                "CREATE TABLE refinery_schema_history (
                    applied_on TEXT NOT NULL,
                    checksum TEXT NOT NULL
                )",
            )
            .unwrap();
            conn.execute(
                "INSERT INTO refinery_schema_history (applied_on, checksum) VALUES (?1, '0')",
                params![applied_on],
            )
            .unwrap();

            assert!(matches!(
                assert_schema_history_well_formed(&conn),
                Err(WalletStorageError::SchemaHistoryMalformed { .. })
            ));
        }
    }

    /// The helper returns false on a brand-new in-memory DB (no
    /// `refinery_schema_history`), and true after the table is created.
    #[test]
    fn has_schema_history_distinguishes_fresh_vs_migrated() {
        let conn = Connection::open_in_memory().unwrap();
        assert!(
            !has_schema_history(&conn).unwrap(),
            "fresh in-memory DB has no schema-history table"
        );
        conn.execute(
            "CREATE TABLE refinery_schema_history (version INTEGER PRIMARY KEY)",
            [],
        )
        .unwrap();
        assert!(
            has_schema_history(&conn).unwrap(),
            "schema-history table is present after creation"
        );
    }

    fn table_exists(conn: &Connection, name: &str) -> bool {
        conn.query_row(
            "SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?1",
            [name],
            |_| Ok(()),
        )
        .is_ok()
    }

    fn column_exists(conn: &Connection, table: &str, column: &str) -> bool {
        let mut stmt = conn
            .prepare(&format!("PRAGMA table_info({table})"))
            .unwrap();
        let cols: Vec<String> = stmt
            .query_map([], |r| r.get::<_, String>(1))
            .unwrap()
            .filter_map(Result::ok)
            .collect();
        cols.iter().any(|c| c == column)
    }

    /// The initial schema (V001) creates the DashPay sync-correctness
    /// objects directly — the `contacts.payment_channel_broken` column and
    /// the `ignored_senders` table. V001-V007 are published and immutable;
    /// this test pins that these objects survive the full migration set.
    #[test]
    fn v001_creates_dashpay_sync_schema() {
        let mut conn = Connection::open_in_memory().unwrap();
        run(&mut conn).unwrap();

        assert!(
            table_exists(&conn, "ignored_senders"),
            "V001 must create the ignored-senders table"
        );
        assert!(
            column_exists(&conn, "contacts", "payment_channel_broken"),
            "V001 must create the contacts.payment_channel_broken column"
        );
    }
}
