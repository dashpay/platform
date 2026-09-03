//! Hydrate a [`PlatformWalletManager`] from its persister.

use std::collections::BTreeMap;
use std::sync::Arc;

use crate::changeset::{ClientStartState, ClientWalletStartState, PlatformWalletPersistence};
use crate::error::PlatformWalletError;
use crate::wallet::core::WalletGeneration;
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
        // The generation travels with the id: a rollback may only remove the
        // registration THIS call published (see the rollback block below).
        let mut inserted_in_wallets: Vec<(WalletId, Arc<crate::wallet::core::WalletGeneration>)> =
            Vec::new();
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
            self.wallets.rcu(|wallets| {
                let mut wallets = std::collections::BTreeMap::clone(wallets);
                wallets.insert(wallet_id, Arc::clone(&platform_wallet));
                wallets
            });
            inserted_in_wallets.push((wallet_id, Arc::clone(platform_wallet.generation())));

            // Re-seed the balance atomic now that the wallet is published.
            //
            // The seed above ran before `insert_wallet`, and the wallet
            // becomes SPV-visible the moment that insert lands — several
            // `.await`s before the `rcu` above. Any `BlockProcessed` for it
            // in that window finds the wallet absent from the map and its
            // snapshot is dropped, leaving the atomic at the persisted total
            // while the inner `ManagedWalletInfo` balance has moved on.
            // `register_wallet` closes the same window this way; without it
            // here, a restored wallet whose catch-up completes inside the
            // window keeps a stale total on screen with no later event
            // guaranteed to correct it.
            //
            // Last writer wins between this seed and the handler: if SPV
            // processes another block between the read below and the `set`,
            // the atomic briefly goes back to the older totals. The next
            // balance-bearing event corrects it, and during catch-up those
            // arrive continuously — which is why the seed is worth more than
            // the window it can briefly re-open.
            {
                let wm = self.wallet_manager.read().await;
                if let Some(info) = wm.get_wallet_info(&wallet_id) {
                    let b = &info.core_wallet.balance;
                    platform_wallet.balance().set(
                        b.confirmed(),
                        b.unconfirmed(),
                        b.immature(),
                        b.locked(),
                    );
                }
            }
        }

        if let Some(err) = load_error {
            // Walk back every wallet committed in this call so the
            // manager state matches what it was before. Order:
            // remove from `self.wallets` first (UI surface), then
            // from the inner `wallet_manager`.
            // Generation-checked, exactly like `remove_wallet`'s own removal:
            // a concurrent removal frees an id and a registration can publish
            // a DIFFERENT generation under it before this rollback runs.
            // Removing by id alone would delete that live wallet — one this
            // call never created and whose owner is still using it.
            let rolled_back = std::cell::RefCell::new(Vec::<WalletId>::new());
            if !inserted_in_wallets.is_empty() {
                self.wallets.rcu(|wallets| {
                    // `rcu` may retry, so this is rebuilt per attempt rather
                    // than accumulated across them.
                    let ours = rollback_targets(&inserted_in_wallets, wallets);
                    let mut next = std::collections::BTreeMap::clone(wallets);
                    for id in &ours {
                        next.remove(id);
                    }
                    *rolled_back.borrow_mut() = ours;
                    next
                });
            }
            let rolled_back = rolled_back.into_inner();
            // Wait-free, purely for the diagnostics below: an id still in the
            // map after the rollback is one a same-id re-registration owns.
            let still_mapped = self.wallets.load();
            if !inserted_in_manager.is_empty() {
                let mut wm = self.wallet_manager.write().await;
                for id in &inserted_in_manager {
                    // A published id whose generation is no longer ours belongs
                    // to a newer registration; taking it out of the inner
                    // manager would strip a live wallet of its backing. An id
                    // that never reached `self.wallets` (this call failed
                    // between the two inserts) has no such owner and is unwound
                    // as before.
                    let published = inserted_in_wallets.iter().any(|(w, _)| w == id);
                    if published && !rolled_back.contains(id) {
                        // Two distinct states reach here, and saying the wrong
                        // one sends whoever reads this after a
                        // wallet-disappeared report chasing the wrong
                        // generation: either something else already removed
                        // the entry (a completed concurrent `remove_wallet`),
                        // or a same-id re-registration published a generation
                        // that is not ours. Only the second leaves anything in
                        // place.
                        if still_mapped.contains_key(id) {
                            tracing::warn!(
                                wallet_id = %hex::encode(id),
                                "rollback after load failure: a new generation was registered \
                                 under this id, leaving the new registration in place"
                            );
                        } else {
                            tracing::warn!(
                                wallet_id = %hex::encode(id),
                                "rollback after load failure: this id was already removed by \
                                 something else; nothing left to roll back"
                            );
                        }
                        continue;
                    }
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

/// Of the registrations this load published, the ones a rollback may still
/// take back: those whose map entry is *still the same generation* this call
/// inserted.
///
/// A concurrent `remove_wallet` frees an id, and a registration can publish a
/// different generation under it before a later iteration's failure reaches
/// the rollback. Removing by id alone would delete that live wallet — one this
/// call never created and whose owner is still using it. Same rule
/// `remove_wallet` applies to its own removal.
///
/// Pure so the invariant is unit-testable without racing a real load against a
/// real re-registration.
fn rollback_targets(
    published: &[(WalletId, Arc<WalletGeneration>)],
    current: &BTreeMap<WalletId, Arc<PlatformWallet>>,
) -> Vec<WalletId> {
    published
        .iter()
        .filter(|(id, generation)| {
            current
                .get(id)
                .is_some_and(|wallet| Arc::ptr_eq(wallet.generation(), generation))
        })
        .map(|(id, _)| *id)
        .collect()
}

#[cfg(test)]
mod idempotent_load_tests {
    use std::collections::BTreeMap;
    use std::sync::Arc;

    use super::rollback_targets;
    use crate::wallet::core::WalletGeneration;

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

    /// Two entries: the real wallet under its true id, and the same wallet
    /// under a key that cannot be the id it recomputes to. The second entry
    /// sorts last, so the loader publishes the first and then fails the
    /// id-match check — the only way to drive the rollback without racing a
    /// real failure.
    struct MismatchedSecondWalletPersister {
        wallet: Wallet,
        managed: ManagedWalletInfo,
    }

    impl PlatformWalletPersistence for MismatchedSecondWalletPersister {
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
            let entry = || ClientWalletStartState {
                wallet: self.wallet.clone(),
                wallet_info: self.managed.clone(),
                identity_manager: IdentityManagerStartState::default(),
                unused_asset_locks: BTreeMap::new(),
            };
            let mut wallets = BTreeMap::new();
            wallets.insert(self.wallet.compute_wallet_id(), entry());
            // Sorts after any real id, so it is processed second.
            wallets.insert([0xFF; 32], entry());
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

    /// `dashpay/platform#4309`-adjacent lifecycle hazard: a rollback must not
    /// remove a registration it did not make.
    ///
    /// The interleaving: this load publishes generation G1 under an id, a
    /// concurrent `remove_wallet` frees that id, a registration publishes G2
    /// under it, and only then does a later iteration of this load fail and
    /// reach the rollback. Removing by id alone deletes G2 — a live wallet
    /// whose owner is still using it, and one this call never created.
    ///
    /// Both halves are pinned: the entry is reclaimed while it is still ours,
    /// and refused once it is not. The inner-manager rollback keys off this
    /// same answer, so a wallet left in `self.wallets` is never stripped of
    /// its backing either.
    #[tokio::test]
    async fn rollback_only_reclaims_the_generation_this_load_published() {
        let ctx = TestWalletContext::new_random();
        let expected_id = ctx.wallet.compute_wallet_id();
        let manager = make_manager(SingleWalletPersister {
            wallet: ctx.wallet,
            managed: ctx.managed_wallet,
        });
        manager
            .load_from_persistor()
            .await
            .expect("first load succeeds");

        let published = manager.wallets.load();
        let wallet = published
            .get(&expected_id)
            .expect("the load registered the wallet");
        let ours = Arc::clone(wallet.generation());

        assert_eq!(
            rollback_targets(&[(expected_id, Arc::clone(&ours))], &published),
            vec![expected_id],
            "a registration still holding this load's generation is ours to roll back"
        );

        // The same id, a different generation — what a removal plus a
        // re-registration leaves behind.
        let superseding = Arc::new(WalletGeneration::new());
        assert!(
            !Arc::ptr_eq(&ours, &superseding),
            "the fixture must model two distinct generations"
        );
        assert!(
            rollback_targets(&[(expected_id, superseding)], &published).is_empty(),
            "a generation this load never published must survive its rollback"
        );
    }

    /// Drives the rollback itself, not just the predicate it consults.
    ///
    /// `rollback_only_reclaims_the_generation_this_load_published` covers
    /// `rollback_targets` in isolation; this one fails a load AFTER a wallet
    /// has been published, so the `rcu` closure, the per-attempt verdict
    /// hand-off, and the branch that decides whether the inner manager entry
    /// is removed all execute. Inverting that decision leaves the sibling
    /// test green while stripping a live wallet of its backing, so the two
    /// are not redundant.
    #[tokio::test]
    async fn a_failed_load_rolls_back_the_wallet_it_had_already_published() {
        let ctx = TestWalletContext::new_random();
        let expected_id = ctx.wallet.compute_wallet_id();
        let sdk = Arc::new(dash_sdk::SdkBuilder::new_mock().build().expect("mock sdk"));
        let event_handler: Arc<dyn PlatformEventHandler> = Arc::new(NoopEventHandler);
        let manager = Arc::new(PlatformWalletManager::new(
            sdk,
            Arc::new(MismatchedSecondWalletPersister {
                wallet: ctx.wallet,
                managed: ctx.managed_wallet,
            }),
            event_handler,
        ));

        let result = manager.load_from_persistor().await;
        assert!(
            result.is_err(),
            "the id-mismatched second entry must fail the load"
        );

        assert!(
            manager.get_wallet(&expected_id).await.is_none(),
            "the wallet published before the failure must be rolled back out of the map"
        );
        assert!(
            manager
                .wallet_manager
                .read()
                .await
                .get_wallet(&expected_id)
                .is_none(),
            "and out of the inner manager, so a retry can re-insert it"
        );
    }
}
