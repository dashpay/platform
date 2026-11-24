//! Event handling for the platform wallet.
//!
//! [`PlatformEventHandler`] extends dash-spv's [`EventHandler`] with
//! platform-specific events. Applications implement this trait to receive
//! all events by reference (no cloning).
//!
//! [`PlatformEventManager`] dispatches events to registered handlers.
//! It implements [`EventHandler`] so it can be passed directly to
//! `DashSpvClient`, and supports dynamic handler registration via
//! lock-free `ArcSwap`.

#[cfg(feature = "shielded")]
use std::any::Any;
#[cfg(feature = "shielded")]
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::Arc;

use arc_swap::ArcSwap;

use dash_spv::{SyncEvent, SyncProgress};

pub use dash_spv::EventHandler;
pub use key_wallet_manager::WalletEvent;

use crate::manager::dpns_sync::DpnsSyncPassSummary;
use crate::manager::platform_address_sync::PlatformAddressSyncSummary;
#[cfg(feature = "shielded")]
use crate::manager::shielded_sync::ShieldedSyncPassSummary;
#[cfg(feature = "shielded")]
use crate::wallet::shielded::ShieldedBalanceUpdate;

/// Extension of [`EventHandler`] for platform-wallet consumers.
///
/// Implementors receive all SPV events via the [`EventHandler`] supertrait,
/// plus platform-specific events via methods defined here.
pub trait PlatformEventHandler: EventHandler {
    /// Fired after each [`PlatformAddressSyncManager`] pass completes,
    /// including passes that produced no updates.
    ///
    /// Default impl is a no-op so existing handlers don't have to care.
    ///
    /// [`PlatformAddressSyncManager`]: crate::manager::platform_address_sync::PlatformAddressSyncManager
    fn on_platform_address_sync_completed(&self, _summary: &PlatformAddressSyncSummary) {}

    /// Fired after each [`DpnsSyncManager`] marketplace pass completes,
    /// including passes that produced no delta. Hosts refresh
    /// marketplace UI from the mirrored rows and — when the summary
    /// reports a name departing an identity — re-run their
    /// main-username selection / profile display for that identity.
    ///
    /// Default impl is a no-op so existing handlers don't have to care.
    ///
    /// [`DpnsSyncManager`]: crate::manager::dpns_sync::DpnsSyncManager
    fn on_dpns_marketplace_sync_completed(&self, _summary: &DpnsSyncPassSummary) {}

    /// Fired after each [`ShieldedSyncManager`] pass completes,
    /// including passes that produced no updates or skipped every
    /// wallet because none had a bound shielded sub-wallet yet.
    ///
    /// Called from inside the pass, before it counts as finished, so a
    /// stop or a Clear that drains the pass waits for this to return:
    /// hand the event on and return. Do not wait here for a shielded
    /// operation (a bind, an added account, a transfer): during a Clear it
    /// waits for the Clear while the Clear waits for this handler, until
    /// `PlatformWalletManager::clear_shielded` returns `ShutdownIncomplete`
    /// with the sync loop stopped.
    ///
    /// Default impl is a no-op so existing handlers don't have to
    /// care.
    ///
    /// [`ShieldedSyncManager`]: crate::manager::shielded_sync::ShieldedSyncManager
    #[cfg(feature = "shielded")]
    fn on_shielded_sync_completed(&self, _summary: &ShieldedSyncPassSummary) {}

    /// Fired periodically during a shielded sync pass — once per
    /// completed chunk inside `sync_shielded_notes`. Carries the
    /// cumulative count of encrypted notes scanned so far in the
    /// current pass and the latest block height observed.
    ///
    /// Network-scoped, not per-wallet: a single sync pass covers
    /// every IVK on the coordinator, so the "wallet that's
    /// progressing" isn't a meaningful concept at this granularity.
    /// Clients that bind a single wallet at a time can scope
    /// per-wallet from context.
    ///
    /// Lets clients render a live progress counter / `ProgressView`
    /// during long initial syncs (a cold sync of a 1M-note pool can
    /// take 20+ min in a single `sync_shielded_notes` call; without
    /// this event there's no signal between start and end).
    ///
    /// Default impl is a no-op.
    #[cfg(feature = "shielded")]
    fn on_shielded_sync_progress(&self, _cumulative_scanned: u64, _block_height: u64) {}

