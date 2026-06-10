//! `dashpay_profiles` + `dashpay_payments_overlay` writers.
//!
//! # Precondition
//!
//! Every `identity_id` in the supplied profile / payment maps MUST
//! already exist in the `identities` table and belong to the flush's
//! `wallet_id`. The writer relies on the
//! `identities(identity_id, wallet_id)` row produced by
//! [`super::identities::apply`] (in the same transaction or earlier)
//! for parenting; the FK to `identities(identity_id)` enforces the
//! existence half, but not the wallet match. The precondition check
//! below runs in every build and propagates
//! [`WalletStorageError::WalletIdMismatch`] on a mis-attributed caller.

use std::collections::BTreeMap;

use rusqlite::{params, Transaction};

use dpp::prelude::Identifier;
use platform_wallet::wallet::identity::{DashPayProfile, PaymentEntry};
use platform_wallet::wallet::platform_wallet::WalletId;

use crate::sqlite::error::WalletStorageError;
use crate::sqlite::schema::blob;

/// Both dashpay tables are keyed by identity only; their FK targets
/// `identities(identity_id)` so cascade flows through the
/// `wallet_metadata → identities` chain.
///
/// The `wallet_id` parameter is kept on the signature for symmetry
/// with the persister's `write_changeset_in_one_tx` dispatch table,
/// and feeds the precondition check; it does not feed any column.
pub fn apply(
    tx: &Transaction<'_>,
    wallet_id: &WalletId,
    profiles: Option<&BTreeMap<Identifier, Option<DashPayProfile>>>,
    payments: Option<&BTreeMap<Identifier, BTreeMap<String, PaymentEntry>>>,
) -> Result<(), WalletStorageError> {
    // Precondition: every identity_id we touch must already belong to
    // the flush-scope wallet (or to no wallet if scope is the
    // sentinel). Cheap SELECT inside the same tx, run in every build.
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
