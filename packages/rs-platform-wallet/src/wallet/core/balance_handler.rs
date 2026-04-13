//! Event handler that updates lock-free `WalletBalance` atomics
//! when `WalletEvent::BalanceUpdated` fires.

use std::collections::BTreeMap;
use std::sync::Arc;

use dash_spv::EventHandler;
use tokio::sync::RwLock;

use crate::events::{PlatformEventHandler, WalletEvent};
use crate::wallet::platform_wallet::WalletId;
use crate::wallet::PlatformWallet;

/// Updates `PlatformWallet`'s lock-free `WalletBalance` atomics on
/// `BalanceUpdated` events.
///
/// Registered in `PlatformWalletManager` handler list. The handler
/// holds an `Arc` clone of the manager's `wallets` map (a *separate*
/// lock from the heavily-contended `wallet_manager` SPV write lock).
/// SPV holds the wallet-manager write lock for the entire duration of
/// block processing — looking the balance up through *that* lock would
/// silently lose every event during initial sync. The wallets map is
/// only written by manager lifecycle methods (`create_wallet_from_*`,
/// `remove_wallet`), so a `try_read()` here essentially never
/// contends.
pub struct BalanceUpdateHandler {
    wallets: Arc<RwLock<BTreeMap<WalletId, Arc<PlatformWallet>>>>,
}

impl BalanceUpdateHandler {
    pub fn new(wallets: Arc<RwLock<BTreeMap<WalletId, Arc<PlatformWallet>>>>) -> Self {
        Self { wallets }
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
            // try_read on the wallets map (NOT the wallet_manager
            // SPV-contended lock). The map is only written by manager
            // lifecycle methods, so this almost never contends.
            let Ok(wallets) = self.wallets.try_read() else {
                tracing::debug!(
                    wallet = %hex::encode(wallet_id),
                    "BalanceUpdated dropped: wallets-map lock contended"
                );
                return;
            };
            if let Some(pw) = wallets.get(wallet_id) {
                pw.balance()
                    .set(*spendable, *unconfirmed, *immature, *locked);
            }
        }
    }
}

impl PlatformEventHandler for BalanceUpdateHandler {}
