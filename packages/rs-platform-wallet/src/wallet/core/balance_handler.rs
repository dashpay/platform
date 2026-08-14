//! Event handler that updates lock-free `WalletBalance` atomics
//! when an upstream `WalletEvent` carries a fresh balance snapshot.

use std::collections::BTreeMap;
use std::sync::Arc;

use dash_spv::EventHandler;
use tokio::sync::RwLock;

use crate::events::{PlatformEventHandler, WalletEvent};
use crate::wallet::platform_wallet::WalletId;
use crate::wallet::PlatformWallet;

/// Updates `PlatformWallet`'s lock-free `WalletBalance` atomics when an
/// upstream `WalletEvent` carries a balance snapshot.
///
/// Upstream's atomic event design embeds the post-change `WalletCoreBalance`
/// in every variant that mutates balance — `TransactionDetected`,
/// `TransactionInstantLocked`, and `BlockProcessed`. `SyncHeightAdvanced`
/// alone has no balance (it's a checkpoint marker, not a state change).
/// This handler routes each balance-bearing event into the per-wallet
/// `WalletBalance` atomics so SwiftUI subscribers see the new totals.
///
/// Registered in `PlatformWalletManager`'s handler list. The handler
/// holds an `Arc` clone of the manager's `wallets` map (a *separate*
/// lock from the heavily-contended `wallet_manager` SPV write lock).
/// SPV holds the wallet-manager write lock for the entire duration of
/// block processing — looking the balance up through *that* lock would
/// silently lose every event during initial sync. The wallets map is
/// only written by manager lifecycle methods (`create_wallet_from_*`,
/// `remove_wallet`), so a `try_read()` here essentially never contends.
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
        let (wallet_id, balance) = match event {
            WalletEvent::TransactionDetected {
                wallet_id, balance, ..
            }
            | WalletEvent::TransactionInstantLocked {
                wallet_id, balance, ..
            }
            | WalletEvent::BlockProcessed {
                wallet_id, balance, ..
            }
            // A sweep is the one event that can lower the balance: the
            // removed transactions' outputs are gone from the UTXO set.
            // The snapshot it carries is post-removal, like every other
            // variant's, so it routes identically — dropping it would
            // leave the corrected-away amount on screen until the next
            // balance-bearing event happened to arrive.
            | WalletEvent::TransactionsSwept {
                wallet_id, balance, ..
            } => (wallet_id, balance),
            // No balance on SyncHeightAdvanced — checkpoint advance only.
            WalletEvent::SyncHeightAdvanced { .. } => return,
            // No balance on ChainLockProcessed — chainlocks only
            // promote finality (`InBlock` → `InChainLockedBlock`)
            // and/or advance `last_applied_chain_lock`; neither
            // changes UTXO state or balances.
            WalletEvent::ChainLockProcessed { .. } => return,
        };

        // try_read on the wallets map (NOT the wallet_manager
        // SPV-contended lock). The map is only written by manager
        // lifecycle methods, so this almost never contends.
        let Ok(wallets) = self.wallets.try_read() else {
            tracing::debug!(
                wallet = %hex::encode(wallet_id),
                "Wallet balance update dropped: wallets-map lock contended"
            );
            return;
        };
        if let Some(pw) = wallets.get(wallet_id) {
            pw.balance().set(
                balance.confirmed(),
                balance.unconfirmed(),
                balance.immature(),
                balance.locked(),
            );
        }
    }
}

impl PlatformEventHandler for BalanceUpdateHandler {}
