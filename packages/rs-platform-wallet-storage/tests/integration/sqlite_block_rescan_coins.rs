//! Coins a block credits only when dash-spv re-scans it for the addresses
//! its own outputs derive reach the SQLite store through the event adapter,
//! and a reload restores every one of them.

use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::time::Duration;

use crate::common::fresh_persister;
use dashcore::OutPoint;
use key_wallet::account::{AccountType, StandardAccountType};
use key_wallet::managed_account::address_pool::AddressPoolType;
use key_wallet::managed_account::managed_account_trait::ManagedAccountTrait;
use key_wallet::wallet::initialization::WalletAccountCreationOptions;
use key_wallet::wallet::ManagedWalletInfo;
use key_wallet::Wallet;
use platform_wallet::changeset::{
    spawn_wallet_event_adapter, AccountAddressPoolEntry, AccountRegistrationEntry,
    PlatformWalletChangeSet, PlatformWalletPersistence, WalletMetadataEntry,
};
use platform_wallet::key_wallet_manager::WalletEvent;
use platform_wallet::test_support::{
    block_paying, external_addresses, process_block_with_rescans, transaction_paying,
    wallet_manager_with_core_wallet, wallet_manager_with_persistence,
};
use platform_wallet::wallet::platform_wallet::{PlatformWalletInfo, WalletId};
use platform_wallet_storage::{SqlitePersister, SqlitePersisterConfig};
use tokio::sync::{mpsc, RwLock};

const VALUE: u64 = 10_000_000;
const PAST_GAP: u32 = 25;
const HEIGHT: u32 = 100;

fn external_gap(wallet: &Wallet, account: AccountType) -> u32 {
    ManagedWalletInfo::from_wallet(wallet, 0)
        .accounts
        .funds_account(&account)
        .expect("account")
        .managed_account_type()
        .address_pools()
        .into_iter()
        .find(|p| p.pool_type == AddressPoolType::External)
        .expect("external pool")
        .gap_limit
}

fn register(persister: &SqlitePersister, wallet_id: WalletId, wallet: &Wallet) {
    let manifest = wallet
        .accounts
        .all_accounts()
        .into_iter()
        .map(|a| AccountRegistrationEntry {
            account_type: a.account_type,
            account_xpub: a.account_xpub,
        })
        .collect();
    let pools = ManagedWalletInfo::from_wallet(wallet, 0)
        .accounts
        .all_funding_accounts()
        .into_iter()
        .flat_map(|account| {
            let account_type = account.managed_account_type().to_account_type();
            account
                .managed_account_type()
                .address_pools()
                .into_iter()
                .map(move |pool| AccountAddressPoolEntry {
                    account_type,
                    pool_type: pool.pool_type,
                    addresses: pool.addresses.values().cloned().collect(),
                })
                .collect::<Vec<_>>()
        })
        .collect();
    persister
        .store(
            wallet_id,
            PlatformWalletChangeSet {
                wallet_metadata: Some(WalletMetadataEntry {
                    network: key_wallet::Network::Testnet,
                    wallet_group_id: [0u8; 32],
                    birth_height: 0,
                }),
                account_registrations: manifest,
                account_address_pools: pools,
                ..Default::default()
            },
        )
        .expect("register the wallet");
}

/// Runs the event adapter over every event the manager queued, into
/// `persister`, until it has stored them all.
async fn persist_queued(
    manager: &Arc<RwLock<platform_wallet::key_wallet_manager::WalletManager<PlatformWalletInfo>>>,
    queued: &mut mpsc::UnboundedReceiver<WalletEvent>,
    persister: &Arc<SqlitePersister>,
) {
    let (tx, rx) = mpsc::unbounded_channel();
    while let Ok(event) = queued.try_recv() {
        tx.send(event).expect("adapter channel");
    }
    drop(tx);
    let adapter = spawn_wallet_event_adapter(
        Arc::clone(manager),
        Arc::downgrade(persister),
        rx,
        Arc::new(AtomicBool::new(false)),
        tokio_util::sync::CancellationToken::new(),
    );
    tokio::time::timeout(Duration::from_secs(60), adapter)
        .await
        .expect("the adapter drains")
        .expect("the adapter task");
}

/// The coins every funds account of the wallet holds after a reload of the
/// store at `path`. Which account a coin past the registered pool snapshot
/// lands in is the rehydrate's placement, not this persistence path.
fn reloaded_coins(path: &std::path::Path, wallet_id: WalletId) -> Vec<OutPoint> {
    let reopened = SqlitePersister::open(SqlitePersisterConfig::new(path)).expect("reopen");
    let state = reopened.load().expect("load");
    let slice = state.wallets.get(&wallet_id).expect("the wallet loads");
    slice
        .wallet_info
        .accounts
        .all_funding_accounts()
        .into_iter()
        .flat_map(|account| account.utxos.keys().copied())
        .collect()
}

