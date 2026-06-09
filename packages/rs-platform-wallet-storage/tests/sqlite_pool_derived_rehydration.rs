#![allow(clippy::field_reassign_with_default)]

//! Genesis-rescan rehydration of `core_derived_addresses` from
//! `account_address_pools` snapshots, and the flush blast-radius
//! containment for an unspent UTXO at a genuinely-undeclared address.
//!
//! On a `birth_height = 0` rescan SPV can match a UTXO at a registered
//! pool address before the live `addresses_derived` event for it lands.
//! `account_address_pools` already holds that address (with its real
//! `used` flag), so `apply_pools` mirrors it into `core_derived_addresses`
//! in the same transaction — the UTXO writer's account lookup resolves
//! and the flush commits instead of dropping the whole changeset.

mod common;

use common::{ensure_wallet_meta, fresh_persister, wid};

use key_wallet::wallet::initialization::WalletAccountCreationOptions;
use key_wallet::wallet::managed_wallet_info::ManagedWalletInfo;
use key_wallet::wallet::Wallet;
use key_wallet::AddressInfo;
use platform_wallet::changeset::{
    AccountAddressPoolEntry, CoreChangeSet, PlatformWalletChangeSet, PlatformWalletPersistence,
};
use platform_wallet::wallet::platform_wallet::WalletId;
use platform_wallet_storage::sqlite::schema::core_state;
use platform_wallet_storage::{SqlitePersister, SqlitePersisterConfig};

/// Snapshot a freshly seeded wallet's Standard BIP44 external pool as a
/// single `AccountAddressPoolEntry`, mirroring the production registration
/// round in `wallet_lifecycle.rs`. One pool keeps the derived-row keys
/// `(account_type, address)` unique across the returned set, so the test
/// reads back exactly what it wrote — the cross-account label collapse
/// the schema PK allows is exercised elsewhere, not load-bearing here.
fn standard_external_pool(info: &ManagedWalletInfo) -> AccountAddressPoolEntry {
    use key_wallet::account::AccountType;
    use key_wallet::managed_account::address_pool::AddressPoolType;
    for managed in info.all_managed_accounts() {
        let account_type = managed.managed_account_type().to_account_type();
        if !matches!(account_type, AccountType::Standard { index: 0, .. }) {
            continue;
        }
        for pool in managed.managed_account_type().address_pools() {
            if pool.pool_type != AddressPoolType::External {
                continue;
            }
            let infos: Vec<AddressInfo> = pool.addresses.values().cloned().collect();
            if infos.is_empty() {
                continue;
            }
            return AccountAddressPoolEntry {
                account_type,
                pool_type: pool.pool_type,
                addresses: infos,
            };
        }
    }
    panic!("wallet must expose a non-empty Standard BIP44 external pool");
}

/// A wallet's Standard BIP44 external pool plus its first `AddressInfo` —
/// the load-bearing target the UTXO writer must resolve.
fn wallet_with_pools(seed_byte: u8) -> (Vec<AccountAddressPoolEntry>, AddressInfo) {
    let seed = [seed_byte; 64];
    let wallet = Wallet::from_seed_bytes(
        seed,
        key_wallet::Network::Testnet,
        WalletAccountCreationOptions::Default,
    )
    .unwrap();
    let info = ManagedWalletInfo::from_wallet(&wallet, 0);
    let pool = standard_external_pool(&info);
    let target = pool
        .addresses
        .first()
        .cloned()
        .expect("non-empty external pool");
    (vec![pool], target)
}

fn utxo_at(addr: &dashcore::Address, vout: u32, value: u64) -> key_wallet::Utxo {
    use dashcore::hashes::Hash;
    key_wallet::Utxo {
        outpoint: dashcore::OutPoint {
            txid: dashcore::Txid::from_byte_array([0x42; 32]),
            vout,
        },
        txout: dashcore::TxOut {
            value,
            script_pubkey: addr.script_pubkey(),
        },
        address: addr.clone(),
        height: 1,
        is_coinbase: false,
        is_confirmed: true,
        is_instantlocked: false,
        is_locked: false,
        is_trusted: false,
    }
}

fn reopen(path: &std::path::Path) -> SqlitePersister {
    SqlitePersister::open(SqlitePersisterConfig::new(path)).expect("reopen")
}

