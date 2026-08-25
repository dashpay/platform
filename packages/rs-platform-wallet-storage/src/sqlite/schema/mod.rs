//! Per-area SQLite writers + readers, one submodule per table or cluster.
//!
//! Encoding policy: scalars that fan out to per-row indexes go into typed
//! columns (heights, hashes, outpoints, flags); `_blob` columns carry the
//! full sub-changeset entry via [`blob::encode`] / [`blob::decode`]. Schema
//! evolution is gated by the refinery migration version — blobs carry no
//! inline revision tag.

pub mod accounts;
pub mod asset_locks;
pub mod blob;
pub mod contacts;
pub mod core_pool;
pub mod core_state;
pub mod dashpay;
pub mod dpns_name_states;
pub mod identities;
pub mod identity_keys;
pub mod invitations;
pub mod pending_contact_crypto;
pub mod platform_addrs;
#[cfg(feature = "shielded")]
pub mod shielded_viewing_keys;
pub mod token_balances;
pub mod tracked_masternodes;
pub mod versions;
pub mod wallets;

/// Map a `WalletId` to a nullable `wallet_id` column: the all-zero
/// sentinel becomes NULL, the storage spelling of "owned by no wallet".
///
/// Shared by `identities` and `identity_keys` so both spell the unowned
/// scope the same way — a raw `wallet_id.as_slice()` would store 32 zero
/// bytes, a value that looks like a wallet id, satisfies nothing, and
/// silently fails to match the NULL the readers and guards look for.
pub(crate) fn wallet_id_to_param(
    wallet_id: &platform_wallet::wallet::platform_wallet::WalletId,
) -> Option<&[u8]> {
    if wallet_id.iter().all(|b| *b == 0) {
        None
    } else {
        Some(wallet_id.as_slice())
    }
}

pub(crate) fn id32(
    column: &'static str,
    bytes: &[u8],
) -> Result<[u8; 32], crate::sqlite::error::WalletStorageError> {
    <[u8; 32]>::try_from(bytes).map_err(|_| {
        crate::sqlite::error::WalletStorageError::InvalidWalletIdLength {
            column,
            actual: bytes.len(),
        }
    })
}

/// Reject any `identity_id` in `touched` whose `identities` row does not
/// belong to `wallet_id` (NULL wallet_id matches the all-zero sentinel),
/// returning [`WalletStorageError::WalletIdMismatch`] on the first offender.
/// Absent rows are left to the child-table FK.
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
            // Row absent — let the child-table FK reject the write.
            continue;
        };
        // INTENTIONAL: arms below zero-pad a non-32-byte stored wallet_id into
        // the diagnostic `found` field — cosmetic only, a mismatch still errors.
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
    use super::id32;
    use crate::sqlite::error::WalletStorageError;

    #[test]
    fn id32_reports_column_and_actual_length() {
        let error = id32("example.owner_id", &[0u8; 7]).unwrap_err();
        assert!(matches!(
            error,
            WalletStorageError::InvalidWalletIdLength {
                column: "example.owner_id",
                actual: 7
            }
        ));
    }
}
