//! Event handler that updates lock-free `WalletBalance` atomics
//! when `WalletEvent::BalanceUpdated` fires.

use std::sync::Arc;

use dash_spv::EventHandler;
use key_wallet_manager::WalletManager;
use tokio::sync::RwLock;

use crate::events::{PlatformEventHandler, WalletEvent};
use crate::wallet::platform_wallet::PlatformWalletInfo;

/// Updates `PlatformWalletInfo.balance` atomics on `BalanceUpdated` events.
///
/// Registered in `PlatformWalletManager` handler list. On each
/// `BalanceUpdated` event, acquires a WalletManager read lock to find
/// the wallet's `Arc<WalletBalance>` and update it.
///
/// This only fires when balance actually changes (WalletManager compares
/// snapshots before/after), so the read lock acquisition is rare — not
/// on every block.
pub struct BalanceUpdateHandler {
    wallet_manager: Arc<RwLock<WalletManager<PlatformWalletInfo>>>,
}

impl BalanceUpdateHandler {
    pub fn new(wallet_manager: Arc<RwLock<WalletManager<PlatformWalletInfo>>>) -> Self {
        Self { wallet_manager }
    }
}

impl EventHandler for BalanceUpdateHandler {
    fn on_wallet_event(&self, event: &WalletEvent) {
        if let WalletEvent::BalanceUpdated {
            wallet_id,
            spendable,
            unconfirmed,
            immature,
            locked,
        } = event
        {
            // Try non-blocking read — if the lock is held (SPV processing),
            // the atomics will be updated on the next event.
            if let Some(wm) = self.wallet_manager.try_read().ok() {
                if let Some(info) = wm.get_wallet_info(wallet_id) {
                    info.balance.set(*spendable, *unconfirmed, *immature, *locked);
                }
            }
        }
    }
}

impl PlatformEventHandler for BalanceUpdateHandler {}
