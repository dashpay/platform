#![cfg(feature = "sqlite")]

//! A wallet whose Core history was wiped (what V021 does to an older store)
//! rebuilds it by rescanning from its birth height. These tests drive that
//! rescan through the real manager, event adapter, store and load replay.

mod common;

use std::collections::BTreeMap;
use std::sync::Arc;

use common::{mine, persist_events, registration};
use dashcore::hashes::Hash;
use dashcore::transaction::special_transaction::asset_lock::AssetLockPayload;
use dashcore::transaction::special_transaction::TransactionPayload;
use dashcore::{Address, Network, OutPoint, ScriptBuf, Transaction, TxOut};
use key_wallet::managed_account::managed_account_trait::ManagedAccountTrait;
use key_wallet::managed_account::transaction_record::{TransactionDirection, TransactionRecord};
use key_wallet::transaction_checking::TransactionType;
use key_wallet::wallet::managed_wallet_info::asset_lock_builder::AssetLockFundingType;
use key_wallet::wallet::managed_wallet_info::wallet_info_interface::WalletInfoInterface;
use platform_wallet::changeset::{
    AssetLockChangeSet, AssetLockEntry, ClientStartState, PlatformWalletChangeSet,
    PlatformWalletPersistence,
};
use platform_wallet::key_wallet_manager::WalletManager;
use platform_wallet::test_support::core_fixtures::{
    foreign_address, outside, pay, spend_of, wallet_with_receive_address,
};
use platform_wallet::wallet::asset_lock::tracked::AssetLockStatus;
use platform_wallet::wallet::platform_wallet::{PlatformWalletInfo, WalletId};
use platform_wallet_storage::sqlite::schema::blob;
use platform_wallet_storage::{SqlitePersister, SqlitePersisterConfig};
use tokio::sync::RwLock;

/// Blocks the rescan finds, in chain order: a funding payment, a spend of its
/// first output (the original bug's shape: the spend's input details depend on
/// the funding being known), and an asset lock burning its second output with
/// no change.
struct Chain {
    funding: Transaction,
    spend: Transaction,
    lock: Transaction,
}

impl Chain {
    fn new(address: &Address) -> Self {
        let funding = spend_of(
            &[outside(21)],
            vec![pay(100_000, address), pay(20_000, address)],
        );
        let spend = spend_of(
            &[OutPoint::new(funding.txid(), 0)],
            vec![pay(60_000, &foreign_address(9)), pay(39_000, address)],
        );
        let burn = TxOut {
            value: 19_000,
            script_pubkey: ScriptBuf::new_op_return(&[]),
        };
        let mut lock = spend_of(&[OutPoint::new(funding.txid(), 1)], vec![burn]);
        lock.version = 3;
        lock.special_transaction_payload =
            Some(TransactionPayload::AssetLockPayloadType(AssetLockPayload {
                version: 1,
                credit_outputs: vec![pay(19_000, &foreign_address(8))],
            }));
        Self {
            funding,
            spend,
            lock,
        }
    }

    fn blocks(&self) -> [(Transaction, u32); 3] {
        [
            (self.funding.clone(), 100),
            (self.spend.clone(), 200),
            (self.lock.clone(), 300),
        ]
    }
}

/// A registered wallet with no Core history, as V021 leaves it, and its first
/// receive address.
fn wiped_wallet(persister: &SqlitePersister) -> (WalletId, Address) {
    let (wallet, _, address) = wallet_with_receive_address();
    persister
        .store(wallet.wallet_id, registration(&wallet, None))
        .unwrap();
    (wallet.wallet_id, address)
}

