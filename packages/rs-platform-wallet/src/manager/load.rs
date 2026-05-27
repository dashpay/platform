//! Hydrate a [`PlatformWalletManager`] from its persister.

use std::collections::BTreeMap;
use std::sync::Arc;

#[cfg(feature = "shielded")]
use crate::changeset::ShieldedSyncStartState;
use crate::changeset::{
    ClientStartState, ClientWalletStartState, PlatformAddressSyncStartState,
    PlatformWalletPersistence,
};
use crate::error::PlatformWalletError;
use crate::wallet::core::WalletBalance;
use crate::wallet::identity::IdentityManager;
use crate::wallet::platform_wallet::{PlatformWalletInfo, WalletId};
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
            platform_addresses,
            wallets,
            #[cfg(feature = "shielded")]
            shielded,
        } = self.persister.load().map_err(|e| {
            PlatformWalletError::WalletCreation(format!(
                "Failed to load persisted client state: {}",
                e
            ))
        })?;

        let orphan_count = platform_addresses.len();
        let wallets_empty = wallets.is_empty();

        // Stash the platform-address + shielded slices in the cache so
        // any later `register_wallet` / `bind_shielded` calls drain
        // from there instead of re-issuing `persister.load()` per
        // wallet (CODE-017). Done BEFORE the CODE-001 gate so even the
        // refusal path leaves the cache populated — the host's
        // per-wallet `register_wallet` fallback then runs at the
        // already-cached zero-load cost.
        *self.persisted_addresses.write().await = Some(platform_addresses);
        #[cfg(feature = "shielded")]
        {
            *self.persisted_shielded.write().await = Some(Arc::new(shielded));
        }

        // Refuse to silently drop persisted platform-address slices
        // when the persister returned `wallets={}` despite having
        // populated `platform_addresses`. That shape is the contract
        // signature of a persister whose `wallets` rehydration is
        // unimplemented (`LOAD_UNIMPLEMENTED = &["ClientStartState::wallets"]`
        // on `SqlitePersister` as of #3625; the rehydration ships in
        // #3692). Without this gate the loop below executes zero
        // iterations and the cached slices are never consumed.
        // Host falls back to per-wallet `register_wallet` (which now
        // drains the cache populated above).
        if wallets_empty && orphan_count > 0 {
            return Err(PlatformWalletError::PersistorMissingWalletRehydration {
                unimplemented: vec!["ClientStartState::wallets".to_string()],
                orphan_addresses_count: orphan_count,
            });
        }

        let persister_dyn: Arc<dyn PlatformWalletPersistence> = Arc::clone(&self.persister) as _;

        // Track every wallet successfully inserted into
        // `wallet_manager` and `self.wallets` during this call so the
        // batch is transactional: if any later iteration fails (id
        // mismatch, `initialize_from_persisted` error), we walk back
        // every prior insert before bailing. Without this, a clean
        // retry would collide on `WalletManager::insert_wallet`
        // returning `WalletAlreadyExists` for every previously-loaded
        // wallet — half-poisoning the manager until the process
        // restarts. The orphan state is observable across the FFI
        // boundary with no Swift-side reset path, so transactional
        // semantics matter for this hydration API.
        let mut inserted_in_manager: Vec<WalletId> = Vec::new();
        let mut inserted_in_wallets: Vec<WalletId> = Vec::new();
        let mut load_error: Option<PlatformWalletError> = None;

        'load: for (expected_wallet_id, wallet_state) in wallets {
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
            // handle to validate against. Track success in
            // `inserted_in_manager` so the batch-rollback at the
            // bottom can unwind on any later-iteration failure.
            let wallet_id = {
                let mut wm = self.wallet_manager.write().await;
                match wm.insert_wallet(wallet, platform_info) {
                    Ok(id) => id,
                    Err(e) => {
                        load_error = Some(PlatformWalletError::WalletCreation(format!(
                            "Failed to register persisted wallet in WalletManager: {}",
                            e
                        )));
                        break 'load;
                    }
                }
            };
            inserted_in_manager.push(wallet_id);

            if wallet_id != expected_wallet_id {
                load_error = Some(PlatformWalletError::WalletCreation(format!(
                    "Persisted wallet id {} does not match recomputed id {}",
                    hex::encode(expected_wallet_id),
                    hex::encode(wallet_id)
                )));
                break 'load;
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

            // Initialize the platform-address provider. If the cached
            // snapshot carried a slice for this wallet, restore it
            // directly; otherwise do a fresh scan from the live wallet
            // manager. Failures break to the rollback path below.
            let persisted_slice = self
                .persisted_addresses
                .write()
                .await
                .as_mut()
                .and_then(|m| m.remove(&wallet_id));
            if let Some(persisted) = persisted_slice {
                if let Err(e) = platform_wallet
                    .platform()
                    .initialize_from_persisted(persisted)
                    .await
                {
                    load_error = Some(PlatformWalletError::WalletCreation(format!(
                        "Failed to restore platform address state: {}",
                        e
                    )));
                    break 'load;
                }
            } else {
                platform_wallet.platform().initialize().await;
            }

            let platform_wallet = Arc::new(platform_wallet);
            let mut wallets_guard = self.wallets.write().await;
            wallets_guard.insert(wallet_id, platform_wallet);
            drop(wallets_guard);
            inserted_in_wallets.push(wallet_id);
        }

        if let Some(err) = load_error {
            // Walk back every wallet committed in this call so the
            // manager state matches what it was before. Order:
            // remove from `self.wallets` first (UI surface), then
            // from the inner `wallet_manager`.
            if !inserted_in_wallets.is_empty() {
                let mut wallets_guard = self.wallets.write().await;
                for id in &inserted_in_wallets {
                    wallets_guard.remove(id);
                }
            }
            if !inserted_in_manager.is_empty() {
                let mut wm = self.wallet_manager.write().await;
                for id in &inserted_in_manager {
                    let _ = wm.remove_wallet(id);
                }
            }
            return Err(err);
        }

        Ok(())
    }

    /// Drain this wallet's persisted platform-address slice from the
    /// shared cache, populating the cache via a single
    /// `persister.load()` if it hasn't been populated yet (CODE-017).
    ///
    /// Returns `Ok(None)` when the persister has no slice for this
    /// wallet — caller should fall back to `platform().initialize()`.
    /// The slice is **removed** on return so a subsequent call for
    /// the same wallet drops through to the no-slice branch.
    pub(super) async fn take_persisted_platform_addresses(
        &self,
        wallet_id: &WalletId,
    ) -> Result<Option<PlatformAddressSyncStartState>, PlatformWalletError> {
        self.ensure_persisted_state_loaded().await?;
        Ok(self
            .persisted_addresses
            .write()
            .await
            .as_mut()
            .and_then(|m| m.remove(wallet_id)))
    }

    /// Snapshot of the persisted shielded state, populating the cache
    /// via a single `persister.load()` if needed. The snapshot is
    /// shared (`Arc`) so multiple
    /// [`PlatformWallet::bind_shielded_with_snapshot`] calls reuse the
    /// same allocation; restore is filtered per-wallet at consume time.
    /// Returns `Ok(None)` when no shielded state was persisted.
    ///
    /// Hosts that drive `bind_shielded` themselves (the FFI layer)
    /// should fetch the snapshot here once and pass it through to
    /// every wallet bind so the shielded restore step skips its own
    /// `persister.load()` (CODE-017).
    ///
    /// [`PlatformWallet::bind_shielded_with_snapshot`]:
    ///     crate::wallet::PlatformWallet::bind_shielded_with_snapshot
    #[cfg(feature = "shielded")]
    pub async fn cached_persisted_shielded(
        &self,
    ) -> Result<Option<Arc<ShieldedSyncStartState>>, PlatformWalletError> {
        self.ensure_persisted_state_loaded().await?;
        Ok(self
            .persisted_shielded
            .read()
            .await
            .as_ref()
            .map(Arc::clone))
    }

    /// Drop any persisted slice for `wallet_id` from the address
    /// cache. Called from `remove_wallet` so a future re-registration
    /// of the same id cannot re-apply stale persisted state. The
    /// shielded cache is **not** invalidated per-wallet: it's a shared
    /// snapshot and a re-bind for the new wallet under a fresh
    /// `WalletId` filter is a no-op (restore_for_wallet filters by
    /// wallet_id). CODE-017.
    pub(super) async fn invalidate_persisted_for_wallet(&self, wallet_id: &WalletId) {
        if let Some(map) = self.persisted_addresses.write().await.as_mut() {
            map.remove(wallet_id);
        }
    }

    /// Populate `persisted_addresses` (and `persisted_shielded`) from a
    /// single `persister.load()` call if either cache slot is still
    /// `None`. Idempotent — a second call after population is a cheap
    /// read-lock check.
    async fn ensure_persisted_state_loaded(&self) -> Result<(), PlatformWalletError> {
        // Fast path: cache already populated.
        if self.persisted_addresses.read().await.is_some() {
            return Ok(());
        }
        // Slow path: take write locks and double-check before issuing
        // the load — a concurrent caller may have populated between
        // the read above and the writes here.
        let mut addr_guard = self.persisted_addresses.write().await;
        if addr_guard.is_some() {
            return Ok(());
        }
        let ClientStartState {
            platform_addresses,
            wallets: _,
            #[cfg(feature = "shielded")]
            shielded,
        } = self.persister.load().map_err(|e| {
            PlatformWalletError::WalletCreation(format!(
                "Failed to load persisted client state: {}",
                e
            ))
        })?;
        *addr_guard = Some(platform_addresses);
        #[cfg(feature = "shielded")]
        {
            *self.persisted_shielded.write().await = Some(Arc::new(shielded));
        }
        Ok(())
    }
}
