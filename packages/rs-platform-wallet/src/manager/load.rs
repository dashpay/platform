//! Hydrate a [`PlatformWalletManager`] from its persister.

use std::collections::BTreeMap;
use std::sync::Arc;

use crate::changeset::{ClientStartState, ClientWalletStartState, PlatformWalletPersistence};
use crate::error::PlatformWalletError;
use crate::wallet::core::WalletGeneration;
use crate::wallet::identity::IdentityManager;
use crate::wallet::platform_wallet::{PlatformWalletInfo, WalletId};
use crate::wallet::PlatformWallet;

use super::{wallet_lifecycle::retry_transient, PlatformWalletManager};

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
        let start_state = match retry_transient(|| self.persister.load()).await {
            Ok(state) => state,
            Err(e) => {
                // Preserve the typed source chain (Debug carries the real
                // cause — e.g. a bincode decode failure) instead of flattening
                // it to a Display string, and release the wallet-event adapter
                // so a reconstruct on the same path doesn't hit `AlreadyOpen`
                // masking this error.
                tracing::debug!(error = ?e, "persister load failed during rehydration");
                let report = self.shutdown().await;
                if !report.all_clean() {
                    tracing::warn!(?report, "wallet workers unclean after aborting rehydration");
                }
                return Err(PlatformWalletError::PersisterLoad(e));
            }
        };
        let ClientStartState {
            mut platform_addresses,
            wallets,
            // Shielded restore happens lazily on `bind_shielded`,
            // not here — drop the snapshot at this entry point.
            #[cfg(feature = "shielded")]
                shielded: _,
        } = start_state;

        // Tracked (wallet-independent) masternodes ride the same startup
        // hydration; a failure logs and starts empty rather than failing
        // wallet restore.
        self.load_tracked_masternodes_from_persistence();

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

            let generation = Arc::new(WalletGeneration::new());
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
            generation.set(
                core_balance.confirmed(),
                core_balance.unconfirmed(),
                core_balance.immature(),
                core_balance.locked(),
            );
            let platform_info = PlatformWalletInfo {
                core_wallet: wallet_info,
                generation: Arc::clone(&generation),
                identity_manager: IdentityManager::from(identity_manager),
                tracked_asset_locks,
                dpns_name_states: std::collections::BTreeMap::new(),
            };

            // Canonical id recomputed from the wallet's own key material.
            // Computed up front — before `insert_wallet` consumes `wallet` —
            // so we can both validate it against the persisted map key and
            // detect a wallet that an earlier load already registered.
            let wallet_id = wallet.compute_wallet_id();

            if wallet_id != expected_wallet_id {
                load_error = Some(PlatformWalletError::WalletCreation(format!(
                    "Persisted wallet id {} does not match recomputed id {}",
                    hex::encode(expected_wallet_id),
                    hex::encode(wallet_id)
                )));
                break 'load;
            }

            // Insert into `wallet_manager` first so we have a wallet handle
            // to build the `PlatformWallet` against. Track success in
            // `inserted_in_manager` so the batch-rollback at the bottom can
            // unwind on any later-iteration failure.
            //
            // Idempotent: a client re-activates its per-network manager on
            // every SDK emission (network switch, devnet reconfigure, or a
            // plain StateFlow re-emission), which re-runs this loader against
            // a manager that already holds the wallet. Re-inserting would
            // surface `WalletExists`, and with no manager-reset path across
            // the FFI boundary that error crashes the app on the main thread.
            // A wallet already present was fully hydrated (manager +
            // `self.wallets`) by the earlier call, so skip it. Deliberately
            // do NOT record it in `inserted_in_manager`, so this call's
            // rollback only unwinds inserts this call actually made.
            //
            // The existence check and the insert share one write-lock scope
            // so a concurrent loader can't slip between them (TOCTOU).
            {
                let mut wm = self.wallet_manager.write().await;
                if wm.get_wallet(&wallet_id).is_some() {
                    continue 'load;
                }
                if let Err(e) = wm.insert_wallet(wallet, platform_info) {
                    load_error = Some(PlatformWalletError::WalletCreation(format!(
                        "Failed to register persisted wallet in WalletManager: {}",
                        e
                    )));
                    break 'load;
                }
            }
            inserted_in_manager.push(wallet_id);

            let broadcaster = Arc::new(crate::broadcaster::SpvBroadcaster::new(Arc::clone(
                &self.spv_manager,
            )));
            let platform_wallet = PlatformWallet::new(
                Arc::clone(&self.sdk),
                wallet_id,
                Arc::clone(&self.wallet_manager),
                generation,
                Arc::clone(&self.lock_notify),
                Arc::clone(&persister_dyn),
                broadcaster,
            );

            // Initialize the platform-address provider. If the snapshot
            // carried a slice for this wallet, restore it directly;
            // otherwise do a fresh scan from the live wallet manager.
            // Failures break to the rollback path below.
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
                    if let Err(e) = wm.remove_wallet(id) {
                        tracing::warn!(
                            wallet_id = %hex::encode(id),
                            error = %e,
                            "rollback after load failure: remove_wallet failed"
                        );
                    }
                }
            }
            // Release the wallet-event adapter so a reconstruct on the same
            // persister path doesn't hit `AlreadyOpen` (see the early-return
            // path above).
            let report = self.shutdown().await;
            if !report.all_clean() {
                tracing::warn!(
                    ?report,
                    "wallet workers left unclean after rolling back a failed rehydration"
                );
            }
            return Err(err);
        }

        Ok(())
    }
}