/// Load the store at `path`, run `blocks` through a manager holding the loaded
/// wallet and its tracked asset locks, persist every event, and load again.
async fn rescan(
    path: &std::path::Path,
    wallet_id: WalletId,
    blocks: &[(Transaction, u32)],
) -> ClientStartState {
    let persister = Arc::new(SqlitePersister::open(SqlitePersisterConfig::new(path)).unwrap());
    let loaded = persister
        .load()
        .unwrap()
        .wallets
        .remove(&wallet_id)
        .unwrap();
    assert!(
        loaded.wallet_info.accounts.standard_bip44_accounts[&0]
            .transactions()
            .is_empty(),
        "the wiped wallet loads without history"
    );
    let mut manager = WalletManager::<PlatformWalletInfo>::new(Network::Testnet);
    let mut events = manager.take_persistence_receiver().unwrap();
    let mut info = PlatformWalletInfo::from_wallet(&loaded.wallet, 0);
    info.core_wallet = loaded.wallet_info;
    info.tracked_asset_locks = loaded.unused_asset_locks.into_values().flatten().collect();
    manager.insert_wallet(loaded.wallet, info).unwrap();
    let manager = Arc::new(RwLock::new(manager));
    for (tx, height) in blocks {
        mine(&manager, wallet_id, tx.clone(), *height).await;
        persist_events(&manager, &persister, &mut events).await;
    }
    drop((manager, events, persister));
    SqlitePersister::open(SqlitePersisterConfig::new(path))
        .unwrap()
        .load()
        .unwrap()
}

/// The record the store holds for `tx`. Wallet-level accounting lives here:
/// the in-memory account record keeps upstream's account-local view (an asset
/// lock with no change reads `Outgoing` there, live or reloaded alike).
fn stored(path: &std::path::Path, wallet_id: &WalletId, tx: &Transaction) -> TransactionRecord {
    let conn = rusqlite::Connection::open(path).unwrap();
    let bytes: Vec<u8> = conn
        .query_row(
            "SELECT record_blob FROM core_transactions WHERE wallet_id = ?1 AND txid = ?2",
            rusqlite::params![wallet_id.as_slice(), tx.txid().as_byte_array().as_slice()],
            |row| row.get(0),
        )
        .unwrap_or_else(|_| panic!("the rescan stores {}", tx.txid()));
    blob::decode(&bytes).unwrap()
}

/// R1: a rescan from birth through the pinned key-wallet stores correct
/// accounting — input details, net, direction (an asset lock with no change
/// is `Internal`) — and the store's own load replay restores the spent coins.
#[tokio::test]
async fn should_rebuild_accounting_when_a_wiped_wallet_rescans_from_birth() {
    let (persister, _dir, path) = common::fresh_persister();
    let (wallet_id, address) = wiped_wallet(&persister);
    drop(persister);
    let chain = Chain::new(&address);

    let state = rescan(&path, wallet_id, &chain.blocks()).await;

    let input = |value| (0, value, address.clone());
    // (case, transaction, input details as (index, value, address), net, direction)
    let cases = [
        (
            "funding",
            &chain.funding,
            vec![],
            120_000,
            TransactionDirection::Incoming,
        ),
        (
            "spend",
            &chain.spend,
            vec![input(100_000)],
            -61_000,
            TransactionDirection::Outgoing,
        ),
        (
            "asset lock",
            &chain.lock,
            vec![input(20_000)],
            -20_000,
            TransactionDirection::Internal,
        ),
    ];
    for (case, tx, inputs, net, direction) in cases {
        let record = stored(&path, &wallet_id, tx);
        let details: Vec<_> = record
            .input_details
            .iter()
            .map(|d| (d.index, d.value, d.address.clone()))
            .collect();
        assert_eq!(details, inputs, "{case}");
        assert_eq!(
            (record.net_amount, record.direction),
            (net, direction),
            "{case}"
        );
    }
    assert_eq!(
        stored(&path, &wallet_id, &chain.lock).transaction_type,
        TransactionType::AssetLock
    );
    let info = &state.wallets[&wallet_id].wallet_info;
    let coins: Vec<_> = info.accounts.standard_bip44_accounts[&0]
        .utxos
        .keys()
        .copied()
        .collect();
    assert_eq!(
        coins,
        vec![OutPoint::new(chain.spend.txid(), 1)],
        "only the change is spendable"
    );
    assert_eq!(info.balance.total(), 39_000);
    let replayed = info.accounts.standard_bip44_accounts[&0].transactions();
    for tx in [&chain.funding, &chain.spend, &chain.lock] {
        assert!(
            replayed.contains_key(&tx.txid()),
            "the load replays {}",
            tx.txid()
        );
    }
}

