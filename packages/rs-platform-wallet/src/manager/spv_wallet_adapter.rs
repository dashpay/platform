//! SPV wallet adapter implementing WalletInterface from key-wallet-manager.

use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};

use async_trait::async_trait;
use dashcore::{Address as DashAddress, Block, OutPoint, Transaction, Txid};
use key_wallet::transaction_checking::{BlockInfo, TransactionContext, WalletTransactionChecker};
use key_wallet::wallet::managed_wallet_info::wallet_info_interface::WalletInfoInterface;
use key_wallet_manager::{
    BlockProcessingResult, MempoolTransactionResult, WalletEvent, WalletInterface,
};
use tokio::sync::broadcast;

use crate::events::{PlatformWalletEvent, TransactionStatus};
use crate::wallet::PlatformWallet;

/// Adapter that bridges `PlatformWallet` to `key-wallet-manager`'s `WalletInterface`.
///
/// Used by `PlatformWalletManager` to integrate with `DashSpvClient`.
pub(crate) struct SpvWalletAdapter {
    wallet: PlatformWallet,
    event_tx: broadcast::Sender<WalletEvent>,
    platform_event_tx: broadcast::Sender<PlatformWalletEvent>,
    synced_height: AtomicU32,
    filter_committed_height: AtomicU32,
    /// Monotonic counter incremented when monitored addresses or watched outpoints change.
    /// SPV uses this to detect bloom filter staleness.
    monitor_revision: AtomicU64,
}

impl SpvWalletAdapter {
    /// Create a new adapter for a platform wallet.
    pub(crate) fn new(
        wallet: PlatformWallet,
        platform_event_tx: broadcast::Sender<PlatformWalletEvent>,
    ) -> Self {
        let (event_tx, _) = broadcast::channel(256);
        Self {
            wallet,
            event_tx,
            platform_event_tx,
            synced_height: AtomicU32::new(0),
            filter_committed_height: AtomicU32::new(0),
            monitor_revision: AtomicU64::new(0),
        }
    }

    /// Update transaction status in CoreWallet and emit event if changed.
    async fn track_status(&self, txid: Txid, new_status: TransactionStatus) {
        if let Some(old_status) = self
            .wallet
            .core
            .update_transaction_status(txid, new_status)
            .await
        {
            let _ = self
                .platform_event_tx
                .send(PlatformWalletEvent::TransactionStatusChanged {
                    txid,
                    old_status,
                    new_status,
                });
        }
    }
}

#[async_trait]
impl WalletInterface for SpvWalletAdapter {
    async fn process_block(&mut self, block: &Block, block_height: u32) -> BlockProcessingResult {
        let mut wallet = self.wallet.core.wallet.write().await;
        let mut wallet_info = self.wallet.core.wallet_info.write().await;

        let context = TransactionContext::InBlock(BlockInfo::new(
            block_height,
            block.header.block_hash(),
            block.header.time,
        ));

        let mut new_txids = Vec::new();
        let mut existing_txids = Vec::new();
        let mut new_addresses = Vec::new();

        for tx in &block.txdata {
            let result = wallet_info
                .check_core_transaction(tx, context, &mut wallet, true, true)
                .await;
            if result.is_relevant {
                if result.is_new_transaction {
                    new_txids.push(tx.txid());
                } else {
                    existing_txids.push(tx.txid());
                }
            }
            if !result.new_addresses.is_empty() {
                new_addresses.extend(result.new_addresses);
            }
        }

        self.synced_height.store(block_height, Ordering::Relaxed);

        // If we generated new addresses, bump the monitor revision so SPV
        // knows to rebuild the bloom filter.
        if !new_addresses.is_empty() {
            self.monitor_revision.fetch_add(1, Ordering::Relaxed);
        }

        // Track all relevant transactions as Confirmed.
        for txid in new_txids.iter().chain(existing_txids.iter()) {
            self.track_status(*txid, TransactionStatus::Confirmed).await;
        }

        BlockProcessingResult {
            new_txids,
            existing_txids,
            new_addresses,
        }
    }

    async fn process_mempool_transaction(
        &mut self,
        tx: &Transaction,
        is_instant_send: bool,
    ) -> MempoolTransactionResult {
        let mut wallet = self.wallet.core.wallet.write().await;
        let mut wallet_info = self.wallet.core.wallet_info.write().await;

        let context = if is_instant_send {
            TransactionContext::InstantSend
        } else {
            TransactionContext::Mempool
        };

        let result = wallet_info
            .check_core_transaction(tx, context, &mut wallet, true, false)
            .await;

        if !result.new_addresses.is_empty() {
            self.monitor_revision.fetch_add(1, Ordering::Relaxed);
        }

        // Track relevant mempool transactions.
        if result.is_relevant {
            let status = if is_instant_send {
                TransactionStatus::InstantSendLocked
            } else {
                TransactionStatus::Unconfirmed
            };
            self.track_status(tx.txid(), status).await;
        }

        MempoolTransactionResult {
            is_relevant: result.is_relevant,
            net_amount: result.total_received as i64 - result.total_sent as i64,
            is_outgoing: result.total_sent > result.total_received,
            addresses: Vec::new(),
            new_addresses: result.new_addresses,
        }
    }

    fn monitored_addresses(&self) -> Vec<DashAddress> {
        if let Ok(wallet_info) = self.wallet.core.wallet_info.try_read() {
            wallet_info.monitored_addresses()
        } else {
            Vec::new()
        }
    }

    fn watched_outpoints(&self) -> Vec<OutPoint> {
        if let Ok(wallet_info) = self.wallet.core.wallet_info.try_read() {
            wallet_info
                .get_spendable_utxos()
                .iter()
                .map(|utxo| utxo.outpoint)
                .collect()
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
        self.filter_committed_height
            .store(height, Ordering::Relaxed);
    }

    fn monitor_revision(&self) -> u64 {
        self.monitor_revision.load(Ordering::Relaxed)
    }

    fn process_instant_send_lock(&mut self, txid: Txid) {
        if let Ok(mut wallet_info) = self.wallet.core.wallet_info.try_write() {
            wallet_info.mark_instant_send_utxos(&txid);
        }
        // Update status — can't await in a sync method, so use try_write.
        if let Ok(mut statuses) = self.wallet.core.transaction_statuses.try_write() {
            let old = statuses.get(&txid).copied();
            let new_status = TransactionStatus::InstantSendLocked;
            if old.map_or(true, |old| new_status > old) {
                statuses.insert(txid, new_status);
                if let Some(old_status) = old {
                    let _ = self.platform_event_tx.send(
                        PlatformWalletEvent::TransactionStatusChanged {
                            txid,
                            old_status,
                            new_status,
                        },
                    );
                }
            }
        }
    }

    fn subscribe_events(&self) -> broadcast::Receiver<WalletEvent> {
        self.event_tx.subscribe()
    }

    async fn earliest_required_height(&self) -> u32 {
        if let Ok(wallet_info) = self.wallet.core.wallet_info.try_read() {
            wallet_info.birth_height()
        } else {
            0
        }
    }

    async fn describe(&self) -> String {
        format!(
            "SpvWalletAdapter(wallet_id={})",
            hex::encode(self.wallet.wallet_id())
        )
    }
}
