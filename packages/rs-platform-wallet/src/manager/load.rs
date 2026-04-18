//! Hydrate a [`PlatformWalletManager`] from its persister.

use std::collections::BTreeMap;
use std::sync::Arc;

use crate::changeset::{
    ClientStartState, ClientWalletStartState, PlatformAddressSyncStartState,
    PlatformWalletPersistence,
};
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
            let platform_info = PlatformWalletInfo {
                core_wallet: wallet_info,
                balance: Arc::clone(&balance),
                identity_manager: IdentityManager::from(identity_manager),
                tracked_asset_locks,
                token_watched: BTreeMap::new(),
                token_balances: BTreeMap::new(),
            };

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
                return Err(PlatformWalletError::WalletCreation(format!(
                    "Persisted wallet id {} does not match recomputed id {}",
                    hex::encode(expected_wallet_id),
                    hex::encode(wallet_id)
                )));
            }

            let broadcaster = Arc::new(crate::broadcaster::SpvBroadcaster::new(Arc::clone(
                &self.spv,
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
            // carried a per-wallet slice, peel it off and pass a
            // single-wallet [`PlatformAddressSyncStartState`] down;
            // otherwise do a fresh scan from the live wallet manager.
            let slice = platform_addresses
                .as_mut()
                .and_then(|pa| pa.per_wallet.remove(&wallet_id));
            if let Some(per_wallet_entry) = slice {
                let (sync_height, sync_timestamp, last_known_recent_block) = platform_addresses
                    .as_ref()
                    .map(|pa| {
                        (
                            pa.sync_height,
                            pa.sync_timestamp,
                            pa.last_known_recent_block,
                        )
                    })
                    .unwrap_or((0, 0, 0));
                let mut per_wallet = BTreeMap::new();
                per_wallet.insert(wallet_id, per_wallet_entry);
                let persisted = PlatformAddressSyncStartState {
                    per_wallet,
                    sync_height,
                    sync_timestamp,
                    last_known_recent_block,
                };
                platform_wallet
                    .platform()
                    .initialize_from_persisted(persisted)
                    .await
                    .map_err(|e| {
                        PlatformWalletError::WalletCreation(format!(
                            "Failed to restore platform address state: {}",
                            e
                        ))
                    })?;
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
