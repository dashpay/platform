//! Platform address wallet for DIP-17 platform payment addresses.

use std::collections::BTreeMap;
use std::sync::Arc;
use tokio::sync::Mutex;

use dpp::address_funds::PlatformAddress;
use dpp::fee::Credits;
use tokio::sync::RwLock;

use crate::wallet::platform_wallet::{PlatformWalletInfo, WalletId};
use key_wallet_manager::WalletManager;

use super::provider::PlatformPaymentAddressAccountProvider;

/// Platform address wallet providing DIP-17 platform payment address functionality.
#[derive(Clone)]
pub struct PlatformAddressWallet {
    pub(crate) sdk: Arc<dash_sdk::Sdk>,
    /// The shared wallet manager lock for all mutable wallet state.
    pub(crate) wallet_manager: Arc<RwLock<WalletManager<PlatformWalletInfo>>>,
    /// Identifies which wallet within the manager this sub-wallet operates on.
    pub(crate) wallet_id: WalletId,
    /// Per-account address providers, retaining incremental sync state across calls.
    pub(crate) providers: Arc<Mutex<BTreeMap<u32, PlatformPaymentAddressAccountProvider>>>,
}

impl PlatformAddressWallet {
    /// Create a new PlatformAddressWallet, initializing a provider for each
    /// existing platform payment account in the wallet.
    pub(crate) fn new(
        sdk: Arc<dash_sdk::Sdk>,
        wallet_manager: Arc<RwLock<WalletManager<PlatformWalletInfo>>>,
        wallet_id: WalletId,
    ) -> Self {
        // Collect account indices under a short-lived read lock.
        let account_indices: Vec<u32> = {
            let wm = wallet_manager.blocking_read();
            wm.get_wallet_info(&wallet_id)
                .map(|info| {
                    info.core_wallet
                        .accounts
                        .platform_payment_accounts
                        .keys()
                        .map(|k| k.account)
                        .collect()
                })
                .unwrap_or_default()
        };

        // Create a provider for each account (from_wallet acquires its own lock).
        let mut providers = BTreeMap::new();
        for account_index in account_indices {
            match PlatformPaymentAddressAccountProvider::from_wallet(
                wallet_manager.clone(),
                wallet_id,
                account_index,
            ) {
                Ok(provider) => {
                    providers.insert(account_index, provider);
                }
                Err(e) => {
                    tracing::warn!(
                        "Failed to create provider for account {}: {}",
                        account_index,
                        e
                    );
                }
            }
        }

        Self {
            sdk,
            wallet_manager,
            wallet_id,
            providers: Arc::new(Mutex::new(providers)),
        }
    }

    /// Get the network from the SDK.
    pub fn network(&self) -> key_wallet::Network {
        self.sdk.network
    }
}

impl PlatformAddressWallet {
    /// Get all platform addresses with their cached balances.
    ///
    /// Returns the balances from the last call to [`sync_balances`](Self::sync_balances),
    /// [`transfer`](Self::transfer), or [`withdraw`](Self::withdraw).
    pub async fn addresses_with_balances(&self) -> Vec<(PlatformAddress, Credits)> {
        let wm = self.wallet_manager.read().await;
        wm.get_wallet_info(&self.wallet_id)
            .and_then(|info| info.core_wallet.first_platform_payment_managed_account())
            .map(|account| {
                account
                    .address_balances
                    .iter()
                    .map(|(p2pkh, &bal)| (PlatformAddress::P2pkh(p2pkh.to_bytes()), bal))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get total platform credits across all addresses.
    ///
    /// Returns the sum of all cached balances.
    pub async fn total_credits(&self) -> Credits {
        let wm = self.wallet_manager.read().await;
        wm.get_wallet_info(&self.wallet_id)
            .and_then(|info| info.core_wallet.first_platform_payment_managed_account())
            .map(|account| account.total_credit_balance())
            .unwrap_or(0)
    }
}

impl std::fmt::Debug for PlatformAddressWallet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PlatformAddressWallet")
            .field("network", &self.sdk.network)
            .finish()
    }
}
