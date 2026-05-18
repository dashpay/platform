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
