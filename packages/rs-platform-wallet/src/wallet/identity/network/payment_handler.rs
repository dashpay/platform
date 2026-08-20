//! Event handler that drives the DashPay payment hooks off upstream
//! `WalletEvent`s.
//!
//! Registered as one of the [`PlatformEventHandler`]s in
//! [`PlatformEventManager`](crate::events::PlatformEventManager), this
//! keeps the DashPay-payment domain logic out of the generic
//! core-changeset bridge ([`spawn_wallet_event_adapter`]): the bridge
//! projects every event into a `CoreChangeSet` and persists it, while
//! this handler independently records incoming payments and confirms
//! sent ones.
//!
//! # Why it spawns
//!
//! [`PlatformEventHandler::on_wallet_event`] is synchronous and is
//! dispatched from dash-spv's wallet-event broadcast monitor, which can
//! fire while SPV holds the wallet-manager write lock. The payment hooks
//! are async and take that same write lock, so they cannot run inline.
//! The handler therefore captures an owned copy of the event and spawns
//! a task that queues on the write lock and runs once SPV releases it.
//! Every hook path is idempotent per txid (re-detections converge and
//! the recurring reconcile sweep backfills anything a lagged broadcast
//! dropped), so running off the core-store bridge's ordering is safe —
//! a payment row's only foreign key is to its `identities` parent, never
//! to a core transaction row.

use std::sync::Arc;
use std::{future::Future, sync::Mutex};

use dash_spv::EventHandler;
use key_wallet::managed_account::transaction_record::TransactionRecord;
use key_wallet_manager::{WalletEvent, WalletId, WalletManager};
use tokio::sync::RwLock;

use crate::changeset::traits::PlatformWalletPersistence;
use crate::events::PlatformEventHandler;
use crate::wallet::platform_wallet::PlatformWalletInfo;

/// Records incoming DashPay payments and confirms sent ones in response
/// to upstream `WalletEvent`s.
///
/// Holds the manager's `wallet_manager` (for the in-memory identity /
/// payment state the hooks mutate) and an `Arc<dyn PlatformWalletPersistence>`
/// (to persist the resulting payment entries). Both are cheap `Arc`
/// clones taken at manager construction.
pub(crate) struct DashPayPaymentHandler {
    wallet_manager: Arc<RwLock<WalletManager<PlatformWalletInfo>>>,
    persister: Arc<dyn PlatformWalletPersistence>,
    tasks: PaymentTaskTracker,
}

struct PaymentTaskState {
    accepting: bool,
    handles: Vec<tokio::task::JoinHandle<()>>,
}

/// Admission gate and join barrier for asynchronous payment hooks.
///
/// Event dispatch is synchronous, so the gate uses a short standard mutex:
/// checking admission and recording the resulting task happen atomically with
/// respect to shutdown. No mutex guard is held while a task is awaited.
struct PaymentTaskTracker {
    state: Mutex<PaymentTaskState>,
}

impl PaymentTaskTracker {
    fn new() -> Self {
        Self {
            state: Mutex::new(PaymentTaskState {
                accepting: true,
                handles: Vec::new(),
            }),
        }
    }

    fn spawn<F>(&self, future: F) -> bool
    where
        F: Future<Output = ()> + Send + 'static,
    {
        let mut state = self.state.lock().expect("payment task mutex poisoned");
        if !state.accepting {
            return false;
        }

        // Completed tasks cannot fire another callback and need not be kept
        // until manager destruction. This bounds the tracker during long syncs.
        state.handles.retain(|handle| !handle.is_finished());
        state.handles.push(tokio::spawn(future));
        true
    }

    /// Unbounded drain — test-only convenience over
    /// [`quiesce_within`](Self::quiesce_within) with an effectively
    /// infinite budget.
    #[cfg(test)]
    async fn quiesce(&self) {
        let _ = self
            .quiesce_within(std::time::Duration::from_secs(60 * 60))
            .await;
    }

