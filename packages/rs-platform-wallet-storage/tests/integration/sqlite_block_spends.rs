//! Block spends of the wallet's coins reach the SQLite store through the
//! event adapter, and the store drops them only for a truncation, a
//! rewind, a reset, a header-chain mismatch or the wallet's deletion.

use crate::common;

use std::collections::{BTreeMap, BTreeSet};
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use common::{ensure_wallet_meta, fresh_persister};
use dashcore::ephemerealdata::chain_lock::ChainLock;
use dashcore::ephemerealdata::instant_lock::InstantLock;
use dashcore::{BlockHash, OutPoint, ScriptBuf, Transaction, TxIn, TxOut, Txid, Witness};
use key_wallet::test_utils::sign_owned_inputs;
use key_wallet::wallet::initialization::WalletAccountCreationOptions;
use key_wallet::{Utxo, Wallet};
use platform_wallet::changeset::changeset::SweepBatch;
use platform_wallet::changeset::{
    spawn_wallet_event_adapter, AccountRegistrationEntry, BlockSpend, ClientStartState,
    CoreChangeSet, PersistenceCapabilities, PersistenceError, PlatformWalletChangeSet,
    PlatformWalletPersistence, WalletMetadataEntry,
};
use platform_wallet::key_wallet_manager::{WalletEvent, WalletInterface, WalletManager};
use platform_wallet::test_support::core_fixtures::{
    foreign_address, outside, pay, spend_of, wallet_with_receive_address,
};
use platform_wallet::test_support::wallet_manager_with_persistence;
use platform_wallet::wallet::platform_wallet::{PlatformWalletInfo, WalletId};
use platform_wallet_storage::sqlite::schema::{blob, core_state};
use platform_wallet_storage::{SqlitePersister, SqlitePersisterConfig};
use rusqlite::OptionalExtension;
use tokio::sync::{mpsc, RwLock};

/// The SQLite persister, recording each stored round's truncation height.
struct Recording {
    inner: SqlitePersister,
    truncations: Mutex<Vec<Option<u32>>>,
}

impl PlatformWalletPersistence for Recording {
    fn persistence_capabilities(&self) -> PersistenceCapabilities {
        self.inner.persistence_capabilities()
    }

    fn store(
        &self,
        wallet_id: WalletId,
        changeset: PlatformWalletChangeSet,
    ) -> Result<(), PersistenceError> {
        if let Some(core) = changeset.core.as_ref() {
            self.truncations
                .lock()
                .unwrap()
                .push(core.block_spends_truncated_above);
        }
        self.inner.store(wallet_id, changeset)
    }

    fn flush(&self, wallet_id: WalletId) -> Result<(), PersistenceError> {
        self.inner.flush(wallet_id)
    }

    fn load(&self) -> Result<ClientStartState, PersistenceError> {
        self.inner.load()
    }
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
                ..Default::default()
            },
        )
        .expect("register the wallet");
}

