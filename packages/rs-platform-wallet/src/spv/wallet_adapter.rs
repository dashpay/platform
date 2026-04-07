//! SPV wallet adapter implementing WalletInterface from key-wallet-manager.
//!
//! Bridges the entire wallet collection to `DashSpvClient` — processes blocks
//! and mempool transactions against ALL managed wallets, not just one.

use std::collections::BTreeMap;
use std::sync::Arc;

use async_trait::async_trait;
use dashcore::{Address as DashAddress, Block, OutPoint, Transaction, Txid};
use key_wallet::changeset::{Merge as KwMerge, WalletChangeSet as KwWalletChangeSet};
use key_wallet::transaction_checking::{BlockInfo, TransactionContext, WalletTransactionChecker};
use key_wallet::wallet::managed_wallet_info::wallet_info_interface::WalletInfoInterface;
use key_wallet_manager::{
    BlockProcessingResult, MempoolTransactionResult, WalletEvent, WalletInterface,
};
use tokio::sync::{broadcast, RwLock};

use crate::changeset::{ChainChangeSet, PlatformWalletChangeSet};
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
    wallets: Arc<RwLock<BTreeMap<WalletId, Arc<PlatformWallet>>>>,
    sync_state: Arc<super::sync_state::SpvSyncState>,
    /// Wallet event sender — required by `WalletInterface::subscribe_events()`.
    /// The SPV client's event monitor subscribes to this channel.
    event_tx: broadcast::Sender<WalletEvent>,
}

impl SpvWalletAdapter {
    pub(crate) fn new(
        wallets: Arc<RwLock<BTreeMap<WalletId, Arc<PlatformWallet>>>>,
        sync_state: Arc<super::sync_state::SpvSyncState>,
    ) -> Self {
        let (event_tx, _) = broadcast::channel(256);
        Self {
            wallets,
            sync_state,
            event_tx,
        }
    }
}

#[async_trait]
impl WalletInterface for SpvWalletAdapter {
    async fn process_block(&mut self, block: &Block, block_height: u32) -> BlockProcessingResult {
        let wallets = self.wallets.read().await;

        let block_hash = block.header.block_hash();
        let context = TransactionContext::InBlock(BlockInfo::new(
            block_height,
            block_hash,
            block.header.time,
        ));

        let mut new_txids = Vec::new();
        let mut existing_txids = Vec::new();
        let mut new_addresses = Vec::new();

        for wallet in wallets.values() {
            let mut w = wallet.core.wallet.write().await;
            let mut wi = wallet.core.wallet_info_mut().await;

            // Accumulate key-wallet changesets across all transactions in the block.
            let mut block_changeset = KwWalletChangeSet::default();

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
                if result.is_relevant || result.state_modified {
                    // key-wallet's changeset has the full delta.
                    block_changeset.merge(result.changeset);
                }
                if !result.new_addresses.is_empty() {
                    new_addresses.extend(result.new_addresses);
                }
            }

            // Build and stage the changeset for this wallet.
            let changeset = PlatformWalletChangeSet {
                wallet: if block_changeset.is_empty() {
                    None
                } else {
                    Some(block_changeset)
                },
                chain: Some(ChainChangeSet {
                    height: Some(block_height),
                    block_hash: Some(block_hash),
                }),
                ..Default::default()
            };
            wallet.queue_persist(changeset);
        }

        self.sync_state.update_synced_height(block_height);

        if !new_addresses.is_empty() {
            self.sync_state.bump_monitor_revision();
        }

        // Transaction status is tracked natively in key-wallet's TransactionRecord.context
        // via check_core_transaction — no separate status tracking needed.

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