/// Genesis-rescan persist: a wallet registered with pools but with NO
/// live `addresses_derived` event still resolves the account index of a
/// UTXO landing on a pool address — `apply_pools` mirrored the pool into
/// `core_derived_addresses` in the same round.
#[test]
fn genesis_rescan_utxo_at_pool_address_persists() {
    let (persister, _tmp, _path) = fresh_persister();
    let w: WalletId = wid(0xA0);
    ensure_wallet_meta(&persister, &w);

    let (snapshots, target) = wallet_with_pools(0x11);
    let addr = target.address.clone();

    // Registration round carries pools only — no addresses_derived.
    persister
        .store(
            w,
            PlatformWalletChangeSet {
                account_address_pools: snapshots,
                ..Default::default()
            },
        )
        .unwrap();

    // SPV matches a UTXO at the pool address before any derive-event.
    persister
        .store(
            w,
            PlatformWalletChangeSet {
                core: Some(CoreChangeSet {
                    new_utxos: vec![utxo_at(&addr, 0, 555_000)],
                    ..Default::default()
                }),
                ..Default::default()
            },
        )
        .expect("UTXO at a pool address must persist without a derive-event");

    let conn = persister.lock_conn_for_test();
    let by_account = core_state::list_unspent_utxos(&conn, &w).unwrap();
    let total: usize = by_account.values().map(|v| v.len()).sum();
    assert_eq!(total, 1, "the pool-address UTXO must be persisted");
    let derived = core_state::list_derived_addresses_for_test(&conn, &w).unwrap();
    assert!(
        derived.iter().any(|r| r.address == addr.to_string()),
        "apply_pools must have mirrored the pool address into core_derived_addresses"
    );
}

/// Row-shape parity: a `core_derived_addresses` row written via
/// `apply_pools` is byte-identical (account_index, derivation_path, used)
/// to the row the live `core_state::apply` writes for the same address —
/// the two sources share one helper, so they cannot drift.
#[test]
fn pool_and_live_derived_rows_are_identical() {
    let (snapshots, target) = wallet_with_pools(0x22);
    let addr = target.address.clone();

    // Locate the account_type + pool_type that owns the target address so
    // the live event describes the same derivation.
    let owning = snapshots
        .iter()
        .find(|p| p.addresses.iter().any(|ai| ai.address == addr))
        .expect("owning pool");

    // Row A — written by apply_pools (with the real `used` from the pool).
    let row_pool = {
        let (persister, _tmp, _path) = fresh_persister();
        let w: WalletId = wid(0xB1);
        ensure_wallet_meta(&persister, &w);
        persister
            .store(
                w,
                PlatformWalletChangeSet {
                    account_address_pools: snapshots.clone(),
                    ..Default::default()
                },
            )
            .unwrap();
        let conn = persister.lock_conn_for_test();
        core_state::list_derived_addresses_for_test(&conn, &w)
            .unwrap()
            .into_iter()
            .find(|r| r.address == addr.to_string())
            .expect("pool-written row")
    };

    // Row B — written by the live core_state::apply derive path.
    let row_live = {
        let (persister, _tmp, _path) = fresh_persister();
        let w: WalletId = wid(0xB2);
        ensure_wallet_meta(&persister, &w);
        let derived = platform_wallet::DerivedAddress {
            account_type: owning.account_type,
            pool_type: owning.pool_type,
            derivation_index: target.index,
            address: addr.clone(),
            public_key: dashcore::PublicKey::from_slice(&[
                0x02, 0x79, 0xbe, 0x66, 0x7e, 0xf9, 0xdc, 0xbb, 0xac, 0x55, 0xa0, 0x62, 0x95, 0xce,
                0x87, 0x0b, 0x07, 0x02, 0x9b, 0xfc, 0xdb, 0x2d, 0xce, 0x28, 0xd9, 0x59, 0xf2, 0x81,
                0x5b, 0x16, 0xf8, 0x17, 0x98,
            ])
            .unwrap(),
        };
        persister
            .store(
                w,
                PlatformWalletChangeSet {
                    core: Some(CoreChangeSet {
                        addresses_derived: vec![derived],
                        ..Default::default()
                    }),
                    ..Default::default()
                },
            )
            .unwrap();
        let conn = persister.lock_conn_for_test();
        core_state::list_derived_addresses_for_test(&conn, &w)
            .unwrap()
            .into_iter()
            .find(|r| r.address == addr.to_string())
            .expect("live-written row")
    };

    assert_eq!(
        row_pool.account_type, row_live.account_type,
        "account_type label must match"
    );
    assert_eq!(
        row_pool.account_index, row_live.account_index,
        "account_index must match"
    );
    assert_eq!(
        row_pool.derivation_path, row_live.derivation_path,
        "derivation_path must be rendered identically by both sources"
    );
    // The live path hardcodes used=false; an unused pool address agrees.
    assert!(
        !target.used,
        "fixture relies on a fresh (unused) first external address"
    );
    assert_eq!(row_pool.used, row_live.used, "used flag must match");
}

