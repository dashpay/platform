//! Reshape the V001 base schema for the rehydration workstream (#3968).
//!
//! Everything here was originally written into `V001__initial.rs` in place.
//! That is not available: `v4.2-dev` publishes V001-V006, and refinery
//! validates an applied migration's checksum against the embedded migration of
//! the same version, so editing a published body stops every database that
//! applied it from opening. The reshape therefore APPENDS, and V001-V006 stay
//! byte-identical to what `v4.2-dev` shipped.
//!
//! Seven changes, in dependency order:
//!
//! 1. Stamp the header `application_id`, so a wallet database is
//!    distinguishable from a foreign refinery-versioned SQLite file.
//! 2. `wallet_metadata` -> `wallets`. With `legacy_alter_table` off (the
//!    default since SQLite 3.25) this rewrites the FK clause of every
//!    referencing table and the body and target of
//!    `cascade_meta_on_wallet_delete`, so V003-V006's FK declarations follow
//!    the rename without being edited.
//! 3. `account_registrations` gains the discriminators that keep distinct
//!    accounts off one primary key, and its `account_type` domain widens to
//!    the split standard labels while still admitting the pre-split one.
//! 4. `account_address_pools` and `core_derived_addresses` are dropped;
//!    `core_address_pool` (V008) replaces both.
//! 5. `core_sync_state` gains the applied-ChainLock column.
//! 6. `identities.wallet_index` -> `identity_index`, which is what the column
//!    always meant.
//! 7. `identity_keys` gains its own `wallet_id` scope and a compound FK, so a
//!    key can only be filed under the wallet that owns its identity.
//!
//! Table rebuilds copy into an FK-declaring twin under
//! `PRAGMA foreign_keys = ON`, so each is preceded by an explicit orphan sweep
//! — the same policy, and the same reasoning, as
//! `V004__asset_lock_recovered_status.rs`.

