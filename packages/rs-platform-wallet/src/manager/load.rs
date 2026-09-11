//! Hydrate a [`PlatformWalletManager`] from its persister.

use std::collections::BTreeMap;
use std::sync::Arc;

use crate::changeset::{ClientStartState, ClientWalletStartState, PlatformWalletPersistence};
use crate::error::PlatformWalletError;
use crate::wallet::core::WalletGeneration;
use crate::wallet::identity::IdentityManager;
use crate::wallet::platform_wallet::{PlatformWalletInfo, WalletId};
use crate::wallet::PlatformWallet;

use std::time::Duration;

use crate::broadcaster::{BroadcastError, TransactionBroadcaster};
use key_wallet::transaction_checking::transaction_context::TransactionContext;
use key_wallet::transaction_checking::wallet_checker::WalletTransactionChecker;

use super::{run_blocking_load, PlatformWalletManager};

/// How long the load-time re-dispatch waits for the SPV transport before
/// giving up for this launch. Readiness means the client started AND at
/// least one peer is connected; zero peers turns a send into a definitive
/// rejection rather than a retry, so waiting is the cheaper mistake.
///
/// Generous on purpose: a simulator reaches readiness in seconds, but a
/// cold device on a slow network can take far longer, and giving up early
/// silently defers the send to the next launch — the very delay this whole
/// path exists to remove. Nothing is blocked on the wait; it runs on a
/// detached task.
const RESEND_TRANSPORT_READY_WAIT: Duration = Duration::from_secs(90);

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
    /// persister cannot produce the snapshot, and
    /// [`PersisterRestore`](PlatformWalletError::PersisterRestore) when a
    /// wallet in the snapshot cannot have its platform-address state rebuilt.
    /// A persisted wallet whose id disagrees with its own key material, or one
    /// the inner [`WalletManager`] refuses, is neither a read nor a restore
    /// failure and stays
    /// [`WalletCreation`](PlatformWalletError::WalletCreation).
    ///
    /// Persister errors surface after one attempt so the caller controls retry
    /// policy. The synchronous read runs on the blocking pool.
    ///
    /// Any `Err` rolls back partial inserts and leaves the manager usable: fix
    /// the store and call again, or reconstruct. Reconstructing over the same
    /// path needs every strong persister reference released first. Dropping
    /// the manager releases its own references; wallet handles, workers and
    /// in-flight operations can retain others. [`shutdown`](Self::shutdown)
    /// takes `&self`, so it cannot release the manager's own `Arc<P>`.
    ///
    /// [`WalletManager`]: key_wallet_manager::WalletManager
    pub async fn load_from_persistor(&self) -> Result<(), PlatformWalletError> {
        let persister = Arc::clone(&self.persister);
        let start_state = match run_blocking_load(move || persister.load()).await {
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
        // Re-dispatches owed by this load, held until the rollback point has
        // passed. See the push site for why they cannot be spawned inline.
        #[allow(clippy::type_complexity)]
        let mut pending_resends: Vec<(
            WalletId,
            Arc<WalletGeneration>,
            Arc<crate::broadcaster::SpvBroadcaster>,
            Vec<dashcore::Transaction>,
        )> = Vec::new();
        // The generation travels with the id: a rollback may only remove the
        // registration THIS call published (see the rollback block below).
        let mut inserted_in_wallets: Vec<(WalletId, Arc<crate::wallet::core::WalletGeneration>)> =
            Vec::new();
        let mut load_error: Option<PlatformWalletError> = None;

        'load: for (expected_wallet_id, wallet_state) in wallets {
            let ClientWalletStartState {
                mut wallet,
                mut wallet_info,
                identity_manager,
                unused_asset_locks,
                unconfirmed_outgoing_txs,
            } = wallet_state;

            // Replay the sends the host still holds as unconfirmed, before
            // anything reads the restored balance.
            //
            // Their spend effect is never persisted: `isSpent` stays `false`
            // on the input row until the spending transaction reaches a
            // block, because a mempool-only sighting is reversible by
            // eviction. So the UTXO restore above has just handed those
            // inputs back as spendable. A live process was still correct —
            // it held the effect in memory — and until now a restart
            // recovered it only by re-observing the transaction on the
            // network. A transaction that never reached the network cannot
            // be re-observed, so its input stayed spendable for good and the
            // balance re-counted the coin.
            //
            // Routing each record through the ordinary mempool check (rather
            // than inserting it into `transactions_mut()` raw, the way the
            // asset-lock record restore does) is the whole point: it runs
            // `update_utxos`, which drops the input from `utxos` and records
            // it in `spent_outpoints`, reproducing exactly the state the
            // live process held. A raw insert would leave `spent_outpoints`
            // empty AND make every later re-dispatch a no-op, because
            // `has_transaction` would then report the record as not new.
            //
            // `update_state` and `update_balance` are both on: the balance
            // this produces is what `generation.set(..)` mirrors a few lines
            // below, and the UI reads that.
            if !unconfirmed_outgoing_txs.is_empty() {
                let mut replayed = 0usize;
                for tx in &unconfirmed_outgoing_txs {
                    let result = wallet_info
                        .check_core_transaction(
                            tx,
                            TransactionContext::Mempool,
                            &mut wallet,
                            true,
                            true,
                        )
                        .await;
                    if result.is_relevant {
                        replayed += 1;
                    }
                }
                tracing::info!(
                    wallet_id = %hex::encode(expected_wallet_id),
                    offered = unconfirmed_outgoing_txs.len(),
                    replayed,
                    "load: replayed unconfirmed outgoing sends"
                );
            }

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

            // The fence map is per WALLET, not per generation. On a first load
            // the registry is empty and this is a fresh map; a re-load — or a
            // load that follows a removal — inherits whatever pending spends the
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

            // Give the replayed sends an owner again on the network side.
            //
            // dash-spv's rebroadcast timer is the only thing that retries a
            // transaction whose broadcast saw no acceptance signal, and its
            // `broadcasts` map is process-local: it is filled at the
            // broadcast call and never seeded from persisted rows. So a send
            // that did not reach the network before the app was closed had
            // nobody left to resend it — measured, it never went out again.
            // Re-dispatching here hands it back to that timer.
            //
            // Deliberately fire-and-forget on a detached task: this must not
            // hold up the load, and the verdict is only logged. The platform
            // broadcaster has one entry point and it waits for acceptance
            // (`TransactionBroadcaster::broadcast` → `broadcast_and_wait`),
            // which is harmless here — nothing is blocked on this task, and
            // dash-spv has already taken ownership by the time the wait ends.
            // The app registers no listener for that event, so a late
            // `Uncertain` cannot surface a stray dialog.
            //
            // For the case this exists for — a send that never reached the
            // network — `MaybeSent` is the EXPECTED answer, not a failure:
            // the transaction goes out, no peer echoes it back inside the
            // acceptance window, and the rebroadcast timer takes it from
            // there. Logging that at warn would make the healthy path look
            // broken.
            //
            // Safe against double-spending: this re-sends the SAME signed
            // bytes, which is idempotent for the network, and `start_broadcast`
            // is idempotent per txid. The real hazard would be re-dispatching
            // without the accounting replay above — the input would be
            // selectable again and this wallet could sign a conflicting
            // transaction. That is why the two halves ship together.
            if !unconfirmed_outgoing_txs.is_empty() {
                // Queued, not spawned: a later iteration can still fail and
                // roll this registration back, and a task already waiting on
                // transport readiness would outlive it and rebroadcast for a
                // wallet that no longer exists. Spawned after the rollback
                // point instead, with the generation carried along so the
                // task can tell whether the registration it was created for
                // is still the live one.
                pending_resends.push((
                    wallet_id,
                    Arc::clone(&generation),
                    Arc::clone(&broadcaster),
                    unconfirmed_outgoing_txs,
                ));
            }

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
                    // Wrap the already-typed error rather than stringify it, so
                    // its concrete variant and source chain survive — the same
                    // shape `register_wallet` returns for this same failure.
                    load_error = Some(PlatformWalletError::from_restore_failure(e));
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

        // Past the rollback point: every registration here is one this load
        // actually committed, so the transactions now have a wallet to belong
        // to for as long as it stays registered.
        //
        // Detached on purpose — nothing may block the load on transport
        // readiness — which is why each task re-checks that its wallet is
        // still the live registration before putting anything on the wire. A
        // wallet removed while the task waits leaves the generation pointer
        // pointing at nothing the map holds any more, and the re-dispatch is
        // abandoned rather than broadcasting on behalf of a wallet that is
        // gone.
        for (wallet_id, generation, broadcaster, txs) in pending_resends {
            let wallets = Arc::clone(&self.wallets);
            tokio::spawn(async move {
                if !broadcaster
                    .wait_until_ready(RESEND_TRANSPORT_READY_WAIT)
                    .await
                {
                    tracing::warn!(
                        pending = txs.len(),
                        "load: broadcast transport not ready; leaving unconfirmed \
                         sends for the next launch"
                    );
                    return;
                }
                let still_live = wallets
                    .load()
                    .get(&wallet_id)
                    .is_some_and(|wallet| Arc::ptr_eq(wallet.generation(), &generation));
                if !still_live {
                    tracing::info!(
                        wallet_id = %hex::encode(wallet_id),
                        pending = txs.len(),
                        "load: wallet no longer registered; abandoning the re-dispatch"
                    );
                    return;
                }
                for tx in txs {
                    let txid = tx.txid();
                    match broadcaster.broadcast(&tx).await {
                        Ok(_) => tracing::info!(
                            %txid,
                            "load: re-dispatched unconfirmed send, accepted"
                        ),
                        // Expected for the orphaned case: sent, no acceptance
                        // signal, now owned by the rebroadcast timer.
                        Err(BroadcastError::MaybeSent { reason }) => tracing::info!(
                            %txid,
                            %reason,
                            "load: re-dispatched unconfirmed send, no acceptance signal yet — \
                             handed to the rebroadcast timer"
                        ),
                        // Provably never sent: worth a warning, since nothing
                        // carried it and the next launch is the only remaining
                        // chance.
                        Err(e) => tracing::warn!(
                            %txid,
                            error = ?e,
                            "load: re-dispatch was not sent"
                        ),
                    }
                }
            });
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
    use crate::events::PlatformEventHandler;
    use crate::test_support::NoopTestEventHandler;
    use crate::wallet::platform_wallet::WalletId;
    use crate::PlatformWalletManager;

    /// Persister that hands back one wallet plus the outgoing sends the
    /// host still holds as unconfirmed — the shape `loadWalletList`
    /// produces for a send whose broadcast got no acceptance signal.
    struct PendingSendPersister {
        wallet: Wallet,
        managed: ManagedWalletInfo,
        pending: Vec<dashcore::Transaction>,
    }

    impl PlatformWalletPersistence for PendingSendPersister {
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
            let mut wallets = BTreeMap::new();
            wallets.insert(
                self.wallet.compute_wallet_id(),
                ClientWalletStartState {
                    wallet: self.wallet.clone(),
                    wallet_info: self.managed.clone(),
                    identity_manager: IdentityManagerStartState::default(),
                    unused_asset_locks: BTreeMap::new(),
                    unconfirmed_outgoing_txs: self.pending.clone(),
                },
            );
            Ok(ClientStartState {
                wallets,
                ..Default::default()
            })
        }
    }

    /// A transaction spending `previous_output` to somewhere that is not
    /// this wallet — enough for the mempool check to see the input leave.
    fn spend_to(previous_output: dashcore::OutPoint, value: u64) -> dashcore::Transaction {
        dashcore::Transaction {
            version: 2,
            lock_time: 0,
            input: vec![dashcore::TxIn {
                previous_output,
                script_sig: dashcore::ScriptBuf::new(),
                sequence: 0xffff_ffff,
                witness: Default::default(),
            }],
            output: vec![dashcore::TxOut {
                value,
                script_pubkey: dashcore::ScriptBuf::from_hex(
                    "76a914000000000000000000000000000000000000000088ac",
                )
                .expect("static foreign p2pkh script"),
            }],
            special_transaction_payload: None,
        }
    }

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
                    unconfirmed_outgoing_txs: Vec::new(),
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
                unconfirmed_outgoing_txs: Vec::new(),
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

    /// A send the host still holds as unconfirmed has to be replayed at
    /// load, or the coin it spent comes back as spendable.
    ///
    /// The spend effect is never persisted — `isSpent` stays false on the
    /// input row until the spending transaction reaches a block, because a
    /// mempool-only sighting is reversible by eviction — so the restored
    /// UTXO set hands that input straight back. A running app is still
    /// correct, holding the effect in memory; across a restart it used to
    /// be recovered only by re-observing the transaction on the network,
    /// which never happens for a send that did not reach the network. The
    /// balance then re-counted the coin, permanently (support ticket
    /// 32189).
    ///
    /// Asserting on `balance()` rather than on the account internals is
    /// deliberate: that is the number the UI reads, and it is mirrored
    /// from the replayed state a few lines after the replay runs.
    #[tokio::test]
    async fn load_replays_an_unconfirmed_outgoing_send() {
        let (ctx, funding) = TestWalletContext::new_random()
            .with_mempool_funding(100_000)
            .await;
        let wallet_id = ctx.wallet.compute_wallet_id();
        let funded_outpoint = dashcore::OutPoint {
            txid: funding.txid(),
            vout: 0,
        };

        // Without a replay this is what the restore alone would leave
        // standing, so it is also the failure the assertion below catches.
        let spend = spend_to(funded_outpoint, 74_000);
        let manager = make_pending_manager(PendingSendPersister {
            wallet: ctx.wallet,
            managed: ctx.managed_wallet,
            pending: vec![spend],
        });

        manager
            .load_from_persistor()
            .await
            .expect("the wallet must load");

        let wallet = manager
            .get_wallet(&wallet_id)
            .await
            .expect("the loaded wallet must be registered");
        let balance = wallet.balance();
        let total = balance.confirmed() + balance.unconfirmed();
        assert_eq!(
            total, 0,
            "the replayed send spends the only coin, so nothing may remain \
             spendable; {} duffs left means the input came back",
            total
        );
    }

    fn make_pending_manager(
        persister: PendingSendPersister,
    ) -> Arc<PlatformWalletManager<PendingSendPersister>> {
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
    /// be a no-op `Ok(())` — NOT a `WalletExists`-wrapped
    /// `WalletCreation` error, which crashes the app on the main
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

        // Re-hydrating with the wallet already present must be a silent
        // no-op, not `Failed to register persisted wallet in WalletManager:
        // Wallet already exists`.
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

    /// Lifecycle hazard: a rollback must not remove a registration it did not
    /// make.
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
        let event_handler: Arc<dyn PlatformEventHandler> = Arc::new(NoopTestEventHandler);
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
    /// its `persister` field, the `DashPayPaymentHandler`, and the
    /// `IdentitySyncManager` — the wallet-event adapter deliberately excluded.
    const MANAGER_PERSISTER_HOLDERS: usize = 3;

    /// Persister whose `load()` always fails.
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

    /// Fails `load()` permanently once, then succeeds.
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
    async fn transient_load_failure_during_startup_rehydration_surfaces_immediately() {
        let persister = Arc::new(TransientOnceLoadPersister {
            load_calls: AtomicUsize::new(0),
        });
        let probe = Arc::clone(&persister);
        let manager = make_manager(persister);

        let err = manager
            .load_from_persistor()
            .await
            .expect_err("transient startup load failure must reach the caller");

        match err {
            PlatformWalletError::PersisterLoad(source) => assert!(source.is_transient()),
            other => panic!("expected transient PersisterLoad, got {other:?}"),
        }
        assert_eq!(probe.load_calls.load(Ordering::SeqCst), 1);
    }

    /// Isolating by construction: the count is read on a live, idle manager
    /// with nothing dropped or aborted, so no teardown path can stand in for
    /// the weak-reference property.
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

    /// Running the manager-wide, one-way `shutdown()` on this failure path
    /// seals every coordinator's admission gate and joins the wallet-event
    /// adapter, so the retry returns `Ok(())` onto a manager that can never
    /// sync or persist again (#4133).
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

        // The adapter's receiver is taken exactly once, so a joined adapter
        // cannot be respawned: `Ok` means the reused manager still persists.
        let report = manager.shutdown().await;
        assert_eq!(
            report.per_worker.get(&WalletWorker::EventAdapter),
            Some(&WorkerStatus::Ok),
            "the wallet-event adapter must still have been running for \
             shutdown to join it: {report:?}"
        );
    }

    /// Dropping the manager after a failed load releases the persister — the
    /// precondition for reconstructing on the same path without a spurious
    /// `WalletStorageError::AlreadyOpen` masking the real error (#4133).
    ///
    /// Isolates nothing: the count is the product of the whole teardown, so one
    /// participant may regress while another still releases.
    /// `adapter_holds_no_strong_persister_reference` pins the weak reference.
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

    /// A dirty drop releases the persister **synchronously**, bounded only by a
    /// batch commit in flight (see
    /// `an_in_flight_commit_holds_a_strong_persister_reference` in
    /// `changeset::core_bridge`); the adapter is idle here.
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
