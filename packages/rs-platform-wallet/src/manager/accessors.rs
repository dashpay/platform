//! Read-only accessors on [`PlatformWalletManager`].

use std::sync::Arc;

use key_wallet::account::AccountType;
use key_wallet::WalletCoreBalance;

use crate::changeset::PlatformWalletPersistence;
use crate::manager::identity_sync::IdentitySyncManager;
use crate::manager::platform_address_sync::PlatformAddressSyncManager;
use crate::spv::SpvRuntime;
use crate::wallet::platform_wallet::WalletId;
use crate::wallet::PlatformWallet;

use super::PlatformWalletManager;

impl<P: PlatformWalletPersistence + 'static> PlatformWalletManager<P> {
    /// The SDK instance.
    pub fn sdk(&self) -> &dash_sdk::Sdk {
        &self.sdk
    }

    /// Access the SPV runtime for sync control.
    pub fn spv(&self) -> &SpvRuntime {
        &self.spv_manager
    }

    /// Clone the `Arc<SpvRuntime>` so callers (e.g. FFI) can invoke
    /// [`SpvRuntime::spawn_in_background`] which takes `&Arc<Self>`.
    pub fn spv_arc(&self) -> Arc<SpvRuntime> {
        Arc::clone(&self.spv_manager)
    }

    /// Access the platform-address sync coordinator.
    pub fn platform_address_sync(&self) -> &PlatformAddressSyncManager {
        &self.platform_address_sync_manager
    }

    /// Clone the `Arc<PlatformAddressSyncManager>` so callers (e.g. FFI)
    /// can invoke [`PlatformAddressSyncManager::start`] which takes
    /// `&Arc<Self>`.
    pub fn platform_address_sync_arc(&self) -> Arc<PlatformAddressSyncManager> {
        Arc::clone(&self.platform_address_sync_manager)
    }

    /// Access the per-identity token state sync coordinator.
    pub fn identity_sync(&self) -> &IdentitySyncManager<P> {
        &self.identity_sync_manager
    }

    /// Clone the `Arc<IdentitySyncManager<P>>` so callers (e.g. FFI)
    /// can invoke [`IdentitySyncManager::start`] which takes
    /// `&Arc<Self>`.
    pub fn identity_sync_arc(&self) -> Arc<IdentitySyncManager<P>> {
        Arc::clone(&self.identity_sync_manager)
    }

    /// Get a clone of a wallet by its ID.
    pub async fn get_wallet(&self, wallet_id: &WalletId) -> Option<Arc<PlatformWallet>> {
        let wallets = self.wallets.read().await;
        wallets.get(wallet_id).cloned()
    }

    /// List all wallet IDs.
    pub async fn wallet_ids(&self) -> Vec<WalletId> {
        let wallets = self.wallets.read().await;
        wallets.keys().copied().collect()
    }

    /// Read per-account balance snapshots for a wallet.
    ///
    /// Returns the current `WalletCoreBalance` for every account in the
    /// wallet's `ManagedAccountCollection`. Each entry's balance is the
    /// live in-memory value maintained by `update_balance()` during SPV
    /// processing — no disk I/O. Uses `blocking_read` on the wallet
    /// manager lock; safe from non-async FFI context but must NOT be
    /// called from within a tokio async task.
    pub fn account_balances_blocking(
        &self,
        wallet_id: &WalletId,
    ) -> Vec<(AccountType, WalletCoreBalance)> {
        let wm = self.wallet_manager.blocking_read();
        let Some(info) = wm.get_wallet_info(wallet_id) else {
            return Vec::new();
        };
        info.core_wallet
            .accounts
            .all_accounts()
            .iter()
            .filter(|account| {
                matches!(
                    account.managed_account_type.to_account_type(),
                    AccountType::Standard { .. }
                        | AccountType::CoinJoin { .. }
                        | AccountType::IdentityTopUp { .. }
                        | AccountType::DashpayReceivingFunds { .. }
                        | AccountType::DashpayExternalAccount { .. }
                        | AccountType::PlatformPayment { .. }
                )
            })
            .map(|account| {
                (
                    account.managed_account_type.to_account_type(),
                    account.balance,
                )
            })
            .collect()
    }
}
