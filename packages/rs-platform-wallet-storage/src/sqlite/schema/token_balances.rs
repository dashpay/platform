//! `token_balances` table writer.
//!
//! # Precondition
//!
//! Every `identity_id` in the supplied changeset MUST already exist in
//! the `identities` table and belong to the flush's `wallet_id` (or
//! have a NULL `identities.wallet_id` when the scope is the all-zero
//! sentinel). The writer relies on
//! [`super::identities::apply`] for parenting; the FK to
//! `identities(identity_id)` enforces existence but not the wallet
//! match. The precondition check below runs in every build and
//! propagates [`WalletStorageError::WalletIdMismatch`] on a
//! mis-attributed caller.

use rusqlite::{params, Transaction};

use platform_wallet::changeset::TokenBalanceChangeSet;
use platform_wallet::wallet::platform_wallet::WalletId;

use crate::sqlite::error::WalletStorageError;
use crate::sqlite::util::safe_cast;

/// `token_balances` is keyed by `(identity_id, token_id)`. The caller
/// supplies a [`WalletId`] for symmetry with sibling writers and to
/// feed the precondition check; it does not feed any column, because
/// cascade flows
/// `wallet_metadata → identities → token_balances` through the
/// nullable `identities.wallet_id` FK.
//
// Orphan-row policy: there is no automatic prune API. Cascade flows
// through `identities`; hosts that delete identities out-of-band must
// prune `token_balances` themselves.
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
