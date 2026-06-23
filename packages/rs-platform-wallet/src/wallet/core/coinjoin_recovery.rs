//! One-time CoinJoin recovery: widen a CoinJoin account's address gap limit
//! and eagerly generate the addresses so SPV watches the wider window.
//!
//! CoinJoin "mixed coins" are scattered across the CoinJoin derivation path
//! (BIP44 purpose 4') with holes wider than the SDK's default gap limit (30),
//! so a fresh post-migration scan would silently miss deep coins. For wallets
//! flagged (by the app) as having used CoinJoin in DashSync, the app widens the
//! gap — matching DashSync's `SEQUENCE_GAP_LIMIT_INITIAL_COINJOIN` of 400 —
//! before starting SPV, runs the recovery scan, then reverts to the default.
//!
//! The actual gap-limit widening + address materialization lives upstream in
//! key-wallet
//! ([`ManagedCoreFundsAccount::set_coinjoin_gap_limit`](key_wallet::managed_account::ManagedCoreFundsAccount::set_coinjoin_gap_limit));
//! this wrapper only resolves the account under the wallet lock and delegates.
//!
//! ## Scope: external CoinJoin chain only
//!
//! This widens only the CoinJoin account's external pool (`.../0/i`). DashSync
//! CoinJoin also puts mixing *change* on the internal chain (`.../1/i`), but the
//! SDK's CoinJoin account models only the external chain, and a migrated wallet's
//! internal-chain mixed coins arrive as **imported spendable UTXOs** — not via an
//! SPV address scan — so no gap widening is needed to discover them. The sweep
//! still signs them regardless of chain: key-wallet's `coinjoin_sweep` resolver
//! re-derives both `/0/` and `/1/` from the account xpub. If a future migration
//! path instead relied on SPV to *re-discover* internal-chain coins, this
//! recovery would also need to materialize the internal pool.

use crate::broadcaster::TransactionBroadcaster;
use crate::{CoreWallet, PlatformWalletError};

impl<B: TransactionBroadcaster + ?Sized> CoreWallet<B> {
    /// Widen the CoinJoin account's single-pool gap limit to `gap_limit` and
    /// generate addresses so SPV watches the wider window. Returns the pool's
    /// highest generated index.
    ///
    /// Resolves the account's watch-only public xpub and managed account under
    /// the wallet write lock, then delegates the widening + address
    /// materialization + monitor-revision bump to
    /// [`ManagedCoreFundsAccount::set_coinjoin_gap_limit`](key_wallet::managed_account::ManagedCoreFundsAccount::set_coinjoin_gap_limit),
    /// which rejects a zero gap limit and clamps to `MAX_GAP_LIMIT`. Derivation
    /// uses the account's public xpub only — no private key crosses any boundary.
    pub async fn set_coinjoin_gap_limit(
        &self,
        account_index: u32,
        gap_limit: u32,
    ) -> Result<u32, PlatformWalletError> {
        let mut wm = self.wallet_manager.write().await;
        let (wallet, info) = wm.get_wallet_and_info_mut(&self.wallet_id).ok_or_else(|| {
            PlatformWalletError::WalletNotFound("Wallet not found in wallet manager".to_string())
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
                    "CoinJoin account {account_index} not found"
                ))
            })?;

        let managed_account = info
            .core_wallet
            .accounts
            .coinjoin_accounts
            .get_mut(&account_index)
            .ok_or_else(|| {
                PlatformWalletError::WalletNotFound(format!(
                    "CoinJoin managed account {account_index} not found"
                ))
            })?;

        managed_account
            .set_coinjoin_gap_limit(account_xpub, gap_limit)
            .map_err(|e| PlatformWalletError::AddressOperation(e.to_string()))
    }
}
