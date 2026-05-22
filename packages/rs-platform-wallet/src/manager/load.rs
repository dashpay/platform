//! Hydrate a [`PlatformWalletManager`] from its persister.

use std::collections::BTreeMap;
use std::sync::Arc;

use key_wallet::wallet::managed_wallet_info::ManagedWalletInfo;

use crate::changeset::{ClientStartState, ClientWalletStartState, PlatformWalletPersistence};
use crate::error::PlatformWalletError;
use crate::events::PlatformEvent;
use crate::manager::load_outcome::{LoadOutcome, SkipReason};
use crate::seed_provider::SeedProvider;
use crate::wallet::core::WalletBalance;
use crate::wallet::identity::IdentityManager;
use crate::wallet::platform_wallet::{PlatformWalletInfo, WalletId};
use crate::wallet::PlatformWallet;

use super::PlatformWalletManager;

impl<P: PlatformWalletPersistence + 'static> PlatformWalletManager<P> {
    /// Load every persisted wallet, re-deriving each signing
    /// [`Wallet`](key_wallet::Wallet) from the runtime
    /// [`SeedProvider`](crate::seed_provider::SeedProvider) and
    /// rehydrating the manager's `wallet_manager` and `wallets` maps.
    ///
    /// The persister never holds key material — its `load()` returns a
    /// keyless reconstruction snapshot. For each persisted wallet this
    /// fetches the seed/mnemonic from `seeds`, runs the fail-closed
    /// wrong-seed gate, mints `ManagedWalletInfo`, applies the rebuilt
    /// core state + identities + `Consumed`-filtered asset locks, and
    /// registers it.
    ///
    /// # Skip vs hard-fail
    ///
    /// - **Seed unavailable** (the provider returns
    ///   [`SeedUnavailable`](crate::seed_provider::SeedUnavailable)): the
    ///   wallet is **skipped** — never inserted into `wallet_manager` /
    ///   `self.wallets`, recorded in [`LoadOutcome::skipped`], and a
    ///   [`PlatformEvent::WalletSkippedOnLoad`] is emitted. One
    ///   unavailable seed never aborts the others; the call still
    ///   returns `Ok`.
    /// - **Seed present but wrong** (fails the
    ///   [`rehydrate_wallet`](super::rehydrate::rehydrate_wallet) gate):
    ///   a fail-closed [`PlatformWalletError::WrongSeedForDatabase`] —
    ///   **not** a skip, not in `skipped`, no skip event. Aborts the
    ///   batch (rollback).
    ///
    /// Platform-address provider state is restored per wallet via
    /// [`initialize_from_persisted`](crate::wallet::platform_addresses::PlatformAddressWallet::initialize_from_persisted),
    /// or a fresh
    /// [`initialize`](crate::wallet::platform_addresses::PlatformAddressWallet::initialize)
    /// when the snapshot carries no slice for it.
    pub async fn load_from_persistor(
        &self,
        seeds: &dyn SeedProvider,
    ) -> Result<LoadOutcome, PlatformWalletError> {
        let ClientStartState {
            mut platform_addresses,
            wallets,
            // Shielded restore happens lazily on `bind_shielded`,
            // not here — drop the snapshot at this entry point.
            #[cfg(feature = "shielded")]
                shielded: _,
        } = self.persister.load().map_err(|e| {
            PlatformWalletError::WalletCreation(format!(
                "Failed to load persisted client state: {}",
                e
            ))
        })?;

        let persister_dyn: Arc<dyn PlatformWalletPersistence> = Arc::clone(&self.persister) as _;

        // Transactional batch: every wallet inserted into
        // `wallet_manager` / `self.wallets` is tracked so a later hard
        // error walks back every prior insert. Skipped wallets never
        // enter either map, so the rollback path never sees them.
        let mut inserted_in_manager: Vec<WalletId> = Vec::new();
        let mut inserted_in_wallets: Vec<WalletId> = Vec::new();
        let mut load_error: Option<PlatformWalletError> = None;
        let mut outcome = LoadOutcome::default();

        'load: for (expected_wallet_id, wallet_state) in wallets {
            let ClientWalletStartState {
                network,
                birth_height,
                account_manifest,
                core_state,
                identity_manager,
                unused_asset_locks,
                contacts,
                identity_keys,
            } = wallet_state;

            // Resolve the runtime secret. Seed unavailable ⇒ skip
            // BEFORE any `insert_wallet`: the wallet never enters
            // `wallet_manager` / `self.wallets` (absent, not degraded).
            // A wrong (present-but-mismatched) seed is a hard error
            // from the gate below, NOT a skip.
            let secret = match seeds.seed_for(expected_wallet_id) {
                Ok(s) => s,
                Err(unavailable) => {
                    let reason = SkipReason::from(unavailable);
                    outcome.skipped.push((expected_wallet_id, reason));
                    self.event_manager
                        .on_platform_event(&PlatformEvent::WalletSkippedOnLoad {
                            wallet_id: expected_wallet_id,
                            reason,
                        });
                    continue 'load;
                }
            };

            // Seed present — re-derive + fail-closed wrong-seed gate.
            let wallet = match super::rehydrate::rehydrate_wallet(
                &secret,
                network,
                expected_wallet_id,
                &account_manifest,
            ) {
                Ok(w) => w,
                Err(e) => {
                    load_error = Some(e);
                    break 'load;
                }
            };
            // `secret` is dropped at the end of this iteration —
            // transient mnemonic/seed bytes zeroized; never logged,
            // never in an error.

            // Mint the managed-info skeleton from the re-derived
            // wallet, then apply the keyless persisted core state
            // (UTXOs, sync watermarks, per-account balances). A wallet
            // with persisted UTXOs but no funds account hard-fails here
            // rather than reconstructing a silent zero balance.
            let mut wallet_info = ManagedWalletInfo::from_wallet(&wallet, birth_height);
            if let Err(e) =
                super::rehydrate::apply_persisted_core_state(&mut wallet_info, &core_state)
            {
                load_error = Some(e);
                break 'load;
            }

            // Flatten the (account → outpoint → lock) map.
            let mut tracked_asset_locks = BTreeMap::new();
            for (_account_index, account_locks) in unused_asset_locks {
                tracked_asset_locks.extend(account_locks);
            }

            let balance = Arc::new(WalletBalance::new());
            let core_balance = &wallet_info.balance;
            balance.set(
                core_balance.confirmed(),
                core_balance.unconfirmed(),
                core_balance.immature(),
                core_balance.locked(),
            );
            // Build the identity manager from the (id, balance,
            // revision) skeleton, then layer the persisted PUBLIC
            // contacts + identity keys onto it — the same routing the
            // runtime changeset-replay path uses.
            let mut identity_manager = IdentityManager::from(identity_manager);
            identity_manager.apply_contacts_and_keys(contacts, identity_keys, network);
            let platform_info = PlatformWalletInfo {
                core_wallet: wallet_info,
                balance: Arc::clone(&balance),
                identity_manager,
                tracked_asset_locks,
            };

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

            // No post-insert id re-check: the constant-time
            // `rehydrate_wallet` wrong-seed gate already proved
            // `compute_wallet_id() == expected_wallet_id` before this
            // wallet was built (a mismatch is the typed, fail-closed
            // `WrongSeedForDatabase` raised above).

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

            if let Some(persisted) = platform_addresses.remove(&wallet_id) {
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
            outcome.loaded.push(wallet_id);
        }

        if let Some(err) = load_error {
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

        Ok(outcome)
    }
}
