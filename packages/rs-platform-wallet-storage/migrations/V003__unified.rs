//! Unified additive migration for `platform-wallet-storage` (#3968).
//!
//! The entire migration set remains editable in place until this crate's first
//! release. Numbered V003, not V002: PR #4019 (ADDR-09,
//! `V002__address_height_pin.rs`) independently claimed version 2 — two
//! migrations cannot share a version number (refinery's
//! `refinery_schema_history` collides on it), so this one sequences after.
//! V003 lifts `max_supported_version()`
//! from 2 to 3 automatically (the value is derived from the embedded list)
//! and lands three concerns in one migration event:
//!
//! - `core_address_pool` — per-index address-pool rows with a `used` flag,
//!   the first-class row store that replaces `core_utxos` script-derivation
//!   for the address-reuse guard. `account_type` and `pool_type` are both in
//!   the primary key: `account_type` so two accounts that collapse to the same
//!   `(account_index, key_class)` sentinel (e.g. `IdentityRegistration` and
//!   `ProviderVotingKeys`, both `0, 0`) never overwrite each other, and
//!   `pool_type` so an External (receive) and Internal (change) pool never
//!   collide at the same `address_index`. The PK also carries the DashPay
//!   `(user_identity_id, friend_identity_id)` pair, mirroring
//!   `account_registrations` (V001): `DashpayReceivingFunds` accounts all
//!   collapse to `(account_type='dashpay_receiving', account_index=0)`, so
//!   without the identity pair two contacts on one wallet would upsert onto
//!   the same PK and silently overwrite each other's pool rows. `script` (the
//!   address' `script_pubkey`) is stored so the reader returns used addresses
//!   verbatim and the UTXO writer can attribute an outpoint to its owning
//!   account, both without re-deriving.
//! - `meta_data_versions` — per-`(wallet_id, domain)` monotonic `seq`
//!   bumped inside the flush transaction, the cache-invalidation keystone.
//!   No FK (a domain row may be written before its typed parent syncs,
//!   mirroring the `meta_*` tables); a soft-cascade trigger reaps rows on
//!   wallet delete.
//! - `meta_store_generation` — a single-row store-generation token,
//!   initialized with `randomblob(16)` so the rendered SQL stays deterministic (the
//!   content fingerprint pins the text, the runtime value is unique per
//!   store). Regenerated on restore.
//!
//! No MAC column ships here — manifest authentication is deferred out of
//! this workstream (dev-plan §7).

pub fn migration() -> String {
    "\
CREATE TABLE core_address_pool (
    wallet_id BLOB NOT NULL,
    account_type TEXT NOT NULL,
    account_index INTEGER NOT NULL,
    key_class INTEGER NOT NULL DEFAULT 0,
    user_identity_id BLOB NOT NULL DEFAULT (zeroblob(32)),
    friend_identity_id BLOB NOT NULL DEFAULT (zeroblob(32)),
    pool_type INTEGER NOT NULL CHECK (pool_type IN (0, 1, 2, 3)),
    address_index INTEGER NOT NULL,
    script BLOB NOT NULL,
    used INTEGER NOT NULL DEFAULT 0 CHECK (used IN (0, 1)),
    PRIMARY KEY (wallet_id, account_type, account_index, key_class, user_identity_id, friend_identity_id, pool_type, address_index),
    FOREIGN KEY (wallet_id) REFERENCES wallets(wallet_id) ON DELETE CASCADE
);

CREATE INDEX idx_core_address_pool_used
    ON core_address_pool(wallet_id, used);

-- The UTXO writer attributes an outpoint to its owning account by matching
-- the outpoint's script against a pool row.
CREATE INDEX idx_core_address_pool_script
    ON core_address_pool(wallet_id, script);

CREATE TABLE meta_data_versions (
    wallet_id BLOB NOT NULL,
    domain TEXT NOT NULL,
    seq INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (wallet_id, domain)
);

-- Soft-cascade reap, matching the meta_* tables: no FK (a domain may be
-- bumped before its typed parent exists), so a trigger clears rows when
-- the owning wallet is deleted.
CREATE TRIGGER cascade_meta_data_versions_on_wallet_delete
AFTER DELETE ON wallets
FOR EACH ROW
BEGIN
    DELETE FROM meta_data_versions WHERE wallet_id = OLD.wallet_id;
END;

CREATE TABLE meta_store_generation (
    id INTEGER NOT NULL PRIMARY KEY CHECK (id = 0),
    generation BLOB NOT NULL
);

INSERT INTO meta_store_generation (id, generation) VALUES (0, randomblob(16));
"
    .to_string()
}
