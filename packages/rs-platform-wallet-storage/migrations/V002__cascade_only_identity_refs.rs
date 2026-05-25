//! V002 — cascade-only references on identity-owned tables (CODE-002).
//!
//! V001 keyed every identity-owned table by `(wallet_id, identity_id,
//! ...)` and bolted a direct `FOREIGN KEY (wallet_id) REFERENCES
//! wallet_metadata` onto each. That double-link forced consumers
//! (`identity_sync`) to invent a sentinel `WalletId` when they only
//! held an identity id, breaking FK enforcement and silently dropping
//! token-balance writes.
//!
//! V002 drops the direct `wallet_id` reference from every identity-owned
//! table. The cascade chain is now:
//!
//! ```text
//! wallet_metadata
//!   └─(ON DELETE CASCADE)─ identities  (wallet_id NULLable)
//!         └─(ON DELETE CASCADE)─ identity_keys
//!         └─(ON DELETE CASCADE)─ dashpay_profiles
//!         └─(ON DELETE CASCADE)─ dashpay_payments_overlay
//!         └─(ON DELETE CASCADE)─ token_balances
//! ```
//!
//! `identities.wallet_id` is now NULL-allowed so identity-only flows
//! (no parent wallet, e.g. the identity-sync manager populating rows
//! before any wallet is registered) work without a sentinel.
//!
//! Forward-only. Dev DBs that carry the V001 sentinel rows
//! (`wallet_id = X'00…00'` in `token_balances`) are refused: a
//! temp-table CHECK constraint named `sentinel_count` fails when the
//! row count is non-zero, aborting the migration transaction.
//! `SqlitePersister::open` re-classifies that raw rusqlite error into
//! the typed
//! [`crate::sqlite::error::WalletStorageError::MigrationRequiresManualCleanup`]
//! variant so the operator gets a non-cryptic message. The operator
//! must manually drop the sentinel rows before re-running migrations:
//!
//! ```text
//! DELETE FROM token_balances
//!   WHERE wallet_id = X'0000000000000000000000000000000000000000000000000000000000000000';
//! ```