/// Load-path rehydrate: a DB with pool snapshots but ZERO derived rows is
/// repopulated by `load`, and a second `load` is a no-op (no duplicates).
#[test]
fn load_rehydrates_derived_rows_from_pools_idempotently() {
    let (persister, _tmp, path) = fresh_persister();
    let w: WalletId = wid(0xC0);
    ensure_wallet_meta(&persister, &w);

    let (snapshots, target) = wallet_with_pools(0x33);
    let addr = target.address.clone();

    persister
        .store(
            w,
            PlatformWalletChangeSet {
                account_address_pools: snapshots,
                ..Default::default()
            },
        )
        .unwrap();

    // Simulate an already-persisted prod DB: pools present, derived table
    // empty (the bug — derived rows were never written for pools).
    {
        let conn = persister.lock_conn_for_test();
        conn.execute(
            "DELETE FROM core_derived_addresses WHERE wallet_id = ?1",
            rusqlite::params![w.as_slice()],
        )
        .unwrap();
        let n = core_state::list_derived_addresses_for_test(&conn, &w)
            .unwrap()
            .len();
        assert_eq!(n, 0, "precondition: derived table emptied");
    }
    drop(persister);

    let p2 = reopen(&path);
    PlatformWalletPersistence::load(&p2).expect("first load");
    let first = {
        let conn = p2.lock_conn_for_test();
        core_state::list_derived_addresses_for_test(&conn, &w).unwrap()
    };
    assert!(
        first.iter().any(|r| r.address == addr.to_string()),
        "load must rehydrate derived rows from pools"
    );
    let count_after_first = first.len();

    // A second load must not duplicate or re-insert (table already full).
    PlatformWalletPersistence::load(&p2).expect("second load");
    let count_after_second = {
        let conn = p2.lock_conn_for_test();
        core_state::list_derived_addresses_for_test(&conn, &w)
            .unwrap()
            .len()
    };
    assert_eq!(
        count_after_first, count_after_second,
        "second load must be a no-op (no duplicate derived rows)"
    );
}

/// Blast-radius isolation: a batch mixing a valid pool-address UTXO, a
/// sync-height bump, and ONE unspent UTXO at a genuinely undeclared
/// address (not in pools, not derived) commits the valid UTXO + height
/// and SKIPS only the bad UTXO — no error escapes, so the buffer drains
/// instead of looping.
#[test]
fn undeclared_unspent_utxo_is_skipped_not_fatal() {
    use dashcore::hashes::Hash;
    let (persister, _tmp, _path) = fresh_persister();
    let w: WalletId = wid(0xD0);
    ensure_wallet_meta(&persister, &w);

    let (snapshots, good) = wallet_with_pools(0x44);
    let good_addr = good.address.clone();

    // A genuinely undeclared address: not in any pool, never derived.
    let undeclared = {
        use dashcore::address::Payload;
        use dashcore::PubkeyHash;
        dashcore::Address::new(
            dashcore::Network::Testnet,
            Payload::PubkeyHash(PubkeyHash::from_byte_array([0xEE; 20])),
        )
    };
    assert_ne!(undeclared, good_addr, "fixture sanity");

    persister
        .store(
            w,
            PlatformWalletChangeSet {
                account_address_pools: snapshots,
                ..Default::default()
            },
        )
        .unwrap();

    persister
        .store(
            w,
            PlatformWalletChangeSet {
                core: Some(CoreChangeSet {
                    new_utxos: vec![
                        utxo_at(&good_addr, 0, 100_000),
                        utxo_at(&undeclared, 9, 200_000),
                    ],
                    last_processed_height: Some(123),
                    synced_height: Some(123),
                    ..Default::default()
                }),
                ..Default::default()
            },
        )
        .expect("mixed batch must commit; the undeclared UTXO is skipped, not fatal");

    let conn = persister.lock_conn_for_test();
    let by_account = core_state::list_unspent_utxos(&conn, &w).unwrap();
    let all: Vec<_> = by_account.values().flatten().collect();
    assert_eq!(
        all.len(),
        1,
        "only the good UTXO commits; the bad one is skipped"
    );
    assert!(
        all.iter().all(|r| r.value == 100_000),
        "the committed UTXO is the good one"
    );

    // The sync-height bump committed in the same transaction.
    let synced: Option<i64> = conn
        .query_row(
            "SELECT synced_height FROM core_sync_state WHERE wallet_id = ?1",
            rusqlite::params![w.as_slice()],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(
        synced,
        Some(123),
        "sync-height must commit alongside valid records"
    );
}