                // key-wallet's changeset has the full delta (including status).
                let changeset = PlatformWalletChangeSet {
                    wallet: if result.changeset.is_empty() {
                        None
                    } else {
                        Some(result.changeset)
                    },
                    ..Default::default()
                };
                wallet.queue_persist(changeset);
            }

            if !result.new_addresses.is_empty() {
                self.sync_state.bump_monitor_revision();
                combined.new_addresses.extend(result.new_addresses);
            }
        }

        combined
    }

    fn monitored_addresses(&self) -> Vec<DashAddress> {
        if let Ok(wallets) = self.wallets.try_read() {
            let count = wallets.len();
            let addresses: Vec<DashAddress> = wallets
                .values()
                .flat_map(|w| {
                    let addrs = w.core
                        .try_wallet_info()
                        .map(|wi| wi.monitored_addresses())
                        .unwrap_or_default();
                    tracing::debug!("SpvWalletAdapter::monitored_addresses: wallet {} has {} addresses", hex::encode(w.wallet_id()), addrs.len());
                    addrs
                })
                .collect();
            tracing::debug!("SpvWalletAdapter::monitored_addresses: {} wallets, {} total addresses", count, addresses.len());
            addresses
        } else {
            tracing::warn!("SpvWalletAdapter::monitored_addresses: wallets lock contention, returning empty");
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
        self.sync_state.synced_height()
    }

    fn update_synced_height(&mut self, height: u32) {
        self.sync_state.update_synced_height(height);
    }

    fn filter_committed_height(&self) -> u32 {
        self.sync_state.filter_committed_height()
    }

    fn update_filter_committed_height(&mut self, height: u32) {
        self.sync_state.update_filter_committed_height(height);
    }

    fn monitor_revision(&self) -> u64 {
        self.sync_state.monitor_revision()
    }

    fn process_instant_send_lock(&mut self, txid: Txid) {
        if let Ok(wallets) = self.wallets.try_read() {
            for wallet in wallets.values() {
                let mut status_changed = false;

                // Capture the UTXO IS-lock changeset from mark_instant_send_utxos.
                let utxo_cs = if let Some(mut wi) = wallet.core.try_wallet_info_mut() {
                    let (_changed, utxo_cs) = wi.mark_instant_send_utxos(&txid);
                    utxo_cs
                } else {
                    key_wallet::changeset::UtxoChangeSet::default()
                };

                // IS-lock status tracked in key-wallet via mark_instant_send_utxos above.
                status_changed = !utxo_cs.instant_locked.is_empty();

                // Stage a minimal changeset recording the IS-lock status change
                // and the UTXO IS-lock deltas.
                // We don't have the full transaction here, so we only stage if the
                // wallet already tracks this txid (status actually changed).
                if status_changed {
                    if let Some(wi) = wallet.core.try_wallet_info() {
                        // Build a key-wallet changeset from the transaction record.
                        let mut kw_changeset = KwWalletChangeSet::default();
                        for account in wi.accounts.all_accounts() {
                            if let Some(record) = account.transactions.get(&txid) {
                                let block_info = record.context.block_info();
                                let kw_entry = key_wallet::changeset::TransactionEntry {
                                    transaction: record.transaction.clone(),
                                    block_height: block_info.map(|bi| bi.height()),
                                    block_hash: block_info.map(|bi| bi.block_hash()),
                                    timestamp: block_info
                                        .map(|bi| bi.timestamp() as u64)
                                        .unwrap_or(0),
                                    net_amount: record.net_amount,
                                    fee: record.fee,
                                    label: record.label.clone(),
                                    is_instant_locked: true,
                                    is_chain_locked: false,
                                };
                                let mut records = BTreeMap::new();
                                records.insert(txid, kw_entry);
                                kw_changeset.transactions =
                                    Some(key_wallet::changeset::TransactionChangeSet { records });
                                break;
                            }
                        }

                        // Include UTXO IS-lock deltas in the changeset.
                        if !utxo_cs.is_empty() {
                            kw_changeset.utxos.merge(Some(utxo_cs));
                        }

                        if !kw_changeset.is_empty() {
                            let changeset = PlatformWalletChangeSet {
                                wallet: Some(kw_changeset),
                                ..Default::default()
                            };
                            wallet.queue_persist(changeset);
                        }
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
        let count = self.wallets.try_read().map(|w| w.len()).unwrap_or(0);
        format!("SpvWalletAdapter({} wallets)", count)
    }
}
