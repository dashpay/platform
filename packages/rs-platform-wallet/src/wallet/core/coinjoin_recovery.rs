//! One-time CoinJoin recovery: widen a CoinJoin account's address gap limit
//! and eagerly generate the addresses so SPV watches the wider window.
//!
//! CoinJoin "mixed coins" are scattered across the CoinJoin derivation path
//! (BIP44 purpose 4') with holes wider than the SDK's default gap limit (30),
//! so a fresh post-migration scan would silently miss deep coins. For wallets
//! flagged (by the app) as having used CoinJoin in DashSync, the app widens the
//! gap — matching DashSync's `SEQUENCE_GAP_LIMIT_INITIAL_COINJOIN` of 400 —
//! before starting SPV, runs the recovery scan, then reverts to the default.

use key_wallet::gap_limit::MAX_GAP_LIMIT;
use key_wallet::managed_account::address_pool::KeySource;
use key_wallet::managed_account::managed_account_trait::ManagedAccountTrait;

use crate::broadcaster::TransactionBroadcaster;
use crate::{CoreWallet, PlatformWalletError};

impl<B: TransactionBroadcaster + ?Sized> CoreWallet<B> {
    /// Widen the CoinJoin account's single-pool gap limit to `gap_limit` and
    /// generate addresses so indices up to `highest_used + gap_limit` exist
    /// and are watched by SPV. Returns the pool's highest generated index.
    ///
    /// Setting the gap limit alone is **not** enough: SPV only watches
    /// addresses materialized into the pool's script index, so a scan starting
    /// at the default gap (30) never sees — and so never extends toward — a
    /// used CoinJoin address sitting beyond index 30. Pre-generating the wide
    /// window (via [`AddressPool::maintain_gap_limit`]) is what lets the
    /// recovery scan find scattered mixed coins; the in-progress scan then
    /// auto-extends from there as deep addresses are marked used (see
    /// `wallet_checker`'s `maintain_gap_limit` call).
    ///
    /// Idempotent: generation only fills missing indices, so re-running with
    /// the same (or a smaller) limit is a cheap no-op. The widened `gap_limit`
    /// is in-memory only — it is not persisted, so a later wallet load
    /// reconstructs the pool at the default gap. That is the intended
    /// "revert after recovery" behavior: the app re-applies the widen on each
    /// flagged launch and stops once the coins are swept.
    ///
    /// Derivation uses the account's **public** xpub only — no private key
    /// crosses any boundary (CoinJoin receive addresses are non-hardened).
    pub async fn set_coinjoin_gap_limit(
        &self,
        account_index: u32,
        gap_limit: u32,
    ) -> Result<u32, PlatformWalletError> {
        let mut wm = self.wallet_manager.write().await;
        let (wallet, info) = wm.get_wallet_and_info_mut(&self.wallet_id).ok_or_else(|| {
            PlatformWalletError::WalletNotFound(
                "Wallet not found in wallet manager".to_string(),
            )
        })?;

        // Watch-only account xpub for deriving the CoinJoin pool's addresses.
        // Copied out (it's `Copy`) so the immutable `wallet` borrow ends before
        // we take the mutable `info` borrow below.
        let account_xpub = wallet
            .accounts
            .coinjoin_accounts
            .get(&account_index)
            .map(|a| a.account_xpub)
            .ok_or_else(|| {
                PlatformWalletError::WalletNotFound(format!(
                    "CoinJoin account {} not found",
                    account_index
                ))
            })?;
        let key_source = KeySource::Public(account_xpub);

        let managed_account = info
            .core_wallet
            .accounts
            .coinjoin_accounts
            .get_mut(&account_index)
            .ok_or_else(|| {
                PlatformWalletError::WalletNotFound(format!(
                    "CoinJoin managed account {} not found",
                    account_index
                ))
            })?;

        // CoinJoin uses a single address pool. Widen the gap limit, then
        // materialize addresses up to it. Scope the pool borrow so it ends
        // before `bump_monitor_revision` reborrows the managed account.
        let new_highest = {
            let pool = managed_account
                .managed_account_type_mut()
                .address_pools_mut()
                .into_iter()
                .next()
                .ok_or_else(|| {
                    PlatformWalletError::AddressOperation(
                        "CoinJoin account has no address pool".to_string(),
                    )
                })?;

            // Reject a zero gap limit before touching the pool. key-wallet's
            // `maintain_gap_limit` computes `gap_limit - 1` when no address has
            // been used yet, which underflows at 0 (debug panic / release wrap
            // to u32::MAX → ~4B address generations under the write lock).
            let gap_limit = gap_limit.min(MAX_GAP_LIMIT);
            if gap_limit == 0 {
                return Err(PlatformWalletError::AddressOperation(
                    "CoinJoin gap limit must be greater than zero".to_string(),
                ));
            }
            pool.gap_limit = gap_limit;
            pool.maintain_gap_limit(&key_source)
                .map_err(|e| PlatformWalletError::AddressOperation(e.to_string()))?;
            pool.highest_generated.unwrap_or(0)
        };

        // The watched-address set changed — bump the revision so the SPV
        // compact-filter / bloom filter is rebuilt to include the new scripts.
        managed_account.bump_monitor_revision();

        Ok(new_highest)
    }
}