/// Public entry point invoked by `refinery::embed_migrations!`.
///
/// refinery runs the returned SQL verbatim inside one transaction.
/// The leading guard `SELECT` aborts the transaction with a typed
/// error message when sentinel rows exist; everything after the guard
/// is skipped automatically by SQLite when the abort fires.
pub fn migration() -> String {
    // SQLite has no top-level RAISE() outside trigger bodies, so the
    // sentinel guard rides a CHECK constraint on a temp table:
    // inserting a non-zero count fails the CHECK and aborts the
    // implicit refinery transaction. The CHECK's failure surfaces as
    // a `SQLITE_CONSTRAINT_CHECK` error whose message names the temp
    // table (`_v002_sentinel_rows_must_be_zero`); the persister side
    // re-classifies that into
    // `WalletStorageError::MigrationRequiresManualCleanup`.
    //
    // Operators clear the legacy rows manually before re-running:
    //   DELETE FROM token_balances
    //     WHERE wallet_id = X'0000000000000000000000000000000000000000000000000000000000000000';
    let guard = "\
-- V002 pre-flight: refuse if any legacy sentinel wallet_id rows exist.
CREATE TEMP TABLE _v002_sentinel_rows_must_be_zero (
    sentinel_count INTEGER NOT NULL CHECK (sentinel_count = 0)
);
INSERT INTO _v002_sentinel_rows_must_be_zero (sentinel_count) \
    SELECT COUNT(*) FROM token_balances \
    WHERE wallet_id = X'0000000000000000000000000000000000000000000000000000000000000000';
DROP TABLE _v002_sentinel_rows_must_be_zero;
";
    // The migration runner (`sqlite::migrations::run_for_open`)
    // disables `PRAGMA foreign_keys` before BEGIN so the schema rewrite
    // below does not trigger `ON DELETE CASCADE` on child tables when
    // their parent is dropped. The pragma is re-enabled (with
    // read-back assertion) right after the migration completes.
    let body = "\
-- ----- identities: nullable wallet_id, PK = identity_id -----
CREATE TABLE identities_v2 (
    identity_id BLOB NOT NULL PRIMARY KEY,
    wallet_id BLOB,
    wallet_index INTEGER,
    entry_blob BLOB NOT NULL,
    tombstoned INTEGER NOT NULL,
    FOREIGN KEY (wallet_id) REFERENCES wallet_metadata(wallet_id) ON DELETE CASCADE
);
INSERT INTO identities_v2 (identity_id, wallet_id, wallet_index, entry_blob, tombstoned)
    SELECT identity_id, wallet_id, wallet_index, entry_blob, tombstoned FROM identities;
DROP TABLE identities;
ALTER TABLE identities_v2 RENAME TO identities;
CREATE INDEX idx_identities_wallet ON identities(wallet_id);

-- ----- identity_keys: drop wallet_id, FK identity_id only -----
CREATE TABLE identity_keys_v2 (
    identity_id BLOB NOT NULL,
    key_id INTEGER NOT NULL,
    public_key_blob BLOB NOT NULL,
    public_key_hash BLOB NOT NULL,
    derivation_blob BLOB,
    PRIMARY KEY (identity_id, key_id),
    FOREIGN KEY (identity_id) REFERENCES identities(identity_id) ON DELETE CASCADE
);
INSERT INTO identity_keys_v2 (identity_id, key_id, public_key_blob, public_key_hash, derivation_blob)
    SELECT identity_id, key_id, public_key_blob, public_key_hash, derivation_blob FROM identity_keys;
DROP TABLE identity_keys;
ALTER TABLE identity_keys_v2 RENAME TO identity_keys;
CREATE INDEX idx_identity_keys_identity ON identity_keys(identity_id);

-- ----- dashpay_profiles: drop wallet_id, FK identity_id only -----
CREATE TABLE dashpay_profiles_v2 (
    identity_id BLOB NOT NULL PRIMARY KEY,
    profile_blob BLOB NOT NULL,
    FOREIGN KEY (identity_id) REFERENCES identities(identity_id) ON DELETE CASCADE
);
INSERT INTO dashpay_profiles_v2 (identity_id, profile_blob)
    SELECT identity_id, profile_blob FROM dashpay_profiles;
DROP TABLE dashpay_profiles;
ALTER TABLE dashpay_profiles_v2 RENAME TO dashpay_profiles;

-- ----- dashpay_payments_overlay: drop wallet_id, FK identity_id only -----
CREATE TABLE dashpay_payments_overlay_v2 (
    identity_id BLOB NOT NULL,
    payment_id TEXT NOT NULL,
    overlay_blob BLOB NOT NULL,
    PRIMARY KEY (identity_id, payment_id),
    FOREIGN KEY (identity_id) REFERENCES identities(identity_id) ON DELETE CASCADE
);
INSERT INTO dashpay_payments_overlay_v2 (identity_id, payment_id, overlay_blob)
    SELECT identity_id, payment_id, overlay_blob FROM dashpay_payments_overlay;
DROP TABLE dashpay_payments_overlay;
ALTER TABLE dashpay_payments_overlay_v2 RENAME TO dashpay_payments_overlay;

-- ----- token_balances: drop wallet_id, FK identity_id only -----
CREATE TABLE token_balances_v2 (
    identity_id BLOB NOT NULL,
    token_id BLOB NOT NULL,
    balance INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    PRIMARY KEY (identity_id, token_id),
    FOREIGN KEY (identity_id) REFERENCES identities(identity_id) ON DELETE CASCADE
);
INSERT INTO token_balances_v2 (identity_id, token_id, balance, updated_at)
    SELECT identity_id, token_id, balance, updated_at FROM token_balances;
DROP TABLE token_balances;
ALTER TABLE token_balances_v2 RENAME TO token_balances;
";
    let mut sql = String::with_capacity(guard.len() + body.len());
    sql.push_str(guard);
    sql.push_str(body);
    sql
}
