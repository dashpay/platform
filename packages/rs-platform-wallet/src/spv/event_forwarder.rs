//! Forwards SPV events from `DashSpvClient` to the unified `PlatformWalletEvent`
//! broadcast channel.
//!
//! This forwarder exists because platform-wallet needs SPV events internally
//! (e.g. `AssetLockManager::wait_for_proof` subscribes for InstantLock/ChainLock
//! events). A broadcast channel allows multiple consumers to subscribe:
//!
//! - **AssetLockManager** — listens for finality proofs during asset lock lifecycle
//! - **Application** (e.g. evo-tool) — subscribes via `PlatformWalletManager::subscribe_events()`
//!   for status display, connection health, and wallet reconciliation
//!
//! Accepting a custom `EventHandler` from the app instead would prevent
//! platform-wallet's own components from receiving events.

use dash_spv::EventHandler;
use key_wallet_manager::WalletEvent;
use tokio::sync::broadcast;

use crate::events::{PlatformWalletEvent, SpvEvent};

// TODO: Clonning events bad idea, better call event handlers
/*
impl EventHandler for SpvEventForwarder {
    fn on_sync_event(&self, event: &SyncEvent) {
        // Call each registered listener by reference — no clone
        for listener in &self.sync_listeners {
            listener.on_sync_event(event);
        }
    }
}
 */

/// Implements `dash_spv::EventHandler` to forward SPV events into the
/// platform wallet's unified `PlatformWalletEvent` broadcast channel.
pub(crate) struct SpvEventForwarder {
    event_tx: broadcast::Sender<PlatformWalletEvent>,
}

impl SpvEventForwarder {
    pub(crate) fn new(event_tx: broadcast::Sender<PlatformWalletEvent>) -> Self {
        Self { event_tx }
    }

    fn send(&self, event: PlatformWalletEvent) {
        let _ = self.event_tx.send(event);
    }
}

impl EventHandler for SpvEventForwarder {
    fn on_sync_event(&self, event: &dash_spv::sync::SyncEvent) {
        self.send(PlatformWalletEvent::Spv(SpvEvent::Sync(event.clone())));
    }

    fn on_network_event(&self, event: &dash_spv::network::NetworkEvent) {
        self.send(PlatformWalletEvent::Spv(SpvEvent::Network(event.clone())));
    }

    fn on_progress(&self, progress: &dash_spv::sync::SyncProgress) {
        self.send(PlatformWalletEvent::Spv(SpvEvent::Progress(
            progress.clone(),
        )));
    }

    fn on_wallet_event(&self, event: &WalletEvent) {
        self.send(PlatformWalletEvent::Wallet(event.clone()));
    }

    fn on_error(&self, error: &str) {
        tracing::error!("SPV error: {}", error);
    }
}
