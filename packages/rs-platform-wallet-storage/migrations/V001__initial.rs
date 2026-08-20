//! Initial schema for `platform-wallet-storage`.
//!
//! Hand-written `CREATE TABLE … PRIMARY KEY … FOREIGN KEY …` SQL run
//! through refinery. SQLite has no `ALTER TABLE ADD CONSTRAINT`, so the
//! FK clause must live inside `CREATE TABLE`; that requirement is why
//! the schema is emitted as explicit DDL rather than a query-builder.
//!
//! Per-wallet tables carry `wallet_id BLOB` in (or as all of) their
//! primary key plus a native `FOREIGN KEY (wallet_id) REFERENCES
//! wallets(wallet_id) ON DELETE CASCADE`. Identity-owned
//! tables (`dashpay_profiles`, `dashpay_payments_overlay`,
//! `token_balances`) are keyed by `identity_id` only; their FK targets
//! `identities(identity_id)` so cascade flows `wallets →
//! identities → child` through the nullable `identities.wallet_id`
//! link. `identity_keys` additionally carries its own `wallet_id`
//! column (so per-wallet reads stay a direct `WHERE wallet_id = ?`)
//! and its FK is compound — `(wallet_id, identity_id)` — so a key can
//! only be filed under the wallet that owns the identity.
//! `identities.wallet_id` is NULL-allowed so identity-only flows (no
//! parent wallet, e.g. the identity-sync manager populating rows
//! before any wallet is registered) work without a placeholder.
//! A NULL-owned identity CAN hold keys: `identity_keys.wallet_id` is
//! nullable too, and NULL on both sides spells the same unowned scope.
//! Such a key is guarded by a trigger pair rather than by the compound
//! FK, which MATCH SIMPLE leaves dormant once a child key column is
//! NULL. Note this is distinct from an identity observed rather than
//! owned, which stays wallet-scoped — `identity_index` NULL with
//! `wallet_id` set to the discovering wallet.
//!
//! The one relationship that stays a trigger is
//! `core_utxos.spent_in_txid` clearing to NULL on transaction delete —
//! a native composite `ON DELETE SET NULL` would null the NOT-NULL
//! `wallet_id` too (SQLite nulls all FK columns), so the single-column
//! trigger preserves the intended semantics.
//!
//! Foreign-key enforcement is per-connection and is switched on (and
//! read back) at every connection open via `open_conn`
//! (`src/sqlite/conn.rs`).
//!
//! Enum-shaped TEXT columns (`network`, `account_type`, `status`,
//! `state`) carry a `CHECK (col IN (...))` clause whose
//! IN-list is built from the `*_LABELS` const arrays in
//! `crate::sqlite::schema::{wallets, accounts, asset_locks,
//! contacts}`. The consts are the single source of truth shared with
//! the writer mapping functions; the per-module `*_labels_match_enum`
//! unit tests enforce set-equality between each const and its writer's
//! codomain.

fn build_check_in(labels: &[&str]) -> String {
    let quoted = labels
        .iter()
        .map(|l| format!("'{}'", l))
        .collect::<Vec<_>>()
        .join(", ");
    format!("({})", quoted)
}