    /// Fired periodically during a shielded sync pass — once per
    /// committed batch as decrypted commitments are appended to the
    /// local Orchard commitment tree. This is the "checked /
    /// committed-to-tree" signal, distinct from
    /// [`on_shielded_sync_progress`](Self::on_shielded_sync_progress)
    /// (which counts *downloaded* notes): a note is only "checked"
    /// once its commitment has been appended to the tree.
    ///
    /// `leaves_committed` is the cumulative tree leaf count (starts at
    /// the pre-sync tree size and grows as commitments are appended).
    /// `total_target` is the on-chain MMR total leaf count, fetched
    /// once at pass start; `total_target == 0` means the count RPC
    /// was unavailable, so the progress is **indeterminate** and the
    /// host should render a spinner rather than a determinate bar.
    ///
    /// Network-scoped, not per-wallet: a single sync pass covers
    /// every IVK on the coordinator, so the "wallet that's
    /// progressing" isn't a meaningful concept at this granularity.
    ///
    /// Pairs with `on_shielded_sync_progress` to drive a dual
    /// ProgressView ("downloaded" vs "checked") during long cold
    /// syncs.
    ///
    /// Default impl is a no-op.
    #[cfg(feature = "shielded")]
    fn on_shielded_tree_progress(&self, _leaves_committed: u64, _total_target: u64) {}

    /// Fired when a wallet's local shielded balance state changes: the
    /// coordinator's balance emitter read
    /// `NetworkShieldedCoordinator::local_balance_snapshot` for the wallet
    /// and found a state that differs from the one it last delivered, or saw
    /// the wallet for the first time. Covers every cause, including a scan
    /// (completed or failed), a bind, an unregister (the state becomes
    /// `Unbound`), a Clear (successful or failed) and note reservations taken
    /// or released. An unchanged state is not delivered again.
    ///
    /// An update carries the latest state the emitter read, not every
    /// transition: changes that land before it reads again coalesce, so a
    /// reservation taken and released between two reads produces no update.
    /// `update.revision` identifies a delivered state, not a ledger mutation.
    /// It starts at 1 per wallet and strictly increases; a wallet's updates
    /// arrive in revision order, from one task, and none arrives once
    /// `PlatformWalletManager::shutdown` returned with the emitter joined. A
    /// host shows the latest state it received and keeps no balance of its
    /// own.
    ///
    /// No replay: an update is sent when the state changes, so a handler
    /// added after a wallet's state was delivered
    /// ([`PlatformEventManager::add_handler`]) hears about that wallet only
    /// at its next change, and reads
    /// `NetworkShieldedCoordinator::local_balance_snapshot` if it needs the
    /// current state before then.
    ///
    /// A panic in this method is caught and logged by
    /// [`PlatformEventManager::on_shielded_balance_changed`], so it does not
    /// stop later updates; where panics abort the process it is not caught.
    ///
    /// Default impl is a no-op.
    #[cfg(feature = "shielded")]
    fn on_shielded_balance_changed(&self, _update: &ShieldedBalanceUpdate) {}
}

/// Dispatches events to all registered [`PlatformEventHandler`]s.
///
/// Passed to `DashSpvClient` as the `EventHandler` (via `Arc<Self>`).
/// Supports dynamic handler registration via [`add_handler`](Self::add_handler).
///
/// Read path (every event): one atomic pointer load, then iterate.
/// Write path (add_handler): clone Vec + atomic swap — rare, not on SPV hot path.
pub struct PlatformEventManager {
    handlers: ArcSwap<Vec<Arc<dyn PlatformEventHandler>>>,
}

impl PlatformEventManager {
    /// Create a new event manager with initial handlers.
    pub fn new(handlers: Vec<Arc<dyn PlatformEventHandler>>) -> Self {
        Self {
            handlers: ArcSwap::from_pointee(handlers),
        }
    }

    /// Register an additional handler. Lock-free for readers.
    pub fn add_handler(&self, handler: Arc<dyn PlatformEventHandler>) {
        self.handlers.rcu(|current| {
            let mut new = (**current).clone();
            new.push(handler.clone());
            new
        });
    }

