//! Per-area SQLite writers + readers.
//!
//! Each submodule owns one table or a small cluster (e.g. `accounts`
//! owns the registration + address-pool tables). Writers take a
//! `&rusqlite::Transaction` and an already resolved sub-changeset;
//! readers take `&rusqlite::Connection`.
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

use crate::sqlite::error::WalletStorageError;

/// How a per-wallet table is row-scoped against a `wallet_id`.
/// Identity-owned tables (`identity_keys`, `token_balances`,
/// `dashpay_profiles`, `dashpay_payments_overlay`) have no direct
/// `wallet_id` column; they reach the parent wallet only via the
/// cascading FK chain `wallet_metadata → identities → …`.
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
/// in `V001__initial.rs`. Identity-owned children cascade through
/// `identities` (nullable `wallet_id` link) rather than directly off
/// `wallet_metadata`.
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
    ("contacts", WalletScope::DirectColumn),
    ("platform_addresses", WalletScope::DirectColumn),
    ("platform_address_sync", WalletScope::DirectColumn),
    ("asset_locks", WalletScope::DirectColumn),
    ("token_balances", WalletScope::ViaIdentity),
    ("dashpay_profiles", WalletScope::ViaIdentity),
    ("dashpay_payments_overlay", WalletScope::ViaIdentity),
    // Per-object metadata tables (`src/kv.rs`). Registered so
    // `delete_wallet`/`inspect` purge and count them even when the `kv`
    // API is compiled out. `meta_global` has no wallet scope and is
    // intentionally absent — it survives wallet delete.
    ("meta_wallet", WalletScope::DirectColumn),
    ("meta_identity", WalletScope::ViaIdentity),
    ("meta_token", WalletScope::ViaIdentity),
    ("meta_contact", WalletScope::DirectColumn),
    ("meta_platform_address", WalletScope::DirectColumn),
];

/// SQL fragment for counting rows of `table` belonging to a single
/// wallet. `scope` selects the predicate flavour. The fragment includes
/// the leading `SELECT COUNT(*) FROM` so the call site can format it
/// directly and bind a single `?1` parameter (the wallet id bytes).
///
/// CMT-023: `table` is validated against [`PER_WALLET_TABLES`] before
/// interpolation. SQLite cannot bind identifiers as parameters, so the
/// table name is `format!`-spliced into the SQL by design — the
/// allowlist closes the latent injection footgun should this helper
/// ever be reached from caller input. Debug builds panic on an unknown
/// name; release builds return the typed
/// [`WalletStorageError::SchemaInvariantViolated`] so the failure can
/// be inspected up the stack instead of silently producing wrong SQL.
pub fn count_rows_for_wallet_sql(
    table: &str,
    scope: WalletScope,
) -> Result<String, WalletStorageError> {
    let allowed = PER_WALLET_TABLES.iter().any(|(t, _)| *t == table);
    debug_assert!(
        allowed,
        "count_rows_for_wallet_sql: `{table}` is not in PER_WALLET_TABLES; \
         add it to the allowlist or fix the caller",
    );
    if !allowed {
        return Err(WalletStorageError::SchemaInvariantViolated {
            detail: "count_rows_for_wallet_sql called with table outside PER_WALLET_TABLES",
        });
    }
    Ok(match scope {
        WalletScope::DirectColumn => {
            format!("SELECT COUNT(*) FROM {table} WHERE wallet_id = ?1")
        }
        WalletScope::ViaIdentity => format!(
            "SELECT COUNT(*) FROM {table} \
             WHERE identity_id IN (SELECT identity_id FROM identities WHERE wallet_id = ?1)"
        ),
    })
}

/// Defensive check that every `identity_id` in `touched` exists in
/// `identities` and belongs to `wallet_id` (or has NULL wallet_id when
/// scope is the all-zero sentinel). Used by identity-owned writers
/// (`dashpay`, `token_balances`) to reject mis-attributed callers; the
/// check runs in every build.
///
/// Returns [`WalletStorageError::WalletIdMismatch`] for the first
/// offending row found. Rows that don't exist in `identities` aren't
/// flagged here — the FK on the child table will reject the write.
pub(crate) fn assert_identities_belong_to_wallet(
    tx: &rusqlite::Transaction<'_>,
    wallet_id: &platform_wallet::wallet::platform_wallet::WalletId,
    touched: &std::collections::BTreeSet<dpp::prelude::Identifier>,
) -> Result<(), crate::sqlite::error::WalletStorageError> {
    use crate::sqlite::error::WalletStorageError;
    use rusqlite::OptionalExtension;
    let scope_is_sentinel = wallet_id.iter().all(|b| *b == 0);
    let mut stmt = tx.prepare_cached("SELECT wallet_id FROM identities WHERE identity_id = ?1")?;
    for identity_id in touched {
        let row: Option<Option<Vec<u8>>> = stmt
            .query_row(rusqlite::params![identity_id.as_slice()], |row| row.get(0))
            .optional()?;
        let Some(found_wallet_id) = row else {
            // Row absent — FK on the child table will reject the
            // upcoming write with a clearer error than guessing.
            continue;
        };
        match (scope_is_sentinel, found_wallet_id) {
            (true, None) => {} // sentinel scope matches NULL parenting
            (true, Some(found)) => {
                let mut found_arr = [0u8; 32];
                if found.len() == 32 {
                    found_arr.copy_from_slice(&found);
                }
                return Err(WalletStorageError::WalletIdMismatch {
                    expected: [0u8; 32],
                    found: found_arr,
                });
            }
            (false, None) => {
                return Err(WalletStorageError::WalletIdMismatch {
                    expected: *wallet_id,
                    found: [0u8; 32],
                });
            }
            (false, Some(found)) => {
                if found.as_slice() != wallet_id.as_slice() {
                    let mut found_arr = [0u8; 32];
                    if found.len() == 32 {
                        found_arr.copy_from_slice(&found);
                    }
                    return Err(WalletStorageError::WalletIdMismatch {
                        expected: *wallet_id,
                        found: found_arr,
                    });
                }
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// CMT-023: every known per-wallet table compiles into a SQL
    /// fragment without tripping the allowlist guard.
    #[test]
    fn allowlist_accepts_every_known_table() {
        for (table, scope) in PER_WALLET_TABLES {
            count_rows_for_wallet_sql(table, *scope)
                .unwrap_or_else(|e| panic!("allowlisted table `{table}` rejected: {e}"));
        }
    }

    /// CMT-023: a table name outside `PER_WALLET_TABLES` must be
    /// rejected with the typed `SchemaInvariantViolated` error in
    /// release builds. Debug builds panic via `debug_assert!` (not
    /// exercised here — we only assert the typed-error path that
    /// survives `--release`).
    #[cfg(not(debug_assertions))]
    #[test]
    fn allowlist_rejects_unknown_table() {
        let err = count_rows_for_wallet_sql("attacker_injected", WalletScope::DirectColumn)
            .expect_err("must reject unknown table");
        assert!(
            matches!(err, WalletStorageError::SchemaInvariantViolated { .. }),
            "expected SchemaInvariantViolated, got {err:?}"
        );
    }
}