    /// Close admission and join every admitted task, bounded by `budget`.
    ///
    /// Returns `true` when every task terminated. Two phases, each under a
    /// **shared** deadline so the whole drain is bounded by
    /// `budget + PAYMENT_ABORT_GRACE` regardless of how many stragglers
    /// there are: every task gets until `budget` to finish on its own, then
    /// every survivor is aborted *first* and only then confirmed, all
    /// against one [`PAYMENT_ABORT_GRACE`](crate::manager::PAYMENT_ABORT_GRACE)
    /// deadline. (Aborting and confirming one straggler at a time would
    /// cost `survivors × PAYMENT_ABORT_GRACE`, which is not the bound
    /// `shutdown` advertises.)
    ///
    /// A task stuck in a synchronous call (e.g. an FFI persister `store`)
    /// cannot be interrupted by an abort — it is put **back into the
    /// tracker** (so a destroy retry re-joins it instead of silently
    /// detaching a callback-capable task) and the method returns `false`.
    ///
    /// Aborting is safe here: the payment hooks are reconciliation-based
    /// (re-derivable from wallet transaction records on the next launch),
    /// so a hook cancelled between awaits loses no unrecoverable state.
    async fn quiesce_within(&self, budget: std::time::Duration) -> bool {
        let handles = {
            let mut state = self.state.lock().expect("payment task mutex poisoned");
            state.accepting = false;
            std::mem::take(&mut state.handles)
        };

        // Phase 1 — graceful, one shared deadline for all tasks.
        let deadline = tokio::time::Instant::now() + budget;
        let mut survivors = Vec::new();
        for mut handle in handles {
            match tokio::time::timeout_at(deadline, &mut handle).await {
                Ok(Ok(())) => {}
                Ok(Err(error)) => tracing::warn!(?error, "DashPay payment task join error"),
                Err(_) => survivors.push(handle),
            }
        }

        if survivors.is_empty() {
            return true;
        }

        // Phase 2 — abort EVERY straggler before awaiting any of them, so
        // the grace below is paid once rather than once per straggler.
        for handle in &survivors {
            handle.abort();
        }
        let abort_deadline = tokio::time::Instant::now() + crate::manager::PAYMENT_ABORT_GRACE;
        let mut still_live = Vec::new();
        for mut handle in survivors {
            match tokio::time::timeout_at(abort_deadline, &mut handle).await {
                Ok(Err(error)) if !error.is_cancelled() => {
                    tracing::warn!(?error, "DashPay payment task abort join error");
                }
                Ok(_) => {}
                Err(_) => still_live.push(handle),
            }
        }

        if still_live.is_empty() {
            return true;
        }
        tracing::warn!(
            survivors = still_live.len(),
            "DashPay payment tasks did not terminate within the drain budget; \
             keeping them tracked for a retry"
        );
        let mut state = self.state.lock().expect("payment task mutex poisoned");
        state.handles.extend(still_live);
        false
    }

    #[cfg(test)]
    fn is_accepting(&self) -> bool {
        self.state
            .lock()
            .expect("payment task mutex poisoned")
            .accepting
    }
}

impl DashPayPaymentHandler {
    pub(crate) fn new(
        wallet_manager: Arc<RwLock<WalletManager<PlatformWalletInfo>>>,
        persister: Arc<dyn PlatformWalletPersistence>,
    ) -> Self {
        Self {
            wallet_manager,
            persister,
            tasks: PaymentTaskTracker::new(),
        }
    }

    /// Stop admitting callback-bearing work and join every task admitted
    /// before the gate closed, bounded by `budget`. Idempotent.
    ///
    /// Returns `false` when a task outlived the budget (and its post-abort
    /// grace); the straggler stays tracked so a retry can re-join it, and
    /// the caller must report the shutdown non-clean — these tasks clone
    /// the FFI persister and can fire host callbacks.
    #[must_use = "a false return means a callback-capable task is still live; report non-clean"]
    pub(crate) async fn quiesce_within(&self, budget: std::time::Duration) -> bool {
        self.tasks.quiesce_within(budget).await
    }
}