/// The unspent `core_utxos` rows the store holds for each of `addresses`.
fn stored_unspent_rows(path: &std::path::Path, addresses: &[dashcore::Address]) -> Vec<i64> {
    let conn = rusqlite::Connection::open(path).expect("open the store");
    addresses
        .iter()
        .map(|address| {
            conn.query_row(
                "SELECT COUNT(*) FROM core_utxos WHERE spent = 0 AND script = ?1",
                [address.script_pubkey().as_bytes()],
                |row| row.get(0),
            )
            .expect("count rows")
        })
        .collect()
}

async fn every_coin_of_one_transaction_past_the_gap_reloads(account: AccountType) {
    let (persister, _tmp, path) = fresh_persister();
    let persister = Arc::new(persister);
    let wallet = Wallet::new_random(
        key_wallet::Network::Testnet,
        WalletAccountCreationOptions::Default,
    )
    .expect("wallet");
    let count = external_gap(&wallet, account) + PAST_GAP;
    let addresses = external_addresses(&wallet, account, count);
    let manifest_wallet = wallet.clone();
    let (manager, wallet_id, mut queued) = wallet_manager_with_persistence(wallet);
    register(&persister, wallet_id, &manifest_wallet);

    let block = block_paying(&addresses, VALUE, HEIGHT);
    process_block_with_rescans(&manager, wallet_id, &block, HEIGHT).await;
    persist_queued(&manager, &mut queued, &persister).await;
    persister.flush(wallet_id).expect("flush");
    drop(persister);

    assert_eq!(
        stored_unspent_rows(&path, &addresses),
        vec![1; addresses.len()],
        "the store holds one unspent row for every coin the block credited"
    );

    let txid = block.txdata[0].txid();
    let mut reloaded: Vec<u32> = reloaded_coins(&path, wallet_id)
        .into_iter()
        .filter(|o| o.txid == txid)
        .map(|o| o.vout)
        .collect();
    reloaded.sort_unstable();
    assert_eq!(
        reloaded,
        (0..count).collect::<Vec<_>>(),
        "a reload restores every coin the block credited"
    );
}

#[tokio::test]
async fn coinjoin_coins_past_the_gap_in_one_transaction_survive_a_reload() {
    every_coin_of_one_transaction_past_the_gap_reloads(AccountType::CoinJoin { index: 0 }).await;
}

#[tokio::test]
async fn bip44_coins_past_the_gap_in_one_transaction_survive_a_reload() {
    every_coin_of_one_transaction_past_the_gap_reloads(AccountType::Standard {
        index: 0,
        standard_account_type: StandardAccountType::BIP44Account,
    })
    .await;
}

fn bip44_account_0() -> AccountType {
    AccountType::Standard {
        index: 0,
        standard_account_type: StandardAccountType::BIP44Account,
    }
}

/// The coins `account` holds after a reload of the store at `path`.
fn reloaded_account_coins(
    path: &std::path::Path,
    wallet_id: WalletId,
    account: AccountType,
) -> Vec<OutPoint> {
    let reopened = SqlitePersister::open(SqlitePersisterConfig::new(path)).expect("reopen");
    let state = reopened.load().expect("load");
    let slice = state.wallets.get(&wallet_id).expect("the wallet loads");
    slice
        .wallet_info
        .accounts
        .funds_account(&account)
        .expect("account")
        .utxos
        .keys()
        .copied()
        .collect()
}

/// One block pays two funds accounts, CoinJoin past its gap and BIP44
/// within it. The re-scan passes re-send the BIP44 coins; the store holds
/// one unspent row per coin, a reload restores every coin of both, and the
/// BIP44 coins reload into the BIP44 account.
#[tokio::test]
async fn coins_of_two_accounts_at_one_height_survive_a_reload() {
    const BIP44_PAID: u32 = 5;
    let (persister, _tmp, path) = fresh_persister();
    let persister = Arc::new(persister);
    let wallet = Wallet::new_random(
        key_wallet::Network::Testnet,
        WalletAccountCreationOptions::Default,
    )
    .expect("wallet");
    let coinjoin = AccountType::CoinJoin { index: 0 };
    let count = external_gap(&wallet, coinjoin) + PAST_GAP;
    let coinjoin_addresses = external_addresses(&wallet, coinjoin, count);
    let bip44_addresses = external_addresses(&wallet, bip44_account_0(), BIP44_PAID);
    let past_gap = transaction_paying(&coinjoin_addresses, VALUE, 1);
    let within_gap = transaction_paying(&bip44_addresses, VALUE, 2);
    let manifest_wallet = wallet.clone();
    let (manager, wallet_id, mut queued) = wallet_manager_with_persistence(wallet);
    register(&persister, wallet_id, &manifest_wallet);

    let block = dashcore::Block::dummy(HEIGHT, vec![past_gap.clone(), within_gap.clone()]);
    process_block_with_rescans(&manager, wallet_id, &block, HEIGHT).await;
    persist_queued(&manager, &mut queued, &persister).await;
    persister.flush(wallet_id).expect("flush");
    drop(persister);

    assert_eq!(
        stored_unspent_rows(&path, &coinjoin_addresses),
        vec![1; coinjoin_addresses.len()]
    );
    assert_eq!(
        stored_unspent_rows(&path, &bip44_addresses),
        vec![1; bip44_addresses.len()]
    );
    let reloaded = reloaded_coins(&path, wallet_id);
    let vouts_of = |txid: dashcore::Txid| -> Vec<u32> {
        let mut vouts: Vec<u32> = reloaded
            .iter()
            .filter(|o| o.txid == txid)
            .map(|o| o.vout)
            .collect();
        vouts.sort_unstable();
        vouts
    };
    assert_eq!(vouts_of(past_gap.txid()), (0..count).collect::<Vec<_>>());
    assert_eq!(
        vouts_of(within_gap.txid()),
        (0..BIP44_PAID).collect::<Vec<_>>()
    );
    let bip44_held = reloaded_account_coins(&path, wallet_id, bip44_account_0());
    assert!((0..BIP44_PAID).all(|vout| bip44_held.contains(&OutPoint {
        txid: within_gap.txid(),
        vout
    })));
}

