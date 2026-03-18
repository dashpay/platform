//! SPV wallet adapter implementing WalletInterface from key-wallet-manager.

use std::sync::atomic::{AtomicU32, Ordering};

use async_trait::async_trait;
use dashcore::{Address as DashAddress, Block};
use key_wallet::transaction_checking::WalletTransactionChecker;
use key_wallet::wallet::managed_wallet_info::wallet_info_interface::WalletInfoInterface;
use key_wallet_manager::wallet_interface::WalletInterface;
use key_wallet_manager::WalletEvent;
use tokio::sync::broadcast;

use crate::wallet::PlatformWallet;

/// Adapter that bridges `PlatformWallet` to `key-wallet-manager`'s `WalletInterface`.
///
/// Used by `PlatformWalletManager` to integrate with `DashSpvClient`.
pub(crate) struct SpvWalletAdapter {
    wallet: PlatformWallet,
    event_tx: broadcast::Sender<WalletEvent>,
    synced_height: AtomicU32,
    filter_committed_height: AtomicU32,
}

impl SpvWalletAdapter {
    /// Create a new adapter for a platform wallet.
    #[allow(dead_code)]
    pub(crate) fn new(wallet: PlatformWallet) -> Self {
        let (event_tx, _) = broadcast::channel(256);
        Self {
            wallet,
            event_tx,
            synced_height: AtomicU32::new(0),
            filter_committed_height: AtomicU32::new(0),
        }
    }
}

#[async_trait]
impl WalletInterface for SpvWalletAdapter {
    async fn process_block(
        &mut self,
        block: &Block,
        block_height: u32,
    ) -> key_wallet_manager::BlockProcessingResult {
        use key_wallet::transaction_checking::TransactionContext;

        // Lock ordering invariant: always acquire `wallet` before `wallet_info`
        // to prevent deadlocks when other code paths also need both locks.
        let wallet = self.wallet.core.wallet.read().await;
        let mut wallet_info = self.wallet.core.wallet_info.write().await;

        let context = TransactionContext::InBlock {
            block_hash: Some(block.header.block_hash()),
            height: block_height,
            timestamp: Some(block.header.time),
        };

        let mut new_txids = Vec::new();
        let mut existing_txids = Vec::new();
        let mut new_addresses = Vec::new();

        for tx in &block.txdata {
            let result = wallet_info
                .check_core_transaction(tx, context, &wallet, true)
                .await;
            if result.is_relevant {
                new_txids.push(tx.txid());
            }
        }

        self.synced_height.store(block_height, Ordering::Relaxed);

        key_wallet_manager::BlockProcessingResult {
            new_txids,
            existing_txids,
            new_addresses,
        }
    }

    async fn process_mempool_transaction(&mut self, tx: &dashcore::Transaction) {
        use key_wallet::transaction_checking::TransactionContext;

        let wallet = self.wallet.core.wallet.read().await;
        let mut wallet_info = self.wallet.core.wallet_info.write().await;

        let context = TransactionContext::Mempool {};
        let _ = wallet_info
            .check_core_transaction(tx, context, &wallet, false)
            .await;
    }

    fn monitored_addresses(&self) -> Vec<DashAddress> {
        if let Ok(wallet_info) = self.wallet.core.wallet_info.try_read() {
            wallet_info.monitored_addresses()
        } else {
            Vec::new()
        }
    }

    fn synced_height(&self) -> u32 {
        self.synced_height.load(Ordering::Relaxed)
    }

    fn update_synced_height(&mut self, height: u32) {
        self.synced_height.store(height, Ordering::Relaxed);
    }

    fn filter_committed_height(&self) -> u32 {
        self.filter_committed_height.load(Ordering::Relaxed)
    }

    fn update_filter_committed_height(&mut self, height: u32) {
        self.filter_committed_height.store(height, Ordering::Relaxed);
    }

    fn subscribe_events(&self) -> broadcast::Receiver<WalletEvent> {
        self.event_tx.subscribe()
    }

    async fn earliest_required_height(&self) -> u32 {
        0
    }

    async fn describe(&self) -> String {
        format!(
            "SpvWalletAdapter(wallet_id={})",
            hex::encode(self.wallet.wallet_id())
        )
    }
}
