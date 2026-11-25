//! Add `core_address_pool_retired`: a rotated DashPay contact's old pool.
//!
//! A contact who rotates their payment xpub gets a new `DashpayExternalAccount`
//! under the same account key. The registration upsert replaces the account's
//! xpub, but every pool row of the old xpub keeps its
//! `(account, pool_type, address_index)` primary key with the old typed public
//! key, so the new pool's index 0 was refused as a typed-key conflict and the
//! whole registration rolled back on every retry: the contact stayed
//! unpayable.
//!
//! This table keeps the rows an account derived from an xpub it no longer has,
//! keyed by that xpub (so a second rotation never collides with the first).
//! They stay known for history and attribution; nothing that builds a payable
//! pool reads them. Rows move here inside the flush that replaces the
//! account's xpub (see `accounts::apply_registrations`), and a flush stores a
//! pool row of an earlier xpub here rather than in the live pool (see
//! `core_pool::apply_flush_pools`).
//!
//! Idempotent (`IF NOT EXISTS`): a wallet file written by a downstream build
//! that shipped this exact table and index before this migration existed
//! adopts them in place, rows included. The column list is that build's,
//! byte for byte, so an adopted file and a fresh one have the same shape.
//!
//! Purely additive: an upgraded database starts with no retired rows.

pub fn migration() -> String {
    "\
CREATE TABLE IF NOT EXISTS core_address_pool_retired (
    wallet_id BLOB NOT NULL,
    account_type TEXT NOT NULL,
    account_index INTEGER NOT NULL,
    key_class INTEGER NOT NULL DEFAULT 0,
    user_identity_id BLOB NOT NULL DEFAULT (zeroblob(32)),
    friend_identity_id BLOB NOT NULL DEFAULT (zeroblob(32)),
    -- The BIP32 serialization of the xpub these rows were derived from.
    account_xpub BLOB NOT NULL,
    pool_type INTEGER NOT NULL CHECK (pool_type IN (0, 1, 2, 3)),
    address_index INTEGER NOT NULL,
    script BLOB NOT NULL,
    used INTEGER NOT NULL DEFAULT 0 CHECK (used IN (0, 1)),
    public_key BLOB NULL,
    key_type INTEGER NULL CHECK (key_type IS NULL OR key_type IN (0, 1, 2)),
    retired_at INTEGER NOT NULL DEFAULT (unixepoch()),
    PRIMARY KEY (wallet_id, account_type, account_index, key_class, user_identity_id,
                 friend_identity_id, account_xpub, pool_type, address_index),
    FOREIGN KEY (wallet_id) REFERENCES wallets(wallet_id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_core_address_pool_retired_script
    ON core_address_pool_retired(wallet_id, script);
"
    .to_string()
}
