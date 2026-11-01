//! DashPay coreHeight backfill record writer + reader.
//!
//! See [`DashPayBackfillRecord`] for what a record claims. It has no table of
//! its own: the extent (`floor`, `rewound_from`) sits on the wallet's
//! `core_sync_state` row beside the cursor it lowered, and each covered entry
//! is `dashpay_covered_from` on the `account_registrations` row of the
//! receiving account it names. Every write carries the whole record, so the
//! writer replaces rather than folds, and an empty record is an invalidation:
//! it clears whatever is on file.

use dpp::prelude::Identifier;
use key_wallet::account::AccountType;
use rusqlite::{params, Connection, OptionalExtension, Transaction};

use platform_wallet::changeset::{DashPayBackfillCoveredContact, DashPayBackfillRecord};
use platform_wallet::wallet::platform_wallet::WalletId;

use crate::sqlite::error::WalletStorageError;
use crate::sqlite::schema::accounts::{
    account_dashpay_ids, account_index, account_key_class, account_type_db_label,
};
use crate::sqlite::schema::{blob, id32};
use crate::sqlite::util::safe_cast::i64_to_u32;

/// Replace this wallet's backfill record with `record`; an empty record (no
/// covered receiving accounts) only clears the stored one. It is how upstream
/// retracts coverage it can no longer vouch for, and a record with nothing
/// covered claims nothing anyway.
///
/// Runs after the round's account registrations are written. An entry whose
/// receiving account has no `account_registrations` row is dropped: a contact
/// missing from the record is only backfilled again, while one wrongly present
/// would never be. When every entry is dropped the extent is cleared too.
pub fn apply(
    tx: &Transaction<'_>,
    wallet_id: &WalletId,
    record: &DashPayBackfillRecord,
) -> Result<(), WalletStorageError> {
    tx.prepare_cached(
        "UPDATE account_registrations SET dashpay_covered_from = NULL \
         WHERE wallet_id = ?1 AND dashpay_covered_from IS NOT NULL",
    )?
    .execute(params![wallet_id.as_slice()])?;
    let mut mark = tx.prepare_cached(
        "UPDATE account_registrations SET dashpay_covered_from = ?7 \
         WHERE wallet_id = ?1 AND account_type = ?2 AND account_index = ?3 \
           AND key_class = ?4 AND user_identity_id = ?5 AND friend_identity_id = ?6",
    )?;
    let mut marked = 0;
    for entry in record.covered() {
        let account = receiving_account(entry);
        let (user_identity_id, friend_identity_id) = account_dashpay_ids(&account);
        marked += mark.execute(params![
            wallet_id.as_slice(),
            account_type_db_label(&account),
            i64::from(account_index(&account)),
            i64::from(account_key_class(&account)),
            &user_identity_id[..],
            &friend_identity_id[..],
            i64::from(entry.covered_from),
        ])?;
    }
    let extent = (marked > 0).then(|| (i64::from(record.floor), i64::from(record.rewound_from)));
    tx.prepare_cached(
        "INSERT INTO core_sync_state \
            (wallet_id, dashpay_backfill_floor, dashpay_backfill_rewound_from) \
         VALUES (?1, ?2, ?3) \
         ON CONFLICT(wallet_id) DO UPDATE SET \
            dashpay_backfill_floor = excluded.dashpay_backfill_floor, \
            dashpay_backfill_rewound_from = excluded.dashpay_backfill_rewound_from",
    )?
    .execute(params![
        wallet_id.as_slice(),
        extent.map(|(floor, _)| floor),
        extent.map(|(_, rewound_from)| rewound_from),
    ])?;
    Ok(())
}

/// This wallet's backfill record, or the empty record when none is on file.
///
/// Absence reads as "no backfill recorded", which upstream answers by
/// re-running the backfill — slow, never lossy — so a wallet that predates
/// these columns is left on exactly the behaviour it had. Covered accounts
/// without an extent read the same way; the writer never leaves that state.
///
/// # Errors
///
/// A height or index that does not narrow to `u32`, or an identity id that is
/// not 32 bytes, is structural and fatal in every load policy; the columns'
/// CHECK constraints make either one a sign the file was written with them
/// bypassed.
pub fn load_for_wallet(
    conn: &Connection,
    wallet_id: &WalletId,
) -> Result<DashPayBackfillRecord, WalletStorageError> {
    let extent: Option<(Option<i64>, Option<i64>)> = conn
        .prepare_cached(
            "SELECT dashpay_backfill_floor, dashpay_backfill_rewound_from \
             FROM core_sync_state WHERE wallet_id = ?1",
        )?
        .query_row(params![wallet_id.as_slice()], |row| {
            Ok((row.get(0)?, row.get(1)?))
        })
        .optional()?;
    let Some((Some(floor), Some(rewound_from))) = extent else {
        return Ok(DashPayBackfillRecord::default());
    };
    let floor = i64_to_u32("core_sync_state.dashpay_backfill_floor", floor)?;
    let rewound_from = i64_to_u32(
        "core_sync_state.dashpay_backfill_rewound_from",
        rewound_from,
    )?;

    let mut stmt = conn.prepare_cached(
        "SELECT account_index, length(user_identity_id), user_identity_id, \
                length(friend_identity_id), friend_identity_id, dashpay_covered_from \
         FROM account_registrations \
         WHERE wallet_id = ?1 AND dashpay_covered_from IS NOT NULL",
    )?;
    let mut rows = stmt.query(params![wallet_id.as_slice()])?;
    let mut covered = Vec::new();
    while let Some(row) = rows.next()? {
        let account_index = i64_to_u32("account_registrations.account_index", row.get(0)?)?;
        blob::check_fixed_width(row.get(1)?, 32, "account_registrations.user_identity_id")?;
        let owner: Vec<u8> = row.get(2)?;
        blob::check_fixed_width(row.get(3)?, 32, "account_registrations.friend_identity_id")?;
        let contact: Vec<u8> = row.get(4)?;
        let covered_from = i64_to_u32("account_registrations.dashpay_covered_from", row.get(5)?)?;
        covered.push(DashPayBackfillCoveredContact {
            owner: Identifier::from(id32("account_registrations.user_identity_id", &owner)?),
            contact: Identifier::from(id32("account_registrations.friend_identity_id", &contact)?),
            account_index,
            covered_from,
        });
    }
    Ok(DashPayBackfillRecord::from_entries(
        floor,
        rewound_from,
        covered,
    ))
}

/// The receiving account a covered entry names, so its row is addressed
/// through the same typed-column helpers the account writer binds.
fn receiving_account(entry: &DashPayBackfillCoveredContact) -> AccountType {
    AccountType::DashpayReceivingFunds {
        index: entry.account_index,
        user_identity_id: entry.owner.to_buffer(),
        friend_identity_id: entry.contact.to_buffer(),
    }
}
