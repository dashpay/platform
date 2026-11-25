//! A DashPay contact's payment-xpub rotation (V024).
//!
//! The rotated contact's `DashpayExternalAccount` keeps its account key but
//! derives a new pool. The registration flush retires the old xpub's pool rows
//! to `core_address_pool_retired` in the same transaction that replaces the
//! xpub, so the new pool lands, the old addresses stay known for history, and
//! nothing payable is ever built from them.

use std::collections::{BTreeMap, BTreeSet};

use crate::common::{
    ensure_wallet_meta, fresh_persister, fresh_persister_with_mode, wid, FlushMode,
};
use dashcore::{Address, OutPoint, Transaction, Txid};
use key_wallet::account::{AccountType, StandardAccountType};
use key_wallet::bip32::ExtendedPubKey;
use key_wallet::managed_account::address_pool::{AddressPoolType, AddressState, KeySource};
use key_wallet::managed_account::managed_account_trait::ManagedAccountTrait;
use key_wallet::managed_account::transaction_record::{
    InputDetail, OutputDetail, OutputRole, TransactionDirection, TransactionRecord,
};
use key_wallet::managed_account::ManagedCoreFundsAccount;
use key_wallet::transaction_checking::TransactionType;
use key_wallet::wallet::initialization::WalletAccountCreationOptions;
use key_wallet::wallet::Wallet;
use key_wallet::{Account, AddressInfo, Network};
use platform_wallet::changeset::{
    AccountAddressPoolEntry, AccountRegistrationEntry, CoreChangeSet, PersistenceError,
    PlatformWalletChangeSet, PlatformWalletPersistence,
};
use platform_wallet::test_support::core_fixtures::{
    block, foreign_address, outside, pay, spend_of, stored_utxo,
};
use platform_wallet::wallet::platform_wallet::WalletId;
use platform_wallet_storage::sqlite::schema::{blob, core_pool, core_state};
use platform_wallet_storage::{
    LoadCtx, SqlitePersister, SqlitePersisterConfig, WalletStorageError,
};

const OWNER: [u8; 32] = [0x71; 32];
const FRIEND: [u8; 32] = [0x72; 32];

fn reopen(path: &std::path::Path) -> SqlitePersister {
    SqlitePersister::open(SqlitePersisterConfig::new(path)).expect("reopen persister")
}

fn external_account() -> AccountType {
    AccountType::DashpayExternalAccount {
        index: 0,
        user_identity_id: OWNER,
        friend_identity_id: FRIEND,
    }
}

/// A stand-in for the xpub a contact encrypts into their request.
fn contact_xpub(seed_byte: u8) -> ExtendedPubKey {
    Wallet::from_seed_bytes(
        [seed_byte; 64],
        Network::Testnet,
        WalletAccountCreationOptions::Default,
    )
    .expect("wallet")
    .get_bip44_account(0)
    .expect("BIP44 account 0")
    .account_xpub
}

/// The pool entries `register_external_contact_account` persists for `xpub`,
/// with the addresses at `used` marked used.
fn pool_entries(xpub: ExtendedPubKey, used: &[u32]) -> Vec<AccountAddressPoolEntry> {
    pool_entries_extended(xpub, used, 0)
}

/// [`pool_entries`] with `extra` more addresses, as a pool its gap limit
/// extended after payments snapshots it.
fn pool_entries_extended(
    xpub: ExtendedPubKey,
    used: &[u32],
    extra: u32,
) -> Vec<AccountAddressPoolEntry> {
    let account = Account {
        parent_wallet_id: None,
        account_type: external_account(),
        network: Network::Testnet,
        account_xpub: xpub,
        is_watch_only: true,
    };
    let mut managed = ManagedCoreFundsAccount::from_account(&account);
    if extra > 0 {
        for pool in managed.managed_account_type_mut().address_pools_mut() {
            pool.generate_addresses(extra, &KeySource::Public(xpub), true)
                .expect("extend the pool");
        }
    }
    let entries: Vec<AccountAddressPoolEntry> = managed
        .managed_account_type()
        .address_pools()
        .into_iter()
        .filter(|pool| !pool.addresses.is_empty())
        .map(|pool| AccountAddressPoolEntry {
            account_type: external_account(),
            pool_type: pool.pool_type,
            addresses: pool
                .addresses
                .values()
                .cloned()
                .map(|mut info| {
                    if used.contains(&info.index) {
                        info.state = AddressState::Used;
                    }
                    info
                })
                .collect(),
        })
        .collect();
    assert!(!entries.is_empty(), "the external account derives a pool");
    entries
}

/// The changeset `register_external_contact_account` stores.
fn registration(xpub: ExtendedPubKey, used: &[u32]) -> PlatformWalletChangeSet {
    PlatformWalletChangeSet {
        account_registrations: vec![AccountRegistrationEntry {
            account_type: external_account(),
            account_xpub: xpub,
        }],
        account_address_pools: pool_entries(xpub, used),
        ..Default::default()
    }
}

fn infos(entries: &[AccountAddressPoolEntry]) -> Vec<&AddressInfo> {
    entries.iter().flat_map(|e| e.addresses.iter()).collect()
}

fn scripts_of(xpub: ExtendedPubKey) -> BTreeSet<Vec<u8>> {
    infos(&pool_entries(xpub, &[]))
        .into_iter()
        .map(|info| info.script_pubkey.to_bytes())
        .collect()
}

fn address_at(xpub: ExtendedPubKey, index: u32) -> AddressInfo {
    infos(&pool_entries(xpub, &[]))
        .into_iter()
        .find(|info| info.index == index)
        .cloned()
        .expect("derived address at index")
}

fn live_scripts(persister: &SqlitePersister, w: &WalletId) -> BTreeSet<Vec<u8>> {
    let conn = persister.lock_conn_for_test();
    let mut stmt = conn
        .prepare("SELECT script FROM core_address_pool WHERE wallet_id = ?1")
        .unwrap();
    stmt.query_map(rusqlite::params![w.as_slice()], |r| r.get(0))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap()
}

