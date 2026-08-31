//! Event handler that updates lock-free `WalletBalance` atomics
//! when an upstream `WalletEvent` carries a fresh balance snapshot.

use std::collections::BTreeMap;
use std::sync::Arc;

use arc_swap::ArcSwap;
use dash_spv::EventHandler;

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
/// structure from the heavily-contended `wallet_manager` SPV write
/// lock). SPV holds the wallet-manager write lock for the entire
/// duration of block processing — looking the balance up through *that*
/// lock would silently lose every event during initial sync.
///
/// The map is an [`ArcSwap`] so this lookup is wait-free and can never
/// fail: `load()` always returns the latest published map, even while a
/// manager lifecycle write (wallet insert / remove / load) is publishing
/// a new one. That infallibility is load-bearing, not a convenience.
/// `on_wallet_event` is synchronous and the bus neither retries nor
/// coalesces, so a snapshot missed here is gone for good: nothing
/// guarantees a later event carries the same correction, and until one
/// does the wallet displays superseded totals. A fallible lookup (the
/// previous `RwLock::try_read`) dropped exactly that snapshot whenever
/// it raced a lifecycle write.
pub struct BalanceUpdateHandler {
    wallets: Arc<ArcSwap<BTreeMap<WalletId, Arc<PlatformWallet>>>>,
}

impl BalanceUpdateHandler {
    pub fn new(wallets: Arc<ArcSwap<BTreeMap<WalletId, Arc<PlatformWallet>>>>) -> Self {
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
            } => (wallet_id, balance),
            // No balance on SyncHeightAdvanced — checkpoint advance only.
            WalletEvent::SyncHeightAdvanced { .. } => return,
            // No balance on ChainLockProcessed — chainlocks only
            // promote finality (`InBlock` → `InChainLockedBlock`)
            // and/or advance `last_applied_chain_lock`; neither
            // changes UTXO state or balances.
            WalletEvent::ChainLockProcessed { .. } => return,
        };

        // Wait-free snapshot of the wallets map; cannot fail or block,
        // so no balance-bearing event is ever dropped here. A wallet
        // not in the snapshot is one registered concurrently with this
        // event — its creation path re-seeds the balance atomics from
        // the inner wallet after publishing it, covering that window.
        let wallets = self.wallets.load();
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
