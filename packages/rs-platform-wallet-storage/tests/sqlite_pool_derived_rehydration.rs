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
/// round in `wallet_lifecycle.rs`. One pool of distinct BIP32 leaves keeps
/// every derived row unique, so the test reads back exactly what it wrote.
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

/// A live `DerivedAddress` event for one pool `AddressInfo` — a valid,
/// non-UTXO record for the blast-radius batch.
fn derived_for(
    pool: &AccountAddressPoolEntry,
    info: &AddressInfo,
) -> platform_wallet::DerivedAddress {
    // Compressed secp256k1 generator point — a valid placeholder pubkey.
    const PUBKEY_G: [u8; 33] = [
        0x02, 0x79, 0xbe, 0x66, 0x7e, 0xf9, 0xdc, 0xbb, 0xac, 0x55, 0xa0, 0x62, 0x95, 0xce, 0x87,
        0x0b, 0x07, 0x02, 0x9b, 0xfc, 0xdb, 0x2d, 0xce, 0x28, 0xd9, 0x59, 0xf2, 0x81, 0x5b, 0x16,
        0xf8, 0x17, 0x98,
    ];
    platform_wallet::DerivedAddress {
        account_type: pool.account_type,
        pool_type: pool.pool_type,
        derivation_index: info.index,
        address: info.address.clone(),
        public_key: dashcore::PublicKey::from_slice(&PUBKEY_G).expect("valid compressed pubkey"),
    }
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
/// `apply_pools` is byte-identical (account_index, pool_type,
/// derivation_index, used)
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
        row_pool.pool_type, row_live.pool_type,
        "pool_type must match"
    );
    assert_eq!(
        row_pool.derivation_index, row_live.derivation_index,
        "derivation_index must match"
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

/// Partial-state self-heal: a wallet with SOME live-derived rows (one
/// `used = true`) plus a pool address that was never derived is repaired
/// on `load` — the missing address is added with its pool account_index,
/// and every pre-existing live row is left untouched (the reconcile is
/// purely additive, a live row is authoritative).
#[test]
fn load_reconciles_partial_state_without_clobbering_live_rows() {
    let (persister, _tmp, path) = fresh_persister();
    let w: WalletId = wid(0xC5);
    ensure_wallet_meta(&persister, &w);

    let (snapshots, _target) = wallet_with_pools(0x55);
    let pool = &snapshots[0];
    assert!(
        pool.addresses.len() >= 2,
        "fixture needs at least two pool addresses"
    );
    let pool_account_index = i64::from(match pool.account_type {
        key_wallet::account::AccountType::Standard { index, .. } => index,
        _ => unreachable!("fixture uses a Standard account"),
    });

    // A pool address we deliberately pre-seed as a live row, with a
    // non-pool account_index and used=true, so a clobber would be visible.
    let live_addr = pool.addresses[0].address.to_string();
    // A pool address left un-derived — the gap the reconcile must fill.
    let missing_addr = pool.addresses[1].address.to_string();
    const LIVE_ACCOUNT_INDEX: i64 = 4242;
    // Off-pool leaf: a different pool_type/derivation_index than the
    // external-pool leaf the reconcile would assign, so the would-be
    // reconcile insert is a UNIQUE(address) skip, not a PK no-op.
    const LIVE_DERIVATION_INDEX: i64 = 999;

    persister
        .store(
            w,
            PlatformWalletChangeSet {
                account_address_pools: snapshots.clone(),
                ..Default::default()
            },
        )
        .unwrap();

    // Recreate a partial prod DB: drop the auto-mirrored rows, then seed
    // ONLY the live row (authoritative, used=true, off-pool index).
    {
        let conn = persister.lock_conn_for_test();
        conn.execute(
            "DELETE FROM core_derived_addresses WHERE wallet_id = ?1",
            rusqlite::params![w.as_slice()],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO core_derived_addresses \
                (wallet_id, account_type, account_index, pool_type, derivation_index, address, used) \
             VALUES (?1, 'standard', ?2, 'internal', ?3, ?4, 1)",
            rusqlite::params![
                w.as_slice(),
                LIVE_ACCOUNT_INDEX,
                LIVE_DERIVATION_INDEX,
                live_addr
            ],
        )
        .unwrap();
    }
    drop(persister);

    let p2 = reopen(&path);
    PlatformWalletPersistence::load(&p2).expect("load");

    let rows = {
        let conn = p2.lock_conn_for_test();
        core_state::list_derived_addresses_for_test(&conn, &w).unwrap()
    };

    let missing = rows
        .iter()
        .find(|r| r.address == missing_addr)
        .expect("the un-derived pool address must be reconciled on load");
    assert_eq!(
        missing.account_index, pool_account_index,
        "reconciled row must carry the pool account_index"
    );

    let live = rows
        .iter()
        .find(|r| r.address == live_addr)
        .expect("the live row must survive");
    assert_eq!(
        live.account_index, LIVE_ACCOUNT_INDEX,
        "reconcile must NOT overwrite a live row's account_index"
    );
    assert_eq!(
        live.pool_type, "internal",
        "reconcile must NOT overwrite a live row's pool_type"
    );
    assert_eq!(
        live.derivation_index, LIVE_DERIVATION_INDEX,
        "reconcile must NOT overwrite a live row's derivation_index"
    );
    assert!(live.used, "reconcile must NOT clear a live row's used flag");
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
    // A second pool address for a live derive record in the same batch.
    let extra = snapshots[0].addresses[1].clone();
    let extra_derived = derived_for(&snapshots[0], &extra);
    let extra_addr = extra.address.to_string();
    assert_ne!(extra_addr, good_addr.to_string(), "fixture sanity");

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
                account_address_pools: snapshots.clone(),
                ..Default::default()
            },
        )
        .unwrap();

    // Wipe derived rows so the batch's own live derive is the only source
    // of `extra_addr`, making its commit unambiguous.
    {
        let conn = persister.lock_conn_for_test();
        conn.execute(
            "DELETE FROM core_derived_addresses WHERE wallet_id = ?1 AND address = ?2",
            rusqlite::params![w.as_slice(), extra_addr],
        )
        .unwrap();
    }

    persister
        .store(
            w,
            PlatformWalletChangeSet {
                core: Some(CoreChangeSet {
                    addresses_derived: vec![extra_derived],
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

    // A normal valid record in the same batch (the live derive) committed —
    // the skip isolates only the bad UTXO, not the surrounding records.
    let derived = core_state::list_derived_addresses_for_test(&conn, &w).unwrap();
    assert!(
        derived.iter().any(|r| r.address == extra_addr),
        "the live derive record in the mixed batch must commit"
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

/// Build a live `DerivedAddress` for an explicit slot + address — the raw
/// material for the Design-Z invariant tests below.
fn derived_at(
    account_type: key_wallet::account::AccountType,
    pool_type: key_wallet::managed_account::address_pool::AddressPoolType,
    derivation_index: u32,
    address: dashcore::Address,
) -> platform_wallet::DerivedAddress {
    const PUBKEY_G: [u8; 33] = [
        0x02, 0x79, 0xbe, 0x66, 0x7e, 0xf9, 0xdc, 0xbb, 0xac, 0x55, 0xa0, 0x62, 0x95, 0xce, 0x87,
        0x0b, 0x07, 0x02, 0x9b, 0xfc, 0xdb, 0x2d, 0xce, 0x28, 0xd9, 0x59, 0xf2, 0x81, 0x5b, 0x16,
        0xf8, 0x17, 0x98,
    ];
    platform_wallet::DerivedAddress {
        account_type,
        pool_type,
        derivation_index,
        address,
        public_key: dashcore::PublicKey::from_slice(&PUBKEY_G).expect("valid compressed pubkey"),
    }
}

/// An arbitrary testnet P2PKH address from a byte pattern.
fn addr_from(byte: u8) -> dashcore::Address {
    use dashcore::address::Payload;
    use dashcore::hashes::Hash;
    use dashcore::PubkeyHash;
    dashcore::Address::new(
        dashcore::Network::Testnet,
        Payload::PubkeyHash(PubkeyHash::from_byte_array([byte; 20])),
    )
}

/// A Standard BIP44 account type for the explicit-slot fixtures.
fn standard_account() -> key_wallet::account::AccountType {
    key_wallet::account::AccountType::Standard {
        index: 0,
        standard_account_type: key_wallet::account::StandardAccountType::BIP44Account,
    }
}

/// Assert a storage error is a SQLite UNIQUE-constraint violation.
fn assert_unique_violation(err: platform_wallet_storage::WalletStorageError) {
    match err {
        platform_wallet_storage::WalletStorageError::Sqlite(rusqlite::Error::SqliteFailure(
            e,
            _,
        )) => assert_eq!(
            e.code,
            rusqlite::ErrorCode::ConstraintViolation,
            "expected a UNIQUE constraint violation, got {e:?}"
        ),
        other => panic!("expected a SQLite constraint error, got {other:?}"),
    }
}

/// The whole BIP32 leaf grain: a multi-address pool persists ONE row per
/// derivation index — never a single collapsed row. Regression guard for
/// the 1-row collapse a non-leaf PK caused.
#[test]
fn multi_address_pool_persists_one_row_per_leaf() {
    let (persister, _tmp, _path) = fresh_persister();
    let w: WalletId = wid(0xF0);
    ensure_wallet_meta(&persister, &w);

    let (snapshots, _target) = wallet_with_pools(0x22);
    let pool_len = snapshots[0].addresses.len();
    assert!(pool_len >= 2, "fixture needs a multi-address pool");

    persister
        .store(
            w,
            PlatformWalletChangeSet {
                account_address_pools: snapshots,
                ..Default::default()
            },
        )
        .unwrap();

    let conn = persister.lock_conn_for_test();
    let rows = core_state::list_derived_addresses_for_test(&conn, &w).unwrap();
    assert_eq!(
        rows.len(),
        pool_len,
        "every pool address must persist its own row (no PK collapse)"
    );
}

/// Within-pool collision goes LOUD: two distinct `derivation_index` in the
/// SAME pool resolving to the SAME address must NOT silently collapse — the
/// second authoritative write fails on UNIQUE(wallet_id, address).
#[test]
fn within_pool_address_collision_is_loud() {
    use key_wallet::managed_account::address_pool::AddressPoolType;

    let (persister, _tmp, _path) = fresh_persister();
    let w: WalletId = wid(0xF1);
    ensure_wallet_meta(&persister, &w);

    let addr = addr_from(0x71);
    let acct = standard_account();

    let mut conn = persister.lock_conn_for_test();
    let tx = conn.transaction().unwrap();
    core_state::apply(
        &tx,
        &w,
        &CoreChangeSet {
            addresses_derived: vec![derived_at(acct, AddressPoolType::External, 0, addr.clone())],
            ..Default::default()
        },
    )
    .expect("first leaf at a fresh address must persist");

    // Leaf 1 of the SAME pool yielding the SAME address — a distinct PK
    // leaf, so this is a UNIQUE(address) violation, not a PK no-op.
    let err = core_state::apply(
        &tx,
        &w,
        &CoreChangeSet {
            addresses_derived: vec![derived_at(acct, AddressPoolType::External, 1, addr.clone())],
            ..Default::default()
        },
    )
    .expect_err("a different leaf reusing the same address must violate UNIQUE(address)");

    assert_unique_violation(err);
}

/// Cross-pool collision goes loud: the same address at a different
/// pool_type is still a UNIQUE(address) violation.
#[test]
fn cross_pool_address_collision_is_loud() {
    use key_wallet::managed_account::address_pool::AddressPoolType;

    let (persister, _tmp, _path) = fresh_persister();
    let w: WalletId = wid(0xF2);
    ensure_wallet_meta(&persister, &w);

    let addr = addr_from(0x72);
    let acct = standard_account();

    let mut conn = persister.lock_conn_for_test();
    let tx = conn.transaction().unwrap();
    core_state::apply(
        &tx,
        &w,
        &CoreChangeSet {
            addresses_derived: vec![derived_at(acct, AddressPoolType::External, 0, addr.clone())],
            ..Default::default()
        },
    )
    .expect("external-pool leaf must persist");

    let err = core_state::apply(
        &tx,
        &w,
        &CoreChangeSet {
            addresses_derived: vec![derived_at(acct, AddressPoolType::Internal, 0, addr.clone())],
            ..Default::default()
        },
    )
    .expect_err("the same address in a different pool must violate UNIQUE(address)");

    assert_unique_violation(err);
}

/// Reconcile stays non-fatal: a pre-existing live row holds an address at
/// one leaf; the pool snapshot declares the SAME address at a DIFFERENT
/// leaf. On `load`, the gap-fill `INSERT OR IGNORE` must SILENTLY skip the
/// would-be UNIQUE(address) collision rather than aborting the load — and
/// the authoritative live row must survive untouched.
#[test]
fn load_reconcile_silently_skips_unique_address_collision() {
    let (persister, _tmp, path) = fresh_persister();
    let w: WalletId = wid(0xF3);
    ensure_wallet_meta(&persister, &w);

    let (snapshots, target) = wallet_with_pools(0x66);
    let pool_addr = target.address.to_string();

    persister
        .store(
            w,
            PlatformWalletChangeSet {
                account_address_pools: snapshots.clone(),
                ..Default::default()
            },
        )
        .unwrap();

    // Recreate a partial DB: drop the auto-mirrored rows, then seed ONE
    // live row claiming the pool address at a DIFFERENT leaf (off-pool
    // pool_type + derivation_index). Reconcile would try to (re)insert the
    // pool address at its real leaf — a UNIQUE(address) collision.
    const LIVE_ACCOUNT_INDEX: i64 = 0;
    const LIVE_DERIVATION_INDEX: i64 = 7777;
    {
        let conn = persister.lock_conn_for_test();
        conn.execute(
            "DELETE FROM core_derived_addresses WHERE wallet_id = ?1",
            rusqlite::params![w.as_slice()],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO core_derived_addresses \
                (wallet_id, account_type, account_index, pool_type, derivation_index, address, used) \
             VALUES (?1, 'standard', ?2, 'internal', ?3, ?4, 1)",
            rusqlite::params![
                w.as_slice(),
                LIVE_ACCOUNT_INDEX,
                LIVE_DERIVATION_INDEX,
                pool_addr
            ],
        )
        .unwrap();
    }
    drop(persister);

    let p2 = reopen(&path);
    PlatformWalletPersistence::load(&p2).expect("reconcile must not abort load on a UNIQUE skip");

    let rows = {
        let conn = p2.lock_conn_for_test();
        core_state::list_derived_addresses_for_test(&conn, &w).unwrap()
    };
    let at_addr: Vec<_> = rows.iter().filter(|r| r.address == pool_addr).collect();
    assert_eq!(
        at_addr.len(),
        1,
        "UNIQUE(address) guarantees exactly one read-index row per address"
    );
    let live = at_addr[0];
    assert_eq!(
        live.pool_type, "internal",
        "the authoritative live row's pool_type must survive the skipped reconcile insert"
    );
    assert_eq!(
        live.derivation_index, LIVE_DERIVATION_INDEX,
        "the authoritative live row's derivation_index must survive"
    );
    assert!(
        live.used,
        "reconcile must not clear the live row's used flag"
    );
}