#[cfg(test)]
mod idempotent_load_tests {
    use std::collections::BTreeMap;
    use std::sync::Arc;

    use key_wallet::test_utils::TestWalletContext;
    use key_wallet::wallet::ManagedWalletInfo;
    use key_wallet::Wallet;

    use crate::changeset::{
        ClientStartState, ClientWalletStartState, IdentityManagerStartState, PersistenceError,
        PlatformWalletChangeSet, PlatformWalletPersistence,
    };
    use crate::events::{EventHandler, PlatformEventHandler};
    use crate::wallet::platform_wallet::WalletId;
    use crate::PlatformWalletManager;

    /// Persister whose `load()` returns a single-wallet snapshot rebuilt
    /// fresh on every call — `load_from_persistor` moves `wallets` out of
    /// the returned state, so each hydration needs its own copy. Mirrors a
    /// real device where the same persisted rows are handed back on every
    /// `loadFromPersistor` the app fires (once per SDK re-activation).
    struct SingleWalletPersister {
        wallet: Wallet,
        managed: ManagedWalletInfo,
    }

    impl PlatformWalletPersistence for SingleWalletPersister {
        fn store(
            &self,
            _wallet_id: WalletId,
            _changeset: PlatformWalletChangeSet,
        ) -> Result<(), PersistenceError> {
            Ok(())
        }

        fn flush(&self, _wallet_id: WalletId) -> Result<(), PersistenceError> {
            Ok(())
        }

        fn load(&self) -> Result<ClientStartState, PersistenceError> {
            let wallet_id = self.wallet.compute_wallet_id();
            let mut wallets = BTreeMap::new();
            wallets.insert(
                wallet_id,
                ClientWalletStartState {
                    wallet: self.wallet.clone(),
                    wallet_info: self.managed.clone(),
                    identity_manager: IdentityManagerStartState::default(),
                    unused_asset_locks: BTreeMap::new(),
                },
            );
            Ok(ClientStartState {
                wallets,
                ..Default::default()
            })
        }
    }

    struct NoopEventHandler;
    impl EventHandler for NoopEventHandler {}
    impl PlatformEventHandler for NoopEventHandler {}

    fn make_manager(
        persister: SingleWalletPersister,
    ) -> Arc<PlatformWalletManager<SingleWalletPersister>> {
        let sdk = Arc::new(dash_sdk::SdkBuilder::new_mock().build().expect("mock sdk"));
        let event_handler: Arc<dyn PlatformEventHandler> = Arc::new(NoopEventHandler);
        Arc::new(PlatformWalletManager::new(
            sdk,
            Arc::new(persister),
            event_handler,
        ))
    }

