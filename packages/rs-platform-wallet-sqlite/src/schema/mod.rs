//! Per-area SQLite writers + readers.
//!
//! Each submodule owns one table or a small cluster (e.g. `contacts`
//! owns three). Writers take a `&rusqlite::Transaction` and an already
//! resolved sub-changeset; readers take `&rusqlite::Connection`.
//!
//! Encoding policy: complex sub-types from `platform-wallet` are
//! captured field-by-field into typed SQLite columns where possible
//! (heights, hashes, outpoints, flags). For the remainder we store a
//! `_blob` column with a compact, self-describing byte layout
//! ([`blob::encode`] / [`blob::decode`]) — bincode is unavailable
//! because most upstream types do not derive `serde`. The layout is
//! versioned so future migrations can rewrite blobs in place.

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

/// Every per-wallet table — used by `delete_wallet` to count + cascade
/// row removal and by `inspect` for the table summary. `wallet_metadata`
/// is the parent and listed first; everything after it depends on the
/// parent row (cascade triggers wired in `V001__initial.rs`).
pub const PER_WALLET_TABLES: &[&str] = &[
    "wallet_metadata",
    "account_registrations",
    "account_address_pools",
    "core_transactions",
    "core_utxos",
    "core_instant_locks",
    "core_derived_addresses",
    "core_sync_state",
    "identities",
    "identity_keys",
    "contacts_sent",
    "contacts_recv",
    "contacts_established",
    "platform_addresses",
    "platform_address_sync",
    "asset_locks",
    "token_balances",
    "dashpay_profiles",
    "dashpay_payments_overlay",
];
