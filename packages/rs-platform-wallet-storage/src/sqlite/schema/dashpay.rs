//! `dashpay_profiles` + `dashpay_payments_overlay` writers.
//!
//! # Write-only indexed overlay (NOT a rehydration source)
//!
//! These tables are honored on write but `load()` does NOT read them back:
//! DashPay state is rehydrated from the identities `entry_blob`, which is the
//! authoritative load source. They exist for future per-profile/per-payment
//! indexed queries. Round-trip pinned by
//! `tests/sqlite_dashpay_overlay_contract.rs`.
//!
//! # …which is why the payment overlay is also PATCHED into `entry_blob`
//!
//! Because `load()` reads only the identity blob, a payment row that reached
//! the overlay table alone is a write that does not survive a restart. The
//! wallet-event adapter is the single writer of swept sent-payment verdicts
//! and has no other durable channel for them, so [`apply`] finishes by
//! folding each overlay row into the owning identity's `entry_blob`.
//!
//! The fold is a read-modify-write of ONLY the `(txid -> PaymentEntry)` keys
//! the overlay names, run inside the persister's own write transaction:
//! payments the overlay does not mention, and every other field of the
//! entry, are read back and written out unchanged. That is what distinguishes
//! it from shipping a whole `IdentityEntry` from the caller — a snapshot
//! captured before the lock was released would wholesale replace the blob and
//! silently drop any payment another writer committed in between
//! (dashpay/platform#4651). Doing the merge here, in the same transaction,
//! makes it atomic against every other writer on the file.
//!
//! # Precondition
//!
//! Every `identity_id` MUST already exist in `identities` and belong to the
//! flush's `wallet_id`. The FK enforces existence; the wallet match is checked
//! here and propagates [`WalletStorageError::WalletIdMismatch`] on a
//! mis-attributed caller.

use std::collections::BTreeMap;

use rusqlite::{params, Transaction};

use dpp::prelude::Identifier;
use platform_wallet::changeset::IdentityEntry;
use platform_wallet::wallet::identity::{DashPayProfile, PaymentEntry};
use platform_wallet::wallet::platform_wallet::WalletId;

use crate::sqlite::error::WalletStorageError;
use crate::sqlite::schema::blob;
use crate::sqlite::schema::blob::impl_persistable_blob;

// PUBLIC material only: DashPay overlay types reaching `_blob` columns.
impl_persistable_blob!(DashPayProfile, PaymentEntry);

/// Both tables are keyed by identity only; their FK to
/// `identities(identity_id)` cascades via the `wallets → identities` chain.
/// `wallet_id` feeds the precondition check only — no column.
pub fn apply(
    tx: &Transaction<'_>,
    wallet_id: &WalletId,
    profiles: Option<&BTreeMap<Identifier, Option<DashPayProfile>>>,
    payments: Option<&BTreeMap<Identifier, BTreeMap<String, PaymentEntry>>>,
) -> Result<(), WalletStorageError> {
    let touched: std::collections::BTreeSet<Identifier> = profiles
        .iter()
        .flat_map(|m| m.keys().copied())
        .chain(payments.iter().flat_map(|m| m.keys().copied()))
        .collect();
    super::assert_identities_belong_to_wallet(tx, wallet_id, &touched)?;
    if let Some(profiles) = profiles {
        if !profiles.is_empty() {
            let mut delete_stmt =
                tx.prepare_cached("DELETE FROM dashpay_profiles WHERE identity_id = ?1")?;
            let mut insert_stmt = tx.prepare_cached(
                "INSERT INTO dashpay_profiles (identity_id, profile_blob) \
                 VALUES (?1, ?2) \
                 ON CONFLICT(identity_id) DO UPDATE SET profile_blob = excluded.profile_blob",
            )?;
            for (identity_id, profile) in profiles {
                match profile {
                    None => {
                        delete_stmt.execute(params![identity_id.as_slice()])?;
                    }
                    Some(p) => {
                        let payload = blob::encode(p)?;
                        insert_stmt.execute(params![identity_id.as_slice(), payload])?;
                    }
                }
            }
        }
    }
    if let Some(payments) = payments {
        if !payments.is_empty() {
            let mut stmt = tx.prepare_cached(
                "INSERT INTO dashpay_payments_overlay \
                    (identity_id, payment_id, overlay_blob) \
                 VALUES (?1, ?2, ?3) \
                 ON CONFLICT(identity_id, payment_id) DO UPDATE SET overlay_blob = excluded.overlay_blob",
            )?;
            for (identity_id, by_tx) in payments {
                for (tx_id, entry) in by_tx {
                    let payload = blob::encode(entry)?;
                    stmt.execute(params![identity_id.as_slice(), tx_id, payload])?;
                }
            }
            patch_payments_into_entry_blobs(tx, payments)?;
        }
    }
    Ok(())
}

/// Fold the overlay's rows into each owning identity's authoritative
/// `entry_blob`, one identity at a time, inside the caller's transaction.
///
/// Read-modify-write, NOT a replace: the stored entry is decoded, only the
/// `(txid -> PaymentEntry)` keys this round names are inserted-or-replaced in
/// its `dashpay_payments` map, and the entry is written back. Every other
/// payment — including one another writer committed microseconds ago — and
/// every other field survive untouched.
///
/// Runs AFTER `identities::apply_upserts` in `persister::apply_changeset`, so
/// a round that legitimately carries both a full identity snapshot and an
/// overlay ends with the overlay applied ON TOP of the snapshot, which is the
/// order the two mean: the snapshot is the round's view of the identity, the
/// overlay is the round's view of the payments that moved.
///
/// A missing `identities` row is not an error here. The FK would have
/// rejected the overlay insert above, so by this point the row exists for
/// every identity in `payments` unless it was deleted inside this same
/// transaction; nothing is patched in that case and the delete stands.
fn patch_payments_into_entry_blobs(
    tx: &Transaction<'_>,
    payments: &BTreeMap<Identifier, BTreeMap<String, PaymentEntry>>,
) -> Result<(), WalletStorageError> {
    let mut read = tx.prepare_cached(
        "SELECT length(entry_blob), entry_blob FROM identities WHERE identity_id = ?1",
    )?;
    let mut write =
        tx.prepare_cached("UPDATE identities SET entry_blob = ?2 WHERE identity_id = ?1")?;
    for (identity_id, by_tx) in payments {
        if by_tx.is_empty() {
            continue;
        }
        let stored: Option<(i64, Vec<u8>)> = read
            .query_row(params![identity_id.as_slice()], |row| {
                Ok((row.get(0)?, row.get(1)?))
            })
            .map(Some)
            .or_else(|e| match e {
                rusqlite::Error::QueryReturnedNoRows => Ok(None),
                other => Err(other),
            })?;
        let Some((len, payload)) = stored else {
            continue;
        };
        blob::check_size(len)?;
        let mut entry: IdentityEntry = blob::decode(&payload)?;
        for (tx_id, row) in by_tx {
            entry.dashpay_payments.insert(tx_id.clone(), row.clone());
        }
        let patched = blob::encode(&entry)?;
        write.execute(params![identity_id.as_slice(), patched])?;
    }
    Ok(())
}
