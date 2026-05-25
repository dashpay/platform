//! `dashpay_profiles` + `dashpay_payments_overlay` writers.

use std::collections::BTreeMap;

use rusqlite::{params, Transaction};

use dpp::prelude::Identifier;
use platform_wallet::wallet::identity::{DashPayProfile, PaymentEntry};
use platform_wallet::wallet::platform_wallet::WalletId;

use crate::sqlite::error::WalletStorageError;
use crate::sqlite::schema::blob;

/// V002: both dashpay tables are keyed by identity only; their FK
/// targets `identities(identity_id)` so cascade flows through the
/// `wallet_metadata → identities` chain.
///
/// The `_wallet_id` parameter is kept on the signature for source
/// compatibility with the persister's `write_changeset_in_one_tx`
/// dispatch table, but it does not feed any column.
pub fn apply(
    tx: &Transaction<'_>,
    _wallet_id: &WalletId,
    profiles: Option<&BTreeMap<Identifier, Option<DashPayProfile>>>,
    payments: Option<&BTreeMap<Identifier, BTreeMap<String, PaymentEntry>>>,
) -> Result<(), WalletStorageError> {
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
