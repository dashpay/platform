#![allow(clippy::field_reassign_with_default)]

//! Locked outpoints (masternode collateral, and outpoints locked by hand)
//! survive the SQLite store: `core_locked_outpoints` rows come back through
//! `load_state` and `apply_persisted_core_state`, whether or not the wallet
//! already holds the coin, and keep the collateral out of coin selection.

mod common;

use std::collections::BTreeMap;

use common::{ensure_wallet_meta, fresh_persister, wid};
use dashcore::hashes::Hash;
use dashcore::{Address, BlockHash, OutPoint, Transaction, TxIn, TxOut};
use key_wallet::transaction_checking::{BlockInfo, TransactionContext, WalletTransactionChecker};
use key_wallet::wallet::initialization::WalletAccountCreationOptions;
use key_wallet::wallet::managed_wallet_info::wallet_info_interface::WalletInfoInterface;
use key_wallet::wallet::managed_wallet_info::ManagedWalletInfo;
use key_wallet::wallet::Wallet;
use key_wallet::Utxo;
use platform_wallet::changeset::{
    AccountRegistrationEntry, CoreChangeSet, PlatformWalletChangeSet, PlatformWalletPersistence,
};
use platform_wallet::wallet::platform_wallet::WalletId;
use platform_wallet_storage::sqlite::schema::core_state;
use platform_wallet_storage::LoadCtx;

const DUFFS_PER_DASH: u64 = 100_000_000;
const COLLATERAL: u64 = 1_000 * DUFFS_PER_DASH;
const SPARE: u64 = 5 * DUFFS_PER_DASH;
const HEIGHT: u32 = 100;

fn test_wallet() -> Wallet {
    Wallet::from_seed_bytes(
        [0x42; 64],
        key_wallet::Network::Testnet,
        WalletAccountCreationOptions::Default,
    )
    .expect("wallet from seed")
}

fn manifest_for(wallet: &Wallet) -> Vec<AccountRegistrationEntry> {
    wallet
        .accounts
        .all_accounts()
        .into_iter()
        .map(|a| AccountRegistrationEntry {
            account_type: a.account_type,
            account_xpub: a.account_xpub,
        })
        .collect()
}

/// Two receive addresses of the wallet: one for the collateral, one for the
/// spare coin.
fn two_addresses(wallet: &Wallet) -> (Address, Address) {
    let info = ManagedWalletInfo::from_wallet(wallet, 1);
    let mut addresses = WalletInfoInterface::monitored_addresses(&info).into_iter();
    let first = addresses.next().expect("a monitored address");
    let second = addresses.next().expect("a second monitored address");
    (first, second)
}

/// A transaction paying `value` to `address` as output 0. `marker` keeps the
/// txids of otherwise identical transactions apart.
fn payment(address: &Address, value: u64, marker: u8) -> Transaction {
    Transaction {
        version: 2,
        lock_time: 0,
        input: vec![TxIn {
            previous_output: OutPoint::new(dashcore::Txid::from_byte_array([marker; 32]), 0),
            ..TxIn::default()
        }],
        output: vec![TxOut {
            value,
            script_pubkey: address.script_pubkey(),
        }],
        special_transaction_payload: None,
    }
}

fn coin(tx: &Transaction, address: &Address) -> Utxo {
    Utxo {
        outpoint: OutPoint::new(tx.txid(), 0),
        txout: tx.output[0].clone(),
        address: address.clone(),
        height: HEIGHT,
        is_coinbase: false,
        is_confirmed: true,
        is_instantlocked: false,
        is_locked: false,
        is_trusted: false,
    }
}

fn store_core(
    persister: &platform_wallet_storage::SqlitePersister,
    w: WalletId,
    core: CoreChangeSet,
) {
    persister
        .store(
            w,
            PlatformWalletChangeSet {
                core: Some(core),
                ..Default::default()
            },
        )
        .expect("store");
}

fn locks(entries: &[(OutPoint, bool)]) -> BTreeMap<OutPoint, bool> {
    entries.iter().copied().collect()
}

