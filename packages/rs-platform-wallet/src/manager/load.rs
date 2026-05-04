//! Hydrate a [`PlatformWalletManager`] from its persister.

use std::collections::BTreeMap;
use std::sync::Arc;

use crate::changeset::{ClientStartState, ClientWalletStartState, PlatformWalletPersistence};
use crate::error::PlatformWalletError;
use crate::wallet::core::WalletBalance;
use crate::wallet::identity::IdentityManager;
use crate::wallet::platform_wallet::PlatformWalletInfo;
use crate::wallet::PlatformWallet;

use super::PlatformWalletManager;

impl<P: PlatformWalletPersistence + 'static> PlatformWalletManager<P> {
    /// Load the full [`ClientStartState`] from the configured persister
    /// and rehydrate the manager's `wallet_manager` and `wallets` maps.
    ///
    /// For each persisted wallet this builds a `PlatformWalletInfo` from
    /// the snapshot (core wallet info, identity manager, tracked asset
    /// locks) and inserts the `(Wallet, PlatformWalletInfo)` pair into
    /// the inner [`WalletManager`]. A matching [`PlatformWallet`] handle
    /// is then constructed and registered in `self.wallets`.
    ///
    /// If the snapshot includes platform-address provider state, each
    /// per-wallet slice is handed to
    /// [`PlatformAddressWallet::initialize_from_persisted`](crate::wallet::platform_addresses::PlatformAddressWallet::initialize_from_persisted);
    /// wallets missing from that slice get a fresh
    /// [`PlatformAddressWallet::initialize`](crate::wallet::platform_addresses::PlatformAddressWallet::initialize).
    ///
    /// [`WalletManager`]: key_wallet_manager::WalletManager
    pub async fn load_from_persistor(&self) -> Result<(), PlatformWalletError> {
        let ClientStartState {
            mut platform_addresses,
            wallets,
        } = self.persister.load().map_err(|e| {
            PlatformWalletError::WalletCreation(format!(
                "Failed to load persisted client state: {}",
                e
            ))
        })?;

        let persister_dyn: Arc<dyn PlatformWalletPersistence> = Arc::clone(&self.persister) as _;

        for (expected_wallet_id, wallet_state) in wallets {
            let ClientWalletStartState {
                wallet,
                wallet_info,
                identity_manager,
                unused_asset_locks,
            } = wallet_state;

            // Flatten the (account → outpoint → lock) map into the flat
            // OutPoint → TrackedAssetLock map that `PlatformWalletInfo`
            // holds today.
            let mut tracked_asset_locks = BTreeMap::new();
            for (_account_index, account_locks) in unused_asset_locks {
                tracked_asset_locks.extend(account_locks);
            }

            let balance = Arc::new(WalletBalance::new());
            // Mirror the inner `ManagedWalletInfo.balance` (already
            // recomputed from the freshly-loaded UTXO set on the FFI
            // side via `update_balance`) into the lock-free `Arc` the
            // UI reads. Without this, `wallet.balance()` reports zero
            // for restored wallets even though the per-account totals
            // and the inner `core_wallet.balance` are correct.
            // `WalletBalance::set` is `pub(crate)`, which is why this
            // step has to live inside `platform_wallet` rather than
            // the FFI loader.
            let core_balance = &wallet_info.balance;
            balance.set(
                core_balance.confirmed(),
                core_balance.unconfirmed(),
                core_balance.immature(),
                core_balance.locked(),
            );
            let platform_info = PlatformWalletInfo {
                core_wallet: wallet_info,
                balance: Arc::clone(&balance),
                identity_manager: IdentityManager::from(identity_manager),
                tracked_asset_locks,
            };

            // Insert into `wallet_manager` first so we have a wallet
            // handle to validate against, then either keep the
            // registration or roll it back. The two failure modes
            // below — recomputed id mismatch and platform-address
            // restore — used to leave the wallet half-registered:
            // present in `wallet_manager` but absent from
            // `self.wallets`, which broke the manager's invariant
            // that the two collections describe the same set and
            // poisoned any retry path.
            let wallet_id = {
                let mut wm = self.wallet_manager.write().await;
                wm.insert_wallet(wallet, platform_info).map_err(|e| {
                    PlatformWalletError::WalletCreation(format!(
                        "Failed to register persisted wallet in WalletManager: {}",
                        e
                    ))
                })?
            };

            if wallet_id != expected_wallet_id {
                // Roll back the insert before bailing — the wallet
                // we just registered isn't the one the snapshot
                // claimed it was, and leaving it in `wallet_manager`
                // would collide on the next retry.
                let mut wm = self.wallet_manager.write().await;
                let _ = wm.remove_wallet(&wallet_id);
                return Err(PlatformWalletError::WalletCreation(format!(
                    "Persisted wallet id {} does not match recomputed id {}",
                    hex::encode(expected_wallet_id),
                    hex::encode(wallet_id)
                )));
            }

            let broadcaster = Arc::new(crate::broadcaster::SpvBroadcaster::new(Arc::clone(
                &self.spv_manager,
            )));
            let platform_wallet = PlatformWallet::new(
                Arc::clone(&self.sdk),
                wallet_id,
                Arc::clone(&self.wallet_manager),
                balance,
                Arc::clone(&self.lock_notify),
                Arc::clone(&persister_dyn),
                broadcaster,
            );

            // Initialize the platform-address provider. If the snapshot
            // carried a slice for this wallet, restore it directly;
            // otherwise do a fresh scan from the live wallet manager.
            // Roll back the `insert_wallet` on failure so the caller
            // can retry without stepping over a stale registration.
            if let Some(persisted) = platform_addresses.remove(&wallet_id) {
                if let Err(e) = platform_wallet
                    .platform()
                    .initialize_from_persisted(persisted)
                    .await
                {
                    let mut wm = self.wallet_manager.write().await;
                    let _ = wm.remove_wallet(&wallet_id);
                    return Err(PlatformWalletError::WalletCreation(format!(
                        "Failed to restore platform address state: {}",
                        e
                    )));
                }
            } else {
                platform_wallet.platform().initialize().await;
            }

            let platform_wallet = Arc::new(platform_wallet);
            let mut wallets_guard = self.wallets.write().await;
            wallets_guard.insert(wallet_id, platform_wallet);
        }

        Ok(())
    }
}
