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
pub mod invitations;
pub mod pending_contact_crypto;
pub mod platform_addrs;
pub mod token_balances;
pub mod wallet_meta;

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
        // INTENTIONAL: the `Some(found)` arms below zero-pad a stored
        // wallet_id whose width is not 32 into the diagnostic `found` field.
        // This is diagnostic-only and cosmetic — a malformed stored width
        // already triggers a mismatch error; reporting it zero-padded carries
        // no security impact, so a typed length error is not warranted.
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
