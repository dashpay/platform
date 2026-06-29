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
//! # Precondition
//!
//! Every `identity_id` MUST already exist in `identities` and belong to the
//! flush's `wallet_id`. The FK enforces existence; the wallet match is checked
//! here and propagates [`WalletStorageError::WalletIdMismatch`] on a
//! mis-attributed caller.

use std::collections::BTreeMap;

use rusqlite::{params, Transaction};

use dpp::prelude::Identifier;
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
        }
    }
    Ok(())
}
