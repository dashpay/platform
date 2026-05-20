//! Event handler that wakes `AssetLockManager` async waiters on lock events.

use std::sync::Arc;

use dash_spv::EventHandler;
use tokio::sync::Notify;

use crate::events::PlatformEventHandler;

/// Wakes `AssetLockManager::wait_for_proof` / `wait_for_chain_lock`
/// when a lock event arrives from SPV. Registered as one of the handlers
/// in [`PlatformEventManager`](crate::events::PlatformEventManager).
pub struct LockNotifyHandler {
    notify: Arc<Notify>,
}

impl LockNotifyHandler {
    pub fn new(notify: Arc<Notify>) -> Self {
        Self { notify }
    }
}

impl EventHandler for LockNotifyHandler {
    fn on_sync_event(&self, event: &dash_spv::sync::SyncEvent) {
        // Demoted to `debug!` — these events arrive at network
        // frequency on a synced node (every 2.5min for CL on
        // mainnet, plus per-tx IS events). Operator visibility into
        // *what we did* with each event lives on the consumer side
        // (`wait_for_proof`); this handler just wakes them.
        match event {
            dash_spv::sync::SyncEvent::InstantLockReceived { .. } => {
                tracing::debug!("LockNotifyHandler: InstantLockReceived — waking waiters");
                self.notify.notify_waiters();
            }
            dash_spv::sync::SyncEvent::ChainLockReceived {
                chain_lock,
                validated,
            } => {
                tracing::debug!(
                    chain_lock_height = chain_lock.block_height,
                    validated,
                    "LockNotifyHandler: ChainLockReceived — waking waiters"
                );
                self.notify.notify_waiters();
            }
            _ => {}
        }
    }
}

impl PlatformEventHandler for LockNotifyHandler {}