/// Reopen the database and rebuild the wallet the way `load()` does.
fn reload(path: &std::path::Path, w: &WalletId, wallet: &Wallet) -> ManagedWalletInfo {
    let persister = platform_wallet_storage::SqlitePersister::open(
        platform_wallet_storage::SqlitePersisterConfig::new(path),
    )
    .expect("reopen persister");
    let conn = persister.lock_conn_for_test();
    let (core, utxo_accounts) =
        core_state::load_state(&conn, w, key_wallet::Network::Testnet, &LoadCtx::strict())
            .expect("load_state");
    drop(conn);
    let mut info = ManagedWalletInfo::from_wallet(wallet, 1);
    platform_wallet_storage::sqlite::rehydrate::apply_persisted_core_state(
        &mut info,
        &manifest_for(wallet),
        &core,
        &utxo_accounts,
        &Default::default(),
        &LoadCtx::strict(),
    )
    .expect("apply persisted core state");
    info
}

fn spendable_outpoints(info: &ManagedWalletInfo) -> Vec<OutPoint> {
    WalletInfoInterface::get_spendable_utxos(info)
        .into_iter()
        .map(|utxo| utxo.outpoint)
        .collect()
}

#[test]
fn should_restore_a_locked_collateral_in_the_locked_balance_after_reopen() {
    let (persister, _tmp, path) = fresh_persister();
    let w = wid(0xC1);
    ensure_wallet_meta(&persister, &w);
    let wallet = test_wallet();
    let (collateral_address, spare_address) = two_addresses(&wallet);
    let collateral_tx = payment(&collateral_address, COLLATERAL, 1);
    let spare_tx = payment(&spare_address, SPARE, 2);
    let collateral = OutPoint::new(collateral_tx.txid(), 0);

    store_core(
        &persister,
        w,
        CoreChangeSet {
            new_utxos: vec![
                coin(&collateral_tx, &collateral_address),
                coin(&spare_tx, &spare_address),
            ],
            outpoint_locks: locks(&[(collateral, true)]),
            synced_height: Some(HEIGHT),
            last_processed_height: Some(HEIGHT),
            ..Default::default()
        },
    );
    drop(persister);

    let info = reload(&path, &w, &wallet);
    assert!(info.is_outpoint_locked(&collateral));
    let balance = WalletInfoInterface::balance(&info);
    assert_eq!(balance.locked(), COLLATERAL);
    assert_eq!(balance.spendable(), SPARE);
    assert_eq!(
        spendable_outpoints(&info),
        vec![OutPoint::new(spare_tx.txid(), 0)],
        "coin selection must see only the spare coin"
    );
}

#[test]
fn should_restore_a_lock_stored_before_its_coin() {
    let (persister, _tmp, path) = fresh_persister();
    let w = wid(0xC2);
    ensure_wallet_meta(&persister, &w);
    let wallet = test_wallet();
    let (collateral_address, _) = two_addresses(&wallet);
    let collateral_tx = payment(&collateral_address, COLLATERAL, 3);
    let collateral = OutPoint::new(collateral_tx.txid(), 0);

    // The ProRegTx was processed first: the lock lands with no coin behind it.
    store_core(
        &persister,
        w,
        CoreChangeSet {
            outpoint_locks: locks(&[(collateral, true)]),
            ..Default::default()
        },
    );
    // The collateral arrives in a later round.
    store_core(
        &persister,
        w,
        CoreChangeSet {
            new_utxos: vec![coin(&collateral_tx, &collateral_address)],
            synced_height: Some(HEIGHT),
            last_processed_height: Some(HEIGHT),
            ..Default::default()
        },
    );
    drop(persister);

    let info = reload(&path, &w, &wallet);
    let balance = WalletInfoInterface::balance(&info);
    assert_eq!(balance.locked(), COLLATERAL);
    assert_eq!(balance.spendable(), 0);
    assert!(spendable_outpoints(&info).is_empty());
}