/// Where one transaction's coins sit across the wallet's funds accounts, and
/// the wallet's total.
#[derive(Debug, PartialEq)]
struct Placement {
    coinjoin: Vec<u32>,
    bip44: Vec<u32>,
    held: usize,
    distinct: usize,
    total: u64,
}

fn placement(info: &ManagedWalletInfo, txid: dashcore::Txid) -> Placement {
    let vouts = |account: AccountType| -> Vec<u32> {
        let mut vouts: Vec<u32> = info
            .accounts
            .funds_account(&account)
            .expect("account")
            .utxos
            .keys()
            .filter(|o| o.txid == txid)
            .map(|o| o.vout)
            .collect();
        vouts.sort_unstable();
        vouts
    };
    let held: Vec<OutPoint> = info
        .accounts
        .all_funding_accounts()
        .into_iter()
        .flat_map(|account| account.utxos.keys().copied())
        .filter(|o| o.txid == txid)
        .collect();
    Placement {
        coinjoin: vouts(AccountType::CoinJoin { index: 0 }),
        bip44: vouts(bip44_account_0()),
        held: held.len(),
        distinct: held.iter().collect::<std::collections::HashSet<_>>().len(),
        total: info.balance.total(),
    }
}

/// CoinJoin coins past the registered pool snapshot, credited only by the
/// block's re-scan passes, reload into the CoinJoin account, none into
/// BIP44. Replaying the block over the reloaded state credits nothing
/// twice: the coins, their accounts and the balance stay as reloaded.
#[tokio::test]
async fn coinjoin_coins_past_the_pool_snapshot_reload_into_coinjoin_and_replay_once() {
    let (persister, _tmp, path) = fresh_persister();
    let persister = Arc::new(persister);
    let wallet = Wallet::new_random(
        key_wallet::Network::Testnet,
        WalletAccountCreationOptions::Default,
    )
    .expect("wallet");
    let coinjoin = AccountType::CoinJoin { index: 0 };
    let count = external_gap(&wallet, coinjoin) + PAST_GAP;
    let addresses = external_addresses(&wallet, coinjoin, count);
    let manifest_wallet = wallet.clone();
    let (manager, wallet_id, mut queued) = wallet_manager_with_persistence(wallet);
    register(&persister, wallet_id, &manifest_wallet);

    let block = block_paying(&addresses, VALUE, HEIGHT);
    let passes = process_block_with_rescans(&manager, wallet_id, &block, HEIGHT).await;
    assert!(passes > 1, "the block is re-scanned");
    persist_queued(&manager, &mut queued, &persister).await;
    persister.flush(wallet_id).expect("flush");
    drop(persister);
    drop(manager);

    let txid = block.txdata[0].txid();
    let expected = || Placement {
        coinjoin: (0..count).collect(),
        bip44: Vec::new(),
        held: count as usize,
        distinct: count as usize,
        total: u64::from(count) * VALUE,
    };
    let reopened = SqlitePersister::open(SqlitePersisterConfig::new(&path)).expect("reopen");
    let reloaded = reopened
        .load()
        .expect("load")
        .wallets
        .remove(&wallet_id)
        .expect("the wallet loads")
        .wallet_info;
    drop(reopened);
    let after_reload = placement(&reloaded, txid);

    let (replaying, replay_id, _events) =
        wallet_manager_with_core_wallet(manifest_wallet, reloaded);
    assert_eq!(replay_id, wallet_id);
    process_block_with_rescans(&replaying, wallet_id, &block, HEIGHT).await;
    let guard = replaying.read().await;
    let after_replay = placement(
        &guard
            .get_wallet_info(&wallet_id)
            .expect("the wallet")
            .core_wallet,
        txid,
    );
    assert_eq!(
        (after_reload, after_replay),
        (expected(), expected()),
        "(after the reload, after the replay)"
    );
}
