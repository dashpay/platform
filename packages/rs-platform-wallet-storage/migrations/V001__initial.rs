//! Initial schema for `platform-wallet-storage`.
//!
//! Hand-written `CREATE TABLE … PRIMARY KEY … FOREIGN KEY …` SQL run
//! through refinery. SQLite has no `ALTER TABLE ADD CONSTRAINT`, so the
//! FK clause must live inside `CREATE TABLE`; that requirement is why
//! the schema is emitted as explicit DDL rather than a query-builder.
//!
//! Every per-wallet table carries `wallet_id BLOB` in (or as all of)
//! its primary key plus a native `FOREIGN KEY (wallet_id) REFERENCES
//! wallet_metadata(wallet_id) ON DELETE CASCADE`. `identity_keys` and
//! `dashpay_profiles` additionally key into `identities(wallet_id,
//! identity_id)`. The one relationship that stays a trigger is
//! `core_utxos.spent_in_txid` clearing to NULL on transaction delete —
//! a native composite `ON DELETE SET NULL` would null the NOT-NULL
//! `wallet_id` too (SQLite nulls all FK columns), so the single-column
//! trigger preserves the intended semantics.
//!
//! Foreign-key enforcement is per-connection and is switched on (and
//! read back) at every connection open via `open_conn`
//! (`src/sqlite/conn.rs`).

/// The whole schema as one statement batch. refinery runs the returned
/// string verbatim.
const SCHEMA_SQL: &str = "\
CREATE TABLE wallet_metadata (
    wallet_id BLOB NOT NULL PRIMARY KEY,
    network TEXT NOT NULL,
    birth_height INTEGER NOT NULL
);

CREATE TABLE account_registrations (
    wallet_id BLOB NOT NULL,
    account_type TEXT NOT NULL,
    account_index INTEGER NOT NULL,
    account_xpub_bytes BLOB NOT NULL,
    PRIMARY KEY (wallet_id, account_type, account_index),
    FOREIGN KEY (wallet_id) REFERENCES wallet_metadata(wallet_id) ON DELETE CASCADE
);

CREATE TABLE account_address_pools (
    wallet_id BLOB NOT NULL,
    account_type TEXT NOT NULL,
    account_index INTEGER NOT NULL,
    pool_type TEXT NOT NULL,
    snapshot_blob BLOB NOT NULL,
    PRIMARY KEY (wallet_id, account_type, account_index, pool_type),
    FOREIGN KEY (wallet_id) REFERENCES wallet_metadata(wallet_id) ON DELETE CASCADE
);

CREATE TABLE core_transactions (
    wallet_id BLOB NOT NULL,
    txid BLOB NOT NULL,
    height INTEGER,
    block_hash BLOB,
    block_time INTEGER,
    finalized INTEGER NOT NULL,
    record_blob BLOB NOT NULL,
    PRIMARY KEY (wallet_id, txid),
    FOREIGN KEY (wallet_id) REFERENCES wallet_metadata(wallet_id) ON DELETE CASCADE
);

CREATE INDEX idx_core_transactions_height ON core_transactions(wallet_id, height);

CREATE TABLE core_utxos (
    wallet_id BLOB NOT NULL,
    outpoint BLOB NOT NULL,
    value INTEGER NOT NULL,
    script BLOB NOT NULL,
    height INTEGER,
    account_index INTEGER NOT NULL,
    spent INTEGER NOT NULL,
    spent_in_txid BLOB,
    PRIMARY KEY (wallet_id, outpoint),
    FOREIGN KEY (wallet_id) REFERENCES wallet_metadata(wallet_id) ON DELETE CASCADE
);

CREATE INDEX idx_core_utxos_spent ON core_utxos(wallet_id, spent);

-- `spent_in_txid` clears to NULL when its transaction row is deleted.
-- This can't be a native composite `ON DELETE SET NULL` FK to
-- `core_transactions(wallet_id, txid)`: SQLite nulls EVERY column of a
-- composite FK on SET NULL, and `wallet_id` is NOT NULL, so the delete
-- would fail. The single-column trigger nulls only `spent_in_txid`,
-- matching the lazy semantics the prior schema relied on.
CREATE TRIGGER setnull_core_utxos_on_tx_delete
AFTER DELETE ON core_transactions
FOR EACH ROW
BEGIN
    UPDATE core_utxos SET spent_in_txid = NULL
        WHERE wallet_id = OLD.wallet_id AND spent_in_txid = OLD.txid;
END;

CREATE TABLE core_instant_locks (
    wallet_id BLOB NOT NULL,
    txid BLOB NOT NULL,
    islock_blob BLOB NOT NULL,
    PRIMARY KEY (wallet_id, txid),
    FOREIGN KEY (wallet_id) REFERENCES wallet_metadata(wallet_id) ON DELETE CASCADE
);

CREATE TABLE core_derived_addresses (
    wallet_id BLOB NOT NULL,
    account_type TEXT NOT NULL,
    account_index INTEGER NOT NULL,
    address TEXT NOT NULL,
    derivation_path TEXT NOT NULL,
    used INTEGER NOT NULL,
    PRIMARY KEY (wallet_id, account_type, address),
    FOREIGN KEY (wallet_id) REFERENCES wallet_metadata(wallet_id) ON DELETE CASCADE
);

CREATE INDEX idx_core_derived_addresses_addr ON core_derived_addresses(wallet_id, address);

