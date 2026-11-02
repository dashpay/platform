//! Native persistence for per-subwallet Orchard full viewing keys.

use std::collections::BTreeMap;

use platform_wallet::changeset::ShieldedChangeSet;
use platform_wallet::wallet::platform_wallet::WalletId;
use platform_wallet::wallet::shielded::SubwalletId;
use rusqlite::{params, Connection, Transaction};

use crate::sqlite::error::WalletStorageError;
use crate::sqlite::load_ctx::{LoadCtx, LoadSite, SiteCoords};
use crate::sqlite::schema::{blob, id32};

const VIEWING_KEY_WIDTH: usize = 96;

fn tolerate_skipped_row(
    ctx: &LoadCtx,
    error: WalletStorageError,
    row_number: usize,
    wallet_id: Option<&[u8]>,
    account_index: Option<i64>,
) -> Result<(), WalletStorageError> {
    let wallet_id = wallet_id.and_then(|bytes| bytes.try_into().ok());
    let detail = (row_number, account_index);
    ctx.tolerate_at(
        LoadSite::ShieldedViewingKeyRow,
        SiteCoords {
            wallet_id,
            account_type: &"shielded_viewing_key",
            affected: 1,
            detail: Some(&detail),
        },
        error,
    )
}

pub(crate) fn apply(
    tx: &Transaction<'_>,
    wallet_id: &WalletId,
    changeset: &ShieldedChangeSet,
) -> Result<(), WalletStorageError> {
    for (subwallet_id, viewing_key) in &changeset.viewing_keys {
        if subwallet_id.wallet_id != *wallet_id {
            return Err(WalletStorageError::WalletIdMismatch {
                expected: *wallet_id,
                found: subwallet_id.wallet_id,
            });
        }
        blob::check_fixed_width(
            i64::try_from(viewing_key.len()).unwrap_or(i64::MAX),
            VIEWING_KEY_WIDTH,
            "shielded_viewing_keys.viewing_key",
        )?;
    }

    let mut statement = tx.prepare_cached(
        "INSERT INTO shielded_viewing_keys (wallet_id, account_index, viewing_key) \
         VALUES (?1, ?2, ?3) \
         ON CONFLICT(wallet_id, account_index) DO UPDATE SET viewing_key = excluded.viewing_key",
    )?;
    for (subwallet_id, viewing_key) in &changeset.viewing_keys {
        statement.execute(params![
            subwallet_id.wallet_id.as_slice(),
            i64::from(subwallet_id.account_index),
            viewing_key,
        ])?;
    }
    Ok(())
}

/// Read every persisted subwallet viewing key.
///
/// # Errors
///
/// Under [`LoadPolicy::Strict`](crate::LoadPolicy) a row that will not
/// decode aborts the read. Under `Recovery` it is counted and skipped, and
/// that subwallet loads without its viewing key.
pub(crate) fn load_all(
    conn: &Connection,
    ctx: &LoadCtx,
) -> Result<BTreeMap<SubwalletId, Vec<u8>>, WalletStorageError> {
    let mut statement = conn.prepare_cached(
        "SELECT wallet_id, account_index, length(viewing_key), viewing_key \
         FROM shielded_viewing_keys ORDER BY wallet_id, account_index",
    )?;
    let mut rows = statement.query([])?;
    let mut viewing_keys = BTreeMap::new();
    let mut row_number = 0usize;

    while let Some(row) = rows.next()? {
        row_number += 1;
        let wallet_id_bytes = match row.get::<_, Vec<u8>>(0) {
            Ok(value) => value,
            Err(error) => {
                tolerate_skipped_row(ctx, error.into(), row_number, None, None)?;
                continue;
            }
        };
        let account_index = match row.get::<_, i64>(1) {
            Ok(value) => value,
            Err(error) => {
                tolerate_skipped_row(ctx, error.into(), row_number, Some(&wallet_id_bytes), None)?;
                continue;
            }
        };
        let viewing_key_length = match row.get::<_, i64>(2) {
            Ok(value) => value,
            Err(error) => {
                tolerate_skipped_row(
                    ctx,
                    error.into(),
                    row_number,
                    Some(&wallet_id_bytes),
                    Some(account_index),
                )?;
                continue;
            }
        };

        let decoded: Result<(SubwalletId, Vec<u8>), WalletStorageError> = (|| {
            let wallet_id = id32("shielded_viewing_keys.wallet_id", &wallet_id_bytes)?;
            let account_index = u32::try_from(account_index).map_err(|_| {
                WalletStorageError::blob_decode("shielded_viewing_keys.account_index")
            })?;
            blob::check_fixed_width(
                viewing_key_length,
                VIEWING_KEY_WIDTH,
                "shielded_viewing_keys.viewing_key",
            )?;
            let viewing_key = row.get::<_, Vec<u8>>(3)?;
            blob::check_fixed_width(
                i64::try_from(viewing_key.len()).unwrap_or(i64::MAX),
                VIEWING_KEY_WIDTH,
                "shielded_viewing_keys.viewing_key",
            )?;
            Ok((SubwalletId::new(wallet_id, account_index), viewing_key))
        })();

        match decoded {
            Ok((subwallet_id, viewing_key)) => {
                viewing_keys.insert(subwallet_id, viewing_key);
            }
            Err(error) => {
                tolerate_skipped_row(
                    ctx,
                    error,
                    row_number,
                    Some(&wallet_id_bytes),
                    Some(account_index),
                )?;
            }
        }
    }

    Ok(viewing_keys)
}