fn asset_lock_rows(path: &std::path::Path) -> Vec<Vec<rusqlite::types::Value>> {
    let conn = rusqlite::Connection::open(path).unwrap();
    let mut stmt = conn
        .prepare("SELECT * FROM asset_locks ORDER BY outpoint")
        .unwrap();
    let width = stmt.column_count();
    stmt.query_map([], |row| (0..width).map(|i| row.get(i)).collect())
        .unwrap()
        .map(Result::unwrap)
        .collect()
}

/// R2: unconsumed asset locks of a wiped wallet survive the load untouched,
/// and a rescan that finds one of their transactions stores it without
/// dropping any lock.
#[tokio::test]
async fn should_keep_unconsumed_asset_locks_when_a_wiped_wallet_loads_and_rescans() {
    let (persister, _dir, path) = common::fresh_persister();
    let (wallet_id, address) = wiped_wallet(&persister);
    let chain = Chain::new(&address);
    let entry = |transaction: &Transaction, status| {
        let out_point = OutPoint::new(transaction.txid(), 0);
        let entry = AssetLockEntry {
            out_point,
            transaction: transaction.clone(),
            account_index: 0,
            funding_type: AssetLockFundingType::IdentityTopUp,
            identity_index: 0,
            amount_duffs: 19_000,
            status,
            proof: None,
        };
        (out_point, entry)
    };
    let other = |byte| spend_of(&[outside(byte)], vec![]);
    let locks: BTreeMap<_, _> = [
        entry(&other(31), AssetLockStatus::Built),
        entry(&chain.lock, AssetLockStatus::Broadcast),
        entry(&other(32), AssetLockStatus::InstantSendLocked),
        entry(&other(33), AssetLockStatus::ChainLocked),
    ]
    .into();
    let round = PlatformWalletChangeSet {
        asset_locks: Some(AssetLockChangeSet {
            asset_locks: locks.clone(),
            ..Default::default()
        }),
        ..Default::default()
    };
    persister.store(wallet_id, round).unwrap();
    drop(persister);
    let rows = asset_lock_rows(&path);
    assert_eq!(rows.len(), 4);

    let loaded = SqlitePersister::open(SqlitePersisterConfig::new(&path))
        .unwrap()
        .load()
        .unwrap();
    let restored: BTreeMap<_, _> = loaded.wallets[&wallet_id]
        .unused_asset_locks
        .values()
        .flatten()
        .map(|(out_point, lock)| (*out_point, lock.status.clone()))
        .collect();
    let expected: BTreeMap<_, _> = locks
        .iter()
        .map(|(op, e)| (*op, e.status.clone()))
        .collect();
    assert_eq!(restored, expected, "every unconsumed lock loads as stored");
    assert_eq!(
        asset_lock_rows(&path),
        rows,
        "the load leaves the rows untouched"
    );

    let state = rescan(&path, wallet_id, &chain.blocks()).await;

    assert_eq!(
        stored(&path, &wallet_id, &chain.lock).direction,
        TransactionDirection::Internal
    );
    let after: Vec<_> = state.wallets[&wallet_id]
        .unused_asset_locks
        .values()
        .flat_map(|locks| locks.keys().copied())
        .collect();
    assert_eq!(
        after,
        locks.keys().copied().collect::<Vec<_>>(),
        "no lock is dropped"
    );
    assert_eq!(
        asset_lock_rows(&path),
        rows,
        "the rescan leaves the rows untouched"
    );
}
