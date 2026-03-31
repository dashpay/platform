//! Forwards SPV events from `DashSpvClient` to the unified `PlatformWalletEvent` channel.

use dash_spv::EventHandler;
use dash_spv::sync::ProgressPercentage;
use key_wallet_manager::WalletEvent;
use tokio::sync::broadcast;

use crate::events::{FinalityEvent, PlatformWalletEvent, SpvEvent};

/// Implements `dash_spv::EventHandler` to forward SPV events into the
/// platform wallet's unified `PlatformWalletEvent` broadcast channel.
pub(crate) struct SpvEventForwarder {
    event_tx: broadcast::Sender<PlatformWalletEvent>,
}

impl SpvEventForwarder {
    pub(crate) fn new(event_tx: broadcast::Sender<PlatformWalletEvent>) -> Self {
        Self { event_tx }
    }

    /// Best-effort send — drops the event if no receivers are listening.
    fn send(&self, event: PlatformWalletEvent) {
        let _ = self.event_tx.send(event);
    }
}

impl EventHandler for SpvEventForwarder {
    fn on_sync_event(&self, event: &dash_spv::sync::SyncEvent) {
        use dash_spv::sync::SyncEvent;
        match event {
            SyncEvent::SyncComplete { header_tip, .. } => {
                self.send(PlatformWalletEvent::Spv(SpvEvent::SyncComplete {
                    tip_height: *header_tip,
                }));
            }
            SyncEvent::ChainLockReceived { chain_lock, .. } => {
                self.send(PlatformWalletEvent::Finality(FinalityEvent::ChainLock {
                    height: chain_lock.block_height,
                }));
            }
            SyncEvent::InstantLockReceived { instant_lock, .. } => {
                self.send(PlatformWalletEvent::Finality(FinalityEvent::InstantLock {
                    txid: instant_lock.txid,
                }));
            }
            // Other sync events are logged but not forwarded — consumers don't need them.
            _ => {
                tracing::trace!("SPV sync event: {}", event.description());
            }
        }
    }

    fn on_network_event(&self, event: &dash_spv::network::NetworkEvent) {
        use dash_spv::network::NetworkEvent;
        match event {
            NetworkEvent::PeerConnected { address } => {
                self.send(PlatformWalletEvent::Spv(SpvEvent::PeerConnected {
                    address: address.to_string(),
                }));
            }
            NetworkEvent::PeerDisconnected { address } => {
                self.send(PlatformWalletEvent::Spv(SpvEvent::PeerDisconnected {
                    address: address.to_string(),
                }));
            }
            NetworkEvent::PeersUpdated { connected_count, .. } => {
                self.send(PlatformWalletEvent::Spv(SpvEvent::PeersUpdated {
                    connected_count: *connected_count,
                }));
            }
        }
    }

    fn on_progress(&self, progress: &dash_spv::sync::SyncProgress) {
        // Only forward meaningful progress (percentage > 0)
        let pct = progress.percentage();
        if pct > 0.0 {
            // Derive current/total heights from headers progress when available
            let (height, total) = progress
                .headers()
                .map(|h| (h.current_height(), h.target_height()))
                .unwrap_or((0, 0));

            self.send(PlatformWalletEvent::Spv(SpvEvent::SyncProgress {
                height,
                total,
                percentage: pct,
            }));
        }
    }

    fn on_wallet_event(&self, event: &WalletEvent) {
        self.send(PlatformWalletEvent::Wallet(event.clone()));
    }

    fn on_error(&self, error: &str) {
        tracing::error!("SPV error: {}", error);
    }
}
