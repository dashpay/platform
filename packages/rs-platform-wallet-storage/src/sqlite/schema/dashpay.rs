//! `dashpay_profiles` + `dashpay_payments_overlay` writers.

use std::collections::BTreeMap;

use rusqlite::{params, Transaction};

use dpp::prelude::Identifier;
use platform_wallet::wallet::identity::{DashPayProfile, PaymentEntry};
use platform_wallet::wallet::platform_wallet::WalletId;

use crate::sqlite::error::WalletStorageError;
use crate::sqlite::schema::blob;

/// Apply both dashpay overlays.
pub fn apply(
    tx: &Transaction<'_>,
    wallet_id: &WalletId,
    profiles: Option<&BTreeMap<Identifier, Option<DashPayProfile>>>,
    payments: Option<&BTreeMap<Identifier, BTreeMap<String, PaymentEntry>>>,
) -> Result<(), WalletStorageError> {
    if let Some(profiles) = profiles {
        for (identity_id, profile) in profiles {
            match profile {
                None => {
                    tx.execute(
                        "DELETE FROM dashpay_profiles WHERE wallet_id = ?1 AND identity_id = ?2",
                        params![wallet_id.as_slice(), identity_id.as_slice()],
                    )?;
                }
                Some(p) => {
                    let payload = blob::encode(p)?;
                    tx.execute(
                        "INSERT INTO dashpay_profiles (wallet_id, identity_id, profile_blob) \
                         VALUES (?1, ?2, ?3) \
                         ON CONFLICT(wallet_id, identity_id) DO UPDATE SET profile_blob = excluded.profile_blob",
                        params![wallet_id.as_slice(), identity_id.as_slice(), payload],
                    )?;
                }
            }
        }
    }
    if let Some(payments) = payments {
        for (identity_id, by_tx) in payments {
            for (tx_id, entry) in by_tx {
                let payload = blob::encode(entry)?;
                tx.execute(
                    "INSERT INTO dashpay_payments_overlay \
                        (wallet_id, identity_id, payment_id, overlay_blob) \
                     VALUES (?1, ?2, ?3, ?4) \
                     ON CONFLICT(wallet_id, identity_id, payment_id) DO UPDATE SET overlay_blob = excluded.overlay_blob",
                    params![wallet_id.as_slice(), identity_id.as_slice(), tx_id, payload],
                )?;
            }
        }
    }
    Ok(())
}
