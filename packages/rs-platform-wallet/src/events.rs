//! Event handling for the platform wallet.
//!
//! [`PlatformEventHandler`] extends dash-spv's [`EventHandler`] with
//! platform-specific events. Applications implement this trait to receive
//! all events by reference (no cloning).
//!
//! [`PlatformEventManager`] dispatches events to a fixed handler set
//! provided at construction. It implements [`EventHandler`] so it can
//! be passed directly to `DashSpvClient`, mirroring dash-spv's own
//! immutable `event_handlers` model.

use std::sync::Arc;

pub use dash_spv::EventHandler;
pub use key_wallet_manager::WalletEvent;

use crate::manager::dpns_sync::DpnsSyncPassSummary;
use crate::manager::platform_address_sync::PlatformAddressSyncSummary;
#[cfg(feature = "shielded")]
use crate::manager::shielded_sync::ShieldedSyncPassSummary;

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
}

/// Dispatches events to a fixed set of [`PlatformEventHandler`]s.
///
/// Passed to `DashSpvClient` as the `EventHandler` (via `Arc<Self>`).
/// The handler set is supplied once at construction and never mutated,
/// matching the immutable `event_handlers` the wrapped dash-spv layer
/// consumes. Read path (every event): iterate the boxed slice.
pub struct PlatformEventManager {
    handlers: Arc<[Arc<dyn PlatformEventHandler>]>,
}

impl PlatformEventManager {
    /// Create a new event manager with the full handler set.
    pub fn new(handlers: Vec<Arc<dyn PlatformEventHandler>>) -> Self {
        Self {
            handlers: handlers.into(),
        }
    }

    /// Dispatch a platform-address sync completion to every handler.
    ///
    /// Not on the SPV hot path — called once per sync pass (~15s).
    pub fn on_platform_address_sync_completed(&self, summary: &PlatformAddressSyncSummary) {
        for h in self.handlers.iter() {
            h.on_platform_address_sync_completed(summary);
        }
    }

    /// Dispatch a DPNS marketplace sync completion to every handler.
    ///
    /// Not on the SPV hot path — called once per DPNS sync pass
    /// (~60s by default).
    pub fn on_dpns_marketplace_sync_completed(&self, summary: &DpnsSyncPassSummary) {
        for h in self.handlers.iter() {
            h.on_dpns_marketplace_sync_completed(summary);
        }
    }

    /// Dispatch a shielded sync completion to every handler.
    ///
    /// Not on the SPV hot path — called once per shielded sync pass
    /// (~60s by default).
    #[cfg(feature = "shielded")]
    pub fn on_shielded_sync_completed(&self, summary: &ShieldedSyncPassSummary) {
        for h in self.handlers.iter() {
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
        for h in self.handlers.iter() {
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
        for h in self.handlers.iter() {
            h.on_shielded_tree_progress(leaves_committed, total_target);
        }
    }
}

impl EventHandler for PlatformEventManager {
    fn on_sync_event(&self, event: &dash_spv::sync::SyncEvent) {
        for h in self.handlers.iter() {
            h.on_sync_event(event);
        }
    }

    fn on_network_event(&self, event: &dash_spv::network::NetworkEvent) {
        for h in self.handlers.iter() {
            h.on_network_event(event);
        }
    }

    fn on_progress(&self, progress: &dash_spv::sync::SyncProgress) {
        for h in self.handlers.iter() {
            h.on_progress(progress);
        }
    }

    fn on_wallet_event(&self, event: &WalletEvent) {
        for h in self.handlers.iter() {
            h.on_wallet_event(event);
        }
    }

    fn on_error(&self, error: &str) {
        for h in self.handlers.iter() {
            h.on_error(error);
        }
    }
}