/// Retired rows by script: (the xpub they came from, used).
fn retired(persister: &SqlitePersister, w: &WalletId) -> BTreeMap<Vec<u8>, (Vec<u8>, bool)> {
    persister
        .retired_pool_addresses(w, Network::Testnet)
        .expect("read retired pool")
        .into_iter()
        .map(|row| {
            let owner = &row.owner;
            assert_eq!(owner.account_type, "dashpay_external");
            assert_eq!(
                (owner.user_identity_id, owner.friend_identity_id),
                (OWNER, FRIEND)
            );
            (
                row.address.script_pubkey().to_bytes(),
                (row.account_xpub, row.used),
            )
        })
        .collect()
}

/// The xpub and pool scripts `load()` rebuilds the external account from.
fn loaded_account(
    persister: &SqlitePersister,
    w: &WalletId,
) -> (ExtendedPubKey, BTreeSet<Vec<u8>>) {
    let state = persister.load().expect("load");
    let start = state.wallets.get(w).expect("wallet rehydrated");
    let xpub = start
        .wallet
        .accounts
        .account_of_type(external_account())
        .expect("external account restored")
        .account_xpub;
    let managed = start
        .wallet_info
        .all_managed_accounts()
        .into_iter()
        .find(|m| m.managed_account_type().to_account_type() == external_account())
        .expect("managed external account restored");
    let scripts = managed
        .managed_account_type()
        .address_pools()
        .into_iter()
        .flat_map(|pool| pool.addresses.values().map(|i| i.script_pubkey.to_bytes()))
        .collect();
    (xpub, scripts)
}

fn used_scripts(persister: &SqlitePersister, w: &WalletId) -> BTreeSet<Vec<u8>> {
    let conn = persister.lock_conn_for_test();
    core_pool::load_used_addresses(&conn, w, Network::Testnet)
        .expect("used addresses")
        .into_iter()
        .map(|(address, _)| address.script_pubkey().to_bytes())
        .collect()
}

fn storage_error(err: PersistenceError) -> Box<WalletStorageError> {
    match err {
        PersistenceError::Backend { source, .. } => source
            .downcast::<WalletStorageError>()
            .unwrap_or_else(|source| panic!("expected WalletStorageError, got {source}")),
        other => panic!("expected Backend {{ .. }}, got {other:?}"),
    }
}

/// The contact rotates: the new xpub's pool replaces the old one, whose rows
/// (with their used flags) move to history keyed by the old xpub.
#[test]
fn new_xpub_rotation_retires_the_old_pool() {
    let (persister, _tmp, _path) = fresh_persister();
    let w = wid(0xE1);
    ensure_wallet_meta(&persister, &w);
    let (a, b) = (contact_xpub(0x0A), contact_xpub(0x0B));

    persister.store(w, registration(a, &[0, 2])).unwrap();
    persister.store(w, registration(b, &[])).unwrap();

    assert_eq!(live_scripts(&persister, &w), scripts_of(b));
    let history = retired(&persister, &w);
    assert_eq!(
        history.keys().cloned().collect::<BTreeSet<_>>(),
        scripts_of(a)
    );
    for info in infos(&pool_entries(a, &[])) {
        let (xpub, used) = &history[&info.script_pubkey.to_bytes()];
        assert_eq!(xpub, &a.encode().to_vec());
        assert_eq!(*used, matches!(info.index, 0 | 2), "index {}", info.index);
    }
    let (xpub, scripts) = loaded_account(&persister, &w);
    assert_eq!(xpub, b);
    assert_eq!(scripts, scripts_of(b));

    // A second rotation keys its history by its own xpub: no collision.
    let c = contact_xpub(0x0C);
    persister.store(w, registration(c, &[])).unwrap();
    assert_eq!(live_scripts(&persister, &w), scripts_of(c));
    let history = retired(&persister, &w);
    assert_eq!(
        history.keys().cloned().collect::<BTreeSet<_>>(),
        &scripts_of(a) | &scripts_of(b)
    );
}

/// The sweep rebuilding an account from the xpub it already has replaces
/// nothing and retires nothing.
#[test]
fn same_xpub_rebuild_retires_nothing() {
    let (persister, _tmp, _path) = fresh_persister();
    let w = wid(0xE2);
    ensure_wallet_meta(&persister, &w);
    let a = contact_xpub(0x0A);

    persister.store(w, registration(a, &[1])).unwrap();
    persister.store(w, registration(a, &[])).unwrap();

    assert_eq!(live_scripts(&persister, &w), scripts_of(a));
    assert!(retired(&persister, &w).is_empty());
    assert!(used_scripts(&persister, &w).contains(&address_at(a, 1).script_pubkey.to_bytes()));
    assert_eq!(loaded_account(&persister, &w).0, a);
}

/// After a relaunch the account comes back on the new xpub with only the new
/// pool, and an old address is neither in a pool nor used — it is only in the
/// history.
#[test]
fn rotation_survives_a_relaunch() {
    let (persister, _tmp, path) = fresh_persister();
    let w = wid(0xE3);
    ensure_wallet_meta(&persister, &w);
    let (a, b) = (contact_xpub(0x0A), contact_xpub(0x0B));

    persister.store(w, registration(a, &[1])).unwrap();
    persister.store(w, registration(b, &[])).unwrap();
    drop(persister);

    let persister = reopen(&path);
    let (xpub, scripts) = loaded_account(&persister, &w);
    assert_eq!(xpub, b);
    assert_eq!(scripts, scripts_of(b));
    let old = address_at(a, 1).script_pubkey.to_bytes();
    assert!(!scripts.contains(&old));
    assert!(!used_scripts(&persister, &w).contains(&old));
    assert_eq!(retired(&persister, &w)[&old], (a.encode().to_vec(), true));
}

