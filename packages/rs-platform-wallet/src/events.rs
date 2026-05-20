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

use std::sync::Arc;

use arc_swap::ArcSwap;

pub use dash_spv::EventHandler;
pub use key_wallet_manager::WalletEvent;

use crate::manager::load_outcome::SkipReason;
use crate::manager::platform_address_sync::PlatformAddressSyncSummary;
#[cfg(feature = "shielded")]
use crate::manager::shielded_sync::ShieldedSyncPassSummary;
use crate::wallet::platform_wallet::WalletId;

/// Platform-wallet lifecycle event surfaced to app handlers.
///
/// Distinct from the SPV `EventHandler` stream — these are
/// platform-specific notifications the app may react to (toast,
/// telemetry) without threading return values through every call site.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlatformEvent {
    /// A persisted wallet was skipped during
    /// [`load_from_persistor`](crate::PlatformWalletManager::load_from_persistor)
    /// because its seed material was unavailable. Recoverable: re-run
    /// the load once the operator provides / unlocks the material.
    ///
    /// Carries the (public, non-secret) wallet id and the structural
    /// [`SkipReason`]; never any secret byte.
    WalletSkippedOnLoad {
        /// The skipped wallet's id.
        wallet_id: WalletId,
        /// Why it was skipped (seed absent / store locked / store
        /// error). Never a wrong-seed reason.
        reason: SkipReason,
    },
}

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

    /// Fired once per wallet that
    /// [`load_from_persistor`](crate::PlatformWalletManager::load_from_persistor)
    /// skipped because its seed was unavailable.
    ///
    /// Default impl is a no-op so existing handlers don't have to care
    /// (the internal `LockNotifyHandler` / `BalanceUpdateHandler`
    /// ignore it; only the app handler typically reacts).
    fn on_platform_event(&self, _event: &PlatformEvent) {}
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

    /// Dispatch a [`PlatformEvent`] to every handler.
    ///
    /// Not on the SPV hot path — called at most once per wallet during
    /// a single `load_from_persistor` pass.
    pub fn on_platform_event(&self, event: &PlatformEvent) {
        let handlers = self.handlers.load();
        for h in handlers.iter() {
            h.on_platform_event(event);
        }
    }
}

impl EventHandler for PlatformEventManager {
    fn on_sync_event(&self, event: &dash_spv::sync::SyncEvent) {
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

    fn on_progress(&self, progress: &dash_spv::sync::SyncProgress) {
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
