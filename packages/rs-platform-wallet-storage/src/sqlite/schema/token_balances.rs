//! `token_balances` table writer.
//!
//! # Precondition
//!
//! Every `identity_id` MUST already exist in `identities` and belong to the
//! flush's `wallet_id` (or have NULL `wallet_id` for the all-zero sentinel
//! scope). The FK enforces existence; the wallet match is checked here and
//! propagates [`WalletStorageError::WalletIdMismatch`] on a mis-attributed
//! caller.

use rusqlite::{params, Transaction};

use platform_wallet::changeset::TokenBalanceChangeSet;
use platform_wallet::wallet::platform_wallet::WalletId;

use crate::sqlite::error::WalletStorageError;
use crate::sqlite::util::safe_cast;

/// Keyed by `(identity_id, token_id)`. `wallet_id` feeds the precondition
/// check only — no column — since cascade flows
/// `wallets → identities → token_balances` via the nullable
/// `identities.wallet_id` FK. No auto-prune: hosts deleting identities
/// out-of-band must prune `token_balances` themselves.
pub fn apply(
    tx: &Transaction<'_>,
    wallet_id: &WalletId,
    cs: &TokenBalanceChangeSet,
) -> Result<(), WalletStorageError> {
    let touched: std::collections::BTreeSet<dpp::prelude::Identifier> = cs
        .balances
        .keys()
        .map(|(identity_id, _)| *identity_id)
        .chain(
            cs.removed_balances
                .iter()
                .map(|(identity_id, _)| *identity_id),
        )
        .collect();
    super::assert_identities_belong_to_wallet(tx, wallet_id, &touched)?;
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