/// A failure after the retire step rolls the whole flush back: the old
/// account, its pool and an empty history are what a relaunch sees, and the
/// retried registration then rotates cleanly.
#[test]
fn a_failure_mid_flush_rolls_the_rotation_back() {
    let (persister, _tmp, path) = fresh_persister();
    let w = wid(0xE4);
    ensure_wallet_meta(&persister, &w);
    let (a, b) = (contact_xpub(0x0A), contact_xpub(0x0B));
    persister.store(w, registration(a, &[0])).unwrap();

    // Fails the first new-pool insert: the retire copy, the live-row delete
    // and the account upsert have all run inside the flush by then.
    persister
        .lock_conn_for_test()
        .execute_batch(
            "CREATE TEMP TRIGGER fail_new_pool AFTER INSERT ON core_address_pool \
             BEGIN SELECT RAISE(ABORT, 'injected mid-flush failure'); END;",
        )
        .unwrap();
    persister
        .store(w, registration(b, &[]))
        .expect_err("the injected failure fails the flush");
    persister
        .lock_conn_for_test()
        .execute_batch("DROP TRIGGER fail_new_pool;")
        .unwrap();

    assert_eq!(live_scripts(&persister, &w), scripts_of(a));
    assert!(retired(&persister, &w).is_empty());
    drop(persister);

    let persister = reopen(&path);
    let (xpub, scripts) = loaded_account(&persister, &w);
    assert_eq!(xpub, a);
    assert_eq!(scripts, scripts_of(a));
    assert!(used_scripts(&persister, &w).contains(&address_at(a, 0).script_pubkey.to_bytes()));

    persister.store(w, registration(b, &[])).unwrap();
    assert_eq!(live_scripts(&persister, &w), scripts_of(b));
    assert_eq!(
        retired(&persister, &w)
            .keys()
            .cloned()
            .collect::<BTreeSet<_>>(),
        scripts_of(a)
    );
}

fn pools_only(entries: Vec<AccountAddressPoolEntry>) -> PlatformWalletChangeSet {
    PlatformWalletChangeSet {
        account_address_pools: entries,
        ..Default::default()
    }
}

/// A pool snapshot of the retired xpub that arrives after the rotation (taken
/// before it, flushed after) updates the history instead of failing the
/// flush. It is longer than the new pool (the old one's gap limit grew with
/// its payments), and none of its rows enters the live pool, so the new pool
/// still grows past them.
#[test]
fn a_late_snapshot_of_the_retired_xpub_lands_in_history() {
    let (persister, _tmp, _path) = fresh_persister();
    let w = wid(0xE5);
    ensure_wallet_meta(&persister, &w);
    let (a, b) = (contact_xpub(0x0A), contact_xpub(0x0B));
    persister.store(w, registration(a, &[])).unwrap();
    persister.store(w, registration(b, &[])).unwrap();

    let late = pool_entries_extended(a, &[3, 12], 5);
    persister.store(w, pools_only(late.clone())).unwrap();

    assert_eq!(live_scripts(&persister, &w), scripts_of(b));
    let history = retired(&persister, &w);
    assert_eq!(history.len(), infos(&late).len());
    for info in infos(&late) {
        let (xpub, used) = &history[&info.script_pubkey.to_bytes()];
        assert_eq!(xpub, &a.encode().to_vec());
        assert_eq!(*used, matches!(info.index, 3 | 12), "index {}", info.index);
    }

    // The new pool grows over the indices the late rows had.
    let grown = pool_entries_extended(b, &[11], 5);
    persister.store(w, pools_only(grown.clone())).unwrap();
    let grown_scripts: BTreeSet<_> = infos(&grown)
        .iter()
        .map(|i| i.script_pubkey.to_bytes())
        .collect();
    assert_eq!(live_scripts(&persister, &w), grown_scripts);
}

