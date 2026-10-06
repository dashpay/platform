#![cfg(feature = "sqlite")]

mod common;

use std::collections::BTreeSet;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use dashcore::{
    block::{Header, Version},
    bls_sig_utils::BLSSignature,
    ephemerealdata::chain_lock::ChainLock,
    hashes::Hash,
    Block, BlockHash, CompactTarget, Network, OutPoint, ScriptBuf, Transaction, TxIn, TxMerkleNode,
    TxOut, Txid,
};
use key_wallet::{
    transaction_checking::{BlockInfo, TransactionContext, WalletTransactionChecker},
    wallet::{
        initialization::WalletAccountCreationOptions,
        managed_wallet_info::{
            coin_selection::{CoinSelector, SelectionStrategy},
            fee::FeeRate,
        },
        ManagedWalletInfo,
    },
};
use platform_wallet::{
    changeset::{
        spawn_wallet_event_adapter, AccountRegistrationEntry, PlatformWalletChangeSet,
        PlatformWalletPersistence, WalletMetadataEntry,
    },
    key_wallet_manager::{WalletEvent, WalletInterface, WalletManager},
    wallet::platform_wallet::PlatformWalletInfo,
};
use platform_wallet_storage::{SqlitePersister, SqlitePersisterConfig};
use tokio::sync::{mpsc, RwLock};
use tokio_util::sync::CancellationToken;

type Manager = Arc<RwLock<WalletManager<PlatformWalletInfo>>>;

fn transaction(input: OutPoint, outputs: Vec<TxOut>) -> Transaction {
    Transaction {
        version: 1,
        lock_time: 0,
        input: vec![TxIn {
            previous_output: input,
            ..Default::default()
        }],
        output: outputs,
        special_transaction_payload: None,
    }
}

async fn mine(manager: &Manager, wallet_id: [u8; 32], tx: Transaction, height: u32) {
    let block = Block {
        header: Header {
            version: Version::default(),
            prev_blockhash: BlockHash::from_byte_array([height as u8; 32]),
            merkle_root: TxMerkleNode::all_zeros(),
            time: height,
            bits: CompactTarget::from_consensus(0x1d00ffff),
            nonce: 0,
        },
        txdata: vec![tx],
    };
    manager
        .write()
        .await
        .process_block_for_wallets(
            &block,
            block.block_hash(),
            height,
            &BTreeSet::from([wallet_id]),
        )
        .await;
}

async fn persist_events(
    manager: &Manager,
    persister: &Arc<SqlitePersister>,
    events: &mut mpsc::UnboundedReceiver<WalletEvent>,
) {
    // Close one finite batch so completion proves every real manager event was stored.
    let (sender, receiver) = mpsc::unbounded_channel();
    while let Ok(event) = events.try_recv() {
        sender.send(event).unwrap();
    }
    drop(sender);
    let fault = Arc::new(AtomicBool::new(false));
    spawn_wallet_event_adapter(
        Arc::clone(manager),
        Arc::downgrade(persister),
        receiver,
        Arc::clone(&fault),
        CancellationToken::new(),
    )
    .await
    .unwrap();
    assert!(!fault.load(Ordering::Relaxed));
}

fn assert_coins(info: &ManagedWalletInfo, funding: &Transaction, phase: &str) {
    let coins = &info.accounts.standard_bip44_accounts[&0].utxos;
    assert!(
        !coins.contains_key(&OutPoint::new(funding.txid(), 0)),
        "{phase}: spent funding output must not return"
    );
    assert!(coins.contains_key(&OutPoint::new(funding.txid(), 1)));
    assert_eq!(info.balance.total(), 20_000, "{phase}: spendable balance");
    assert!(
        CoinSelector::new(SelectionStrategy::LargestFirst)
            .select_coins(coins.values(), 50_000, FeeRate::default(), 300)
            .is_err(),
        "{phase}: spent output must not fund a new payment"
    );
}