CREATE TABLE core_sync_state (
    wallet_id BLOB NOT NULL PRIMARY KEY,
    last_processed_height INTEGER,
    synced_height INTEGER,
    FOREIGN KEY (wallet_id) REFERENCES wallet_metadata(wallet_id) ON DELETE CASCADE
);

CREATE TABLE identities (
    wallet_id BLOB NOT NULL,
    wallet_index INTEGER,
    identity_id BLOB NOT NULL,
    entry_blob BLOB NOT NULL,
    tombstoned INTEGER NOT NULL,
    PRIMARY KEY (wallet_id, identity_id),
    FOREIGN KEY (wallet_id) REFERENCES wallet_metadata(wallet_id) ON DELETE CASCADE
);

CREATE TABLE identity_keys (
    wallet_id BLOB NOT NULL,
    identity_id BLOB NOT NULL,
    key_id INTEGER NOT NULL,
    public_key_blob BLOB NOT NULL,
    public_key_hash BLOB NOT NULL,
    derivation_blob BLOB,
    PRIMARY KEY (wallet_id, identity_id, key_id),
    FOREIGN KEY (wallet_id) REFERENCES wallet_metadata(wallet_id) ON DELETE CASCADE,
    FOREIGN KEY (wallet_id, identity_id)
        REFERENCES identities(wallet_id, identity_id) ON DELETE CASCADE
);

CREATE INDEX idx_identity_keys_wallet_identity ON identity_keys(wallet_id, identity_id);

CREATE TABLE contacts_sent (
    wallet_id BLOB NOT NULL,
    owner_id BLOB NOT NULL,
    recipient_id BLOB NOT NULL,
    entry_blob BLOB NOT NULL,
    PRIMARY KEY (wallet_id, owner_id, recipient_id),
    FOREIGN KEY (wallet_id) REFERENCES wallet_metadata(wallet_id) ON DELETE CASCADE
);

CREATE TABLE contacts_recv (
    wallet_id BLOB NOT NULL,
    owner_id BLOB NOT NULL,
    sender_id BLOB NOT NULL,
    entry_blob BLOB NOT NULL,
    PRIMARY KEY (wallet_id, owner_id, sender_id),
    FOREIGN KEY (wallet_id) REFERENCES wallet_metadata(wallet_id) ON DELETE CASCADE
);

CREATE TABLE contacts_established (
    wallet_id BLOB NOT NULL,
    owner_id BLOB NOT NULL,
    contact_id BLOB NOT NULL,
    entry_blob BLOB NOT NULL,
    PRIMARY KEY (wallet_id, owner_id, contact_id),
    FOREIGN KEY (wallet_id) REFERENCES wallet_metadata(wallet_id) ON DELETE CASCADE
);

CREATE TABLE platform_addresses (
    wallet_id BLOB NOT NULL,
    account_index INTEGER NOT NULL,
    address_index INTEGER NOT NULL,
    address BLOB NOT NULL,
    balance INTEGER NOT NULL,
    nonce INTEGER NOT NULL,
    PRIMARY KEY (wallet_id, address),
    FOREIGN KEY (wallet_id) REFERENCES wallet_metadata(wallet_id) ON DELETE CASCADE
);

CREATE TABLE platform_address_sync (
    wallet_id BLOB NOT NULL PRIMARY KEY,
    sync_height INTEGER NOT NULL,
    sync_timestamp INTEGER NOT NULL,
    last_known_recent_block INTEGER NOT NULL,
    FOREIGN KEY (wallet_id) REFERENCES wallet_metadata(wallet_id) ON DELETE CASCADE
);

CREATE TABLE asset_locks (
    wallet_id BLOB NOT NULL,
    outpoint BLOB NOT NULL,
    status TEXT NOT NULL,
    account_index INTEGER NOT NULL,
    identity_index INTEGER NOT NULL,
    amount_duffs INTEGER NOT NULL,
    lifecycle_blob BLOB NOT NULL,
    PRIMARY KEY (wallet_id, outpoint),
    FOREIGN KEY (wallet_id) REFERENCES wallet_metadata(wallet_id) ON DELETE CASCADE
);

CREATE TABLE token_balances (
    wallet_id BLOB NOT NULL,
    identity_id BLOB NOT NULL,
    token_id BLOB NOT NULL,
    balance INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    PRIMARY KEY (wallet_id, identity_id, token_id),
    FOREIGN KEY (wallet_id) REFERENCES wallet_metadata(wallet_id) ON DELETE CASCADE
);

CREATE TABLE dashpay_profiles (
    wallet_id BLOB NOT NULL,
    identity_id BLOB NOT NULL,
    profile_blob BLOB NOT NULL,
    PRIMARY KEY (wallet_id, identity_id),
    FOREIGN KEY (wallet_id) REFERENCES wallet_metadata(wallet_id) ON DELETE CASCADE,
    FOREIGN KEY (wallet_id, identity_id)
        REFERENCES identities(wallet_id, identity_id) ON DELETE CASCADE
);

CREATE TABLE dashpay_payments_overlay (
    wallet_id BLOB NOT NULL,
    identity_id BLOB NOT NULL,
    payment_id TEXT NOT NULL,
    overlay_blob BLOB NOT NULL,
    PRIMARY KEY (wallet_id, identity_id, payment_id),
    FOREIGN KEY (wallet_id) REFERENCES wallet_metadata(wallet_id) ON DELETE CASCADE
);
";

pub fn migration() -> String {
    SCHEMA_SQL.to_string()
}
