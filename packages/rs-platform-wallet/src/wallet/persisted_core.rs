//! Shared Core wallet reconstruction from backend-validated persisted rows.

use dashcore::ephemerealdata::chain_lock::ChainLock;
use dashcore::{OutPoint, Txid};
use key_wallet::account::AccountType;
use key_wallet::managed_account::managed_account_trait::ManagedAccountTrait;
use key_wallet::wallet::managed_wallet_info::wallet_info_interface::WalletInfoInterface;
use key_wallet::wallet::managed_wallet_info::ManagedWalletInfo;
use key_wallet::Utxo;
use std::collections::BTreeMap;

/// Validated Core rows, after the backend resolves each coin's owning account.
#[derive(Debug, Default)]
pub struct PersistedCoreState {
    /// Persisted sync height, or no change to the skeleton's initial height.
    pub synced_height: Option<u32>,
    /// Persisted processed height, or no change to the initial height.
    pub last_processed_height: Option<u32>,
    /// Persisted last-sync timestamp.
    pub last_synced: Option<u64>,
    /// Highest persisted ChainLock.
    pub chain_lock: Option<ChainLock>,
    /// Unspent coins paired with an existing funds account.
    pub utxos: Vec<(AccountType, Utxo)>,
    /// Durable spend claims, including unknown claimants and absent transaction bodies.
    pub spent_claims: Vec<(OutPoint, Option<Txid>)>,
}

/// Invalid normalized persisted state; the backend must resolve it before loading.
#[derive(Debug, thiserror::Error)]
pub enum CoreRestoreError {
    /// A coin names an account absent from the restored wallet.
    #[error("persisted coin belongs to missing funds account {0:?}")]
    MissingFundsAccount(AccountType),
    /// Two rows disagree about the claimant of the same output.
    #[error("persisted spend claims disagree for {0}")]
    ConflictingSpendClaims(OutPoint),
}

/// Populate an account skeleton with persisted Core state and recompute its balance.
///
/// Backends retain their decoding, recovery and address-pool policies. All coin
/// owners are validated before mutation. Claims exclude coins even if a backend
/// supplied both rows. Record/proof restoration remains the caller's responsibility;
/// restore claims before replaying any funding transactions.
///
/// # Errors
/// Returns an error before mutation for missing coin owners or conflicting claims.
pub fn restore_core_wallet(
    info: &mut ManagedWalletInfo,
    state: PersistedCoreState,
) -> Result<(), CoreRestoreError> {
    validate_claims(&state.spent_claims)?;
    let mut accounts = info.accounts.all_funding_accounts_mut();
    let owners: Vec<_> = accounts
        .iter()
        .map(|account| account.managed_account_type().to_account_type())
        .collect();
    let targets = state
        .utxos
        .iter()
        .map(|(owner, _)| {
            owners
                .iter()
                .position(|candidate| candidate == owner)
                .ok_or(CoreRestoreError::MissingFundsAccount(*owner))
        })
        .collect::<Result<Vec<_>, _>>()?;
    for ((_, utxo), target) in state.utxos.into_iter().zip(targets) {
        accounts[target].utxos.insert(utxo.outpoint, utxo);
    }
    if let Some(height) = state.synced_height {
        info.metadata.synced_height = height;
    }
    if let Some(height) = state.last_processed_height {
        info.metadata.last_processed_height = height;
    }
    if let Some(timestamp) = state.last_synced {
        info.metadata.last_synced = Some(timestamp);
    }
    if let Some(lock) = state.chain_lock {
        info.metadata.last_applied_chain_lock = Some(lock);
    }
    restore_spent_claims(info, &state.spent_claims)?;
    Ok(())
}

/// Restore durable guards after coins but before replay, without inventing transaction records.
///
/// All funds accounts must already exist; reapply after adding another account.
/// Known claimants must be actual spending transaction IDs, not sweep winner stamps.
/// Returns an error before mutation when rows disagree about a claimant.
pub fn restore_spent_claims(
    info: &mut ManagedWalletInfo,
    claims: &[(OutPoint, Option<Txid>)],
) -> Result<(), CoreRestoreError> {
    validate_claims(claims)?;
    let spent: std::collections::HashSet<_> =
        claims.iter().map(|(outpoint, _)| *outpoint).collect();
    for account in info.accounts.all_funding_accounts_mut() {
        account
            .utxos
            .retain(|outpoint, _| !spent.contains(outpoint));
    }
    info.restore_spent_outpoints(claims);
    info.update_balance();
    Ok(())
}

fn validate_claims(claims: &[(OutPoint, Option<Txid>)]) -> Result<(), CoreRestoreError> {
    let mut seen = BTreeMap::new();
    for (outpoint, claimant) in claims {
        if seen
            .insert(*outpoint, *claimant)
            .is_some_and(|previous| previous != *claimant)
        {
            return Err(CoreRestoreError::ConflictingSpendClaims(*outpoint));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use dashcore::hashes::Hash;
    use key_wallet::test_utils::TestWalletContext;

    #[tokio::test]
    async fn should_exclude_claimed_coins_and_reject_ambiguous_claims_before_mutation() {
        let (ctx, funding) = TestWalletContext::new_random()
            .with_mempool_funding(100_000)
            .await;
        let coin = ctx
            .managed_wallet
            .accounts
            .all_funding_accounts()
            .into_iter()
            .find_map(|a| {
                a.utxos
                    .values()
                    .next()
                    .map(|coin| (a.managed_account_type().to_account_type(), coin.clone()))
            })
            .expect("funded account");
        let outpoint = OutPoint::new(funding.txid(), 0);
        let mut info = ManagedWalletInfo::from_wallet(&ctx.wallet, 0);
        let original_height = info.metadata.synced_height;
        let result = restore_core_wallet(
            &mut info,
            PersistedCoreState {
                synced_height: Some(100),
                utxos: vec![coin.clone()],
                spent_claims: vec![
                    (outpoint, None),
                    (outpoint, Some(Txid::from_byte_array([7; 32]))),
                ],
                ..Default::default()
            },
        );
        assert!(matches!(
            result,
            Err(CoreRestoreError::ConflictingSpendClaims(_))
        ));
        assert_eq!(info.metadata.synced_height, original_height);
        assert!(info
            .accounts
            .all_funding_accounts()
            .iter()
            .all(|a| a.utxos.is_empty()));
        let mut missing_coin = coin.clone();
        missing_coin.0 = AccountType::CoinJoin { index: 999 };
        assert!(matches!(
            restore_core_wallet(
                &mut info,
                PersistedCoreState {
                    synced_height: Some(100),
                    utxos: vec![coin.clone(), missing_coin],
                    ..Default::default()
                }
            ),
            Err(CoreRestoreError::MissingFundsAccount(_))
        ));
        assert_eq!(info.metadata.synced_height, original_height);
        assert!(info
            .accounts
            .all_funding_accounts()
            .iter()
            .all(|a| a.utxos.is_empty()));
        restore_core_wallet(
            &mut info,
            PersistedCoreState {
                utxos: vec![coin],
                spent_claims: vec![(outpoint, None)],
                ..Default::default()
            },
        )
        .unwrap();
        assert!(info
            .accounts
            .all_funding_accounts()
            .iter()
            .all(|a| a.utxos.is_empty()));
        assert_eq!(info.balance.total(), 0);
    }
}