async fn clean_restart(recordless_winner: bool, finalized: bool, pending: bool) {
    let (persister, _dir, path) = common::fresh_persister();
    assert!(persister.load().unwrap().wallets.is_empty());
    let persister = Arc::new(persister);
    let mut manager = WalletManager::<PlatformWalletInfo>::new(Network::Testnet);
    let mut events = manager.take_persistence_receiver().unwrap();
    let wallet_id = manager.create_wallet_from_mnemonic(
        "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about",
        0, WalletAccountCreationOptions::Default,
    ).unwrap();
    let xpub = manager
        .get_wallet(&wallet_id)
        .unwrap()
        .accounts
        .standard_bip44_accounts[&0]
        .account_xpub;
    let address = manager
        .get_wallet_info_mut(&wallet_id)
        .unwrap()
        .core_wallet
        .accounts
        .standard_bip44_accounts
        .get_mut(&0)
        .unwrap()
        .next_receive_address(Some(&xpub), true)
        .unwrap();
    persister
        .store(
            wallet_id,
            PlatformWalletChangeSet {
                wallet_metadata: Some(WalletMetadataEntry {
                    network: Network::Testnet,
                    wallet_group_id: [0; 32],
                    birth_height: 0,
                }),
                account_registrations: manager
                    .get_wallet(&wallet_id)
                    .unwrap()
                    .accounts
                    .all_accounts()
                    .into_iter()
                    .map(|account| AccountRegistrationEntry {
                        account_type: account.account_type,
                        account_xpub: account.account_xpub,
                    })
                    .collect(),
                ..Default::default()
            },
        )
        .unwrap();
    let funding = transaction(
        OutPoint::new(Txid::from_byte_array([13; 32]), 0),
        vec![
            TxOut {
                value: 100_000,
                script_pubkey: address.script_pubkey(),
            },
            TxOut {
                value: 20_000,
                script_pubkey: address.script_pubkey(),
            },
        ],
    );
    let spent = OutPoint::new(funding.txid(), 0);
    let spending = transaction(
        spent,
        vec![TxOut {
            value: 99_000,
            script_pubkey: ScriptBuf::new(),
        }],
    );
    let manager = Arc::new(RwLock::new(manager));
    if recordless_winner {
        // A wallet-paying mempool spend loses to a mined payment to somebody else.
        let loser = transaction(
            spent,
            vec![TxOut {
                value: 98_000,
                script_pubkey: address.script_pubkey(),
            }],
        );
        assert!(
            manager
                .write()
                .await
                .process_mempool_transaction(&loser, None)
                .await
                .is_relevant
        );
        persist_events(&manager, &persister, &mut events).await;
    } else {
        mine(&manager, wallet_id, funding.clone(), 100).await;
        persist_events(&manager, &persister, &mut events).await;
    }
    if pending {
        assert!(
            manager
                .write()
                .await
                .process_mempool_transaction(&spending, None)
                .await
                .is_relevant
        );
    } else {
        mine(&manager, wallet_id, spending.clone(), 200).await;
    }
    persist_events(&manager, &persister, &mut events).await;
    if finalized {
        let mut guard = manager.write().await;
        guard.apply_chain_lock(ChainLock {
            block_height: 300,
            block_hash: BlockHash::from_byte_array([30; 32]),
            signature: BLSSignature::from([0; 96]),
        });
        if !recordless_winner {
            guard.update_wallet_synced_height(&wallet_id, 300);
        }
        drop(guard);
        persist_events(&manager, &persister, &mut events).await;
    }
    if recordless_winner {
        let records: i64 = persister
            .lock_conn_for_test()
            .query_row(
                "SELECT COUNT(*) FROM core_transactions WHERE record_blob IS NOT NULL",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(
            records, 0,
            "the live event path must produce the recordless state"
        );
        let claims: i64 = persister
            .lock_conn_for_test()
            .query_row(
                "SELECT COUNT(*) FROM core_utxos WHERE spent = 1",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(claims, 1, "sweep must persist the winner's input claim");
    }
    let (mut uninterrupted_wallet, mut uninterrupted_info) = {
        let guard = manager.read().await;
        (
            guard.get_wallet(&wallet_id).unwrap().clone(),
            guard
                .get_wallet_info(&wallet_id)
                .unwrap()
                .core_wallet
                .clone(),
        )
    };
    drop(events);
    drop(manager);
    drop(persister);
    let reopened = SqlitePersister::open(SqlitePersisterConfig::new(path)).unwrap();
    let mut restored = reopened.load().unwrap().wallets.remove(&wallet_id).unwrap();
    let context = TransactionContext::InBlock(BlockInfo::new(
        100,
        BlockHash::from_byte_array([100; 32]),
        100,
    ));
    uninterrupted_info
        .check_core_transaction(
            &funding,
            context.clone(),
            &mut uninterrupted_wallet,
            true,
            true,
        )
        .await;
    assert_coins(&uninterrupted_info, &funding, "uninterrupted wallet");
    restored
        .wallet_info
        .check_core_transaction(&funding, context, &mut restored.wallet, true, true)
        .await;
    assert_coins(&restored.wallet_info, &funding, "restored wallet");
}

#[tokio::test]
async fn should_preserve_spend_after_clean_event_persistence_and_restart() {
    clean_restart(false, false, false).await;
}

#[tokio::test]
async fn should_preserve_pending_spend_after_clean_event_persistence_and_restart() {
    clean_restart(false, false, true).await;
}

#[tokio::test]
async fn should_preserve_finalized_spend_after_clean_event_persistence_and_restart() {
    clean_restart(false, true, false).await;
}

#[tokio::test]
async fn should_preserve_recordless_winner_after_clean_event_persistence_and_restart() {
    clean_restart(true, false, false).await;
}

#[tokio::test]
async fn should_preserve_finalized_recordless_winner_after_clean_event_persistence_and_restart() {
    clean_restart(true, true, false).await;
}