impl EventHandler for DashPayPaymentHandler {
    fn on_wallet_event(&self, event: &WalletEvent) {
        if !drives_payment_hooks(event) {
            return;
        }
        // Capture owned clones so the async hooks can outlive this
        // synchronous dispatch. See the module docs for why this runs on
        // its own task rather than inline.
        let wallet_manager = Arc::clone(&self.wallet_manager);
        let persister = Arc::clone(&self.persister);
        let event = event.clone();
        self.tasks.spawn(async move {
            let wallet_id = event.wallet_id();
            let wallet_persister =
                crate::wallet::persister::WalletPersister::new(wallet_id, persister);
            run_dashpay_payment_hooks(&wallet_manager, &wallet_id, &wallet_persister, &event).await;
        });
    }
}

impl PlatformEventHandler for DashPayPaymentHandler {}

/// Transaction records carried by `event` that should drive the DashPay
/// payment hooks (live incoming-record recording + sent-payment confirm).
///
/// [`WalletEvent::TransactionDetected`] is the first off-chain sighting of
/// a transaction — mempool, or a direct InstantSend lock — so its
/// `record.context` is not yet block-confirmed.
/// [`WalletEvent::BlockProcessed`] carries the records a block changed:
/// `inserted` (first stored in this block) and `updated`
/// (previously-known records that this block confirmed). A wallet sees its
/// *own* broadcast in the mempool first, so that transaction reaches a
/// confirmed context only via `BlockProcessed.updated` — routing solely
/// `TransactionDetected` is the gap that left sent payments stuck
/// `Pending`: the confirm hook early-returns on the unconfirmed mempool
/// sighting and never sees the confirming block. `matured` is
/// coinbase-maturity only — never a DashPay payment — so it is excluded.
fn dashpay_payment_records(event: &WalletEvent) -> Vec<&TransactionRecord> {
    // Exhaustive on purpose (no `_` arm): a new upstream `WalletEvent`
    // variant that carries transaction records must fail to compile here
    // rather than be silently dropped — routing only `TransactionDetected`
    // is exactly the gap that left sent payments stuck `Pending`.
    match event {
        WalletEvent::TransactionDetected { record, .. } => vec![record.as_ref()],
        WalletEvent::BlockProcessed {
            inserted, updated, ..
        } => inserted.iter().chain(updated.iter()).collect(),
        // `TransactionsSwept` carries txids, not records: the wallet has
        // already dropped the records these name. Its payment consequence
        // — failing the matching `Pending` sent payments, since a swept
        // transaction can never confirm — is NOT this handler's to apply:
        // a sweep never re-emits once its round is durable, so the flip
        // must ride the sweep's own atomic store round, and the
        // wallet-event adapter owns that (see
        // `payments::SweptPaymentFlips`). Routing it here as well would
        // race a second, separately persisted write against that round.
        WalletEvent::TransactionInstantLocked { .. }
        | WalletEvent::TransactionsSwept { .. }
        | WalletEvent::SyncHeightAdvanced { .. }
        | WalletEvent::ChainLockProcessed { .. } => Vec::new(),
    }
}

/// Whether `event` is worth spawning a payment-hook task for.
///
/// Covers the record-bearing events ([`dashpay_payment_records`]) plus
/// [`WalletEvent::TransactionInstantLocked`], which drives the sent-payment
/// confirm by txid alone (no record). A `BlockProcessed` that changed no
/// records — the common case while syncing past empty blocks — has no
/// payment work, so it is skipped rather than spawning a task that would
/// only take and release the wallet-manager write lock for nothing.
/// Allocation-free.
fn drives_payment_hooks(event: &WalletEvent) -> bool {
    match event {
        WalletEvent::TransactionDetected { .. } | WalletEvent::TransactionInstantLocked { .. } => {
            true
        }
        WalletEvent::BlockProcessed {
            inserted, updated, ..
        } => !inserted.is_empty() || !updated.is_empty(),
        // No records to route (see `dashpay_payment_records`), so a task
        // here would take and release the wallet-manager write lock for
        // nothing. The sweep's payment consequence rides the wallet-event
        // adapter's own store round instead — see `dashpay_payment_records`.
        WalletEvent::TransactionsSwept { .. }
        | WalletEvent::SyncHeightAdvanced { .. }
        | WalletEvent::ChainLockProcessed { .. } => false,
    }
}

