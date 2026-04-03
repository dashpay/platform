//! SPV wallet adapter implementing WalletInterface from key-wallet-manager.
//!
//! Bridges the entire wallet collection to `DashSpvClient` — processes blocks
//! and mempool transactions against ALL managed wallets, not just one.

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::sync::Arc;

use async_trait::async_trait;
use dashcore::{Address as DashAddress, Block, OutPoint, Transaction, Txid};
use key_wallet::transaction_checking::{BlockInfo, TransactionContext, WalletTransactionChecker};
use key_wallet::wallet::managed_wallet_info::wallet_info_interface::WalletInfoInterface;
use key_wallet_manager::{
    BlockProcessingResult, MempoolTransactionResult, WalletEvent, WalletInterface,
};
use tokio::sync::{broadcast, RwLock};

use crate::events::{PlatformWalletEvent, TransactionStatus};
use crate::wallet::platform_wallet::WalletId;
use crate::wallet::PlatformWallet;

/// Adapter that bridges ALL managed `PlatformWallet`s to `key-wallet-manager`'s
/// `WalletInterface`.
///
/// When a block or mempool transaction arrives, the adapter iterates every
/// wallet in the collection and checks the transaction against each wallet's
/// `ManagedWalletInfo`. This ensures all wallets see incoming transactions
/// regardless of which wallet was added first.
pub(crate) struct SpvWalletAdapter {
    wallets: Arc<RwLock<BTreeMap<WalletId, PlatformWallet>>>,
    event_tx: broadcast::Sender<WalletEvent>,
    platform_event_tx: broadcast::Sender<PlatformWalletEvent>,
    synced_height: AtomicU32,
    filter_committed_height: AtomicU32,
    /// Shared with `SpvRuntime` so wallet add/remove can bump it externally.
    monitor_revision: Arc<AtomicU64>,
}

impl SpvWalletAdapter {
    pub(crate) fn new(
        wallets: Arc<RwLock<BTreeMap<WalletId, PlatformWallet>>>,
        platform_event_tx: broadcast::Sender<PlatformWalletEvent>,
        monitor_revision: Arc<AtomicU64>,
    ) -> Self {
        let (event_tx, _) = broadcast::channel(256);
        Self {
            wallets,
            event_tx,
            platform_event_tx,
            synced_height: AtomicU32::new(0),
            filter_committed_height: AtomicU32::new(0),
            monitor_revision,
        }
    }

    /// Update transaction status in a wallet's CoreWallet.
    async fn track_status_for_wallet(
        &self,
        wallet: &PlatformWallet,
        txid: Txid,
        new_status: TransactionStatus,
    ) {
        wallet
            .core
            .update_transaction_status(txid, new_status)
            .await;
    }
}

#[async_trait]
impl WalletInterface for SpvWalletAdapter {
    async fn process_block(&mut self, block: &Block, block_height: u32) -> BlockProcessingResult {
        let wallets = self.wallets.read().await;

        let context = TransactionContext::InBlock(BlockInfo::new(
            block_height,
            block.header.block_hash(),
            block.header.time,
        ));

        let mut new_txids = Vec::new();
        let mut existing_txids = Vec::new();
        let mut new_addresses = Vec::new();

        for wallet in wallets.values() {
            let mut w = wallet.core.wallet.write().await;
            let mut wi = wallet.core.wallet_info_mut().await;

            for tx in &block.txdata {
                let result = wi
                    .check_core_transaction(tx, context, &mut w, true, true)
                    .await;
                if result.is_relevant {
                    let txid = tx.txid();
                    if result.is_new_transaction {
                        if !new_txids.contains(&txid) {
                            new_txids.push(txid);
                        }
                    } else if !existing_txids.contains(&txid) {
                        existing_txids.push(txid);
                    }
                }
                if !result.new_addresses.is_empty() {
                    new_addresses.extend(result.new_addresses);
                }
            }
        }

        self.synced_height.store(block_height, Ordering::Relaxed);

        if !new_addresses.is_empty() {
            self.monitor_revision.fetch_add(1, Ordering::Relaxed);
        }

        // Track all relevant transactions as Confirmed and refresh cached balance.
        for wallet in wallets.values() {
            for txid in new_txids.iter().chain(existing_txids.iter()) {
                self.track_status_for_wallet(wallet, *txid, TransactionStatus::Confirmed)
                    .await;
            }
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
        let wallets = self.wallets.read().await;

        let context = if is_instant_send {
            TransactionContext::InstantSend
        } else {
            TransactionContext::Mempool
        };

        let mut combined = MempoolTransactionResult::default();

        for wallet in wallets.values() {
            let mut w = wallet.core.wallet.write().await;
            let mut wi = wallet.core.wallet_info_mut().await;

            let result = wi
                .check_core_transaction(tx, context, &mut w, true, false)
                .await;

            if result.is_relevant {
                combined.is_relevant = true;
                combined.net_amount += result.total_received as i64 - result.total_sent as i64;
                if result.total_sent > result.total_received {
                    combined.is_outgoing = true;
                }

                let status = if is_instant_send {
                    TransactionStatus::InstantSendLocked
                } else {
                    TransactionStatus::Unconfirmed
                };
                self.track_status_for_wallet(wallet, tx.txid(), status)
                    .await;
            }

            if result.is_relevant {
            }

            if !result.new_addresses.is_empty() {
                self.monitor_revision.fetch_add(1, Ordering::Relaxed);
                combined.new_addresses.extend(result.new_addresses);
            }
        }

        combined
    }

    fn monitored_addresses(&self) -> Vec<DashAddress> {
        if let Ok(wallets) = self.wallets.try_read() {
            wallets
                .values()
                .flat_map(|w| {
                    w.core
                        .try_wallet_info()
                        .map(|wi| wi.monitored_addresses())
                        .unwrap_or_default()
                })
                .collect()
        } else {
            Vec::new()
        }
    }

    fn watched_outpoints(&self) -> Vec<OutPoint> {
        if let Ok(wallets) = self.wallets.try_read() {
            wallets
                .values()
                .flat_map(|w| {
                    w.core
                        .try_wallet_info()
                        .map(|wi| {
                            wi.get_spendable_utxos()
                                .iter()
                                .map(|utxo| utxo.outpoint)
                                .collect::<Vec<_>>()
                        })
                        .unwrap_or_default()
                })
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
        if let Ok(wallets) = self.wallets.try_read() {
            for wallet in wallets.values() {
                if let Some(mut wi) = wallet.core.try_wallet_info_mut() {
                    wi.mark_instant_send_utxos(&txid);
                }
                if let Ok(mut statuses) = wallet.core.transaction_statuses.try_write() {
                    let new_status = TransactionStatus::InstantSendLocked;
                    let old = statuses.get(&txid).copied();
                    if old.map_or(true, |old| new_status > old) {
                        statuses.insert(txid, new_status);
                    }
                }
            }
        }
    }

    fn subscribe_events(&self) -> broadcast::Receiver<WalletEvent> {
        self.event_tx.subscribe()
    }

    async fn earliest_required_height(&self) -> u32 {
        if let Ok(wallets) = self.wallets.try_read() {
            wallets
                .values()
                .filter_map(|w| w.core.try_wallet_info().map(|wi| wi.birth_height()))
                .min()
                .unwrap_or(0)
        } else {
            0
        }
    }

    async fn describe(&self) -> String {
        let count = self
            .wallets
            .try_read()
            .map(|w| w.len())
            .unwrap_or(0);
        format!("SpvWalletAdapter({} wallets)", count)
    }
}
