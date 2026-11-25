//! A transaction the wallet swept as a conflict loser stays dismissed
//! across a restart: the event adapter stores the dismissal with its sweep,
//! and a reload hands it back to key-wallet, so a peer delivering the loser
//! again does not credit it.

use std::sync::Arc;

use crate::common::{fresh_persister, persist_queued, register};
use dashcore::OutPoint;
use key_wallet::account::{AccountType, StandardAccountType};
use key_wallet::test_utils::sign_owned_inputs;
use key_wallet::wallet::initialization::WalletAccountCreationOptions;
use key_wallet::wallet::ManagedWalletInfo;
use key_wallet::Wallet;
use platform_wallet::changeset::PlatformWalletPersistence;
use platform_wallet::key_wallet_manager::{WalletInterface, WalletManager};
use platform_wallet::test_support::core_fixtures::{foreign_address, pay, spend_of};
use platform_wallet::test_support::{
    external_addresses, process_block_with_rescans, transaction_paying,
    wallet_manager_with_core_wallet, wallet_manager_with_persistence,
};
use platform_wallet::wallet::platform_wallet::{PlatformWalletInfo, WalletId};
use platform_wallet_storage::{SqlitePersister, SqlitePersisterConfig};
use tokio::sync::RwLock;

const VALUE: u64 = 10_000_000;
const CHANGE: u64 = VALUE / 2 - 1_000;

fn bip44_account_0() -> AccountType {
    AccountType::Standard {
        index: 0,
        standard_account_type: StandardAccountType::BIP44Account,
    }
}

async fn holds(
    manager: &RwLock<WalletManager<PlatformWalletInfo>>,
    wallet_id: WalletId,
    outpoint: OutPoint,
) -> bool {
    manager
        .read()
        .await
        .get_wallet_info(&wallet_id)
        .expect("wallet")
        .core_wallet
        .accounts
        .all_funding_accounts()
        .into_iter()
        .any(|account| account.utxos.contains_key(&outpoint))
}

async fn balance(manager: &RwLock<WalletManager<PlatformWalletInfo>>, wallet_id: WalletId) -> u64 {
    manager
        .read()
        .await
        .get_wallet_info(&wallet_id)
        .expect("wallet")
        .core_wallet
        .balance
        .total()
}

fn reload(path: &std::path::Path, wallet_id: WalletId) -> ManagedWalletInfo {
    SqlitePersister::open(SqlitePersisterConfig::new(path))
        .expect("reopen")
        .load()
        .expect("load")
        .wallets
        .remove(&wallet_id)
        .expect("the wallet loads")
        .wallet_info
}

/// A block mines a spend of the wallet's coin that beats a pending spend
/// paying change back. The loser is swept and dismissed, both are stored,
/// and after a restart a peer delivering the loser again credits nothing.
#[tokio::test]
async fn a_swept_loser_delivered_again_after_a_restart_is_not_credited() {
    let (persister, _tmp, path) = fresh_persister();
    let persister = Arc::new(persister);
    let wallet = Wallet::new_random(
        key_wallet::Network::Testnet,
        WalletAccountCreationOptions::Default,
    )
    .expect("wallet");
    let addresses = external_addresses(&wallet, bip44_account_0(), 2);
    let funding = transaction_paying(&[addresses[0].clone()], VALUE, 1);
    let coin = OutPoint::new(funding.txid(), 0);
    let winner = spend_of(&[coin], vec![pay(VALUE - 1_000, &foreign_address(8))]);

    let (manager, wallet_id, mut queued) = wallet_manager_with_persistence(wallet.clone());
    register(&persister, wallet_id, &wallet);
    process_block_with_rescans(
        &manager,
        wallet_id,
        &dashcore::Block::dummy(1, vec![funding.clone()]),
        1,
    )
    .await;
    let unsigned = spend_of(
        &[coin],
        vec![
            pay(VALUE / 2, &foreign_address(7)),
            pay(CHANGE, &addresses[1]),
        ],
    );
    let loser = sign_owned_inputs(
        &wallet,
        &manager
            .read()
            .await
            .get_wallet_info(&wallet_id)
            .expect("wallet")
            .core_wallet,
        unsigned,
        &[&funding],
    )
    .await;
    let change = OutPoint::new(loser.txid(), 1);
    manager
        .write()
        .await
        .process_mempool_transaction(&loser, None)
        .await;
    assert!(
        holds(&manager, wallet_id, change).await,
        "the loser pays change back"
    );
    process_block_with_rescans(
        &manager,
        wallet_id,
        &dashcore::Block::dummy(2, vec![winner]),
        2,
    )
    .await;
    assert!(
        manager.read().await.is_dismissed(&loser.txid()),
        "the mined winner sweeps the loser"
    );
    persist_queued(&manager, &mut queued, &persister).await;
    persister.flush(wallet_id).expect("flush");
    drop(persister);

    let (manager, wallet_id, _queued) =
        wallet_manager_with_core_wallet(wallet, reload(&path, wallet_id));
    assert!(
        manager.read().await.is_dismissed(&loser.txid()),
        "the reload dismisses the loser again"
    );
    manager
        .write()
        .await
        .process_mempool_transaction(&loser, None)
        .await;
    assert!(!holds(&manager, wallet_id, change).await);
    assert_eq!(balance(&manager, wallet_id).await, 0);
}