// The trigger pair created below is the PERMISSIVE original: it rejects a
// NULL-scoped key whose identity is wallet-owned, but accepts one naming an
// identity that does not exist at all. `V015` replaces it with the inverted
// condition that closes both cases. Deliberately not written in its final form
// here — V015 carries the fix and its own coverage, and collapsing the two
// would delete that coverage to save one migration.
pub fn migration() -> String {
    // FROZEN, like V001's domains: never interpolate a live `*_LABELS` const
    // into a migration. `account_type_labels_frozen_in_v007` pins the live
    // const to this list, so an added variant fails a test with instructions
    // rather than rewriting this body's checksum.
    //
    // `standard` is the pre-split label `v4.2-dev` wrote for BOTH standard
    // variants. It is ADMITTED rather than rewritten: which variant such a row
    // is lives in `account_xpub_bytes` and is not SQL-reachable, so any rewrite
    // here would be a guess, and guessing wrong turns a row that loads today
    // into a fatal mismatch under the default load policy. The reader carries
    // the equivalence instead (`accounts::db_label_matches_entry`).
    let account_type_check = build_check_in(&[
        "standard",
        "standard_bip44",
        "standard_bip32",
        "coinjoin",
        "identity_registration",
        "identity_topup",
        "identity_topup_unbound",
        "identity_invitation",
        "asset_lock_address_topup",
        "asset_lock_shielded_topup",
        "provider_voting",
        "provider_owner",
        "provider_operator",
        "provider_platform",
        "dashpay_receiving",
        "dashpay_external",
        "platform_payment",
    ]);
    // Splice the constant in decimal — `PRAGMA` takes no bound params.
    let application_id = crate::sqlite::conn::APPLICATION_ID;

    format!(
        "\
PRAGMA application_id = {application_id};

ALTER TABLE wallet_metadata RENAME TO wallets;

-- `account_registrations` rebuild. The widened primary key admits accounts
-- that shared (account_type, account_index) under the old one: the
-- PlatformPayment key_class, and the DashPay (user, friend) identity pair.
-- Sentinel defaults stand in for variants without that axis.
CREATE TABLE account_registrations_new (
    wallet_id BLOB NOT NULL,
    account_type TEXT NOT NULL CHECK (account_type IN {account_type_check}),
    account_index INTEGER NOT NULL,
    key_class INTEGER NOT NULL DEFAULT 0,
    user_identity_id BLOB NOT NULL DEFAULT (zeroblob(32)),
    friend_identity_id BLOB NOT NULL DEFAULT (zeroblob(32)),
    account_xpub_bytes BLOB NOT NULL,
    PRIMARY KEY (wallet_id, account_type, account_index, key_class, user_identity_id, friend_identity_id),
    FOREIGN KEY (wallet_id) REFERENCES wallets(wallet_id) ON DELETE CASCADE
);

-- Orphan policy: a row whose wallet was deleted while FK enforcement happened
-- to be off is unreachable garbage (every read path keys through `wallets`),
-- but copying it into the FK-declared twin aborts this whole migration with
-- 'FOREIGN KEY constraint failed'. Drop such rows explicitly — the same
-- outcome the declared ON DELETE CASCADE would have produced had enforcement
-- been on when the wallet was deleted.
DELETE FROM account_registrations WHERE wallet_id NOT IN (SELECT wallet_id FROM wallets);

-- Labels are copied verbatim, legacy `standard` included. New rows use the
-- split labels; a pre-split row keeps the only label its column ever held,
-- and the blob it is paired with stays the sole authority on which standard
-- variant it is.
INSERT INTO account_registrations_new
    (wallet_id, account_type, account_index, account_xpub_bytes)
SELECT wallet_id, account_type, account_index, account_xpub_bytes
FROM account_registrations;

DROP TABLE account_registrations;
ALTER TABLE account_registrations_new RENAME TO account_registrations;

-- Superseded by `core_address_pool` (V008), which stores per-index rows
-- instead of an opaque pool snapshot and a separate derived-address table.
DROP TABLE account_address_pools;
DROP TABLE core_derived_addresses;

-- Bincode-encoded `dashcore::ephemerealdata::chain_lock::ChainLock`.
-- NULL until the first ChainLock has been applied and flushed.
ALTER TABLE core_sync_state ADD COLUMN last_applied_chain_lock BLOB;

-- An identity's index in its wallet's identity sequence, which is what this
-- column always held; `wallet_index` read as an index OF a wallet.
ALTER TABLE identities RENAME COLUMN wallet_index TO identity_index;

-- Parent key for `identity_keys`' compound FK. SQLite requires the referenced
-- columns to carry a UNIQUE index. Adds no new restriction: `identity_id` is
-- already PRIMARY KEY, so `(wallet_id, identity_id)` is unique for free.
CREATE UNIQUE INDEX idx_identities_wallet_identity ON identities(wallet_id, identity_id);

-- `identity_keys` rebuild: the table gains its own wallet scope, so per-wallet
-- reads stay a direct `WHERE wallet_id = ?`, and a compound FK so a key can
-- only be filed under the wallet that OWNS the identity. The single-column
-- form allowed a key to name an identity parented to a different wallet — a
-- row the per-wallet reader can never resolve, surfacing much later as a fatal
-- OrphanedIdentityEntry.
--
-- `wallet_id` is NULLABLE and deliberately NOT part of the key: NULL is the
-- canonical `owned by no wallet`, matching `identities.wallet_id`. SQLite's
-- default MATCH SIMPLE skips FK enforcement entirely when ANY column of the
-- child key is NULL, so for a NULL-scoped row BOTH FKs below are dormant and
-- neither constrains which identity the key names. The trigger pair after this
-- table replaces that dormancy; the FKs alone are NOT sufficient.
CREATE TABLE identity_keys_new (
    wallet_id BLOB,
    identity_id BLOB NOT NULL,
    key_id INTEGER NOT NULL,
    public_key_blob BLOB NOT NULL,
    public_key_hash BLOB NOT NULL,
    -- Reserved for a future typed projection; always NULL today.
    -- derivation_indices lives inside public_key_blob (the IdentityKeyWire
    -- blob is the single source of truth).
    derivation_blob BLOB,
    PRIMARY KEY (identity_id, key_id),
    FOREIGN KEY (wallet_id) REFERENCES wallets(wallet_id) ON DELETE CASCADE,
    FOREIGN KEY (wallet_id, identity_id)
        REFERENCES identities(wallet_id, identity_id) ON DELETE CASCADE
);

-- Same orphan policy as above: a key naming an identity that no longer exists
-- is unreachable, and would abort the copy against the re-declared FK.
DELETE FROM identity_keys WHERE identity_id NOT IN (SELECT identity_id FROM identities);

-- The scope is denormalised from the identity that owns the key, which is the
-- only value the compound FK will accept.
INSERT INTO identity_keys_new
    (wallet_id, identity_id, key_id, public_key_blob, public_key_hash)
SELECT i.wallet_id, k.identity_id, k.key_id, k.public_key_blob, k.public_key_hash
FROM identity_keys k
JOIN identities i ON i.identity_id = k.identity_id;

DROP TABLE identity_keys;
ALTER TABLE identity_keys_new RENAME TO identity_keys;

CREATE INDEX idx_identity_keys_wallet_identity ON identity_keys(wallet_id, identity_id);

-- The NULL-scope guard the dormant FKs cannot provide: a key filed as unowned
-- must name an identity that is itself unowned. Without this a NULL-scoped key
-- could name a wallet-OWNED identity — the corruption shape the compound FK
-- was added to stop, re-entering through the NULL door.
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

-- Necessary twin, NOT a redundant copy — do not simplify away. The primary key
-- is (identity_id, key_id), so the writer's upsert resolves an existing key to
-- DO UPDATE, and an UPDATE never fires a BEFORE INSERT trigger. Without this
-- one the guard above is bypassed by the ordinary re-save path, which is the
-- path real writes take.
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
"
    )
}

fn build_check_in(labels: &[&str]) -> String {
    let quoted = labels
        .iter()
        .map(|l| format!("'{}'", l))
        .collect::<Vec<_>>()
        .join(", ");
    format!("({})", quoted)
}
