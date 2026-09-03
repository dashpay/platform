//! Hydrate a [`PlatformWalletManager`] from its persister.

use std::collections::BTreeMap;
use std::sync::Arc;

use crate::changeset::{ClientStartState, ClientWalletStartState, PlatformWalletPersistence};
use crate::error::PlatformWalletError;
use crate::wallet::core::WalletGeneration;
use crate::wallet::identity::IdentityManager;
use crate::wallet::platform_wallet::{PlatformWalletInfo, WalletId};
use crate::wallet::PlatformWallet;

use super::{retry_transient_load, PlatformWalletManager};

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
    /// # Errors
    ///
    /// Returns [`PersisterLoad`](PlatformWalletError::PersisterLoad) when the
    /// persister cannot produce the snapshot, or the per-wallet restore error
    /// when a wallet in it cannot be rebuilt.
    ///
    /// Any `Err` leaves the manager exactly as it was before the call —
    /// partial inserts are rolled back — and it stays usable: fix the store
    /// and call again, or tear it down and reconstruct. Reconstructing over
    /// the same persister path needs the persister released first:
    /// [`shutdown`](Self::shutdown) releases it before returning, and a plain
    /// drop releases it once the last strong reference goes (the wallet-event
    /// adapter holds only a weak one; a batch commit in flight holds a strong
    /// one until it finishes).
    ///
    /// [`WalletManager`]: key_wallet_manager::WalletManager
    pub async fn load_from_persistor(&self) -> Result<(), PlatformWalletError> {
        let persister = Arc::clone(&self.persister);
        let start_state = match retry_transient_load(move || persister.load()).await {
            Ok(state) => state,
            Err(e) => {
                // Debug, not Display: it carries the real cause (e.g. a
                // bincode decode failure) rather than flattening the chain.
                tracing::debug!(error = ?e, "persister load failed during rehydration");
                return Err(PlatformWalletError::from_load_failure(e));
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

            // Canonical id recomputed from the wallet's own key material.
            // Computed up front — before `insert_wallet` consumes `wallet` —
            // so we can both validate it against the persisted map key
            // (below) and key this generation's in-broadcast fence map by it.
            let wallet_id = wallet.compute_wallet_id();

            // The fence map is per WALLET, not per generation
            // (`dashpay/platform#4309`, review round 8). On a first load the
            // registry is empty and this is a fresh map; a re-load — or a load
            // that follows a removal — inherits whatever pending spends the
            // previous generation under this id left standing, rather than
            // handing the restored UTXOs back unprotected.
            //
            // A fresh PROCESS still starts empty: this registry is not durable.
            // See `InBroadcastFences` for what closing that half requires.
            let generation = Arc::new(WalletGeneration::with_fences(
                self.in_broadcast_fences_for(&wallet_id),
            ));
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
                observed_input_conflicts: Default::default(),
                core_wallet: wallet_info,
                generation: Arc::clone(&generation),
                identity_manager: IdentityManager::from(identity_manager),
                tracked_asset_locks,
                dpns_name_states: std::collections::BTreeMap::new(),
            };
            // Seed the double-spend screen's session memory from the
            // freshly restored state: it closes the race where SPV's
            // chainlock dispatcher promotion-evicts a restored spender
            // before the first catch-up resume ever reads it.
            crate::wallet::asset_lock::sync::recovery::seed_observed_input_conflicts(
                &platform_info,
            );

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
    use crate::events::PlatformEventHandler;
    use crate::test_support::NoopTestEventHandler;
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

    fn make_manager(
        persister: SingleWalletPersister,
    ) -> Arc<PlatformWalletManager<SingleWalletPersister>> {
        let sdk = Arc::new(dash_sdk::SdkBuilder::new_mock().build().expect("mock sdk"));
        let event_handler: Arc<dyn PlatformEventHandler> = Arc::new(NoopTestEventHandler);
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

    use dash_async::WorkerStatus;

    use super::*;
    use crate::changeset::{PersistenceError, PersistenceErrorKind, PlatformWalletChangeSet};
    use crate::events::PlatformEventHandler;
    use crate::manager::WalletWorker;
    use crate::test_support::NoopTestEventHandler;

    /// Strong `Arc<P>` clones a freshly built [`PlatformWalletManager`] holds:
    /// its own `persister` field, the `DashPayPaymentHandler` on the event
    /// fan-out, and the `IdentitySyncManager`. The wallet-event adapter is
    /// deliberately absent — it keeps a `Weak<P>` and upgrades per batch.
    const MANAGER_PERSISTER_HOLDERS: usize = 3;

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

    /// `load()` fails permanently once and succeeds from then on — the host
    /// path of "surface the error, fix the store, call again".
    #[derive(Default)]
    struct FatalOnceLoadPersister {
        load_calls: AtomicUsize,
    }

    impl PlatformWalletPersistence for FatalOnceLoadPersister {
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
                return Err(PersistenceError::backend("simulated fatal load failure"));
            }
            Ok(ClientStartState::default())
        }
    }

    fn make_manager<P: PlatformWalletPersistence + 'static>(
        persister: Arc<P>,
    ) -> PlatformWalletManager<P> {
        let sdk = Arc::new(dash_sdk::SdkBuilder::new_mock().build().expect("mock sdk"));
        let handler: Arc<dyn PlatformEventHandler> = Arc::new(NoopTestEventHandler);
        PlatformWalletManager::new(sdk, persister, handler)
    }

    #[tokio::test]
    async fn transient_load_failure_during_startup_rehydration_is_retried() {
        let persister = Arc::new(TransientOnceLoadPersister {
            load_calls: AtomicUsize::new(0),
        });
        let probe = Arc::clone(&persister);
        let manager = make_manager(persister);

        manager
            .load_from_persistor()
            .await
            .expect("transient startup load failure must be retried");

        assert_eq!(probe.load_calls.load(Ordering::SeqCst), 2);
    }

    /// The wallet-event adapter must keep a `Weak<P>`, never a strong clone.
    ///
    /// Isolating by construction: the count is read on a live, idle manager
    /// with nothing dropped, cancelled or aborted, so no teardown path and no
    /// abort timing can stand in for the property. Restoring a strong `Arc<P>`
    /// in `run_wallet_event_adapter` turns it red.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn adapter_holds_no_strong_persister_reference() {
        let persister = Arc::new(FailingLoadPersister);
        let probe = Arc::clone(&persister);
        let _manager = make_manager(persister);

        assert_eq!(
            Arc::strong_count(&probe),
            MANAGER_PERSISTER_HOLDERS + 1,
            "expected exactly {} strong persister references — the manager's \
             own `persister` field, the DashPayPaymentHandler on the event \
             fan-out, the IdentitySyncManager, and this test's probe. The idle \
             wallet-event adapter must not be among them: it holds a Weak<P> \
             and upgrades it per batch",
            MANAGER_PERSISTER_HOLDERS + 1
        );
    }

    /// A failed `load_from_persistor` must leave the manager usable: the host
    /// fixes its store and calls again.
    ///
    /// Both failure paths used to run the manager-wide, one-way `shutdown()`,
    /// which seals every coordinator's admission gate and joins the
    /// wallet-event adapter — so the retry returned `Ok(())` onto a manager
    /// that could never sync or persist again (issue #4133).
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn manager_stays_usable_after_a_failed_load() {
        let manager = make_manager(Arc::new(FatalOnceLoadPersister::default()));

        let err = manager
            .load_from_persistor()
            .await
            .expect_err("the first load must fail");
        assert!(
            matches!(err, PlatformWalletError::PersisterLoad(_)),
            "load failure must surface as the typed PersisterLoad variant, got {err:?}"
        );

        manager
            .load_from_persistor()
            .await
            .expect("a load retried after a failed one must succeed");

        assert!(
            !manager.identity_sync_manager.sync_admission_closed(),
            "a failed load must leave sync admission open — a sealed gate \
             makes every later `Ok(())` a lie"
        );

        // The adapter is the only writer of core wallet events to the
        // persister and its receiver is taken exactly once, so a joined
        // adapter cannot be respawned: `Ok` here means the reused manager
        // still persists.
        let report = manager.shutdown().await;
        assert_eq!(
            report.per_worker.get(&WalletWorker::EventAdapter),
            Some(&WorkerStatus::Ok),
            "the wallet-event adapter must still have been running for \
             shutdown to join it: {report:?}"
        );
    }

    /// End to end: a failed `load_from_persistor` surfaces the typed
    /// `PersisterLoad` error, and dropping the manager afterwards releases the
    /// persister — the precondition for reconstructing on the same path
    /// without a spurious `WalletStorageError::AlreadyOpen` masking the real
    /// error (issue #4133).
    ///
    /// Isolates nothing: the final count is the product of the whole teardown,
    /// so it stays green while any one participant regresses as long as
    /// another still releases. `adapter_holds_no_strong_persister_reference`
    /// is the test that pins the weak adapter reference.
    // TODO: cover the composed open -> failed load -> reopen from
    // platform-wallet-storage; neither side asserts it today.
    // Multi-thread: dropping the manager runs upstream's `Drop`, whose
    // `ThreadRegistry::shutdown()` asserts a multi-thread runtime.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn failed_load_releases_persister_for_reconstruct() {
        let persister = Arc::new(FailingLoadPersister);
        let probe = Arc::clone(&persister);
        let manager = make_manager(persister);

        let err = manager
            .load_from_persistor()
            .await
            .expect_err("load must fail");
        assert!(
            matches!(err, PlatformWalletError::PersisterLoad(_)),
            "load failure must surface as the typed PersisterLoad variant, got {err:?}"
        );
        assert_eq!(
            Arc::strong_count(&probe),
            MANAGER_PERSISTER_HOLDERS + 1,
            "a failed load tears nothing down, so the manager's own references \
             must be exactly as they were before the call"
        );

        drop(manager);
        assert_eq!(
            Arc::strong_count(&probe),
            1,
            "after a failed load and a drop nothing may still hold the persister"
        );
    }

    /// Dropping the manager without `shutdown` releases the persister
    /// **synchronously**: every strong clone lives in the manager's own
    /// fields, and the wallet-event adapter holds only a `Weak<P>`.
    ///
    /// The one bound: a batch commit in flight upgrades that weak reference
    /// for the duration of its `store()`, so a drop racing a commit releases
    /// when that commit returns (`an_in_flight_commit_holds_a_strong_persister_reference`
    /// in `changeset::core_bridge`). The adapter is idle here, so release is
    /// immediate.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn dropping_manager_releases_persister_synchronously_when_adapter_idle() {
        let persister = Arc::new(FailingLoadPersister);
        let probe = Arc::clone(&persister);
        let manager = make_manager(persister);
        assert_eq!(
            Arc::strong_count(&probe),
            MANAGER_PERSISTER_HOLDERS + 1,
            "the manager must hold its persister before the drop for this to \
             mean anything"
        );

        // Dirty drop: `shutdown` is never called, so nothing joins the adapter.
        drop(manager);

        assert_eq!(
            Arc::strong_count(&probe),
            1,
            "dropping the manager must release the persister immediately — an \
             idle adapter holds no strong reference to await"
        );
    }
}
