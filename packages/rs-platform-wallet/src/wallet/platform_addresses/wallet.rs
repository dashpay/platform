//! Platform address wallet for DIP-17 platform payment addresses.

use std::collections::BTreeMap;
use std::sync::Arc;

use arc_swap::ArcSwap;
use dpp::address_funds::PlatformAddress;
use dpp::fee::Credits;
use tokio::sync::RwLock;

use crate::error::PlatformWalletError;
use crate::wallet::platform_wallet::{PlatformWalletInfo, WalletId};
use key_wallet_manager::WalletManager;

use crate::wallet::persister::WalletPersister;

use super::provider::PlatformPaymentAddressAccountProvider;

/// Provider map type alias for readability.
type ProviderMap = BTreeMap<u32, Arc<RwLock<PlatformPaymentAddressAccountProvider>>>;

/// Platform address wallet providing DIP-17 platform payment address functionality.
#[derive(Clone)]
pub struct PlatformAddressWallet {
    pub(crate) sdk: Arc<dash_sdk::Sdk>,
    /// The shared wallet manager lock for all mutable wallet state.
    pub(crate) wallet_manager: Arc<RwLock<WalletManager<PlatformWalletInfo>>>,
    /// Identifies which wallet within the manager this sub-wallet operates on.
    pub(crate) wallet_id: WalletId,
    /// Per-account address providers, retaining incremental sync state across calls.
    /// Each provider has its own `RwLock` so syncing different accounts is independent.
    /// The outer `ArcSwap` allows lock-free reads; writes (adding a new account) are
    /// rare and use clone-and-swap.
    pub(crate) providers: Arc<ArcSwap<ProviderMap>>,
    /// Per-wallet persistence handle for queuing changesets.
    pub(crate) persister: WalletPersister,
}

impl PlatformAddressWallet {
    /// Create a new PlatformAddressWallet without initializing providers.
    ///
    /// Call [`initialize`] afterwards to create providers for existing
    /// platform payment accounts.
    pub(crate) fn new(
        sdk: Arc<dash_sdk::Sdk>,
        wallet_manager: Arc<RwLock<WalletManager<PlatformWalletInfo>>>,
        wallet_id: WalletId,
        persister: WalletPersister,
    ) -> Self {
        Self {
            sdk,
            wallet_manager,
            wallet_id,
            providers: Arc::new(ArcSwap::from_pointee(BTreeMap::new())),
            persister,
        }
    }

    /// Initialize providers for all existing platform payment accounts.
    ///
    /// Creates a [`PlatformPaymentAddressAccountProvider`] for each account
    /// found in the wallet. Safe to call multiple times — existing providers
    /// are preserved (use [`add_provider`] for new accounts).
    pub async fn initialize(&self) {
        let account_indices: Vec<u32> = {
            let wm = self.wallet_manager.read().await;
            wm.get_wallet_info(&self.wallet_id)
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

        let current = self.providers.load();
        let mut new_map = (**current).clone();

        for account_index in account_indices {
            if new_map.contains_key(&account_index) {
                continue; // Already initialized
            }
            match PlatformPaymentAddressAccountProvider::from_wallet(
                self.wallet_manager.clone(),
                self.wallet_id,
                account_index,
            )
            .await
            {
                Ok(provider) => {
                    new_map.insert(account_index, Arc::new(RwLock::new(provider)));
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

        self.providers.store(Arc::new(new_map));
    }

    /// Get the network from the SDK.
    pub fn network(&self) -> key_wallet::Network {
        self.sdk.network
    }

    /// Add a provider for a new account index.
    ///
    /// Returns an error if a provider already exists for this index.
    pub async fn add_provider(&self, account_index: u32) -> Result<(), PlatformWalletError> {
        let current = self.providers.load();
        if current.contains_key(&account_index) {
            return Err(PlatformWalletError::AddressOperation(format!(
                "Provider already exists for account index {}",
                account_index
            )));
        }

        let provider = PlatformPaymentAddressAccountProvider::from_wallet(
            self.wallet_manager.clone(),
            self.wallet_id,
            account_index,
        )
        .await?;

        let mut new_map = (**current).clone();
        new_map.insert(account_index, Arc::new(RwLock::new(provider)));
        self.providers.store(Arc::new(new_map));

        Ok(())
    }

    /// Restore the incremental-sync watermark on every active provider.
    ///
    /// Called during persisted-state replay so the next `sync_balances`
    /// call resumes from where the previous session left off instead of
    /// doing a full rescan. Zero-valued arguments are ignored (they mean
    /// "no stored watermark" — the provider keeps its fresh-start state).
    ///
    /// All accounts are set to the same watermark — platform-wallet
    /// persists a single per-wallet value (the max across accounts on
    /// flush). A subsequent sync will rewind accounts that were ahead
    /// to this floor, so no range can be silently skipped.
    pub(crate) async fn apply_sync_state(&self, height: Option<u64>, timestamp: Option<u64>) {
        if height.is_none() && timestamp.is_none() {
            return;
        }
        let h = height.unwrap_or(0);
        let t = timestamp.unwrap_or(0);
        for provider_lock in self.providers.load().values() {
            let mut provider = provider_lock.write().await;
            provider.set_stored_sync_state(h, t);
        }
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
