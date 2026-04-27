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
    /// Build a notify-handler that wakes waiters on the given `Notify`
    /// when an InstantSend or ChainLock event arrives from SPV.
    pub fn new(notify: Arc<Notify>) -> Self {
        Self { notify }
    }
}

impl EventHandler for LockNotifyHandler {
    fn on_sync_event(&self, event: &dash_spv::sync::SyncEvent) {
        if matches!(
            event,
            dash_spv::sync::SyncEvent::InstantLockReceived { .. }
                | dash_spv::sync::SyncEvent::ChainLockReceived { .. }
        ) {
            self.notify.notify_waiters();
        }
    }
}

impl PlatformEventHandler for LockNotifyHandler {}