/// Manual mode merges buffered changesets, so one flush can carry a stale
/// pool snapshot ahead of the rotation that retires it, and two rotations of
/// one account. Every row lands by the xpub it derives from: the last
/// registration's live, each earlier one's in its history.
#[test]
fn rotations_merged_into_one_flush_land_by_derivation() {
    let (persister, _tmp, _path) = fresh_persister_with_mode(FlushMode::Manual);
    let w = wid(0xE8);
    ensure_wallet_meta(&persister, &w);
    let (a, b, c) = (contact_xpub(0x0A), contact_xpub(0x0B), contact_xpub(0x0C));
    persister.store(w, registration(a, &[])).unwrap();
    persister.flush(w).unwrap();

    persister
        .store(w, pools_only(pool_entries_extended(a, &[4], 3)))
        .unwrap();
    persister.store(w, registration(b, &[2])).unwrap();
    persister.store(w, registration(c, &[])).unwrap();
    persister.flush(w).unwrap();

    assert_eq!(live_scripts(&persister, &w), scripts_of(c));
    let history = retired(&persister, &w);
    let a_rows = infos(&pool_entries_extended(a, &[], 3))
        .iter()
        .map(|i| {
            (
                i.script_pubkey.to_bytes(),
                (a.encode().to_vec(), i.index == 4),
            )
        })
        .collect::<Vec<_>>();
    let b_rows = infos(&pool_entries(b, &[]))
        .iter()
        .map(|i| {
            (
                i.script_pubkey.to_bytes(),
                (b.encode().to_vec(), i.index == 2),
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(history, a_rows.into_iter().chain(b_rows).collect());
    assert_eq!(loaded_account(&persister, &w).0, c);
}

/// A contact rotating back to an xpub they had gets that generation's rows
/// back, used flags included, and the history keeps only the xpub replaced.
#[test]
fn rotating_back_restores_the_earlier_generation() {
    let (persister, _tmp, _path) = fresh_persister();
    let w = wid(0xE9);
    ensure_wallet_meta(&persister, &w);
    let (a, b) = (contact_xpub(0x0A), contact_xpub(0x0B));
    persister.store(w, registration(a, &[1])).unwrap();
    persister.store(w, registration(b, &[0])).unwrap();
    persister.store(w, registration(a, &[])).unwrap();

    assert_eq!(live_scripts(&persister, &w), scripts_of(a));
    let used = used_scripts(&persister, &w);
    assert!(used.contains(&address_at(a, 1).script_pubkey.to_bytes()));
    assert!(!used.contains(&address_at(b, 0).script_pubkey.to_bytes()));
    let history = retired(&persister, &w);
    assert_eq!(
        history.keys().cloned().collect::<BTreeSet<_>>(),
        scripts_of(b)
    );
    assert_eq!(
        history[&address_at(b, 0).script_pubkey.to_bytes()],
        (b.encode().to_vec(), true)
    );
    assert_eq!(loaded_account(&persister, &w).0, a);
}

/// A snapshot from an xpub the account never had is still a typed-key
/// conflict: only rows of a generation the account had are history.
#[test]
fn a_snapshot_of_an_unknown_xpub_still_conflicts() {
    let (persister, _tmp, _path) = fresh_persister();
    let w = wid(0xE6);
    ensure_wallet_meta(&persister, &w);
    let (a, c) = (contact_xpub(0x0A), contact_xpub(0x0C));
    persister.store(w, registration(a, &[])).unwrap();

    let err = persister
        .store(
            w,
            PlatformWalletChangeSet {
                account_address_pools: pool_entries(c, &[]),
                ..Default::default()
            },
        )
        .expect_err("a foreign typed key conflicts");
    assert!(
        matches!(
            *storage_error(err),
            WalletStorageError::TypedPoolKeyConflict { .. }
        ),
        "expected TypedPoolKeyConflict"
    );
    assert_eq!(live_scripts(&persister, &w), scripts_of(a));
    assert!(retired(&persister, &w).is_empty());
}

/// The tables of `conn`'s main schema, in creation order.
fn table_names(conn: &rusqlite::Connection) -> Vec<String> {
    conn.prepare(
        "SELECT name FROM main.sqlite_master \
         WHERE type = 'table' AND name NOT LIKE 'sqlite_%' \
           AND name <> 'refinery_schema_history' ORDER BY rowid",
    )
    .unwrap()
    .query_map([], |r| r.get(0))
    .unwrap()
    .collect::<Result<_, _>>()
    .unwrap()
}

/// The columns of `main.table` on `conn`, quoted for SQL.
fn quoted_columns(conn: &rusqlite::Connection, table: &str) -> String {
    conn.prepare("SELECT name FROM pragma_table_info(?1, 'main') ORDER BY cid")
        .unwrap()
        .query_map([table], |r| r.get::<_, String>(0))
        .unwrap()
        .map(|name| format!("\"{}\"", name.unwrap()))
        .collect::<Vec<_>>()
        .join(", ")
}

/// Replaces the file at `path` with a file of the schema `version` left on
/// disk, holding the rows the wallet wrote. The crate's migrations build a new
/// file up to `version`, so every later migration is missing from its schema
/// and its history whatever those migrations are. Each of its tables then
/// takes the rows of the same table in the written file, attached as
/// `written`, in the columns the table has at `version`, which holds while
/// later migrations only add tables and columns. `finish` runs last, in the
/// same transaction.
fn rebuild_at_version(
    path: &std::path::Path,
    version: i32,
    finish: impl FnOnce(&rusqlite::Connection),
) {
    let rebuilt = path.with_extension("rebuilt");
    let mut conn = rusqlite::Connection::open(&rebuilt).unwrap();
    platform_wallet_storage::sqlite::migrations::runner()
        .set_target(refinery::Target::Version(version))
        .run(&mut conn)
        .unwrap();
    conn.execute(
        "ATTACH DATABASE ?1 AS written",
        [path.to_str().expect("utf-8 path")],
    )
    .unwrap();
    conn.execute_batch("PRAGMA foreign_keys = OFF; BEGIN;")
        .unwrap();
    for table in table_names(&conn) {
        let columns = quoted_columns(&conn, &table);
        conn.execute_batch(&format!(
            "DELETE FROM main.\"{table}\";
             INSERT INTO main.\"{table}\" ({columns}) SELECT {columns} FROM written.\"{table}\";"
        ))
        .unwrap();
    }
    finish(&conn);
    conn.execute_batch("COMMIT; DETACH DATABASE written;")
        .unwrap();
    let max_version: i64 = conn
        .query_row(
            "SELECT MAX(version) FROM refinery_schema_history",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(max_version, i64::from(version));
    drop(conn);
    for suffix in ["-wal", "-shm"] {
        let mut sidecar = path.as_os_str().to_owned();
        sidecar.push(suffix);
        if let Err(e) = std::fs::remove_file(&sidecar) {
            assert_eq!(e.kind(), std::io::ErrorKind::NotFound, "remove {sidecar:?}");
        }
    }
    std::fs::rename(&rebuilt, path).unwrap();
}

/// A database written before V024 migrates in place: its pool is untouched,
/// the history starts empty, and the contact then rotates like on a fresh
/// file.
#[test]
fn a_pre_v024_database_migrates_in_place() {
    let (persister, _tmp, path) = fresh_persister();
    let w = wid(0xE7);
    ensure_wallet_meta(&persister, &w);
    let a = contact_xpub(0x0A);
    persister.store(w, registration(a, &[2])).unwrap();
    drop(persister);
    rebuild_at_version(&path, 23, |_| {});

    let persister = reopen(&path);
    assert_eq!(live_scripts(&persister, &w), scripts_of(a));
    assert!(retired(&persister, &w).is_empty());
    assert!(used_scripts(&persister, &w).contains(&address_at(a, 2).script_pubkey.to_bytes()));
    assert_eq!(loaded_account(&persister, &w).0, a);

    let b = contact_xpub(0x0B);
    persister.store(w, registration(b, &[])).unwrap();
    assert_eq!(live_scripts(&persister, &w), scripts_of(b));
    assert_eq!(
        retired(&persister, &w)
            .keys()
            .cloned()
            .collect::<BTreeSet<_>>(),
        scripts_of(a)
    );
}

/// The table and index exactly as a downstream build created them, before
/// V024 existed, without `IF NOT EXISTS`.
const DOWNSTREAM_RETIRED_POOL_DDL: &str = "\
CREATE TABLE core_address_pool_retired (
    wallet_id BLOB NOT NULL,
    account_type TEXT NOT NULL,
    account_index INTEGER NOT NULL,
    key_class INTEGER NOT NULL DEFAULT 0,
    user_identity_id BLOB NOT NULL DEFAULT (zeroblob(32)),
    friend_identity_id BLOB NOT NULL DEFAULT (zeroblob(32)),
    -- The BIP32 serialization of the xpub these rows were derived from.
    account_xpub BLOB NOT NULL,
    pool_type INTEGER NOT NULL CHECK (pool_type IN (0, 1, 2, 3)),
    address_index INTEGER NOT NULL,
    script BLOB NOT NULL,
    used INTEGER NOT NULL DEFAULT 0 CHECK (used IN (0, 1)),
    public_key BLOB NULL,
    key_type INTEGER NULL CHECK (key_type IS NULL OR key_type IN (0, 1, 2)),
    retired_at INTEGER NOT NULL DEFAULT (unixepoch()),
    PRIMARY KEY (wallet_id, account_type, account_index, key_class, user_identity_id,
                 friend_identity_id, account_xpub, pool_type, address_index),
    FOREIGN KEY (wallet_id) REFERENCES wallets(wallet_id) ON DELETE CASCADE
);

CREATE INDEX idx_core_address_pool_retired_script
    ON core_address_pool_retired(wallet_id, script);";

/// Creates the retired table in a file [`rebuild_at_version`] builds, through
/// [`DOWNSTREAM_RETIRED_POOL_DDL`], with the rows the wallet wrote to it.
fn create_retired_table_as_downstream(conn: &rusqlite::Connection) {
    conn.execute_batch(&format!(
        "{DOWNSTREAM_RETIRED_POOL_DDL}
         INSERT INTO main.core_address_pool_retired
             SELECT * FROM written.core_address_pool_retired;"
    ))
    .unwrap();
}

/// Everything SQLite knows about the retired table's shape: its stored DDL,
/// columns, indexes and foreign keys.
fn retired_table_shape(conn: &rusqlite::Connection) -> Vec<String> {
    let mut shape = Vec::new();
    for sql in [
        "SELECT type || ' ' || name || ' ' || tbl_name || ' ' || ifnull(sql, 'NULL') FROM sqlite_master \
         WHERE tbl_name = 'core_address_pool_retired' ORDER BY type, name",
        "SELECT cid || ' ' || name || ' ' || type || ' ' || \"notnull\" || ' ' || \
                ifnull(dflt_value, 'NULL') || ' ' || pk || ' ' || hidden \
         FROM pragma_table_xinfo('core_address_pool_retired')",
        "SELECT name || ' ' || \"unique\" || ' ' || origin || ' ' || partial \
         FROM pragma_index_list('core_address_pool_retired') ORDER BY name",
        "SELECT seqno || ' ' || cid || ' ' || ifnull(name, '') \
         FROM pragma_index_info('idx_core_address_pool_retired_script')",
        "SELECT id || ' ' || seq || ' ' || \"table\" || ' ' || \"from\" || ' ' || \
                \"to\" || ' ' || on_update || ' ' || on_delete || ' ' || \"match\" \
         FROM pragma_foreign_key_list('core_address_pool_retired')",
    ] {
        let mut stmt = conn.prepare(sql).unwrap();
        let rows = stmt
            .query_map([], |r| r.get::<_, String>(0))
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        assert!(!rows.is_empty(), "no shape rows for {sql}");
        shape.extend(rows);
    }
    shape
}

/// A file a downstream build wrote already holds `core_address_pool_retired`
/// and its index, with rows, but no V024 in its history. V024 adopts them in
/// place: the rows survive, the reader and the next rotation use them, and the
/// file ends up with exactly the shape of a fresh one.
#[test]
fn v024_adopts_a_downstream_retired_pool_table() {
    let (persister, _tmp, path) = fresh_persister();
    let w = wid(0xEB);
    ensure_wallet_meta(&persister, &w);
    let (a, b) = (contact_xpub(0x0A), contact_xpub(0x0B));
    persister.store(w, registration(a, &[1])).unwrap();
    persister.store(w, registration(b, &[])).unwrap();
    let before = retired(&persister, &w);
    assert_eq!(
        before.keys().cloned().collect::<BTreeSet<_>>(),
        scripts_of(a)
    );
    let fresh_shape = retired_table_shape(&persister.lock_conn_for_test());
    drop(persister);

    // A V023 file with the table the downstream DDL creates, carrying its
    // rows.
    rebuild_at_version(&path, 23, create_retired_table_as_downstream);
    {
        let conn = rusqlite::Connection::open(&path).unwrap();
        let version: i64 = conn
            .query_row(
                "SELECT MAX(version) FROM refinery_schema_history",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(version, 23);
    }

    let persister = reopen(&path);
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
        assert_eq!(retired_table_shape(&conn), fresh_shape);
    }
    assert_eq!(retired(&persister, &w), before);
    assert_eq!(live_scripts(&persister, &w), scripts_of(b));
    assert_eq!(loaded_account(&persister, &w).0, b);

    // A rotation back restores the adopted generation, used flag included.
    persister.store(w, registration(a, &[])).unwrap();
    assert_eq!(live_scripts(&persister, &w), scripts_of(a));
    assert!(used_scripts(&persister, &w).contains(&address_at(a, 1).script_pubkey.to_bytes()));
    assert_eq!(
        retired(&persister, &w)
            .keys()
            .cloned()
            .collect::<BTreeSet<_>>(),
        scripts_of(b)
    );
}

/// The history never blocks a live write: a retired generation whose xpub no
/// longer decodes is skipped, and the contact's own pool still stores.
#[test]
fn a_corrupt_retired_xpub_does_not_block_live_writes() {
    let (persister, _tmp, _path) = fresh_persister();
    let w = wid(0xEA);
    ensure_wallet_meta(&persister, &w);
    let (a, b) = (contact_xpub(0x0A), contact_xpub(0x0B));
    persister.store(w, registration(a, &[])).unwrap();
    persister.store(w, registration(b, &[])).unwrap();
    persister
        .lock_conn_for_test()
        .execute(
            "UPDATE core_address_pool_retired SET account_xpub = X'00' WHERE wallet_id = ?1",
            rusqlite::params![w.as_slice()],
        )
        .unwrap();

    persister
        .store(w, pools_only(pool_entries_extended(b, &[1], 2)))
        .unwrap();
    assert!(used_scripts(&persister, &w).contains(&address_at(b, 1).script_pubkey.to_bytes()));
}

/// Re-registering the same xpub decoded under another network (decoding maps
/// every non-mainnet network to Testnet) is not a rotation: the live pool is
/// left alone.
#[test]
fn the_same_xpub_under_another_network_is_not_a_rotation() {
    let (persister, _tmp, _path) = fresh_persister();
    let w = wid(0xEC);
    ensure_wallet_meta(&persister, &w);
    let a = contact_xpub(0x0A);
    persister.store(w, registration(a, &[1])).unwrap();
    persister
        .lock_conn_for_test()
        .execute_batch(
            "CREATE TEMP TRIGGER no_pool_delete BEFORE DELETE ON core_address_pool \
             BEGIN SELECT RAISE(ABORT, 'the live pool was retired'); END;",
        )
        .unwrap();

    let regtest = ExtendedPubKey {
        network: Network::Regtest,
        ..a
    };
    persister
        .store(
            w,
            PlatformWalletChangeSet {
                account_registrations: vec![AccountRegistrationEntry {
                    account_type: external_account(),
                    account_xpub: regtest,
                }],
                ..Default::default()
            },
        )
        .unwrap();
    assert_eq!(live_scripts(&persister, &w), scripts_of(a));
    assert!(retired(&persister, &w).is_empty());
    assert!(used_scripts(&persister, &w).contains(&address_at(a, 1).script_pubkey.to_bytes()));
}

/// A coin an older build persisted at a contact's address stays with the
/// contact account, outside the wallet's balance, after the contact rotates
/// away from that address: the retired generation still names its owner.
#[test]
fn a_stale_contact_coin_stays_out_of_the_balance_after_a_rotation() {
    let (persister, _tmp, path) = fresh_persister();
    let w = wid(0xED);
    ensure_wallet_meta(&persister, &w);
    register_funds_account(&persister, &w);
    let (a, b) = (contact_xpub(0x0A), contact_xpub(0x0B));
    persister.store(w, registration(a, &[])).unwrap();
    let stale = address_at(a, 0).script_pubkey.to_bytes();
    let outpoint = blob::encode_outpoint(&OutPoint {
        txid: <Txid as dashcore::hashes::Hash>::from_byte_array([0x5C; 32]),
        vout: 0,
    })
    .unwrap();
    persister
        .lock_conn_for_test()
        .execute(
            "INSERT INTO core_utxos (wallet_id, outpoint, value, script, spent) \
             VALUES (?1, ?2, 70000, ?3, 0)",
            rusqlite::params![w.as_slice(), outpoint, stale],
        )
        .unwrap();
    let total = |persister: &SqlitePersister| {
        let state = persister.load().expect("load");
        state
            .wallets
            .get(&w)
            .expect("wallet")
            .wallet_info
            .balance
            .total()
    };
    assert_eq!(
        total(&persister),
        0,
        "a contact's coin is outside the balance"
    );

    persister.store(w, registration(b, &[])).unwrap();
    drop(persister);
    let persister = reopen(&path);
    assert_eq!(loaded_account(&persister, &w).0, b);
    assert_eq!(
        total(&persister),
        0,
        "still outside the balance after the rotation"
    );
}

fn funds_account() -> AccountType {
    AccountType::Standard {
        index: 0,
        standard_account_type: StandardAccountType::BIP44Account,
    }
}

/// Registers the wallet's first funds account, the one a coin no pool row
/// names falls back to on load.
fn register_funds_account(persister: &SqlitePersister, w: &WalletId) {
    let funds = AccountRegistrationEntry {
        account_type: funds_account(),
        account_xpub: contact_xpub(0x01),
    };
    persister
        .store(
            *w,
            PlatformWalletChangeSet {
                account_registrations: vec![funds],
                ..Default::default()
            },
        )
        .unwrap();
}

/// The funds account's address at `index` of its `pool_type` chain.
fn funds_address(pool_type: AddressPoolType, index: u32) -> Address {
    let account = Account {
        parent_wallet_id: None,
        account_type: funds_account(),
        network: Network::Testnet,
        account_xpub: contact_xpub(0x01),
        is_watch_only: true,
    };
    ManagedCoreFundsAccount::from_account(&account)
        .managed_account_type()
        .address_pools()
        .into_iter()
        .filter(|pool| pool.pool_type == pool_type)
        .flat_map(|pool| pool.addresses.values().cloned())
        .find(|info| info.index == index)
        .expect("derived funds address")
        .address
}

/// After a load: the wallet's balance, and whether a funds account and a
/// contact account hold `outpoint`.
fn loaded_coin(
    persister: &SqlitePersister,
    w: &WalletId,
    outpoint: &OutPoint,
) -> (u64, bool, bool) {
    let state = persister.load().expect("load");
    let info = &state.wallets.get(w).expect("wallet rehydrated").wallet_info;
    let in_funds = info
        .accounts
        .all_funding_accounts()
        .iter()
        .any(|account| account.utxos.contains_key(outpoint));
    let in_contact = info
        .accounts
        .dashpay_external_accounts
        .values()
        .any(|account| account.utxos.contains_key(outpoint));
    (info.balance.total(), in_funds, in_contact)
}

fn core_round(core: CoreChangeSet) -> PlatformWalletChangeSet {
    PlatformWalletChangeSet {
        core: Some(core),
        ..Default::default()
    }
}

/// A mined record of `tx` as `account_type` hands it to the store.
fn record(
    account_type: AccountType,
    tx: &Transaction,
    direction: TransactionDirection,
    input_details: Vec<InputDetail>,
    output_details: Vec<OutputDetail>,
    net_amount: i64,
) -> TransactionRecord {
    TransactionRecord::new(
        tx.clone(),
        account_type,
        block(100),
        TransactionType::Standard,
        direction,
        input_details,
        output_details,
        net_amount,
    )
}

fn output(tx: &Transaction, index: u32, role: OutputRole, address: &Address) -> OutputDetail {
    OutputDetail {
        index,
        role,
        address: Some(address.clone()),
        value: tx.output[index as usize].value,
    }
}

fn stored_record(persister: &SqlitePersister, w: &WalletId, txid: &Txid) -> TransactionRecord {
    core_state::get_tx_record(&persister.lock_conn_for_test(), w, txid, &LoadCtx::strict())
        .expect("read record")
        .expect("record stored")
}

fn roles(record: &TransactionRecord) -> Vec<(u32, OutputRole)> {
    record
        .output_details
        .iter()
        .map(|detail| (detail.index, detail.role))
        .collect()
}

/// A script only the contact's retired generation holds.
fn assert_retired_only(persister: &SqlitePersister, w: &WalletId, address: &Address) {
    let script = address.script_pubkey().to_bytes();
    assert!(retired(persister, w).contains_key(&script));
    assert!(!live_scripts(persister, w).contains(&script));
}

/// A coin the contact received at an address of the generation it rotated
/// away from, already spent, with the records of its receipt and of the
/// contact's transaction spending it, the latter attributing its input to
/// that address. None of it is the wallet's: after a reload the coin is in no
/// account and outside the balance, the receipt credits the wallet nothing,
/// and the spend debits it nothing.
#[test]
fn a_spent_coin_at_a_retired_contact_address_and_its_spender_are_not_the_wallets() {
    let (persister, _tmp, path) = fresh_persister();
    let w = wid(0xEE);
    ensure_wallet_meta(&persister, &w);
    register_funds_account(&persister, &w);
    let (a, b) = (contact_xpub(0x0A), contact_xpub(0x0B));
    persister.store(w, registration(a, &[0])).unwrap();
    persister.store(w, registration(b, &[])).unwrap();

    let old = address_at(a, 0).address;
    let receipt_tx = spend_of(&[outside(0xC0)], vec![pay(70_000, &old)]);
    let coin = stored_utxo(&receipt_tx, 0, old.clone());
    let receipt = record(
        external_account(),
        &receipt_tx,
        TransactionDirection::Incoming,
        vec![],
        vec![output(&receipt_tx, 0, OutputRole::Received, &old)],
        70_000,
    );
    let payee = foreign_address(0xF1);
    let spend = spend_of(&[coin.outpoint], vec![pay(69_000, &payee)]);
    let spender = record(
        external_account(),
        &spend,
        TransactionDirection::Outgoing,
        vec![InputDetail {
            index: 0,
            value: 70_000,
            address: old.clone(),
        }],
        vec![output(&spend, 0, OutputRole::Sent, &payee)],
        -70_000,
    );
    persister
        .store(
            w,
            core_round(CoreChangeSet {
                records: vec![receipt.clone()],
                new_utxos: vec![coin.clone()],
                ..Default::default()
            }),
        )
        .unwrap();
    persister
        .store(
            w,
            core_round(CoreChangeSet {
                records: vec![spender.clone()],
                spent_utxos: vec![coin.clone()],
                ..Default::default()
            }),
        )
        .unwrap();
    assert_retired_only(&persister, &w, &old);
    let spent: bool = persister
        .lock_conn_for_test()
        .query_row(
            "SELECT spent FROM core_utxos WHERE wallet_id = ?1 AND outpoint = ?2",
            rusqlite::params![w.as_slice(), blob::encode_outpoint(&coin.outpoint).unwrap()],
            |r| r.get(0),
        )
        .unwrap();
    assert!(spent);
    drop(persister);

    let persister = reopen(&path);
    assert_eq!(
        loaded_coin(&persister, &w, &coin.outpoint),
        (0, false, false)
    );
    let stored = stored_record(&persister, &w, &receipt.txid);
    assert_eq!(stored.net_amount, 0);
    assert_eq!(stored.direction, TransactionDirection::Incoming);
    assert_eq!(roles(&stored), vec![(0, OutputRole::Sent)]);
    let stored = stored_record(&persister, &w, &spender.txid);
    assert_eq!(stored.net_amount, 0);
    assert!(
        stored.input_details.is_empty(),
        "no debit from the contact's address"
    );
    assert_eq!(stored.direction, TransactionDirection::Incoming);
    assert_eq!(roles(&stored), vec![(0, OutputRole::Sent)]);
}

const FUNDING: u64 = 100_000;
const TO_CURRENT: u64 = 30_000;
const TO_OLD: u64 = 20_000;
const CHANGE: u64 = 49_000;
const FEE: u64 = FUNDING - TO_CURRENT - TO_OLD - CHANGE;

/// One transaction from the wallet pays the contact's current address, an
/// address of the generation the contact rotated away from, and change back
/// to the wallet. The record arrives with both contact outputs as the
/// wallet's own and their coins stored next to the change, as a build from
/// before contact slices were left out of the record stored them. Returns
/// that record.
fn store_mixed_payment(persister: &SqlitePersister, w: &WalletId) -> TransactionRecord {
    register_funds_account(persister, w);
    let (a, b) = (contact_xpub(0x0A), contact_xpub(0x0B));
    persister.store(*w, registration(a, &[1])).unwrap();
    persister.store(*w, registration(b, &[])).unwrap();
    let from = funds_address(AddressPoolType::External, 0);
    let change = funds_address(AddressPoolType::Internal, 0);
    let funding = stored_utxo(
        &spend_of(&[outside(0xF0)], vec![pay(FUNDING, &from)]),
        0,
        from.clone(),
    );
    persister
        .store(
            *w,
            core_round(CoreChangeSet {
                new_utxos: vec![funding.clone()],
                ..Default::default()
            }),
        )
        .unwrap();

    let (current, old) = (address_at(b, 0).address, address_at(a, 1).address);
    let tx = spend_of(
        &[funding.outpoint],
        vec![
            pay(TO_CURRENT, &current),
            pay(TO_OLD, &old),
            pay(CHANGE, &change),
        ],
    );
    let incoming = record(
        funds_account(),
        &tx,
        TransactionDirection::Internal,
        vec![InputDetail {
            index: 0,
            value: FUNDING,
            address: from,
        }],
        vec![
            output(&tx, 0, OutputRole::Received, &current),
            output(&tx, 1, OutputRole::Received, &old),
            output(&tx, 2, OutputRole::Change, &change),
        ],
        -(FEE as i64),
    );
    persister
        .store(
            *w,
            core_round(CoreChangeSet {
                records: vec![incoming.clone()],
                spent_utxos: vec![funding],
                new_utxos: vec![
                    stored_utxo(&tx, 0, current.clone()),
                    stored_utxo(&tx, 1, old.clone()),
                    stored_utxo(&tx, 2, change),
                ],
                ..Default::default()
            }),
        )
        .unwrap();
    assert!(live_scripts(persister, w).contains(&current.script_pubkey().to_bytes()));
    assert_retired_only(persister, w, &old);
    incoming
}

/// Only the change is the wallet's: both contact payments and the fee are
/// what it spent.
fn assert_counts_only_the_change(record: &TransactionRecord) {
    assert_eq!(record.net_amount, -((TO_CURRENT + TO_OLD + FEE) as i64));
    assert_eq!(
        roles(record),
        vec![
            (0, OutputRole::Sent),
            (1, OutputRole::Sent),
            (2, OutputRole::Change)
        ]
    );
    assert_eq!(record.input_details.len(), 1);
    assert_eq!(record.input_details[0].value, FUNDING);
    assert_eq!(record.direction, TransactionDirection::Outgoing);
}

/// After a load only the change is in the balance, in a funds account; the
/// current address's coin is in the contact account and the old address's
/// coin in none.
fn assert_only_the_change_loads(persister: &SqlitePersister, w: &WalletId, txid: Txid) {
    let at = |vout| loaded_coin(persister, w, &OutPoint::new(txid, vout));
    assert_eq!(at(0), (CHANGE, false, true));
    assert_eq!(at(1), (CHANGE, false, false));
    assert_eq!(at(2), (CHANGE, true, false));
}

/// A payment to a contact's current and old address with change counts only
/// the change as the wallet's, right after it is stored and after a reload,
/// and only the change loads into the balance.
#[test]
fn a_payment_to_a_contacts_current_and_old_address_counts_only_the_change() {
    let (persister, _tmp, path) = fresh_persister();
    let w = wid(0xF0);
    ensure_wallet_meta(&persister, &w);
    let incoming = store_mixed_payment(&persister, &w);
    assert_counts_only_the_change(&stored_record(&persister, &w, &incoming.txid));
    drop(persister);

    let persister = reopen(&path);
    assert_counts_only_the_change(&stored_record(&persister, &w, &incoming.txid));
    assert_only_the_change_loads(&persister, &w, incoming.txid);
}

/// Rebuilds the file at `path` as a V020 file that already holds the retired
/// table, the order of a file a downstream build wrote: that build created the
/// table before V021 existed. The retired table comes through the downstream
/// DDL with its rows, and the record of `txid` is put back as the wallet
/// handed it to the store, before any repair.
fn rebuild_at_v020_with_the_retired_table(
    path: &std::path::Path,
    w: &WalletId,
    as_handed: &TransactionRecord,
) {
    rebuild_at_version(path, 20, |conn| {
        create_retired_table_as_downstream(conn);
        conn.execute(
            "UPDATE core_transactions SET record_blob = ?1 WHERE wallet_id = ?2 AND txid = ?3",
            rusqlite::params![
                blob::encode(as_handed).unwrap(),
                w.as_slice(),
                AsRef::<[u8]>::as_ref(&as_handed.txid)
            ],
        )
        .unwrap();
    });
}

/// V021 replays history through the same accounting. On a file that already
/// holds the retired table when V021 runs, it repairs the payment to the
/// contact's current and old address to count only the change.
#[test]
fn v021_counts_only_the_change_on_a_file_that_already_holds_the_retired_table() {
    let (persister, _tmp, path) = fresh_persister();
    let w = wid(0xF1);
    ensure_wallet_meta(&persister, &w);
    let incoming = store_mixed_payment(&persister, &w);
    let history = retired(&persister, &w);
    drop(persister);
    rebuild_at_v020_with_the_retired_table(&path, &w, &incoming);

    let persister = reopen(&path);
    assert_eq!(retired(&persister, &w), history);
    let stored = stored_record(&persister, &w, &incoming.txid);
    assert_counts_only_the_change(&stored);
    let original: Vec<u8> = persister
        .lock_conn_for_test()
        .query_row(
            "SELECT record_blob FROM core_transaction_record_originals \
             WHERE wallet_id = ?1 AND txid = ?2",
            rusqlite::params![w.as_slice(), AsRef::<[u8]>::as_ref(&incoming.txid)],
            |r| r.get(0),
        )
        .expect("V021 repaired the record and kept the one it replaced");
    assert_eq!(original, blob::encode(&incoming).unwrap());
    assert_only_the_change_loads(&persister, &w, incoming.txid);
}

/// The contact rotates A → B → A. A payment to an A address that arrived while
/// A was retired is a payment to the contact: while A is retired its coin
/// loads into no account, after the rotation back it reloads into the contact
/// account, outside the balance either way, and the transaction's
/// `net_amount` is 0.
#[test]
fn a_coin_at_a_restored_generation_reloads_into_the_contact_account() {
    let (persister, _tmp, path) = fresh_persister();
    let w = wid(0xF2);
    ensure_wallet_meta(&persister, &w);
    register_funds_account(&persister, &w);
    let (a, b) = (contact_xpub(0x0A), contact_xpub(0x0B));
    persister.store(w, registration(a, &[])).unwrap();
    persister.store(w, registration(b, &[])).unwrap();

    let restored = address_at(a, 2).address;
    let tx = spend_of(&[outside(0xC1)], vec![pay(40_000, &restored)]);
    let coin = stored_utxo(&tx, 0, restored.clone());
    let payment = record(
        external_account(),
        &tx,
        TransactionDirection::Incoming,
        vec![],
        vec![output(&tx, 0, OutputRole::Received, &restored)],
        40_000,
    );
    assert_retired_only(&persister, &w, &restored);
    persister
        .store(
            w,
            core_round(CoreChangeSet {
                records: vec![payment.clone()],
                new_utxos: vec![coin.clone()],
                ..Default::default()
            }),
        )
        .unwrap();
    assert_eq!(
        loaded_coin(&persister, &w, &coin.outpoint),
        (0, false, false)
    );
    persister.store(w, registration(a, &[])).unwrap();
    assert_eq!(live_scripts(&persister, &w), scripts_of(a));
    drop(persister);

    let persister = reopen(&path);
    assert_eq!(loaded_account(&persister, &w).0, a);
    assert_eq!(
        loaded_coin(&persister, &w, &coin.outpoint),
        (0, false, true)
    );
    let stored = stored_record(&persister, &w, &payment.txid);
    assert_eq!(stored.net_amount, 0);
    assert_eq!(stored.direction, TransactionDirection::Incoming);
    assert_eq!(roles(&stored), vec![(0, OutputRole::Sent)]);
}
