#![cfg(feature = "sqlite")]

use crate::common;

use std::sync::Arc;

use common::{mine, persist_events, registration, Case};
use dashcore::{
    bls_sig_utils::BLSSignature, ephemerealdata::chain_lock::ChainLock, hashes::Hash, BlockHash,
    Network, OutPoint, ScriptBuf, Transaction, TxOut,
};
use key_wallet::wallet::{
    initialization::WalletAccountCreationOptions,
    managed_wallet_info::{
        coin_selection::{CoinSelector, SelectionStrategy},
        fee::FeeRate,
    },
    ManagedWalletInfo,
};
use platform_wallet::{
    changeset::PlatformWalletPersistence,
    key_wallet_manager::{WalletInterface, WalletManager},
    test_support::core_fixtures::{block, check, outside, pay, spend_of},
    wallet::platform_wallet::PlatformWalletInfo,
};
use platform_wallet_storage::{SqlitePersister, SqlitePersisterConfig};
use tokio::sync::RwLock;

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
    let wallet = manager.get_wallet(&wallet_id).unwrap();
    persister
        .store(wallet_id, registration(wallet, None))
        .unwrap();
    let funding = spend_of(
        &[outside(13)],
        vec![pay(100_000, &address), pay(20_000, &address)],
    );
    let spent = OutPoint::new(funding.txid(), 0);
    let burn = TxOut {
        value: 99_000,
        script_pubkey: ScriptBuf::new(),
    };
    let spending = spend_of(&[spent], vec![burn]);
    let manager = Arc::new(RwLock::new(manager));
    if recordless_winner {
        // A wallet-paying mempool spend loses to a mined payment to somebody else.
        let loser = spend_of(&[spent], vec![pay(98_000, &address)]);
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
        let (records, claims): (i64, i64) = persister
            .lock_conn_for_test()
            .query_row(
                "SELECT (SELECT COUNT(*) FROM core_transactions WHERE record_blob IS NOT NULL), \
                        (SELECT COUNT(*) FROM core_utxos WHERE spent = 1)",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        assert_eq!(
            (records, claims),
            (0, 1),
            "the live event path must produce the recordless state, \
             and the sweep must persist the winner's input claim"
        );
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
    check(
        &mut uninterrupted_info,
        &mut uninterrupted_wallet,
        &funding,
        block(100),
    )
    .await;
    assert_coins(&uninterrupted_info, &funding, "uninterrupted wallet");
    check(
        &mut restored.wallet_info,
        &mut restored.wallet,
        &funding,
        block(100),
    )
    .await;
    assert_coins(&restored.wallet_info, &funding, "restored wallet");
}

/// A spend, or a recordless winner, persisted through the real wallet event
/// adapter survives a clean restart: funding redelivery returns the spent
/// coin neither to the uninterrupted wallet nor to the restored one.
#[tokio::test]
async fn should_preserve_spend_after_clean_event_persistence_and_restart() {
    // (case, recordless winner, finalized, pending spend)
    for (label, recordless_winner, finalized, pending) in [
        ("mined spend", false, false, false),
        ("pending spend", false, false, true),
        ("finalized spend", false, true, false),
        ("recordless winner", true, false, false),
        ("finalized recordless winner", true, true, false),
    ] {
        let _case = Case(label.into());
        clean_restart(recordless_winner, finalized, pending).await;
    }
}
