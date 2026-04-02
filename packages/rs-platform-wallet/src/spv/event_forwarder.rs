//! Forwards SPV events from `DashSpvClient` to the unified `PlatformWalletEvent` channel.

use dash_spv::EventHandler;
use key_wallet_manager::WalletEvent;
use tokio::sync::broadcast;

use crate::events::{PlatformWalletEvent, SpvEvent};

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
        self.send(PlatformWalletEvent::Spv(SpvEvent::Progress(progress.clone())));
    }

    fn on_wallet_event(&self, event: &WalletEvent) {
        self.send(PlatformWalletEvent::Wallet(event.clone()));
    }

    fn on_error(&self, error: &str) {
        tracing::error!("SPV error: {}", error);
    }
}