    /// The app re-activates its per-network manager on every SDK emission,
    /// which re-runs `load_from_persistor` against a manager that already
    /// holds the persisted wallet. The second (and every later) call must
    /// be a no-op `Ok(())` — NOT the `WalletExists`-wrapped
    /// `WalletCreation` error that used to crash the app on the main
    /// thread. Exactly one wallet stays registered across the calls.
    #[tokio::test]
    async fn repeated_load_from_persistor_is_idempotent() {
        let ctx = TestWalletContext::new_random();
        let expected_id = ctx.wallet.compute_wallet_id();
        let manager = make_manager(SingleWalletPersister {
            wallet: ctx.wallet,
            managed: ctx.managed_wallet,
        });

        manager
            .load_from_persistor()
            .await
            .expect("first load registers the persisted wallet");
        assert_eq!(
            manager.wallet_ids().await,
            vec![expected_id],
            "first load must register exactly the persisted wallet"
        );

        // Re-hydrating with the wallet already present used to surface
        // `Failed to register persisted wallet in WalletManager: Wallet
        // already exists`. It must now be a silent no-op.
        manager
            .load_from_persistor()
            .await
            .expect("second load must be an idempotent no-op, not an error");
        manager
            .load_from_persistor()
            .await
            .expect("third load must also be idempotent");

        assert_eq!(
            manager.wallet_ids().await,
            vec![expected_id],
            "idempotent reloads must not duplicate or drop the wallet"
        );
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    use super::*;
    use crate::changeset::{PersistenceError, PersistenceErrorKind, PlatformWalletChangeSet};
    use crate::events::{EventHandler, PlatformEventHandler};

    /// Persister whose `load()` always fails — the failure path under test.
    struct FailingLoadPersister;

    impl PlatformWalletPersistence for FailingLoadPersister {
        fn store(
            &self,
            _wallet_id: WalletId,
            _changeset: PlatformWalletChangeSet,
        ) -> Result<(), PersistenceError> {
            Ok(())
        }

        fn flush(&self, _wallet_id: WalletId) -> Result<(), PersistenceError> {
            Ok(())
        }

        fn load(&self) -> Result<ClientStartState, PersistenceError> {
            Err(PersistenceError::backend("simulated load failure"))
        }
    }

    struct TransientOnceLoadPersister {
        load_calls: AtomicUsize,
    }

    impl PlatformWalletPersistence for TransientOnceLoadPersister {
        fn store(
            &self,
            _wallet_id: WalletId,
            _changeset: PlatformWalletChangeSet,
        ) -> Result<(), PersistenceError> {
            Ok(())
        }

        fn flush(&self, _wallet_id: WalletId) -> Result<(), PersistenceError> {
            Ok(())
        }

        fn load(&self) -> Result<ClientStartState, PersistenceError> {
            if self.load_calls.fetch_add(1, Ordering::SeqCst) == 0 {
                return Err(PersistenceError::backend_with_kind(
                    PersistenceErrorKind::Transient,
                    "simulated transient load failure",
                ));
            }
            Ok(ClientStartState::default())
        }
    }

    struct NoopEventHandler;
    impl EventHandler for NoopEventHandler {}
    impl PlatformEventHandler for NoopEventHandler {}

    #[tokio::test]
    async fn transient_load_failure_during_startup_rehydration_is_retried() {
        let persister = Arc::new(TransientOnceLoadPersister {
            load_calls: AtomicUsize::new(0),
        });
        let probe = Arc::clone(&persister);
        let sdk = Arc::new(dash_sdk::SdkBuilder::new_mock().build().expect("mock sdk"));
        let handler: Arc<dyn PlatformEventHandler> = Arc::new(NoopEventHandler);
        let manager = PlatformWalletManager::new(sdk, persister, handler);

        manager
            .load_from_persistor()
            .await
            .expect("transient startup load failure must be retried");

        assert_eq!(probe.load_calls.load(Ordering::SeqCst), 2);
    }

    /// A failed `load_from_persistor` must (a) surface the typed `PersisterLoad`
    /// error preserving the source chain, and (b) release the wallet-event
    /// adapter's `Arc<persister>` clone so a reconstruct on the same path
    /// doesn't hit `WalletStorageError::AlreadyOpen` masking the real error
    /// (issue #4133).
    ///
    /// This is a **manager-side proxy**, not a full end-to-end proof: it asserts
    /// the persister's strong count returns to 1 (the test's own probe) after a
    /// failed load + teardown — a lingering adapter clone would keep it above 1
    /// — which is the necessary precondition for a clean re-open. It does not
    /// itself open a real `SqlitePersister`, fail, and re-open on the same path;
    /// the platform-wallet ⇄ platform-wallet-storage dev-dependency cycle
    /// precludes using the concrete persister here. That end-to-end
    /// open → fail → reopen is covered by the storage crate's own round-trip
    /// coverage test.
    // Multi-thread: dropping the manager runs upstream's `Drop`, whose
    // `ThreadRegistry::shutdown()` asserts a multi-thread runtime.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn failed_load_releases_persister_for_reconstruct() {
        let persister = Arc::new(FailingLoadPersister);
        let probe = Arc::clone(&persister);
        let sdk = Arc::new(dash_sdk::SdkBuilder::new_mock().build().expect("mock sdk"));
        let handler: Arc<dyn PlatformEventHandler> = Arc::new(NoopEventHandler);

        let manager = PlatformWalletManager::new(sdk, persister, handler);

        let err = manager
            .load_from_persistor()
            .await
            .expect_err("load must fail");
        assert!(
            matches!(err, PlatformWalletError::PersisterLoad(_)),
            "load failure must surface as the typed PersisterLoad variant, got {err:?}"
        );

        drop(manager);
        // Asserted directly, never polled: the failure path awaits the
        // adapter's `JoinHandle` inside `shutdown`, so the task's clone is
        // already released before `drop` runs. Release on THIS path is
        // synchronous, which is the stronger guarantee — a poll loop (or a
        // `yield_now`, which cedes nothing to another worker) would only
        // hide a regression into eventual release.
        assert_eq!(
            Arc::strong_count(&probe),
            1,
            "after a failed load + teardown nothing may still hold the persister"
        );
    }