    /// Dispatch a platform-address sync completion to every handler.
    ///
    /// Not on the SPV hot path — called once per sync pass (~15s).
    pub fn on_platform_address_sync_completed(&self, summary: &PlatformAddressSyncSummary) {
        let handlers = self.handlers.load();
        for h in handlers.iter() {
            h.on_platform_address_sync_completed(summary);
        }
    }

    /// Dispatch a DPNS marketplace sync completion to every handler.
    ///
    /// Not on the SPV hot path — called once per DPNS sync pass
    /// (~60s by default).
    pub fn on_dpns_marketplace_sync_completed(&self, summary: &DpnsSyncPassSummary) {
        let handlers = self.handlers.load();
        for h in handlers.iter() {
            h.on_dpns_marketplace_sync_completed(summary);
        }
    }

    /// Dispatch a shielded sync completion to every handler.
    ///
    /// Not on the SPV hot path — called once per shielded sync pass
    /// (~60s by default).
    #[cfg(feature = "shielded")]
    pub fn on_shielded_sync_completed(&self, summary: &ShieldedSyncPassSummary) {
        let handlers = self.handlers.load();
        for h in handlers.iter() {
            h.on_shielded_sync_completed(summary);
        }
    }

    /// Dispatch a shielded sync progress event to every handler.
    ///
    /// Called from inside `sync_shielded_notes`'s chunk loop, once
    /// per chunk (~every 2048 notes processed). Cheap-but-frequent
    /// path during a cold sync.
    #[cfg(feature = "shielded")]
    pub fn on_shielded_sync_progress(&self, cumulative_scanned: u64, block_height: u64) {
        let handlers = self.handlers.load();
        for h in handlers.iter() {
            h.on_shielded_sync_progress(cumulative_scanned, block_height);
        }
    }

    /// Dispatch a shielded tree-progress ("checked / committed-to-tree")
    /// event to every handler.
    ///
    /// Called from inside `sync_notes_across`'s batch loop, once per
    /// committed batch as commitments are appended to the local
    /// Orchard tree. `total_target == 0` signals an indeterminate
    /// total (the on-chain MMR leaf count was unavailable). Cheap-but-
    /// frequent path during a cold sync.
    #[cfg(feature = "shielded")]
    pub fn on_shielded_tree_progress(&self, leaves_committed: u64, total_target: u64) {
        let handlers = self.handlers.load();
        for h in handlers.iter() {
            h.on_shielded_tree_progress(leaves_committed, total_target);
        }
    }

    /// Dispatch a shielded balance update to every handler.
    ///
    /// Called from the coordinator's balance emitter, once per delivered
    /// wallet state; the handlers registered now receive it, and a handler
    /// added later receives the next change, not this one (see
    /// [`PlatformEventHandler::on_shielded_balance_changed`]).
    ///
    /// Each handler call is isolated: the emitter is the only task that
    /// delivers balance updates, for every wallet, so a handler that panics
    /// must not unwind it. The panic is logged and the remaining handlers
    /// still receive the update.
    #[cfg(feature = "shielded")]
    pub fn on_shielded_balance_changed(&self, update: &ShieldedBalanceUpdate) {
        let handlers = self.handlers.load();
        for h in handlers.iter() {
            let delivered =
                catch_unwind(AssertUnwindSafe(|| h.on_shielded_balance_changed(update)));
            if let Err(payload) = delivered {
                tracing::error!(
                    wallet_id = %hex::encode(update.wallet_id),
                    revision = update.revision,
                    panic = panic_message(payload.as_ref()),
                    "a shielded balance handler panicked; the other handlers still receive \
                     this update and later ones are delivered"
                );
                dispose_panic_payload(payload);
            }
        }
    }
}

/// The text of a panic payload, for the log: the message of a `panic!` with
/// a string, a placeholder for any other payload.
#[cfg(feature = "shielded")]
fn panic_message(payload: &(dyn Any + Send)) -> &str {
    payload
        .downcast_ref::<&str>()
        .copied()
        .or_else(|| payload.downcast_ref::<String>().map(String::as_str))
        .unwrap_or("non-string panic payload")
}