pub fn migration() -> String {
    let network_check = build_check_in(crate::sqlite::schema::wallets::NETWORK_LABELS);
    let account_type_check =
        build_check_in(crate::sqlite::schema::accounts::ACCOUNT_TYPE_LABELS);
    // FROZEN as of V010: the asset-lock status domain must no longer be
    // interpolated from the live `ASSET_LOCK_STATUS_LABELS` const — a
    // later variant addition would silently rewrite this migration's
    // generated SQL and break its Refinery checksum on every database
    // that already applied it (`abort_divergent` default). New status
    // labels are introduced by APPENDING a migration that rebuilds the
    // table with the widened CHECK (see
    // `V010__asset_lock_recovered_status.rs`); this list stays
    // byte-identical to what V001 shipped with.
    let asset_lock_status_check = build_check_in(&[
        "built",
        "broadcast",
        "is_locked",
        "chain_locked",
        "consumed",
    ]);
    let contact_state_check =
        build_check_in(crate::sqlite::schema::contacts::CONTACT_STATE_LABELS);
    let pending_contact_crypto_kind_check =
        build_check_in(crate::sqlite::schema::pending_contact_crypto::KIND_LABELS);

    // Stamp the header `application_id` so a foreign refinery-versioned
    // SQLite DB can be told apart from a wallet-storage DB (asserted in
    // `open()` pre-migration and in `restore_from`'s staged validation).
    // Splice the constant in decimal — `PRAGMA` takes no bound params.
    let application_id = crate::sqlite::conn::APPLICATION_ID;

    format!(
        "\
PRAGMA application_id = {application_id};

CREATE TABLE wallets (
    wallet_id BLOB NOT NULL PRIMARY KEY,
    network TEXT NOT NULL CHECK (network IN {network_check}),
    birth_height INTEGER NOT NULL
);

CREATE TABLE account_registrations (
    wallet_id BLOB NOT NULL,
    account_type TEXT NOT NULL CHECK (account_type IN {account_type_check}),
    account_index INTEGER NOT NULL,
    -- Discriminators sharing (account_type, account_index) across distinct
    -- accounts: PlatformPayment key_class and the DashPay (user, friend)
    -- identity pair. Sentinel default for variants without that axis.
    key_class INTEGER NOT NULL DEFAULT 0,
    user_identity_id BLOB NOT NULL DEFAULT (zeroblob(32)),
    friend_identity_id BLOB NOT NULL DEFAULT (zeroblob(32)),
    account_xpub_bytes BLOB NOT NULL,
    PRIMARY KEY (wallet_id, account_type, account_index, key_class, user_identity_id, friend_identity_id),
    FOREIGN KEY (wallet_id) REFERENCES wallets(wallet_id) ON DELETE CASCADE
);

CREATE TABLE pending_contact_crypto (
    wallet_id BLOB NOT NULL,
    owner_identity_id BLOB NOT NULL,
    contact_id BLOB NOT NULL,
    kind TEXT NOT NULL CHECK (kind IN {pending_contact_crypto_kind_check}),
    payload BLOB NOT NULL,
    enqueued_at_ms INTEGER NOT NULL,
    PRIMARY KEY (wallet_id, owner_identity_id, contact_id, kind),
    FOREIGN KEY (wallet_id) REFERENCES wallets(wallet_id) ON DELETE CASCADE
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
    FOREIGN KEY (wallet_id) REFERENCES wallets(wallet_id) ON DELETE CASCADE
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
    FOREIGN KEY (wallet_id) REFERENCES wallets(wallet_id) ON DELETE CASCADE
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
    FOREIGN KEY (wallet_id) REFERENCES wallets(wallet_id) ON DELETE CASCADE
);

CREATE TABLE core_sync_state (
    wallet_id BLOB NOT NULL PRIMARY KEY,
    last_processed_height INTEGER,
    synced_height INTEGER,
    -- Bincode-encoded `dashcore::ephemerealdata::chain_lock::ChainLock`.
    -- NULL until the first ChainLock has been applied and flushed.
    last_applied_chain_lock BLOB,
    FOREIGN KEY (wallet_id) REFERENCES wallets(wallet_id) ON DELETE CASCADE
);

CREATE TABLE identities (
    identity_id BLOB NOT NULL PRIMARY KEY,
    wallet_id BLOB,
    identity_index INTEGER,
    entry_blob BLOB NOT NULL,
    tombstoned INTEGER NOT NULL,
    FOREIGN KEY (wallet_id) REFERENCES wallets(wallet_id) ON DELETE CASCADE
);

CREATE INDEX idx_identities_wallet ON identities(wallet_id);

-- Parent key for `identity_keys`' compound FK. SQLite requires the
-- referenced columns to carry a UNIQUE index. Adds no new restriction:
-- `identity_id` is already PRIMARY KEY, so `(wallet_id, identity_id)`
-- is unique for free.
CREATE UNIQUE INDEX idx_identities_wallet_identity ON identities(wallet_id, identity_id);

CREATE TABLE identity_keys (
    -- NULLABLE: NULL is the canonical `owned by no wallet`, matching
    -- `identities.wallet_id`. Read the two FKs below with that in mind —
    -- SQLite's default MATCH SIMPLE skips foreign-key enforcement
    -- entirely when ANY column of the child key is NULL, so for a
    -- NULL-scoped row BOTH FKs are dormant and neither constrains which
    -- identity the key names. The two triggers after this table exist
    -- precisely to replace that dormancy; the FKs alone are NOT
    -- sufficient, and a NULL-scoped row is only as safe as the triggers.
    wallet_id BLOB,
    identity_id BLOB NOT NULL,
    key_id INTEGER NOT NULL,
    public_key_blob BLOB NOT NULL,
    public_key_hash BLOB NOT NULL,
    -- Reserved for a future typed projection; always NULL today.
    -- derivation_indices lives inside public_key_blob (the
    -- IdentityKeyWire blob is the single source of truth).
    derivation_blob BLOB,
    -- `wallet_id` is deliberately NOT part of the key. `identities`
    -- keys on `identity_id` ALONE, so an identity has exactly one row
    -- and exactly one owning wallet — the scope column here carries no
    -- discriminating power, it is a denormalised copy of
    -- `identities.wallet_id`. The wider `(wallet_id, identity_id,
    -- key_id)` key was the enabling condition for the duplicate-row
    -- corruption: it let the same key exist twice under two scopes, a
    -- state the domain (`IdentityKeysChangeSet`, keyed
    -- `(identity_id, key_id)`) cannot express. Narrowing the key makes
    -- that row pair unrepresentable rather than merely rejected.
    PRIMARY KEY (identity_id, key_id),
    -- Belt-and-braces: the compound FK below already implies a live
    -- `wallets` row (the matched `identities` row carries this same
    -- non-NULL wallet_id and is itself FK'd to `wallets`). Kept as an
    -- explicit statement of intent and a second cascade path.
    FOREIGN KEY (wallet_id) REFERENCES wallets(wallet_id) ON DELETE CASCADE,
    -- Compound: a key may only be filed under the wallet that OWNS the
    -- identity. The single-column form allowed a key to name an identity
    -- parented to a different wallet — a row the per-wallet reader can
    -- never resolve, surfacing much later as a fatal OrphanedIdentityEntry.
    FOREIGN KEY (wallet_id, identity_id)
        REFERENCES identities(wallet_id, identity_id) ON DELETE CASCADE
);

CREATE INDEX idx_identity_keys_wallet_identity ON identity_keys(wallet_id, identity_id);

-- The NULL-scope guard the dormant FKs cannot provide: a key filed as
-- unowned must name an identity that is itself unowned. Without this a
-- NULL-scoped key could name a wallet-OWNED identity — a row no
-- per-wallet reader can resolve, which is the corruption shape the
-- compound FK was added to stop, re-entering through the NULL door.
--
-- The converse (a wallet-scoped key naming an unowned identity) needs no
-- trigger: that child key is fully non-NULL, so the compound FK is live
-- and rejects it.
CREATE TRIGGER identity_keys_null_scope_requires_unowned_identity
BEFORE INSERT ON identity_keys
FOR EACH ROW WHEN NEW.wallet_id IS NULL
BEGIN
    SELECT RAISE(ABORT, 'identity_keys.wallet_id is NULL but the identity is wallet-owned')
    WHERE EXISTS (
        SELECT 1 FROM identities i
        WHERE i.identity_id = NEW.identity_id AND i.wallet_id IS NOT NULL
    );
END;

-- Necessary twin, NOT a redundant copy — do not simplify away. The primary
-- key is (identity_id, key_id), so the writer's upsert resolves an
-- existing key to DO UPDATE, and an UPDATE never fires a BEFORE INSERT
-- trigger. Without this one the guard above is bypassed by the ordinary
-- re-save path, which is the path real writes take.
CREATE TRIGGER identity_keys_null_scope_requires_unowned_identity_on_update
BEFORE UPDATE ON identity_keys
FOR EACH ROW WHEN NEW.wallet_id IS NULL
BEGIN
    SELECT RAISE(ABORT, 'identity_keys.wallet_id is NULL but the identity is wallet-owned')
    WHERE EXISTS (
        SELECT 1 FROM identities i
        WHERE i.identity_id = NEW.identity_id AND i.wallet_id IS NOT NULL
    );
END;

CREATE TABLE contacts (
    wallet_id BLOB NOT NULL,
    owner_id BLOB NOT NULL,
    contact_id BLOB NOT NULL,
    state TEXT NOT NULL CHECK (state IN {contact_state_check}),
    outgoing_request BLOB,
    incoming_request BLOB,
    alias TEXT,
    note TEXT,
    is_hidden INTEGER,
    accepted_accounts BLOB,
    -- G1c: set when external-account registration permanently fails for a
    -- contact (so the sync sweep stops retrying a poisoned channel);
    -- cleared on a superseding rotation. Nullable — readers treat NULL as
    -- `false`.
    payment_channel_broken INTEGER,
    updated_at INTEGER NOT NULL DEFAULT (unixepoch()),
    PRIMARY KEY (wallet_id, owner_id, contact_id),
    FOREIGN KEY (wallet_id) REFERENCES wallets(wallet_id) ON DELETE CASCADE
);

-- Ignored senders (per-sender mute = block, reversible — local-only). Keyed by
-- bare `(wallet_id, owner_id, sender_id)`: ignoring is per-sender, NOT
-- per-request, so it suppresses ALL of a sender's incoming contactRequests
-- (including rotated, bumped-`accountReference` ones) and survives a recurring
-- re-sync. Un-ignore deletes the row so the sender's requests resurface. The
-- sync ingest path consults this table before surfacing a received
-- contactRequest in the main pending list.
CREATE TABLE ignored_senders (
    wallet_id BLOB NOT NULL,
    owner_id BLOB NOT NULL,
    sender_id BLOB NOT NULL,
    ignored_at INTEGER NOT NULL DEFAULT (unixepoch()),
    PRIMARY KEY (wallet_id, owner_id, sender_id),
    FOREIGN KEY (wallet_id) REFERENCES wallets(wallet_id) ON DELETE CASCADE
);

CREATE TABLE platform_addresses (
    wallet_id BLOB NOT NULL,
    account_index INTEGER NOT NULL,
    address_index INTEGER NOT NULL,
    address BLOB NOT NULL,
    balance INTEGER NOT NULL,
    nonce INTEGER NOT NULL,
    PRIMARY KEY (wallet_id, address),
    FOREIGN KEY (wallet_id) REFERENCES wallets(wallet_id) ON DELETE CASCADE
);

CREATE TABLE platform_address_sync (
    wallet_id BLOB NOT NULL PRIMARY KEY,
    sync_height INTEGER NOT NULL,
    sync_timestamp INTEGER NOT NULL,
    last_known_recent_block INTEGER NOT NULL,
    FOREIGN KEY (wallet_id) REFERENCES wallets(wallet_id) ON DELETE CASCADE
);

CREATE TABLE asset_locks (
    wallet_id BLOB NOT NULL,
    outpoint BLOB NOT NULL,
    status TEXT NOT NULL CHECK (status IN {asset_lock_status_check}),
    account_index INTEGER NOT NULL,
    identity_index INTEGER NOT NULL,
    amount_duffs INTEGER NOT NULL,
    lifecycle_blob BLOB NOT NULL,
    PRIMARY KEY (wallet_id, outpoint),
    FOREIGN KEY (wallet_id) REFERENCES wallets(wallet_id) ON DELETE CASCADE
);

CREATE TABLE token_balances (
    identity_id BLOB NOT NULL,
    token_id BLOB NOT NULL,
    balance INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    PRIMARY KEY (identity_id, token_id),
    FOREIGN KEY (identity_id) REFERENCES identities(identity_id) ON DELETE CASCADE
);

CREATE TABLE dashpay_profiles (
    identity_id BLOB NOT NULL PRIMARY KEY,
    profile_blob BLOB NOT NULL,
    FOREIGN KEY (identity_id) REFERENCES identities(identity_id) ON DELETE CASCADE
);

CREATE TABLE dashpay_payments_overlay (
    identity_id BLOB NOT NULL,
    payment_id TEXT NOT NULL,
    overlay_blob BLOB NOT NULL,
    PRIMARY KEY (identity_id, payment_id),
    FOREIGN KEY (identity_id) REFERENCES identities(identity_id) ON DELETE CASCADE
);

-- Per-object-type key/value metadata for app-managed data (aliases,
-- flags, notes, sync hints, ordering — anything the host wants to stash
-- alongside a wallet object). One dedicated table per `ObjectId`
-- variant; see `src/kv.rs` and `SCHEMA.md` for the public API. Every
-- table shares the same value contract — `key` (1..=128 chars), opaque
-- `value` BLOB, `updated_at` defaulting to `unixepoch()` — plus a
-- composite PRIMARY KEY of its id column(s) and `key`.
--
-- Unlike every other per-wallet table (hard FOREIGN KEY ON DELETE
-- CASCADE, so the parent must exist at write time), the five scoped
-- meta_* tables carry NO FK: host apps attach metadata to an object
-- before/independently of that object being synced into its typed table
-- (async sync ordering; a global-config persister whose parent tables
-- stay empty). Cleanup is the AFTER DELETE triggers below, which SQLite
-- fires even for parent rows removed by an FK ON DELETE CASCADE — so
-- deleting a wallet transitively cleans every meta_* row carrying that
-- wallet_id (directly or via its identities), including parentless rows
-- whose typed parent object never existed. meta_global is the lone
-- exception: it has no wallet scope and survives wallet deletion.
CREATE TABLE meta_global (
    key        TEXT NOT NULL PRIMARY KEY CHECK (length(key) BETWEEN 1 AND 128),
    value      BLOB NOT NULL,
    updated_at INTEGER NOT NULL DEFAULT (unixepoch())
);

CREATE TABLE meta_wallet (
    wallet_id  BLOB NOT NULL,
    key        TEXT NOT NULL CHECK (length(key) BETWEEN 1 AND 128),
    value      BLOB NOT NULL,
    updated_at INTEGER NOT NULL DEFAULT (unixepoch()),
    PRIMARY KEY (wallet_id, key)
);

CREATE TABLE meta_identity (
    identity_id BLOB NOT NULL,
    key         TEXT NOT NULL CHECK (length(key) BETWEEN 1 AND 128),
    value       BLOB NOT NULL,
    updated_at  INTEGER NOT NULL DEFAULT (unixepoch()),
    PRIMARY KEY (identity_id, key)
);

CREATE TABLE meta_token (
    identity_id BLOB NOT NULL,
    token_id    BLOB NOT NULL,
    key         TEXT NOT NULL CHECK (length(key) BETWEEN 1 AND 128),
    value       BLOB NOT NULL,
    updated_at  INTEGER NOT NULL DEFAULT (unixepoch()),
    PRIMARY KEY (identity_id, token_id, key)
);

CREATE TABLE meta_contact (
    wallet_id  BLOB NOT NULL,
    owner_id   BLOB NOT NULL,
    contact_id BLOB NOT NULL,
    key        TEXT NOT NULL CHECK (length(key) BETWEEN 1 AND 128),
    value      BLOB NOT NULL,
    updated_at INTEGER NOT NULL DEFAULT (unixepoch()),
    PRIMARY KEY (wallet_id, owner_id, contact_id, key)
);

CREATE TABLE meta_platform_address (
    wallet_id  BLOB NOT NULL,
    address    BLOB NOT NULL,
    key        TEXT NOT NULL CHECK (length(key) BETWEEN 1 AND 128),
    value      BLOB NOT NULL,
    updated_at INTEGER NOT NULL DEFAULT (unixepoch()),
    PRIMARY KEY (wallet_id, address, key)
);

-- Soft-cascade cleanup: drop a scope's metadata when its parent object
-- is deleted. SQLite fires these for parents removed by an FK cascade
-- too (e.g. wallets delete → identities cascade → identity
-- trigger), so deleting a wallet cleans its metadata transitively.
--
-- Two root brooms key on the deleted parent's id alone so they reach
-- parentless meta_* rows (metadata written before the typed parent ever
-- existed) just as well as parented ones. The remaining two triggers
-- fire on direct typed-row deletes (a contact or address removed without
-- deleting the wallet) and are idempotent overlaps with the root brooms
-- on the wallet-delete path.

-- Root broom 1: deleting a wallet removes every wallet_id-scoped meta
-- row, parentless included. Keys on wallet_id only, so contact state and
-- whether the typed parent ever existed are both irrelevant.
CREATE TRIGGER cascade_meta_on_wallet_delete
AFTER DELETE ON wallets
FOR EACH ROW
BEGIN
    DELETE FROM meta_wallet           WHERE wallet_id = OLD.wallet_id;
    DELETE FROM meta_contact          WHERE wallet_id = OLD.wallet_id;
    DELETE FROM meta_platform_address WHERE wallet_id = OLD.wallet_id;
END;

-- Root broom 2: the wallet→identities FK cascade fires this per removed
-- identity, brooming its identity-scoped meta even when no token_balances
-- row ever existed (parentless meta_token).
CREATE TRIGGER cascade_meta_on_identity_delete
AFTER DELETE ON identities
FOR EACH ROW
BEGIN
    DELETE FROM meta_identity WHERE identity_id = OLD.identity_id;
    DELETE FROM meta_token    WHERE identity_id = OLD.identity_id;
END;

-- Direct token_balances delete: still wanted when a balance row is
-- removed without deleting its identity. Redundant on the wallet-delete
-- path (root broom 2 already covers it); the DELETE is idempotent.
CREATE TRIGGER cascade_meta_token_on_token_balance_delete
AFTER DELETE ON token_balances
FOR EACH ROW
BEGIN
    DELETE FROM meta_token
        WHERE identity_id = OLD.identity_id AND token_id = OLD.token_id;
END;

-- Direct contacts delete: removing one contact relationship drops its
-- metadata regardless of lifecycle state. Redundant on the wallet-delete
-- path (root broom 1 already covers it); the DELETE is idempotent.
CREATE TRIGGER cascade_meta_contact_on_contact_delete
AFTER DELETE ON contacts
FOR EACH ROW
BEGIN
    DELETE FROM meta_contact
        WHERE wallet_id = OLD.wallet_id
          AND owner_id = OLD.owner_id
          AND contact_id = OLD.contact_id;
END;

-- Direct platform_addresses delete: removing one address drops its
-- metadata. Redundant on the wallet-delete path (root broom 1 already
-- covers it); the DELETE is idempotent.
CREATE TRIGGER cascade_meta_platform_address_on_address_delete
AFTER DELETE ON platform_addresses
FOR EACH ROW
BEGIN
    DELETE FROM meta_platform_address
        WHERE wallet_id = OLD.wallet_id AND address = OLD.address;
END;
"
    )
}