#[tokio::test]
async fn should_lock_a_coin_that_arrives_after_the_reload() {
    let (persister, _tmp, path) = fresh_persister();
    let w = wid(0xC3);
    ensure_wallet_meta(&persister, &w);
    let mut wallet = test_wallet();
    let (collateral_address, _) = two_addresses(&wallet);
    let collateral_tx = payment(&collateral_address, COLLATERAL, 4);
    let collateral = OutPoint::new(collateral_tx.txid(), 0);

    store_core(
        &persister,
        w,
        CoreChangeSet {
            outpoint_locks: locks(&[(collateral, true)]),
            ..Default::default()
        },
    );
    drop(persister);

    let mut info = reload(&path, &w, &wallet);
    assert!(info.is_outpoint_locked(&collateral));
    assert_eq!(WalletInfoInterface::balance(&info).locked(), 0);

    let block = BlockInfo::new(HEIGHT, BlockHash::all_zeros(), 1_700_000_000);
    let result = info
        .check_core_transaction(
            &collateral_tx,
            TransactionContext::InBlock(block),
            &mut wallet,
            true,
            true,
        )
        .await;
    assert!(result.is_relevant, "the collateral pays this wallet");

    let balance = WalletInfoInterface::balance(&info);
    assert_eq!(balance.locked(), COLLATERAL, "the coin arrives locked");
    assert_eq!(balance.spendable(), 0);
    assert!(spendable_outpoints(&info).is_empty());
}

#[test]
fn should_not_restore_an_unlocked_outpoint() {
    let (persister, _tmp, path) = fresh_persister();
    let w = wid(0xC4);
    ensure_wallet_meta(&persister, &w);
    let wallet = test_wallet();
    let (collateral_address, _) = two_addresses(&wallet);
    let collateral_tx = payment(&collateral_address, COLLATERAL, 5);
    let collateral = OutPoint::new(collateral_tx.txid(), 0);

    store_core(
        &persister,
        w,
        CoreChangeSet {
            new_utxos: vec![coin(&collateral_tx, &collateral_address)],
            outpoint_locks: locks(&[(collateral, true)]),
            synced_height: Some(HEIGHT),
            last_processed_height: Some(HEIGHT),
            ..Default::default()
        },
    );
    store_core(
        &persister,
        w,
        CoreChangeSet {
            outpoint_locks: locks(&[(collateral, false)]),
            ..Default::default()
        },
    );
    drop(persister);

    let info = reload(&path, &w, &wallet);
    assert!(!info.is_outpoint_locked(&collateral));
    let balance = WalletInfoInterface::balance(&info);
    assert_eq!(balance.locked(), 0);
    assert_eq!(balance.spendable(), COLLATERAL);
    assert_eq!(spendable_outpoints(&info), vec![collateral]);
}

#[test]
fn should_keep_the_unlock_when_one_round_locks_then_unlocks() {
    let mut first = CoreChangeSet {
        outpoint_locks: locks(&[(OutPoint::new(dashcore::Txid::all_zeros(), 7), true)]),
        ..Default::default()
    };
    let second = CoreChangeSet {
        outpoint_locks: locks(&[(OutPoint::new(dashcore::Txid::all_zeros(), 7), false)]),
        ..Default::default()
    };
    platform_wallet::changeset::Merge::merge(&mut first, second);
    assert_eq!(
        first.outpoint_locks,
        locks(&[(OutPoint::new(dashcore::Txid::all_zeros(), 7), false)])
    );
}

#[test]
fn should_drop_a_wallets_locks_with_the_wallet() {
    let (persister, _tmp, _path) = fresh_persister();
    let w = wid(0xC5);
    let other = wid(0xC6);
    ensure_wallet_meta(&persister, &w);
    ensure_wallet_meta(&persister, &other);
    let outpoint = OutPoint::new(dashcore::Txid::from_byte_array([0x77; 32]), 1);
    for wallet_id in [w, other] {
        store_core(
            &persister,
            wallet_id,
            CoreChangeSet {
                outpoint_locks: locks(&[(outpoint, true)]),
                ..Default::default()
            },
        );
    }

    persister
        .delete_wallet_skip_backup(w)
        .expect("delete wallet");

    let conn = persister.lock_conn_for_test();
    let count = |wallet_id: &WalletId| -> i64 {
        conn.query_row(
            "SELECT COUNT(*) FROM core_locked_outpoints WHERE wallet_id = ?1",
            [wallet_id.as_slice()],
            |row| row.get(0),
        )
        .expect("count locks")
    };
    assert_eq!(count(&w), 0, "the deleted wallet's locks go with it");
    assert_eq!(count(&other), 1, "another wallet's lock stays");
}

#[test]
fn should_attest_that_locks_survive_a_restart() {
    let (persister, _tmp, _path) = fresh_persister();
    assert!(persister
        .persistence_capabilities()
        .contains(platform_wallet::changeset::PersistenceCapabilities::OUTPOINT_LOCKS));
}