/// Drop a caught panic payload inside the isolation boundary. A payload's
/// `Drop` is a handler's code too (`std::panic::panic_any`) and may panic;
/// that panic is caught here, and its own payload is forgotten rather than
/// dropped, since dropping it could panic again outside any guard.
#[cfg(feature = "shielded")]
fn dispose_panic_payload(payload: Box<dyn Any + Send>) {
    if let Err(second) = catch_unwind(AssertUnwindSafe(move || drop(payload))) {
        tracing::error!("a shielded balance handler's panic payload panicked when dropped");
        std::mem::forget(second);
    }
}

impl EventHandler for PlatformEventManager {
    fn on_sync_event(&self, event: &SyncEvent) {
        let handlers = self.handlers.load();
        for h in handlers.iter() {
            h.on_sync_event(event);
        }
    }

    fn on_network_event(&self, event: &dash_spv::network::NetworkEvent) {
        let handlers = self.handlers.load();
        for h in handlers.iter() {
            h.on_network_event(event);
        }
    }

    fn on_progress(&self, progress: &SyncProgress) {
        let handlers = self.handlers.load();
        for h in handlers.iter() {
            h.on_progress(progress);
        }
    }

    fn on_wallet_event(&self, event: &WalletEvent) {
        let handlers = self.handlers.load();
        for h in handlers.iter() {
            h.on_wallet_event(event);
        }
    }

    fn on_error(&self, error: &str) {
        let handlers = self.handlers.load();
        for h in handlers.iter() {
            h.on_error(error);
        }
    }
}

#[cfg(all(test, feature = "shielded"))]
mod tests {
    use super::*;

    use std::sync::Mutex;

    use crate::wallet::shielded::ShieldedLocalBalanceState;

    struct PanickingHandler;

    impl EventHandler for PanickingHandler {}

    impl PlatformEventHandler for PanickingHandler {
        fn on_shielded_balance_changed(&self, _update: &ShieldedBalanceUpdate) {
            panic!("host handler failed");
        }
    }

    #[derive(Default)]
    struct RecordingHandler {
        revisions: Mutex<Vec<u64>>,
    }

    impl EventHandler for RecordingHandler {}

    impl PlatformEventHandler for RecordingHandler {
        fn on_shielded_balance_changed(&self, update: &ShieldedBalanceUpdate) {
            self.revisions
                .lock()
                .expect("recorder")
                .push(update.revision);
        }
    }

    /// A panic payload whose own `Drop` panics.
    struct PanicOnDrop;

    impl Drop for PanicOnDrop {
        fn drop(&mut self) {
            panic!("panic payload dropped");
        }
    }

    struct PanicOnDropHandler;

    impl EventHandler for PanicOnDropHandler {}

    impl PlatformEventHandler for PanicOnDropHandler {
        fn on_shielded_balance_changed(&self, _update: &ShieldedBalanceUpdate) {
            std::panic::panic_any(PanicOnDrop);
        }
    }

    #[test]
    fn should_keep_delivering_past_a_handler_whose_panic_payload_panics_when_dropped() {
        let recorder = Arc::new(RecordingHandler::default());
        let manager = PlatformEventManager::new(vec![
            Arc::new(PanicOnDropHandler) as Arc<dyn PlatformEventHandler>,
            Arc::new(PanickingHandler) as Arc<dyn PlatformEventHandler>,
            Arc::clone(&recorder) as Arc<dyn PlatformEventHandler>,
        ]);
        for revision in [1, 2] {
            manager.on_shielded_balance_changed(&ShieldedBalanceUpdate {
                wallet_id: [2; 32],
                revision,
                state: ShieldedLocalBalanceState::Unbound,
            });
        }
        assert_eq!(*recorder.revisions.lock().expect("recorder"), vec![1, 2]);
    }

    #[test]
    fn should_keep_delivering_balance_updates_past_a_panicking_handler() {
        let recorder = Arc::new(RecordingHandler::default());
        let manager = PlatformEventManager::new(vec![
            Arc::new(PanickingHandler) as Arc<dyn PlatformEventHandler>,
            Arc::clone(&recorder) as Arc<dyn PlatformEventHandler>,
        ]);
        for revision in [1, 2] {
            manager.on_shielded_balance_changed(&ShieldedBalanceUpdate {
                wallet_id: [1; 32],
                revision,
                state: ShieldedLocalBalanceState::Unbound,
            });
        }
        assert_eq!(*recorder.revisions.lock().expect("recorder"), vec![1, 2]);
    }
}