    /// The `Drop` backstop alone (no `shutdown` first) must *eventually* release
    /// the adapter's `Arc<persister>` clone. Unlike the graceful path this is
    /// not synchronous: `Drop::drop` calls `abort()`, which only *requests*
    /// cancellation — the runtime drops the aborted task (and its clone) at its
    /// next poll. So the strong count is polled, not asserted immediately, which
    /// is exactly the "eventual, not synchronous" contract the `Drop` impl's
    /// doc-comment describes. This is the branch the graceful-path test above
    /// never exercises (there `shutdown` has already taken the join handle, so
    /// `Drop`'s `abort` sees `None`).
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn drop_backstop_eventually_releases_persister_without_shutdown() {
        let persister = Arc::new(FailingLoadPersister);
        let probe = Arc::clone(&persister);
        let sdk = Arc::new(dash_sdk::SdkBuilder::new_mock().build().expect("mock sdk"));
        let handler: Arc<dyn PlatformEventHandler> = Arc::new(NoopEventHandler);

        let manager = PlatformWalletManager::new(sdk, persister, handler);
        // The adapter task spawned in `new()` holds a clone, so the count is
        // above the probe before any teardown.
        assert!(
            Arc::strong_count(&probe) > 1,
            "the spawned adapter task must hold an Arc<persister> clone"
        );

        // Dirty drop: never call `shutdown`, so `Drop`'s `abort` is the only
        // thing that can reclaim the adapter's clone.
        drop(manager);

        // Release is eventual: poll until the aborted task is dropped by the
        // runtime rather than asserting immediately. The wait must be a timed
        // sleep, not `yield_now`: the aborted task is reclaimed by whichever
        // worker thread owns it, and yielding this thread never forces that
        // one to run — the whole budget can burn in microseconds while the
        // clone is still live. Breaks on the first observation, so the 2s
        // ceiling is only ever paid by a genuine regression.
        let mut count = Arc::strong_count(&probe);
        for _ in 0..2_000 {
            if count == 1 {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(1)).await;
            count = Arc::strong_count(&probe);
        }
        assert_eq!(
            count, 1,
            "the Drop backstop must eventually release the persister after aborting the adapter"
        );
    }
}
