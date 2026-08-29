//! Bridges `PlatformEventHandler` callbacks to async waiters.
//!
//! [`WaitEventHub`] is installed as the harness's
//! `PlatformEventHandler`. Every SPV / wallet / platform-address
//! sync event calls [`Notify::notify_waiters`]; helpers like
//! [`super::wait::wait_for_balance`] capture `Notified` BEFORE
//! polling so notifications arriving mid-sync aren't lost.
//!
//! Ignored: `on_progress` (per-header-batch noise) and `on_error`
//! (surfaced through tracing; no testable state change).

use platform_wallet::events::{EventHandler, PlatformEventHandler, WalletEvent};
use platform_wallet::PlatformAddressSyncSummary;
use tokio::sync::futures::Notified;
use tokio::sync::Notify;

/// `Notify`-based hub that fans test-relevant events out to async
/// waiters.
///
/// One instance per [`super::harness::E2eContext`]; clone the `Arc`
/// into every [`super::wallet_factory::TestWallet`] via
/// [`super::harness::E2eContext::wait_hub`].
pub struct WaitEventHub {
    notify: Notify,
}

impl WaitEventHub {
    /// Build an empty hub.
    pub fn new() -> Self {
        Self {
            notify: Notify::new(),
        }
    }

    /// Future that resolves the next time *any* relevant event
    /// fires. Pin (e.g. `tokio::pin!`) before awaiting so
    /// notifications arriving between registration and await aren't
    /// dropped.
    pub fn notified(&self) -> Notified<'_> {
        self.notify.notified()
    }

    /// Wake every registered waiter. Test-only nudge for non-event
    /// state changes (e.g. manual cache pokes).
    pub fn notify_all(&self) {
        self.notify.notify_waiters();
    }
}

impl Default for WaitEventHub {
    fn default() -> Self {
        Self::new()
    }
}

impl EventHandler for WaitEventHub {
    fn on_sync_event(&self, _event: &dash_spv::sync::SyncEvent) {
        self.notify.notify_waiters();
    }

    fn on_network_event(&self, _event: &dash_spv::network::NetworkEvent) {
        self.notify.notify_waiters();
    }

    fn on_wallet_event(&self, _event: &WalletEvent) {
        self.notify.notify_waiters();
    }
}

impl PlatformEventHandler for WaitEventHub {
    fn on_platform_address_sync_completed(&self, _summary: &PlatformAddressSyncSummary) {
        self.notify.notify_waiters();
    }
}
