//! Per-area SQLite writers + readers.
//!
//! Each submodule owns one table or a small cluster (e.g. `contacts`
//! owns three). Writers take a `&rusqlite::Transaction` and an already
//! resolved sub-changeset; readers take `&rusqlite::Connection`.
//!
//! Encoding policy: scalars that fan out to per-row indexes go into
//! typed SQLite columns (heights, hashes, outpoints, flags). The
//! `_blob` columns carry the full sub-changeset entry encoded with
//! `bincode::serde::encode_to_vec` against the serde-derived types in
//! `platform-wallet` — see [`blob::encode`] / [`blob::decode`].
//! Schema evolution is gated by the refinery migration version on
//! the database; individual blobs have no inline revision tag.

pub mod accounts;
pub mod asset_locks;
pub mod blob;
pub mod contacts;
pub mod core_state;
pub mod dashpay;
pub mod identities;
pub mod identity_keys;
pub mod platform_addrs;
pub mod token_balances;
pub mod wallet_meta;

/// How a per-wallet table is row-scoped against a `wallet_id`. After
/// the V002 schema migration (CODE-002), identity-owned tables drop
/// their direct `wallet_id` column and reach the parent wallet only
/// via the cascading FK chain `wallet_metadata → identities → …`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WalletScope {
    /// The table carries a `wallet_id` column directly; predicates
    /// like `WHERE wallet_id = ?` work as-is.
    DirectColumn,
    /// The table is keyed by `identity_id`; lookups by wallet must
    /// JOIN through `identities` (`SELECT … WHERE identity_id IN
    /// (SELECT identity_id FROM identities WHERE wallet_id = ?)`).
    ViaIdentity,
}

/// Every per-wallet table — used by `delete_wallet` to count + cascade
/// row removal and by `inspect` for the table summary. `wallet_metadata`
/// is the parent and listed first; everything after it depends on the
/// parent row via the native `ON DELETE CASCADE` foreign keys declared
/// in `V001__initial.rs` (wallet-scoped tables) and
/// `V002__cascade_only_identity_refs.rs` (identity-scoped tables).
pub const PER_WALLET_TABLES: &[(&str, WalletScope)] = &[
    ("wallet_metadata", WalletScope::DirectColumn),
    ("account_registrations", WalletScope::DirectColumn),
    ("account_address_pools", WalletScope::DirectColumn),
    ("core_transactions", WalletScope::DirectColumn),
    ("core_utxos", WalletScope::DirectColumn),
    ("core_instant_locks", WalletScope::DirectColumn),
    ("core_derived_addresses", WalletScope::DirectColumn),
    ("core_sync_state", WalletScope::DirectColumn),
    ("identities", WalletScope::DirectColumn),
    ("identity_keys", WalletScope::ViaIdentity),
    ("contacts_sent", WalletScope::DirectColumn),
    ("contacts_recv", WalletScope::DirectColumn),
    ("contacts_established", WalletScope::DirectColumn),
    ("platform_addresses", WalletScope::DirectColumn),
    ("platform_address_sync", WalletScope::DirectColumn),
    ("asset_locks", WalletScope::DirectColumn),
    ("token_balances", WalletScope::ViaIdentity),
    ("dashpay_profiles", WalletScope::ViaIdentity),
    ("dashpay_payments_overlay", WalletScope::ViaIdentity),
];

/// SQL fragment for counting rows of `table` belonging to a single
/// wallet. `scope` selects the predicate flavour. The fragment includes
/// the leading `SELECT COUNT(*) FROM` so the call site can format it
/// directly and bind a single `?1` parameter (the wallet id bytes).
pub fn count_rows_for_wallet_sql(table: &str, scope: WalletScope) -> String {
    match scope {
        WalletScope::DirectColumn => {
            format!("SELECT COUNT(*) FROM {table} WHERE wallet_id = ?1")
        }
        WalletScope::ViaIdentity => format!(
            "SELECT COUNT(*) FROM {table} \
             WHERE identity_id IN (SELECT identity_id FROM identities WHERE wallet_id = ?1)"
        ),
    }
}
