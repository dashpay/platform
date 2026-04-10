//! Event handling for the platform wallet.
//!
//! [`PlatformEventHandler`] extends dash-spv's [`EventHandler`] with
//! platform-specific events. Applications implement this trait to receive
//! all events by reference (no cloning).
//!
//! [`PlatformEventManager`] dispatches events to registered handlers.
//! It implements [`EventHandler`] so it can be passed directly to
//! `DashSpvClient`, and exposes `emit_*` methods for platform-internal events.

use std::sync::Arc;

pub use dash_spv::EventHandler;
pub use key_wallet_manager::WalletEvent;

/// Extension of [`EventHandler`] for platform-wallet consumers.
///
/// Implementors receive all SPV events via the [`EventHandler`] supertrait,
/// plus platform-specific events via methods defined here.
pub trait PlatformEventHandler: EventHandler {
    // Platform-specific event methods will be added here as needed
    // (e.g. on_identity_registered, on_asset_lock_finalized, etc.)
}

/// Dispatches events to all registered [`PlatformEventHandler`]s.
///
/// Passed to `DashSpvClient` as the `EventHandler` (via `Arc<Self>`).
/// Also held by platform-wallet components to emit platform-internal events.
pub struct PlatformEventManager {
    handlers: Vec<Arc<dyn PlatformEventHandler>>,
}

impl PlatformEventManager {
    /// Create a new event manager with the given handlers.
    pub fn new(handlers: Vec<Arc<dyn PlatformEventHandler>>) -> Self {
        Self { handlers }
    }
}

impl EventHandler for PlatformEventManager {
    fn on_sync_event(&self, event: &dash_spv::sync::SyncEvent) {
        for h in &self.handlers {
            h.on_sync_event(event);
        }
    }

    fn on_network_event(&self, event: &dash_spv::network::NetworkEvent) {
        for h in &self.handlers {
            h.on_network_event(event);
        }
    }

    fn on_progress(&self, progress: &dash_spv::sync::SyncProgress) {
        for h in &self.handlers {
            h.on_progress(progress);
        }
    }

    fn on_wallet_event(&self, event: &WalletEvent) {
        for h in &self.handlers {
            h.on_wallet_event(event);
        }
    }

    fn on_error(&self, error: &str) {
        for h in &self.handlers {
            h.on_error(error);
        }
    }
}
