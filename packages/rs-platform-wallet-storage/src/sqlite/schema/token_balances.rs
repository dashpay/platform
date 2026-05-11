//! `token_balances` table writer.

use rusqlite::{params, Transaction};

use platform_wallet::changeset::TokenBalanceChangeSet;
use platform_wallet::wallet::platform_wallet::WalletId;

use crate::sqlite::error::SqlitePersisterError;

pub fn apply(
    tx: &Transaction<'_>,
    wallet_id: &WalletId,
    cs: &TokenBalanceChangeSet,
) -> Result<(), SqlitePersisterError> {
    let now = chrono::Utc::now().timestamp();
    for ((identity_id, token_id), balance) in &cs.balances {
        tx.execute(
            "INSERT INTO token_balances \
                (wallet_id, identity_id, token_id, balance, updated_at) \
             VALUES (?1, ?2, ?3, ?4, ?5) \
             ON CONFLICT(wallet_id, identity_id, token_id) DO UPDATE SET \
                balance = excluded.balance, \
                updated_at = excluded.updated_at",
            params![
                wallet_id.as_slice(),
                identity_id.as_slice(),
                token_id.as_slice(),
                *balance as i64,
                now,
            ],
        )?;
    }
    for (identity_id, token_id) in &cs.removed_balances {
        tx.execute(
            "DELETE FROM token_balances \
             WHERE wallet_id = ?1 AND identity_id = ?2 AND token_id = ?3",
            params![
                wallet_id.as_slice(),
                identity_id.as_slice(),
                token_id.as_slice()
            ],
        )?;
    }
    Ok(())
}
