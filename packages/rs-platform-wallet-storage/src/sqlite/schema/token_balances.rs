//! `token_balances` table writer.

use rusqlite::{params, Transaction};

use platform_wallet::changeset::TokenBalanceChangeSet;
use platform_wallet::wallet::platform_wallet::WalletId;

use crate::sqlite::error::WalletStorageError;
use crate::sqlite::util::safe_cast;

/// `token_balances` is keyed by `(identity_id, token_id)`. The caller
/// supplies a [`WalletId`] for symmetry with sibling writers — it is
/// unused on this writer because cascade flows
/// `wallet_metadata → identities → token_balances` through the
/// nullable `identities.wallet_id` FK.
//
// Orphan-row policy: there is no automatic prune API. Cascade flows
// through `identities`; hosts that delete identities out-of-band must
// prune `token_balances` themselves.
pub fn apply(
    tx: &Transaction<'_>,
    _wallet_id: &WalletId,
    cs: &TokenBalanceChangeSet,
) -> Result<(), WalletStorageError> {
    if !cs.balances.is_empty() {
        let now = chrono::Utc::now().timestamp();
        let mut stmt = tx.prepare_cached(
            "INSERT INTO token_balances \
                (identity_id, token_id, balance, updated_at) \
             VALUES (?1, ?2, ?3, ?4) \
             ON CONFLICT(identity_id, token_id) DO UPDATE SET \
                balance = excluded.balance, \
                updated_at = excluded.updated_at",
        )?;
        for ((identity_id, token_id), balance) in &cs.balances {
            stmt.execute(params![
                identity_id.as_slice(),
                token_id.as_slice(),
                safe_cast::u64_to_i64("token_balances.balance", *balance)?,
                now,
            ])?;
        }
    }
    if !cs.removed_balances.is_empty() {
        let mut stmt = tx.prepare_cached(
            "DELETE FROM token_balances WHERE identity_id = ?1 AND token_id = ?2",
        )?;
        for (identity_id, token_id) in &cs.removed_balances {
            stmt.execute(params![identity_id.as_slice(), token_id.as_slice()])?;
        }
    }
    Ok(())
}