async fn persist_queued(
    manager: &Arc<RwLock<WalletManager<PlatformWalletInfo>>>,
    queued: &mut mpsc::UnboundedReceiver<WalletEvent>,
    persister: &Arc<Recording>,
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

fn coin(byte: u8) -> OutPoint {
    OutPoint {
        txid: Txid::from_byte_array([byte; 32]),
        vout: 0,
    }
}

fn spending(coin: OutPoint) -> Transaction {
    Transaction {
        version: 2,
        lock_time: 0,
        input: vec![TxIn {
            previous_output: coin,
            script_sig: ScriptBuf::new(),
            sequence: u32::MAX,
            witness: Witness::new(),
        }],
        output: vec![TxOut {
            value: 1_000,
            script_pubkey: dashcore::Address::dummy(key_wallet::Network::Testnet, 9)
                .script_pubkey(),
        }],
        special_transaction_payload: None,
    }
}

/// A store and a manager whose wallet restored claims on three coins, each
/// spent by a foreign transaction in the block at its height (100, 101,
/// 102), with every round persisted. Returns the block hash at each height.
async fn wallet_with_block_spends() -> (
    Arc<Recording>,
    tempfile::TempDir,
    std::path::PathBuf,
    Arc<RwLock<WalletManager<PlatformWalletInfo>>>,
    WalletId,
    mpsc::UnboundedReceiver<WalletEvent>,
    BTreeMap<u32, BlockHash>,
) {
    let (persister, tmp, path) = fresh_persister();
    let wallet = Wallet::new_random(
        key_wallet::Network::Testnet,
        WalletAccountCreationOptions::Default,
    )
    .expect("wallet");
    let manifest_wallet = wallet.clone();
    let (manager, wallet_id, mut queued) = wallet_manager_with_persistence(wallet);
    register(&persister, wallet_id, &manifest_wallet);
    let persister = Arc::new(Recording {
        inner: persister,
        truncations: Mutex::new(Vec::new()),
    });
    let mut hashes = BTreeMap::new();
    {
        let mut wm = manager.write().await;
        wm.get_wallet_info_mut(&wallet_id)
            .unwrap()
            .restore_spent_outpoints(&[(coin(100), None), (coin(101), None), (coin(102), None)]);
        for height in [100u32, 101, 102] {
            let block = dashcore::Block::dummy(height, vec![spending(coin(height as u8))]);
            hashes.insert(height, block.block_hash());
            wm.process_block_for_wallets(
                &block,
                block.block_hash(),
                height,
                &BTreeSet::from([wallet_id]),
            )
            .await;
        }
        wm.update_wallet_last_processed_height(&wallet_id, 102);
        wm.update_wallet_synced_height(&wallet_id, 102);
    }
    persist_queued(&manager, &mut queued, &persister).await;
    persister.flush(wallet_id).expect("flush");
    (persister, tmp, path, manager, wallet_id, queued, hashes)
}

fn stored(persister: &SqlitePersister, wallet_id: &WalletId) -> BTreeSet<OutPoint> {
    persister
        .block_spent_outpoints(wallet_id)
        .expect("read")
        .into_keys()
        .collect()
}

#[tokio::test]
async fn a_truncate_above_through_the_bridge_deletes_block_spends_above_the_fork() {
    let (persister, _tmp, _path, manager, wallet_id, mut queued, hashes) =
        wallet_with_block_spends().await;
    let rows = persister.inner.block_spent_outpoints(&wallet_id).unwrap();
    assert_eq!(rows.len(), 3);
    assert_eq!(rows[&coin(101)].height, 101);
    assert_eq!(rows[&coin(101)].block_hash, hashes[&101]);
    assert_eq!(rows[&coin(101)].spender_txid, spending(coin(101)).txid());
    persister.truncations.lock().unwrap().clear();

    manager
        .write()
        .await
        .truncate_above(101)
        .expect("truncates");
    persist_queued(&manager, &mut queued, &persister).await;
    persister.flush(wallet_id).expect("flush");

    assert!(persister.truncations.lock().unwrap().contains(&Some(101)));
    assert_eq!(
        stored(&persister.inner, &wallet_id),
        BTreeSet::from([coin(100), coin(101)])
    );
}

/// A stored round's truncation deletes the rows above its fork before
/// its own rows are written, and keeps the rest.
#[tokio::test]
async fn a_stored_truncation_deletes_block_spends_above_the_fork() {
    let (persister, _tmp, _path, _manager, wallet_id, _queued, _) =
        wallet_with_block_spends().await;
    let replacement = BlockSpend {
        outpoint: coin(0x77),
        height: 102,
        block_hash: BlockHash::from_byte_array([0x67; 32]),
        spender_txid: spending(coin(0x77)).txid(),
    };

    persister
        .inner
        .store(
            wallet_id,
            PlatformWalletChangeSet {
                core: Some(CoreChangeSet {
                    block_spends_truncated_above: Some(100),
                    block_spends: vec![replacement],
                    ..Default::default()
                }),
                ..Default::default()
            },
        )
        .expect("store");
    persister.inner.flush(wallet_id).expect("flush");

    assert_eq!(
        stored(&persister.inner, &wallet_id),
        BTreeSet::from([coin(100), coin(0x77)])
    );
}

/// A foreign transaction in a block spending a coin the wallet holds a
/// restored claim on reaches the store as a row with its block's height
/// and hash and the spender, through the event adapter, and the row is
/// still there after a restart.
#[tokio::test]
async fn a_captured_foreign_spend_of_a_claimed_coin_is_stored_and_survives_a_restart() {
    let (persister, _tmp, path, manager, wallet_id, queued, hashes) =
        wallet_with_block_spends().await;
    let rows = persister.inner.block_spent_outpoints(&wallet_id).unwrap();
    assert_eq!(
        rows.keys().copied().collect::<BTreeSet<_>>(),
        BTreeSet::from([coin(100), coin(101), coin(102)])
    );
    for height in [100u32, 101, 102] {
        let row = rows[&coin(height as u8)];
        assert_eq!(row.height, height);
        assert_eq!(row.block_hash, hashes[&height]);
        assert_eq!(row.spender_txid, spending(coin(height as u8)).txid());
    }
    drop((persister, manager, queued));

    let reopened = SqlitePersister::open(SqlitePersisterConfig::new(&path)).expect("reopen");
    reopened.load().expect("load");
    assert_eq!(reopened.block_spent_outpoints(&wallet_id).unwrap(), rows);
}

#[tokio::test]
async fn the_hash_check_deletes_rows_off_the_header_chain_at_every_height() {
    let (persister, _tmp, _path, _manager, wallet_id, _queued, hashes) =
        wallet_with_block_spends().await;
    let replaced = BlockHash::from_byte_array([0xEE; 32]);

    let unknown_and_matching = BTreeMap::from([(101, None), (102, Some(hashes[&102]))]);
    assert_eq!(
        persister
            .inner
            .delete_block_spends_off_chain(&wallet_id, &unknown_and_matching)
            .unwrap(),
        0,
        "an unknown header and a matching one keep their rows"
    );
    assert_eq!(stored(&persister.inner, &wallet_id).len(), 3);

    let chain = BTreeMap::from([
        (100, Some(replaced)),
        (101, Some(replaced)),
        (102, Some(hashes[&102])),
    ]);
    let deleted = persister
        .inner
        .delete_block_spends_off_chain(&wallet_id, &chain)
        .expect("check");

    assert_eq!(deleted, 2, "every mismatch goes, whatever its height");
    assert_eq!(
        stored(&persister.inner, &wallet_id),
        BTreeSet::from([coin(102)]),
        "the row on the chain stays"
    );
}

#[tokio::test]
async fn a_found_block_spend_is_stored_with_its_coin_marked_spent() {
    let (persister, _tmp, _path, _manager, wallet_id, _queued, _) =
        wallet_with_block_spends().await;
    let found = BlockSpend {
        outpoint: coin(103),
        height: 103,
        block_hash: BlockHash::from_byte_array([0x33; 32]),
        spender_txid: spending(coin(103)).txid(),
    };
    persister
        .inner
        .record_found_block_spend(&wallet_id, found)
        .expect("record");
    let rows = persister.inner.block_spent_outpoints(&wallet_id).unwrap();
    assert_eq!(rows[&coin(103)], found);
}

#[tokio::test]
async fn a_rewind_deletes_block_spends_above_its_checkpoint() {
    let (persister, _tmp, _path, _manager, wallet_id, _queued, _) =
        wallet_with_block_spends().await;
    assert_eq!(
        persister
            .inner
            .delete_block_spends_above(&wallet_id, 100)
            .unwrap(),
        2
    );
    assert_eq!(
        stored(&persister.inner, &wallet_id),
        BTreeSet::from([coin(100)])
    );
}

#[tokio::test]
async fn a_reset_clears_every_block_spend_of_the_wallet() {
    let (persister, _tmp, _path, _manager, wallet_id, _queued, _) =
        wallet_with_block_spends().await;
    assert_eq!(persister.inner.clear_block_spends(&wallet_id).unwrap(), 3);
    assert!(stored(&persister.inner, &wallet_id).is_empty());
}

#[tokio::test]
async fn deleting_the_wallet_deletes_its_block_spends() {
    let (persister, _tmp, _path, _manager, wallet_id, _queued, _) =
        wallet_with_block_spends().await;
    persister
        .inner
        .delete_wallet_skip_backup(wallet_id)
        .expect("delete");
    assert!(stored(&persister.inner, &wallet_id).is_empty());
}

#[tokio::test]
async fn block_spends_survive_a_restart() {
    let (persister, _tmp, path, manager, wallet_id, queued, hashes) =
        wallet_with_block_spends().await;
    let before = persister.inner.block_spent_outpoints(&wallet_id).unwrap();
    assert_eq!(before.len(), 3);
    drop((persister, manager, queued));

    let reopened = SqlitePersister::open(SqlitePersisterConfig::new(&path)).expect("reopen");
    reopened.load().expect("load");
    let after = reopened.block_spent_outpoints(&wallet_id).unwrap();
    assert_eq!(after, before);
    assert_eq!(after[&coin(102)].block_hash, hashes[&102]);
    assert_eq!(after[&coin(102)].spender_txid, spending(coin(102)).txid());
}

#[tokio::test]
async fn the_block_spends_migration_is_owed_until_marked_and_survives_a_reset() {
    let (persister, _tmp, _path, _manager, wallet_id, _queued, _) =
        wallet_with_block_spends().await;
    assert!(persister
        .inner
        .persistence_capabilities()
        .contains(PersistenceCapabilities::CORE_BLOCK_SPENDS));
    assert!(
        !persister
            .inner
            .block_spends_migration_done(&wallet_id)
            .unwrap(),
        "a store that just attests the bit owes the migration"
    );

    persister
        .inner
        .mark_block_spends_migration_done(&wallet_id)
        .unwrap();
    persister
        .inner
        .mark_block_spends_migration_done(&wallet_id)
        .unwrap();
    assert!(persister
        .inner
        .block_spends_migration_done(&wallet_id)
        .unwrap());

    persister.inner.clear_block_spends(&wallet_id).unwrap();
    assert!(
        persister
            .inner
            .block_spends_migration_done(&wallet_id)
            .unwrap(),
        "a reset keeps the mark"
    );

    persister
        .inner
        .delete_wallet_skip_backup(wallet_id)
        .expect("delete");
    assert!(
        !persister
            .inner
            .block_spends_migration_done(&wallet_id)
            .unwrap(),
        "a wallet delete removes the mark"
    );
}

// --- A held coin a ChainLocked block spent, and the header check -------------

const ROW_HEIGHT: u32 = 100;
const CHAIN_LOCK_HEIGHT: u32 = 150;

fn row_block_hash() -> BlockHash {
    BlockHash::from_byte_array([0xB1; 32])
}

/// A store holding one wallet coin, spent by a foreign transaction in the
/// block at [`ROW_HEIGHT`] and stored as a block spend, with the wallet's
/// ChainLock at [`CHAIN_LOCK_HEIGHT`] stored after it: the row is at or
/// below the ChainLock.
fn chain_locked_block_spend() -> (
    SqlitePersister,
    tempfile::TempDir,
    std::path::PathBuf,
    WalletId,
    Utxo,
) {
    let (persister, tmp, path) = fresh_persister();
    let wallet_id = [0xC1u8; 32];
    ensure_wallet_meta(&persister, &wallet_id);
    let address = foreign_address(0x31);
    let funding = spend_of(&[outside(0x41)], vec![pay(50_000, &address)]);
    let mut held = Utxo::new(
        OutPoint::new(funding.txid(), 0),
        funding.output[0].clone(),
        address,
        90,
        false,
    );
    held.is_confirmed = true;
    persister
        .store(
            wallet_id,
            PlatformWalletChangeSet {
                core: Some(CoreChangeSet {
                    new_utxos: vec![held.clone()],
                    ..Default::default()
                }),
                ..Default::default()
            },
        )
        .unwrap();
    persister
        .store(
            wallet_id,
            PlatformWalletChangeSet {
                core: Some(CoreChangeSet {
                    block_spends: vec![BlockSpend {
                        outpoint: held.outpoint,
                        height: ROW_HEIGHT,
                        block_hash: row_block_hash(),
                        spender_txid: spending(held.outpoint).txid(),
                    }],
                    last_applied_chain_lock: Some(ChainLock::dummy(CHAIN_LOCK_HEIGHT)),
                    ..Default::default()
                }),
                ..Default::default()
            },
        )
        .unwrap();
    persister.flush(wallet_id).unwrap();
    let chain_lock_height: i64 = persister
        .lock_conn_for_test()
        .query_row(
            "SELECT chainlock_height FROM core_sync_state WHERE wallet_id = ?1",
            rusqlite::params![wallet_id.as_slice()],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(chain_lock_height, i64::from(CHAIN_LOCK_HEIGHT));
    assert!(is_spent(&persister, &wallet_id, &held.outpoint));
    (persister, tmp, path, wallet_id, held)
}

/// Whether the store's one coin row (the held coin's) is spent.
fn is_spent(persister: &SqlitePersister, wallet_id: &WalletId, outpoint: &OutPoint) -> bool {
    let conn = persister.lock_conn_for_test();
    let rows: Vec<bool> = conn
        .prepare("SELECT spent FROM core_utxos WHERE wallet_id = ?1")
        .unwrap()
        .query_map(rusqlite::params![wallet_id.as_slice()], |row| row.get(0))
        .unwrap()
        .map(Result::unwrap)
        .collect();
    assert_eq!(rows.len(), 1, "one coin row, {outpoint}'s");
    rows[0]
}

/// What a release of the coin stores: a sweep whose loser (a dropped
/// payment, or a conflict's loser) gave it up.
fn release(persister: &SqlitePersister, wallet_id: WalletId, coin: OutPoint) {
    persister
        .store(
            wallet_id,
            PlatformWalletChangeSet {
                core: Some(CoreChangeSet {
                    sweeps: vec![SweepBatch {
                        txids: vec![Txid::from_byte_array([0x99; 32])],
                        superseded_by: Txid::from_byte_array([0xAB; 32]),
                        winner_mined_height: None,
                        released_outpoints: vec![coin],
                    }],
                    ..Default::default()
                }),
                ..Default::default()
            },
        )
        .unwrap();
    persister.flush(wallet_id).unwrap();
}

/// T-c1: a row at or below the ChainLock whose block hash differs from the
/// known header at its height is deleted, and the coin comes back through
/// the normal release.
#[test]
fn t_c1_a_row_below_the_chain_lock_off_the_header_chain_is_deleted() {
    let (persister, _tmp, _path, wallet_id, held) = chain_locked_block_spend();
    let chain = BTreeMap::from([(ROW_HEIGHT, Some(BlockHash::from_byte_array([0xEE; 32])))]);

    let deleted = persister
        .delete_block_spends_off_chain(&wallet_id, &chain)
        .unwrap();

    assert_eq!(deleted, 1, "the ChainLock does not exempt the row");
    assert!(stored(&persister, &wallet_id).is_empty());
    release(&persister, wallet_id, held.outpoint);
    assert!(
        !is_spent(&persister, &wallet_id, &held.outpoint),
        "with the stale row gone the release frees the coin"
    );
}

/// T-c2: a row at or below the ChainLock whose header matches is kept, and
/// keeps its coin spent through a release.
#[test]
fn t_c2_a_row_below_the_chain_lock_on_the_header_chain_is_kept() {
    let (persister, _tmp, _path, wallet_id, held) = chain_locked_block_spend();
    let chain = BTreeMap::from([(ROW_HEIGHT, Some(row_block_hash()))]);

    assert_eq!(
        persister
            .delete_block_spends_off_chain(&wallet_id, &chain)
            .unwrap(),
        0
    );

    assert_eq!(
        stored(&persister, &wallet_id),
        BTreeSet::from([held.outpoint])
    );
    release(&persister, wallet_id, held.outpoint);
    assert!(is_spent(&persister, &wallet_id, &held.outpoint));
}

/// T-c3: a row at or below the ChainLock with an unknown header (`None`, or
/// a height the chain does not name) is kept, and keeps its coin spent.
#[test]
fn t_c3_a_row_below_the_chain_lock_with_an_unknown_header_is_kept() {
    let (persister, _tmp, _path, wallet_id, held) = chain_locked_block_spend();
    for chain in [
        BTreeMap::from([(ROW_HEIGHT, None)]),
        BTreeMap::from([(ROW_HEIGHT + 1, Some(BlockHash::from_byte_array([0xEE; 32])))]),
        BTreeMap::new(),
    ] {
        assert_eq!(
            persister
                .delete_block_spends_off_chain(&wallet_id, &chain)
                .unwrap(),
            0
        );
    }

    assert_eq!(
        stored(&persister, &wallet_id),
        BTreeSet::from([held.outpoint])
    );
    release(&persister, wallet_id, held.outpoint);
    assert!(is_spent(&persister, &wallet_id, &held.outpoint));
}

/// A row outlives a drop: the release of its coin by a dropped payment's
/// sweep, the ChainLock and height advance past it, and a restart all
/// leave the row, and its coin stays spent and out of the load.
#[test]
fn a_row_outlives_a_drop_the_chain_lock_and_a_restart() {
    let (persister, _tmp, path, wallet_id, held) = chain_locked_block_spend();
    let before = persister.block_spent_outpoints(&wallet_id).unwrap();

    release(&persister, wallet_id, held.outpoint);
    persister
        .store(
            wallet_id,
            PlatformWalletChangeSet {
                core: Some(CoreChangeSet {
                    last_applied_chain_lock: Some(ChainLock::dummy(CHAIN_LOCK_HEIGHT + 50)),
                    synced_height: Some(CHAIN_LOCK_HEIGHT + 50),
                    last_processed_height: Some(CHAIN_LOCK_HEIGHT + 50),
                    ..Default::default()
                }),
                ..Default::default()
            },
        )
        .unwrap();
    persister.flush(wallet_id).unwrap();
    assert_eq!(persister.block_spent_outpoints(&wallet_id).unwrap(), before);
    assert!(is_spent(&persister, &wallet_id, &held.outpoint));
    drop(persister);

    let reopened = SqlitePersister::open(SqlitePersisterConfig::new(&path)).expect("reopen");
    assert_eq!(reopened.block_spent_outpoints(&wallet_id).unwrap(), before);
    assert!(is_spent(&reopened, &wallet_id, &held.outpoint));
}

// --- The migration adopts a downstream build's tables ------------------------

/// The tables and index exactly as a downstream build created them before
/// this migration existed, without `IF NOT EXISTS`.
const DOWNSTREAM_BLOCK_SPENDS_DDL: &str = "\
CREATE TABLE core_block_spends (
    wallet_id BLOB NOT NULL,
    outpoint BLOB NOT NULL,
    height INTEGER NOT NULL,
    block_hash BLOB NOT NULL,
    spender_txid BLOB NOT NULL,
    PRIMARY KEY (wallet_id, outpoint),
    FOREIGN KEY (wallet_id) REFERENCES wallets(wallet_id) ON DELETE CASCADE
);

CREATE INDEX idx_core_block_spends_height
    ON core_block_spends(wallet_id, height);

CREATE TABLE core_block_spends_migrated (
    wallet_id BLOB NOT NULL PRIMARY KEY,
    migrated_at INTEGER NOT NULL DEFAULT (unixepoch()),
    FOREIGN KEY (wallet_id) REFERENCES wallets(wallet_id) ON DELETE CASCADE
);";

/// Everything SQLite knows about the two tables' shape: stored DDL,
/// columns, indexes and foreign keys.
fn block_spend_tables_shape(conn: &rusqlite::Connection) -> Vec<String> {
    let mut shape = Vec::new();
    for table in ["core_block_spends", "core_block_spends_migrated"] {
        for sql in [
            format!(
                "SELECT type || ' ' || name || ' ' || tbl_name || ' ' || ifnull(sql, 'NULL') \
                 FROM sqlite_master WHERE tbl_name = '{table}' ORDER BY type, name"
            ),
            format!(
                "SELECT cid || ' ' || name || ' ' || type || ' ' || \"notnull\" || ' ' || \
                        ifnull(dflt_value, 'NULL') || ' ' || pk || ' ' || hidden \
                 FROM pragma_table_xinfo('{table}')"
            ),
            format!(
                "SELECT name || ' ' || \"unique\" || ' ' || origin || ' ' || partial \
                 FROM pragma_index_list('{table}') ORDER BY name"
            ),
            format!(
                "SELECT id || ' ' || seq || ' ' || \"table\" || ' ' || \"from\" || ' ' || \
                        \"to\" || ' ' || on_update || ' ' || on_delete || ' ' || \"match\" \
                 FROM pragma_foreign_key_list('{table}')"
            ),
        ] {
            let mut stmt = conn.prepare(&sql).unwrap();
            let rows = stmt
                .query_map([], |r| r.get::<_, String>(0))
                .unwrap()
                .collect::<Result<Vec<_>, _>>()
                .unwrap();
            assert!(!rows.is_empty(), "no shape rows for {sql}");
            shape.extend(rows);
        }
    }
    let mut stmt = conn
        .prepare(
            "SELECT seqno || ' ' || cid || ' ' || ifnull(name, '') \
             FROM pragma_index_info('idx_core_block_spends_height')",
        )
        .unwrap();
    shape.extend(
        stmt.query_map([], |r| r.get::<_, String>(0))
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap(),
    );
    shape
}

/// A file a downstream build wrote already holds both tables and the index,
/// with rows, but not this migration in its history. The migration adopts
/// them in place: the rows survive, the readers see them, and the file ends
/// up with exactly the shape of a fresh one.
#[test]
fn the_migration_adopts_downstream_block_spend_tables_with_their_rows() {
    use platform_wallet_storage::sqlite::migrations;

    let mut fresh = rusqlite::Connection::open_in_memory().unwrap();
    migrations::run(&mut fresh).unwrap();
    let fresh_shape = block_spend_tables_shape(&fresh);

    let (persister, _tmp, path) = fresh_persister();
    drop(persister);
    let wallet_id = [0xD1u8; 32];
    let spend = BlockSpend {
        outpoint: coin(0x51),
        height: 120,
        block_hash: BlockHash::from_byte_array([0x52; 32]),
        spender_txid: Txid::from_byte_array([0x53; 32]),
    };
    {
        let mut conn = rusqlite::Connection::open(&path).unwrap();
        conn.execute_batch(
            "DROP TABLE core_block_spends; DROP TABLE core_block_spends_migrated; \
             DROP TABLE core_dismissed_txids; \
             DELETE FROM refinery_schema_history WHERE version >= 25;",
        )
        .unwrap();
        conn.execute_batch(DOWNSTREAM_BLOCK_SPENDS_DDL).unwrap();
        conn.execute(
            "INSERT INTO wallets (wallet_id, network, birth_height) VALUES (?1, 'testnet', 0)",
            rusqlite::params![wallet_id.as_slice()],
        )
        .unwrap();
        let tx = conn.transaction().unwrap();
        tx.execute(
            "INSERT INTO core_block_spends \
                (wallet_id, outpoint, height, block_hash, spender_txid) \
             VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![
                wallet_id.as_slice(),
                platform_wallet_storage::sqlite::schema::blob::encode_outpoint(&spend.outpoint)
                    .unwrap(),
                i64::from(spend.height),
                spend.block_hash.as_byte_array().as_slice(),
                spend.spender_txid.as_byte_array().as_slice(),
            ],
        )
        .unwrap();
        tx.execute(
            "INSERT INTO core_block_spends_migrated (wallet_id) VALUES (?1)",
            rusqlite::params![wallet_id.as_slice()],
        )
        .unwrap();
        tx.commit().unwrap();
    }

    let persister = SqlitePersister::open(SqlitePersisterConfig::new(&path)).expect("reopen");
    {
        let conn = persister.lock_conn_for_test();
        let version: i64 = conn
            .query_row(
                "SELECT MAX(version) FROM refinery_schema_history",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(
            version,
            platform_wallet_storage::sqlite::migrations::max_supported_version()
        );
        assert_eq!(block_spend_tables_shape(&conn), fresh_shape);
    }
    assert_eq!(
        persister.block_spent_outpoints(&wallet_id).unwrap(),
        BTreeMap::from([(spend.outpoint, spend)])
    );
    assert!(persister.block_spends_migration_done(&wallet_id).unwrap());
}

/// Process a block at `height` whose one transaction spends `spent`, and
/// advance the wallet's processed and synced heights to it.
async fn process_spend(
    wm: &mut WalletManager<PlatformWalletInfo>,
    wallet_id: WalletId,
    height: u32,
    spent: OutPoint,
) {
    let block = dashcore::Block::dummy(height, vec![spending(spent)]);
    wm.process_block_for_wallets(
        &block,
        block.block_hash(),
        height,
        &BTreeSet::from([wallet_id]),
    )
    .await;
    wm.update_wallet_last_processed_height(&wallet_id, height);
    wm.update_wallet_synced_height(&wallet_id, height);
}

/// A wallet whose block at 100 spent `coin(0x60)`, stored, and whose
/// old-branch block at 101 spent `coin(0x61)` with its events still queued
/// when a reorg truncates above 100 and a new-branch block at 101 spends
/// `coin(0x62)`. Returns the queued events in emission order.
async fn reorg_with_a_queued_old_branch_event() -> (
    Arc<Recording>,
    tempfile::TempDir,
    Arc<RwLock<WalletManager<PlatformWalletInfo>>>,
    WalletId,
    Vec<WalletEvent>,
) {
    let (persister, tmp, _path) = fresh_persister();
    let wallet = Wallet::new_random(
        key_wallet::Network::Testnet,
        WalletAccountCreationOptions::Default,
    )
    .expect("wallet");
    let manifest_wallet = wallet.clone();
    let (manager, wallet_id, mut queued) = wallet_manager_with_persistence(wallet);
    register(&persister, wallet_id, &manifest_wallet);
    let persister = Arc::new(Recording {
        inner: persister,
        truncations: Mutex::new(Vec::new()),
    });
    let (base, old, new) = (coin(0x60), coin(0x61), coin(0x62));
    {
        let mut wm = manager.write().await;
        wm.get_wallet_info_mut(&wallet_id)
            .unwrap()
            .restore_spent_outpoints(&[(base, None), (old, None), (new, None)]);
        process_spend(&mut wm, wallet_id, 100, base).await;
    }
    persist_queued(&manager, &mut queued, &persister).await;
    persister.flush(wallet_id).expect("flush");
    assert_eq!(stored(&persister.inner, &wallet_id), BTreeSet::from([base]));

    let mut events = Vec::new();
    {
        let mut wm = manager.write().await;
        process_spend(&mut wm, wallet_id, 101, old).await;
        while let Ok(event) = queued.try_recv() {
            events.push(event);
        }
        assert!(!events.is_empty(), "the old branch's block queues an event");
        wm.truncate_above(100).expect("truncates");
        let new_block = dashcore::Block::dummy(101, vec![spending(new)]);
        wm.process_block_for_wallets(
            &new_block,
            new_block.block_hash(),
            101,
            &BTreeSet::from([wallet_id]),
        )
        .await;
    }
    while let Ok(event) = queued.try_recv() {
        events.push(event);
    }
    assert!(
        matches!(
            events.last(),
            Some(WalletEvent::ChainTruncated { height: 100, .. })
        ),
        "the truncation is queued after the old branch's events: {events:?}"
    );
    (persister, tmp, manager, wallet_id, events)
}

async fn persist_events(
    manager: &Arc<RwLock<WalletManager<PlatformWalletInfo>>>,
    events: Vec<WalletEvent>,
    persister: &Arc<Recording>,
    wallet_id: WalletId,
) {
    let (tx, mut rx) = mpsc::unbounded_channel();
    for event in events {
        tx.send(event).expect("channel");
    }
    drop(tx);
    persist_queued(manager, &mut rx, persister).await;
    persister.flush(wallet_id).expect("flush");
}

/// The adapter projects the old branch's queued event after the reorg: it
/// drains the new branch's spend, and the truncation projected after it
/// must not delete that spend's row.
#[tokio::test]
async fn a_spend_captured_after_a_truncation_survives_an_older_queued_event() {
    let (persister, _tmp, manager, wallet_id, events) =
        reorg_with_a_queued_old_branch_event().await;

    persist_events(&manager, events, &persister, wallet_id).await;

    assert_eq!(
        stored(&persister.inner, &wallet_id),
        BTreeSet::from([coin(0x60), coin(0x62)]),
        "the new branch's spend at 101 is kept; the old branch's is gone"
    );
}

/// The host drains and stores the captured spends itself (before a
/// release) between the reorg and the adapter reaching its truncation: the
/// truncation must not delete the row the host just wrote.
#[tokio::test]
async fn a_spend_the_host_stored_after_a_truncation_survives_the_truncation_event() {
    let (persister, _tmp, manager, wallet_id, events) =
        reorg_with_a_queued_old_branch_event().await;
    let spends = manager
        .read()
        .await
        .get_wallet_info(&wallet_id)
        .unwrap()
        .drain_block_spends();
    assert_eq!(
        spends.iter().map(|s| s.outpoint).collect::<Vec<_>>(),
        vec![coin(0x62)]
    );
    persister
        .store(
            wallet_id,
            PlatformWalletChangeSet {
                core: Some(CoreChangeSet {
                    block_spends: spends,
                    ..Default::default()
                }),
                ..Default::default()
            },
        )
        .expect("store");
    persister.flush(wallet_id).expect("flush");
    assert_eq!(
        stored(&persister.inner, &wallet_id),
        BTreeSet::from([coin(0x60), coin(0x62)])
    );

    persist_events(&manager, events, &persister, wallet_id).await;

    assert_eq!(
        stored(&persister.inner, &wallet_id),
        BTreeSet::from([coin(0x60), coin(0x62)]),
        "the truncation at 100 does not delete the new branch's row at 101"
    );
}

/// A wrapper truncation that returned Ok but whose `ChainTruncated` never
/// comes: what a direct call leaves, and what the manager would leave if a
/// later wallet's truncation failed after this one's (it emits the events
/// only once every wallet's returned Ok; at the pinned rev its pre-check
/// makes that failure unreachable). The spends captured after it and after
/// the next truncation must both survive the next `ChainTruncated` round
/// when a late older event drained them first.
#[tokio::test]
async fn a_truncation_without_its_event_keeps_every_later_spend_through_the_next_one() {
    use key_wallet::wallet::managed_wallet_info::wallet_info_interface::WalletInfoInterface;

    let (persister, _tmp, _path) = fresh_persister();
    let wallet = Wallet::new_random(
        key_wallet::Network::Testnet,
        WalletAccountCreationOptions::Default,
    )
    .expect("wallet");
    let manifest_wallet = wallet.clone();
    let (manager, wallet_id, mut queued) = wallet_manager_with_persistence(wallet);
    register(&persister, wallet_id, &manifest_wallet);
    let persister = Arc::new(Recording {
        inner: persister,
        truncations: Mutex::new(Vec::new()),
    });
    let (base, old, first, dropped, second) =
        (coin(0x70), coin(0x71), coin(0x72), coin(0x73), coin(0x74));
    {
        let mut wm = manager.write().await;
        wm.get_wallet_info_mut(&wallet_id)
            .unwrap()
            .restore_spent_outpoints(&[
                (base, None),
                (old, None),
                (first, None),
                (dropped, None),
                (second, None),
            ]);
        process_spend(&mut wm, wallet_id, 100, base).await;
    }
    persist_queued(&manager, &mut queued, &persister).await;
    persister.flush(wallet_id).expect("flush");

    let mut events = Vec::new();
    {
        let mut wm = manager.write().await;
        process_spend(&mut wm, wallet_id, 101, old).await;
        while let Ok(event) = queued.try_recv() {
            events.push(event);
        }
        // What the manager leaves when a later wallet's truncation fails:
        // this wallet's own truncation returned Ok, and no event follows.
        WalletInfoInterface::truncate_above(wm.get_wallet_info_mut(&wallet_id).unwrap(), 100)
            .expect("truncates");
        process_spend(&mut wm, wallet_id, 101, first).await;
        process_spend(&mut wm, wallet_id, 102, dropped).await;
        wm.truncate_above(101).expect("truncates");
        let block = dashcore::Block::dummy(102, vec![spending(second)]);
        wm.process_block_for_wallets(
            &block,
            block.block_hash(),
            102,
            &BTreeSet::from([wallet_id]),
        )
        .await;
    }
    while let Ok(event) = queued.try_recv() {
        events.push(event);
    }
    let truncations: Vec<u32> = events
        .iter()
        .filter_map(|event| match event {
            WalletEvent::ChainTruncated { height, .. } => Some(*height),
            _ => None,
        })
        .collect();
    assert_eq!(
        truncations,
        vec![101],
        "only the manager's truncation emits"
    );

    persist_events(&manager, events, &persister, wallet_id).await;

    assert_eq!(
        stored(&persister.inner, &wallet_id),
        BTreeSet::from([base, first, second]),
        "every spend captured after either truncation is kept"
    );
}

/// Two truncations, the second deeper, and a late older event, all stored
/// in one adapter drain: the spend captured after the deeper truncation is
/// kept, and every spend above it from before is gone.
#[tokio::test]
async fn a_deeper_second_truncation_in_one_drain_keeps_only_the_spends_captured_after_it() {
    let (persister, _tmp, _path) = fresh_persister();
    let wallet = Wallet::new_random(
        key_wallet::Network::Testnet,
        WalletAccountCreationOptions::Default,
    )
    .expect("wallet");
    let manifest_wallet = wallet.clone();
    let (manager, wallet_id, mut queued) = wallet_manager_with_persistence(wallet);
    register(&persister, wallet_id, &manifest_wallet);
    let persister = Arc::new(Recording {
        inner: persister,
        truncations: Mutex::new(Vec::new()),
    });
    let (base, old_101, old_102, between, after) =
        (coin(0x80), coin(0x81), coin(0x82), coin(0x83), coin(0x84));
    {
        let mut wm = manager.write().await;
        wm.get_wallet_info_mut(&wallet_id)
            .unwrap()
            .restore_spent_outpoints(&[
                (base, None),
                (old_101, None),
                (old_102, None),
                (between, None),
                (after, None),
            ]);
        process_spend(&mut wm, wallet_id, 100, base).await;
        process_spend(&mut wm, wallet_id, 101, old_101).await;
    }
    persist_queued(&manager, &mut queued, &persister).await;
    persister.flush(wallet_id).expect("flush");
    assert_eq!(
        stored(&persister.inner, &wallet_id),
        BTreeSet::from([base, old_101])
    );

    {
        let mut wm = manager.write().await;
        process_spend(&mut wm, wallet_id, 102, old_102).await;
        wm.truncate_above(101).expect("truncates");
        process_spend(&mut wm, wallet_id, 102, between).await;
        wm.truncate_above(100).expect("truncates");
        process_spend(&mut wm, wallet_id, 101, after).await;
    }
    persist_queued(&manager, &mut queued, &persister).await;
    persister.flush(wallet_id).expect("flush");

    assert_eq!(
        stored(&persister.inner, &wallet_id),
        BTreeSet::from([base, after]),
        "only the spend captured after the deeper truncation is kept above it"
    );
}

// --- Block spends captured by the forwarded conflict sweep ------------------

/// A store and a manager holding a wallet with an unconfirmed `payment`,
/// beside a second wallet the blocks are fetched for. With `funded`, the
/// wallet holds `coin` (funded at 100 by `funding`) and the payment spends
/// it to a stranger; without, the funding is unknown and the payment pays
/// the wallet. The payment is InstantSend-locked with `locked`. Every
/// round so far is stored.
struct PayingWallet {
    persister: Arc<Recording>,
    _tmp: tempfile::TempDir,
    path: std::path::PathBuf,
    manager: Arc<RwLock<WalletManager<PlatformWalletInfo>>>,
    held: WalletId,
    fetched: WalletId,
    coin: OutPoint,
    funding: Transaction,
    address: dashcore::Address,
    payment: Transaction,
    queued: mpsc::UnboundedReceiver<WalletEvent>,
}

async fn paying_wallet(funded: bool, locked: bool) -> PayingWallet {
    use key_wallet::wallet::managed_wallet_info::wallet_info_interface::WalletInfoInterface;

    let (persister, tmp, path) = fresh_persister();
    let (wallet, core_wallet, address) = wallet_with_receive_address();
    let other = Wallet::new_random(
        key_wallet::Network::Testnet,
        WalletAccountCreationOptions::Default,
    )
    .expect("wallet");
    let mut wm = WalletManager::<PlatformWalletInfo>::new(key_wallet::Network::Testnet);
    let mut queued = wm.take_persistence_receiver().expect("receiver");
    let mut info = PlatformWalletInfo::from_wallet(&wallet, 0);
    info.core_wallet = core_wallet;
    let held = wm.insert_wallet(wallet.clone(), info).expect("insert");
    let other_info = PlatformWalletInfo::from_wallet(&other, 0);
    let fetched = wm.insert_wallet(other.clone(), other_info).expect("insert");
    register(&persister, held, &wallet);
    register(&persister, fetched, &other);

    let funding = spend_of(&[outside(0xF3)], vec![pay(50_000, &address)]);
    let coin = OutPoint::new(funding.txid(), 0);
    let payee = if funded {
        foreign_address(8)
    } else {
        address.clone()
    };
    let payment = sign_owned_inputs(
        &wallet,
        &wm.get_wallet_info(&held).expect("wallet").core_wallet,
        spend_of(&[coin], vec![pay(40_000, &payee)]),
        &[&funding],
    )
    .await;
    if funded {
        let block = dashcore::Block::dummy(100, vec![funding.clone()]);
        wm.process_block_for_wallets(&block, block.block_hash(), 100, &BTreeSet::from([held]))
            .await;
        wm.update_wallet_last_processed_height(&held, 100);
        wm.update_wallet_synced_height(&held, 100);
    }
    let lock = locked.then(|| InstantLock {
        txid: payment.txid(),
        ..InstantLock::default()
    });
    wm.process_mempool_transaction(&payment, lock).await;
    assert_eq!(
        wm.get_wallet_info(&held)
            .unwrap()
            .unconfirmed_spent_outpoints()
            .into_keys()
            .collect::<Vec<_>>(),
        vec![coin]
    );

    let persister = Arc::new(Recording {
        inner: persister,
        truncations: Mutex::new(Vec::new()),
    });
    let manager = Arc::new(RwLock::new(wm));
    persist_queued(&manager, &mut queued, &persister).await;
    persister.flush(held).expect("flush");
    PayingWallet {
        persister,
        _tmp: tmp,
        path,
        manager,
        held,
        fetched,
        coin,
        funding,
        address,
        payment,
        queued,
    }
}

/// The coins the store would offer `wallet_id`.
fn offered(persister: &SqlitePersister, wallet_id: &WalletId) -> BTreeSet<OutPoint> {
    core_state::list_unspent_utxos(&persister.lock_conn_for_test(), wallet_id)
        .unwrap()
        .into_values()
        .flatten()
        .map(|row| row.outpoint)
        .collect()
}

fn has_coin_row(persister: &SqlitePersister, wallet_id: &WalletId, outpoint: &OutPoint) -> bool {
    let key = blob::encode_outpoint(outpoint).unwrap();
    persister
        .lock_conn_for_test()
        .query_row(
            "SELECT 1 FROM core_utxos WHERE wallet_id = ?1 AND outpoint = ?2",
            rusqlite::params![wallet_id.as_slice(), &key[..]],
            |_| Ok(()),
        )
        .optional()
        .unwrap()
        .is_some()
}

/// The ChainLock path: a plain block fetched for another wallet cannot beat
/// the InstantSend-locked payment; the ChainLock above it does. The stored
/// row is in the winner's block, not the ChainLock's, and the header check
/// keeps it.
#[tokio::test]
async fn a_chain_lock_sweep_stores_the_coin_in_the_winners_block_and_the_header_check_keeps_it() {
    let mut p = paying_wallet(true, true).await;
    let winner = spending(p.coin);
    let block = dashcore::Block::dummy(150, vec![winner.clone()]);
    let chain_lock = ChainLock {
        block_hash: BlockHash::from_byte_array([0xCC; 32]),
        ..ChainLock::dummy(160)
    };
    {
        let mut wm = p.manager.write().await;
        wm.process_block_for_wallets(
            &block,
            block.block_hash(),
            150,
            &BTreeSet::from([p.fetched]),
        )
        .await;
        // Only the ChainLock's capture reaches the store.
        wm.get_wallet_info(&p.held).unwrap().drain_block_spends();
        wm.apply_chain_lock(chain_lock.clone());
    }
    persist_queued(&p.manager, &mut p.queued, &p.persister).await;
    p.persister.flush(p.held).expect("flush");

    let expected = BTreeMap::from([(
        p.coin,
        BlockSpend {
            outpoint: p.coin,
            height: 150,
            block_hash: block.block_hash(),
            spender_txid: winner.txid(),
        },
    )]);
    assert_eq!(
        p.persister.inner.block_spent_outpoints(&p.held).unwrap(),
        expected
    );
    let chain = BTreeMap::from([
        (150, Some(block.block_hash())),
        (160, Some(chain_lock.block_hash)),
    ]);
    assert_eq!(
        p.persister
            .inner
            .delete_block_spends_off_chain(&p.held, &chain)
            .unwrap(),
        0
    );
    assert_eq!(
        p.persister.inner.block_spent_outpoints(&p.held).unwrap(),
        expected
    );
    assert!(!offered(&p.persister.inner, &p.held).contains(&p.coin));
}

/// A coin the wallet only has a claim on: its funding is not in the store,
/// so the sweep of the payment holds it as a placeholder, which the
/// ChainLock boundary then collects. The winner's row outlives that, so
/// the funding arriving later, a release and a restart never offer it.
#[tokio::test]
async fn a_claimed_coin_the_forwarded_sweep_took_is_never_offered_once_its_placeholder_is_gone() {
    let mut p = paying_wallet(false, false).await;
    let winner = spending(p.coin);
    {
        let mut wm = p.manager.write().await;
        wm.get_wallet_info_mut(&p.held)
            .unwrap()
            .restore_spent_outpoints(&[(p.coin, Some(p.payment.txid()))]);
        wm.apply_chain_lock(ChainLock::dummy(200));
        let block = dashcore::Block::dummy(150, vec![winner.clone()]);
        wm.process_block_for_wallets(
            &block,
            block.block_hash(),
            150,
            &BTreeSet::from([p.fetched]),
        )
        .await;
        wm.update_wallet_last_processed_height(&p.held, 200);
        wm.update_wallet_synced_height(&p.held, 200);
    }
    persist_queued(&p.manager, &mut p.queued, &p.persister).await;
    p.persister.flush(p.held).expect("flush");
    assert!(
        !has_coin_row(&p.persister.inner, &p.held, &p.coin),
        "the ChainLock boundary collected the placeholder"
    );
    let rows = p.persister.inner.block_spent_outpoints(&p.held).unwrap();
    assert_eq!(rows[&p.coin].spender_txid, winner.txid());
    assert_eq!(rows[&p.coin].height, 150);

    let mut funded = Utxo::new(
        p.coin,
        p.funding.output[0].clone(),
        p.address.clone(),
        100,
        false,
    );
    funded.is_confirmed = true;
    p.persister
        .inner
        .store(
            p.held,
            PlatformWalletChangeSet {
                core: Some(CoreChangeSet {
                    new_utxos: vec![funded],
                    ..Default::default()
                }),
                ..Default::default()
            },
        )
        .unwrap();
    p.persister.inner.flush(p.held).unwrap();
    release(&p.persister.inner, p.held, p.coin);
    assert!(!offered(&p.persister.inner, &p.held).contains(&p.coin));

    let (held, path) = (p.held, p.path.clone());
    drop(p.manager);
    drop(p.persister);
    let reopened = SqlitePersister::open(SqlitePersisterConfig::new(&path)).expect("reopen");
    reopened.load().expect("load");
    assert!(!offered(&reopened, &held).contains(&p.coin));
    assert_eq!(reopened.block_spent_outpoints(&held).unwrap(), rows);
}

/// A late event drains the forwarded sweep's capture after a reorg, and the
/// truncation projected after it keeps the row.
#[tokio::test]
async fn a_forwarded_sweep_capture_after_a_truncation_survives_an_older_queued_event() {
    let mut p = paying_wallet(true, false).await;
    let old = coin(0x65);
    let winner = spending(p.coin);
    let new_block = dashcore::Block::dummy(101, vec![winner.clone()]);
    let mut events = Vec::new();
    {
        let mut wm = p.manager.write().await;
        wm.get_wallet_info_mut(&p.held)
            .unwrap()
            .restore_spent_outpoints(&[(old, None)]);
        process_spend(&mut wm, p.held, 101, old).await;
        while let Ok(event) = p.queued.try_recv() {
            events.push(event);
        }
        wm.truncate_above(100).expect("truncates");
        wm.process_block_for_wallets(
            &new_block,
            new_block.block_hash(),
            101,
            &BTreeSet::from([p.fetched]),
        )
        .await;
    }
    while let Ok(event) = p.queued.try_recv() {
        events.push(event);
    }
    let truncated = events
        .iter()
        .position(|event| matches!(event, WalletEvent::ChainTruncated { height: 100, .. }))
        .expect("the reorg's truncation is queued");
    assert!(
        events[truncated..]
            .iter()
            .any(|event| matches!(event, WalletEvent::TransactionsSwept { .. })),
        "the forwarded sweep runs after the truncation"
    );

    persist_events(&p.manager, events, &p.persister, p.held).await;

    let rows = p.persister.inner.block_spent_outpoints(&p.held).unwrap();
    assert_eq!(
        rows.keys().copied().collect::<BTreeSet<_>>(),
        BTreeSet::from([p.coin]),
        "the old branch's spend is gone and the new branch's is kept"
    );
    assert_eq!(rows[&p.coin].height, 101);
    assert_eq!(rows[&p.coin].block_hash, new_block.block_hash());
    assert_eq!(rows[&p.coin].spender_txid, winner.txid());
}

// --- A coin whose stale row is gone, across a restart ----------------------

/// The header check deletes a stale row, the wallet reopens, and the next
/// release of the coin frees it: nothing but the release is needed.
#[test]
fn a_coin_comes_back_through_the_release_after_a_restart_once_its_stale_row_is_gone() {
    let (persister, _tmp, path, wallet_id, held) = chain_locked_block_spend();
    let chain = BTreeMap::from([(ROW_HEIGHT, Some(BlockHash::from_byte_array([0xEE; 32])))]);
    assert_eq!(
        persister
            .delete_block_spends_off_chain(&wallet_id, &chain)
            .unwrap(),
        1
    );
    drop(persister);

    let reopened = SqlitePersister::open(SqlitePersisterConfig::new(&path)).expect("reopen");
    reopened.load().expect("load");
    assert!(is_spent(&reopened, &wallet_id, &held.outpoint));
    assert!(offered(&reopened, &wallet_id).is_empty());
    release(&reopened, wallet_id, held.outpoint);

    assert!(!is_spent(&reopened, &wallet_id, &held.outpoint));
    assert_eq!(
        offered(&reopened, &wallet_id),
        BTreeSet::from([held.outpoint])
    );
}

/// The wallet delivers the coin unspent again while its row stands (the
/// block that spent it was replaced): the row keeps it out of the load, and
/// once the header check deletes the row the coin is offered, also after a
/// restart. The wallet already holds it unspent, so no release will come.
#[test]
fn a_coin_delivered_unspent_while_its_row_stands_is_offered_once_the_row_is_gone() {
    let (persister, _tmp, path, wallet_id, held) = chain_locked_block_spend();
    persister
        .store(
            wallet_id,
            PlatformWalletChangeSet {
                core: Some(CoreChangeSet {
                    new_utxos: vec![held.clone()],
                    ..Default::default()
                }),
                ..Default::default()
            },
        )
        .unwrap();
    persister.flush(wallet_id).unwrap();
    assert!(
        offered(&persister, &wallet_id).is_empty(),
        "the row hides it"
    );

    let chain = BTreeMap::from([(ROW_HEIGHT, Some(BlockHash::from_byte_array([0xEE; 32])))]);
    assert_eq!(
        persister
            .delete_block_spends_off_chain(&wallet_id, &chain)
            .unwrap(),
        1
    );
    assert_eq!(
        offered(&persister, &wallet_id),
        BTreeSet::from([held.outpoint])
    );
    drop(persister);

    let reopened = SqlitePersister::open(SqlitePersisterConfig::new(&path)).expect("reopen");
    assert_eq!(
        offered(&reopened, &wallet_id),
        BTreeSet::from([held.outpoint])
    );
}
