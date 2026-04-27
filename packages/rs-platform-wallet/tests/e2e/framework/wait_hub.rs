//! Event hub that bridges `PlatformEventHandler` callbacks to async waiters.
//!
//! [`WaitEventHub`] is installed as the test harness's app-level
//! `PlatformEventHandler` (see [`super::harness::E2eContext::build`]).
//! Whenever the SPV / wallet / platform-address-sync subsystems fire an
//! event that might change a wallet's observable state, the hub calls
//! [`tokio::sync::Notify::notify_waiters`]. Async helpers like
//! [`super::wait::wait_for_balance`] grab a [`tokio::sync::Notify::notified`]
//! future *before* polling, so a notification arriving mid-sync isn't
//! lost — that's the whole reason the polling version had to keep
//! waking on a fixed interval.
//!
//! Events that are intentionally ignored:
//!
//! - `on_progress` — fires on every header batch; far too noisy and
//!   irrelevant to the conditions tests wait on.
//! - `on_error` — surfaced through tracing; doesn't itself indicate a
//!   testable state change.

use platform_wallet::events::{EventHandler, PlatformEventHandler, WalletEvent};
use platform_wallet::platform_address_sync::PlatformAddressSyncSummary;
use tokio::sync::futures::Notified;
use tokio::sync::Notify;

/// Notify-based hub that fans test-relevant SPV / platform events out to
/// async waiters.
///
/// Construct one per [`super::harness::E2eContext`] and clone the `Arc`
/// into every [`super::wallet_factory::TestWallet`] via
/// [`super::harness::E2eContext::wait_hub`].
pub struct WaitEventHub {
    notify: Notify,
}

impl WaitEventHub {
    /// Build an empty hub. No waiters until callers grab a
    /// [`Self::notified`] future.
    pub fn new() -> Self {
        Self {
            notify: Notify::new(),
        }
    }

    /// Get a future that resolves the next time *any* relevant event
    /// fires. Pin it (e.g. via `tokio::pin!`) before awaiting — the
    /// reborrow pattern is what guarantees notifications arriving
    /// between "register interest" and "await" aren't dropped.
    pub fn notified(&self) -> Notified<'_> {
        self.notify.notified()
    }

    /// Wake every currently-registered waiter. Test-only helper for
    /// scenarios that need to nudge `wait_for_balance` after a non-event
    /// state change (e.g. a manual cache poke). Not used by the default
    /// e2e flow.
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
