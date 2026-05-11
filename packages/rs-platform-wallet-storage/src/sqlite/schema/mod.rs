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