/// Run the DashPay payment hooks for `event`: record any incoming DashPay
/// payment, then advance a matching sent payment from `Pending` (or a
/// sweep-written `Failed` — the reinstatement correction) to `Confirmed`
/// once its transaction reaches finality (mined or InstantSend-locked).
/// The opposite terminal — `Failed`, when a sweep proves the transaction
/// never can confirm — is applied by the wallet-event adapter on the
/// sweep's own atomic store round, not here (see
/// `payments::SweptPaymentFlips`). All paths are idempotent per txid, so
/// re-detections and repeated block-processing rounds converge without
/// duplicating entries.
pub(crate) async fn run_dashpay_payment_hooks(
    wallet_manager: &Arc<RwLock<WalletManager<PlatformWalletInfo>>>,
    wallet_id: &WalletId,
    persister: &crate::wallet::persister::WalletPersister,
    event: &WalletEvent,
) {
    // An InstantSend lock applied to a previously-seen transaction carries
    // no record — only a txid — and is final for DashPay display, so
    // confirm the matching sent payment directly.
    if let WalletEvent::TransactionInstantLocked { txid, .. } = event {
        crate::wallet::identity::network::confirm_sent_dashpay_payment_by_txid(
            wallet_manager,
            wallet_id,
            persister,
            txid,
        )
        .await;
        return;
    }
    for record in dashpay_payment_records(event) {
        crate::wallet::identity::network::record_incoming_dashpay_payments(
            wallet_manager,
            wallet_id,
            persister,
            record,
        )
        .await;
        crate::wallet::identity::network::confirm_sent_dashpay_payment(
            wallet_manager,
            wallet_id,
            persister,
            record,
        )
        .await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dashcore::blockdata::transaction::Transaction;
    use dashcore::TxIn;
    use key_wallet::account::account_type::StandardAccountType;
    use key_wallet::account::AccountType;
    use key_wallet::managed_account::transaction_record::TransactionDirection;
    use key_wallet::transaction_checking::{TransactionContext, TransactionType};
    use key_wallet::WalletCoreBalance;
    use std::sync::atomic::{AtomicBool, Ordering};

    /// A `TransactionRecord` whose txid is uniquely seeded by `seed` (via a
    /// distinct input outpoint). Context is irrelevant to the routing under
    /// test, so it stays `Mempool`.
    fn record(seed: u8) -> TransactionRecord {
        let tx = Transaction {
            version: 1,
            lock_time: 0,
            input: vec![TxIn {
                previous_output: dashcore::OutPoint::new(dashcore::Txid::from([seed; 32]), 0),
                ..Default::default()
            }],
            output: Vec::new(),
            special_transaction_payload: None,
        };
        TransactionRecord::new(
            tx,
            AccountType::Standard {
                index: 0,
                standard_account_type: StandardAccountType::BIP44Account,
            },
            TransactionContext::Mempool,
            TransactionType::Standard,
            TransactionDirection::Outgoing,
            Vec::new(),
            Vec::new(),
            0,
        )
    }

    fn block_processed(
        inserted: Vec<TransactionRecord>,
        updated: Vec<TransactionRecord>,
        matured: Vec<TransactionRecord>,
    ) -> WalletEvent {
        WalletEvent::BlockProcessed {
            wallet_id: [0u8; 32],
            height: 1,
            chain_lock: None,
            inserted,
            updated,
            matured,
            balance: WalletCoreBalance::default(),
            account_balances: std::collections::BTreeMap::new(),
            addresses_derived: Vec::new(),
        }
    }

    /// `BlockProcessed` is the path by which a wallet's own broadcast
    /// confirms (`updated`), and the path by which a payment first seen in a
    /// block lands (`inserted`); both must drive the DashPay payment hooks.
    /// `matured` is coinbase-maturity only and carries no DashPay payment, so
    /// it is excluded. A regression that re-narrows routing to
    /// `TransactionDetected` — the original sent-payment-stuck-`Pending` bug —
    /// drops the `updated` record and fails this test.
    #[test]
    fn dashpay_payment_records_covers_block_processed_inserted_and_updated() {
        let event = block_processed(vec![record(0x01)], vec![record(0x02)], vec![record(0x03)]);
        let txids: Vec<_> = dashpay_payment_records(&event)
            .iter()
            .map(|r| r.txid)
            .collect();
        assert!(
            txids.contains(&record(0x01).txid),
            "inserted record must drive the payment hooks"
        );
        assert!(
            txids.contains(&record(0x02).txid),
            "updated (just-confirmed) record must drive the payment hooks — \
             this is how a sent payment flips Pending → Confirmed"
        );
        assert!(
            !txids.contains(&record(0x03).txid),
            "matured coinbase is not a DashPay payment and must be excluded"
        );
        assert_eq!(txids.len(), 2, "exactly inserted ∪ updated");
    }

    /// The first mempool sighting still routes its single record (incoming
    /// recording + the early-returning confirm probe).
    #[test]
    fn dashpay_payment_records_covers_transaction_detected() {
        let event = WalletEvent::TransactionDetected {
            wallet_id: [0u8; 32],
            record: Box::new(record(0x07)),
            balance: WalletCoreBalance::default(),
            account_balances: std::collections::BTreeMap::new(),
            addresses_derived: Vec::new(),
        };
        let txids: Vec<_> = dashpay_payment_records(&event)
            .iter()
            .map(|r| r.txid)
            .collect();
        assert_eq!(txids, vec![record(0x07).txid]);
    }

    /// Events with no transaction records contribute nothing, and a
    /// record-less, non-IS event does not drive the payment hooks.
    #[test]
    fn dashpay_payment_records_empty_for_non_record_events() {
        let event = WalletEvent::SyncHeightAdvanced {
            wallet_id: [0u8; 32],
            height: 42,
        };
        assert!(dashpay_payment_records(&event).is_empty());
        assert!(!drives_payment_hooks(&event));
    }

    /// `TransactionInstantLocked` carries no record but DOES drive the
    /// payment hooks — it confirms a sent payment by txid alone (an
    /// InstantSend lock is final for DashPay display).
    #[test]
    fn instant_locked_drives_payment_hooks_without_a_record() {
        use dashcore::ephemerealdata::instant_lock::InstantLock;
        let event = WalletEvent::TransactionInstantLocked {
            wallet_id: [0u8; 32],
            txid: dashcore::Txid::from([0x11; 32]),
            instant_lock: InstantLock::default(),
            balance: WalletCoreBalance::default(),
            account_balances: std::collections::BTreeMap::new(),
        };
        // No record to route, but the event must still drive the hooks.
        assert!(dashpay_payment_records(&event).is_empty());
        assert!(drives_payment_hooks(&event));
    }

    /// `TransactionsSwept` must NOT drive the payment hooks: its payment
    /// consequence — failing the losers' `Pending` sent payments — rides
    /// the wallet-event adapter's own atomic store round (see
    /// `payments::SweptPaymentFlips`), because a sweep never re-emits once
    /// its round is durable and a separately persisted flip that failed
    /// its store would be lost for good. Spawning a hook task here would
    /// race a second write against that round.
    #[test]
    fn transactions_swept_does_not_drive_payment_hooks() {
        let event = WalletEvent::TransactionsSwept {
            wallet_id: [0u8; 32],
            txids: vec![dashcore::Txid::from([0x21; 32])],
            superseded_by: dashcore::Txid::from([0x22; 32]),
            released_outpoints: Vec::new(),
            balance: WalletCoreBalance::default(),
            account_balances: std::collections::BTreeMap::new(),
        };
        assert!(dashpay_payment_records(&event).is_empty());
        assert!(!drives_payment_hooks(&event));
    }

    /// A `BlockProcessed` that changed no records (syncing past an empty
    /// block) has no payment work, so it must not spawn a hook task. Pins
    /// the spawn-skip that keeps initial sync from taking the wallet-manager
    /// write lock once per empty block.
    #[test]
    fn empty_block_processed_does_not_drive_payment_hooks() {
        let event = block_processed(Vec::new(), Vec::new(), vec![record(0x05)]);
        // `matured`-only blocks carry no DashPay payment and no inserted/
        // updated records, so there is nothing to route and nothing to spawn.
        assert!(dashpay_payment_records(&event).is_empty());
        assert!(!drives_payment_hooks(&event));
    }

    #[tokio::test]
    async fn payment_task_quiesce_closes_admission_and_joins_in_flight_work() {
        let tasks = Arc::new(PaymentTaskTracker::new());
        let (started_tx, started_rx) = tokio::sync::oneshot::channel();
        let (release_tx, release_rx) = tokio::sync::oneshot::channel();
        let completed = Arc::new(AtomicBool::new(false));
        let completed_by_task = Arc::clone(&completed);

        assert!(tasks.spawn(async move {
            let _ = started_tx.send(());
            let _ = release_rx.await;
            completed_by_task.store(true, Ordering::SeqCst);
        }));
        started_rx.await.expect("tracked task should start");

        let tasks_for_shutdown = Arc::clone(&tasks);
        let shutdown = tokio::spawn(async move {
            tasks_for_shutdown.quiesce().await;
        });

        while tasks.is_accepting() {
            tokio::task::yield_now().await;
        }
        assert!(!shutdown.is_finished(), "quiesce must await admitted work");

        let ran_after_close = Arc::new(AtomicBool::new(false));
        let ran_after_close_in_task = Arc::clone(&ran_after_close);
        assert!(!tasks.spawn(async move {
            ran_after_close_in_task.store(true, Ordering::SeqCst);
        }));

        release_tx
            .send(())
            .expect("tracked task should still exist");
        shutdown.await.expect("quiesce task should join");
        assert!(completed.load(Ordering::SeqCst));
        assert!(!ran_after_close.load(Ordering::SeqCst));

        // Repeated shutdown is safe and keeps admission closed.
        tasks.quiesce().await;
        assert!(!tasks.spawn(async {}));
    }

    /// A straggler parked at an await point must be reaped by the abort
    /// escalation: `quiesce_within` stays bounded and still reports a
    /// clean drain because the task verifiably terminated.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn quiesce_within_aborts_await_parked_straggler_and_reports_clean() {
        let tasks = Arc::new(PaymentTaskTracker::new());
        assert!(tasks.spawn(async {
            std::future::pending::<()>().await;
        }));

        let drained = tokio::time::timeout(
            std::time::Duration::from_secs(5),
            tasks.quiesce_within(std::time::Duration::from_millis(50)),
        )
        .await
        .expect("bounded quiesce must return");
        assert!(
            drained,
            "an await-parked task is abortable and must count as terminated"
        );
        assert!(!tasks.spawn(async {}), "admission must stay closed");
    }

    /// A straggler stuck in a synchronous call (no await point — an abort
    /// cannot land) must NOT be silently detached: `quiesce_within`
    /// returns `false` and keeps the handle tracked so a retry re-joins
    /// it once the blocking call finally returns.
    #[tokio::test(flavor = "multi_thread", worker_threads = 3)]
    async fn quiesce_within_keeps_unabortable_straggler_tracked_and_reports_false() {
        let tasks = Arc::new(PaymentTaskTracker::new());
        let (started_tx, started_rx) = tokio::sync::oneshot::channel();
        assert!(tasks.spawn(async move {
            let _ = started_tx.send(());
            // Stands in for a synchronous FFI persister call: blocks the
            // task without ever reaching an await point, so the abort in
            // `quiesce_within` cannot take effect.
            std::thread::sleep(std::time::Duration::from_millis(2500));
        }));
        started_rx.await.expect("straggler should start");

        let drained = tasks
            .quiesce_within(std::time::Duration::from_millis(50))
            .await;
        assert!(
            !drained,
            "an uninterruptible straggler must be reported non-clean"
        );

        // The handle stayed tracked: a retry joins it once the blocking
        // call returns.
        let retry = tokio::time::timeout(
            std::time::Duration::from_secs(10),
            tasks.quiesce_within(std::time::Duration::from_secs(10)),
        )
        .await
        .expect("retry quiesce must return");
        assert!(retry, "retry must re-join the tracked straggler");
    }
}
