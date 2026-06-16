#![allow(clippy::field_reassign_with_default)]

//! UTXO account-index resolution against the `account_address_pools`
//! manifest, the emitter-contract guard, and flush blast-radius
//! containment for a UTXO at a genuinely-undeclared address.
//!
//! `core_derived_addresses` is a live-fed indexed cache; the authoritative
//! manifest is `account_address_pools` (kept complete and in-band by the
//! `core_bridge` emitter). On a cache miss the UTXO writer falls back to
//! the manifest, so a UTXO landing on a registered pool address resolves
//! even with no live `addresses_derived` event — no mirror, no reconcile.

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

/// Genesis-rescan persist: a wallet registered with pools but with NO live
/// `addresses_derived` event still resolves the account index of a UTXO
/// landing on a pool address — the UTXO writer falls back to the
/// `account_address_pools` manifest. No mirror writes a derived row.
#[test]
fn genesis_rescan_utxo_at_pool_address_persists() {
    let (persister, _tmp, _path) = fresh_persister();
    let w: WalletId = wid(0xA0);
    ensure_wallet_meta(&persister, &w);

    let (snapshots, target) = wallet_with_pools(0x11);
    let addr = target.address.clone();
    let expected_index = match snapshots[0].account_type {
        key_wallet::account::AccountType::Standard { index, .. } => index,
        _ => unreachable!("fixture uses a Standard account"),
    };

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
    let resolved = by_account
        .get(&expected_index)
        .map(|v| v.len())
        .unwrap_or(0);
    assert_eq!(
        resolved, 1,
        "the UTXO must resolve to the pool's account index via the manifest fallback"
    );
    // The manifest fallback resolves without writing a derived-cache row.
    let derived = core_state::list_derived_addresses_for_test(&conn, &w).unwrap();
    assert!(
        derived.iter().all(|r| r.address != addr.to_string()),
        "no mirror: the pool address must NOT be written into core_derived_addresses"
    );
}

/// Back-compat, no migration: an "old-style" DB (frozen registration pool
/// snapshot + a live-fed `core_derived_addresses` row, never any mirror)
/// resolves both address classes under the new model — a gap-limit address
/// via the live cache hit, a registration-only address via the manifest
/// fallback. The fallback is a strict superset of the old read path.
#[test]
fn old_style_db_resolves_via_cache_and_manifest_fallback() {
    let (persister, _tmp, _path) = fresh_persister();
    let w: WalletId = wid(0xB1);
    ensure_wallet_meta(&persister, &w);

    let (snapshots, registration_target) = wallet_with_pools(0x22);
    let pool = &snapshots[0];
    assert!(
        pool.addresses.len() >= 2,
        "fixture needs two pool addresses"
    );
    let account_index = match pool.account_type {
        key_wallet::account::AccountType::Standard { index, .. } => index,
        _ => unreachable!("fixture uses a Standard account"),
    };
    // A gap-limit address persisted ONLY as a live-fed derived row.
    let gap_addr = pool.addresses[1].address.clone();

    persister
        .store(
            w,
            PlatformWalletChangeSet {
                account_address_pools: snapshots.clone(),
                ..Default::default()
            },
        )
        .unwrap();

    // Seed the live-fed derived row for the gap-limit address (as the live
    // `addresses_derived` path would have, at extension time).
    {
        let conn = persister.lock_conn_for_test();
        conn.execute(
            "INSERT INTO core_derived_addresses \
                (wallet_id, account_type, account_index, pool_type, derivation_index, address, used) \
             VALUES (?1, 'standard_bip44', ?2, 'external', ?3, ?4, 0)",
            rusqlite::params![
                w.as_slice(),
                i64::from(account_index),
                i64::from(pool.addresses[1].index),
                gap_addr.to_string()
            ],
        )
        .unwrap();
    }

    // Two UTXOs: one on the live-cached gap address, one on a
    // registration-only pool address (in the manifest, not in the cache).
    persister
        .store(
            w,
            PlatformWalletChangeSet {
                core: Some(CoreChangeSet {
                    new_utxos: vec![
                        utxo_at(&gap_addr, 0, 111_000),
                        utxo_at(&registration_target.address, 1, 222_000),
                    ],
                    ..Default::default()
                }),
                ..Default::default()
            },
        )
        .expect("both address classes must resolve without migration");

    let conn = persister.lock_conn_for_test();
    let by_account = core_state::list_unspent_utxos(&conn, &w).unwrap();
    let rows = by_account.get(&account_index).expect("account row");
    assert_eq!(
        rows.len(),
        2,
        "both UTXOs resolve to the same account index"
    );
    let values: std::collections::BTreeSet<u64> = rows.iter().map(|r| r.value).collect();
    assert!(
        values.contains(&111_000) && values.contains(&222_000),
        "the cache-hit and manifest-fallback UTXOs both committed"
    );
}

/// Reload-then-resolve: a DB with pool snapshots but ZERO derived rows
/// (no mirror, no reconcile) survives a `load` untouched, and a UTXO that
/// lands on a pool address afterwards still resolves via the manifest
/// fallback. `load` does NOT backfill the derived cache.
#[test]
fn reload_resolves_pool_address_without_reconcile() {
    let (persister, _tmp, path) = fresh_persister();
    let w: WalletId = wid(0xC0);
    ensure_wallet_meta(&persister, &w);

    let (snapshots, target) = wallet_with_pools(0x33);
    let addr = target.address.clone();
    let account_index = match snapshots[0].account_type {
        key_wallet::account::AccountType::Standard { index, .. } => index,
        _ => unreachable!("fixture uses a Standard account"),
    };

    persister
        .store(
            w,
            PlatformWalletChangeSet {
                account_address_pools: snapshots,
                ..Default::default()
            },
        )
        .unwrap();
    drop(persister);

    let p2 = reopen(&path);
    PlatformWalletPersistence::load(&p2).expect("load");

    // load must NOT backfill the derived cache from pools.
    {
        let conn = p2.lock_conn_for_test();
        let n = core_state::list_derived_addresses_for_test(&conn, &w)
            .unwrap()
            .len();
        assert_eq!(n, 0, "load does not reconcile the derived cache");
    }

    p2.store(
        w,
        PlatformWalletChangeSet {
            core: Some(CoreChangeSet {
                new_utxos: vec![utxo_at(&addr, 0, 555_000)],
                ..Default::default()
            }),
            ..Default::default()
        },
    )
    .expect("a UTXO at a pool address resolves via the manifest after reload");

    let conn = p2.lock_conn_for_test();
    let by_account = core_state::list_unspent_utxos(&conn, &w).unwrap();
    assert_eq!(
        by_account.get(&account_index).map(|v| v.len()).unwrap_or(0),
        1,
        "the pool-address UTXO resolves to its account index via the manifest"
    );
}

/// Partial-state resolution: a DB holding a live derived-cache row at an
/// OFF-pool account_index plus a pool address that was never derived. The
/// live cache row stays authoritative (the manifest fallback never
/// overrides a cache hit), and the un-derived pool address resolves via
/// the manifest fallback. No reconcile rewrites anything.
#[test]
fn partial_state_cache_hit_wins_over_manifest_fallback() {
    let (persister, _tmp, _path) = fresh_persister();
    let w: WalletId = wid(0xC5);
    ensure_wallet_meta(&persister, &w);

    let (snapshots, _target) = wallet_with_pools(0x55);
    let pool = &snapshots[0];
    assert!(
        pool.addresses.len() >= 2,
        "fixture needs at least two pool addresses"
    );
    let pool_account_index = match pool.account_type {
        key_wallet::account::AccountType::Standard { index, .. } => index,
        _ => unreachable!("fixture uses a Standard account"),
    };

    // A pool address pre-seeded as a live cache row at an OFF-pool index,
    // so a fallback override would be visible.
    let live_addr = pool.addresses[0].address.clone();
    // A pool address present only in the manifest, never live-derived.
    let manifest_only_addr = pool.addresses[1].address.clone();
    const LIVE_ACCOUNT_INDEX: u32 = 4242;

    persister
        .store(
            w,
            PlatformWalletChangeSet {
                account_address_pools: snapshots.clone(),
                ..Default::default()
            },
        )
        .unwrap();

    {
        let conn = persister.lock_conn_for_test();
        conn.execute(
            "INSERT INTO core_derived_addresses \
                (wallet_id, account_type, account_index, pool_type, derivation_index, address, used) \
             VALUES (?1, 'standard_bip44', ?2, 'internal', 999, ?3, 1)",
            rusqlite::params![w.as_slice(), LIVE_ACCOUNT_INDEX, live_addr.to_string()],
        )
        .unwrap();
    }

    persister
        .store(
            w,
            PlatformWalletChangeSet {
                core: Some(CoreChangeSet {
                    new_utxos: vec![
                        utxo_at(&live_addr, 0, 100_000),
                        utxo_at(&manifest_only_addr, 1, 200_000),
                    ],
                    ..Default::default()
                }),
                ..Default::default()
            },
        )
        .expect("both UTXOs resolve");

    let conn = persister.lock_conn_for_test();
    let by_account = core_state::list_unspent_utxos(&conn, &w).unwrap();

    // The live cache row wins: its UTXO resolves to the off-pool index.
    let live_rows = by_account
        .get(&LIVE_ACCOUNT_INDEX)
        .expect("off-pool index row");
    assert!(
        live_rows.iter().any(|r| r.value == 100_000),
        "the cache hit resolves to the live (off-pool) account_index, not the manifest's"
    );
    // The manifest-only address resolves via the fallback to its pool index.
    let pool_rows = by_account.get(&pool_account_index).expect("pool index row");
    assert!(
        pool_rows.iter().any(|r| r.value == 200_000),
        "the manifest-only address resolves via the fallback"
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

/// A single-address `account_address_pools` snapshot for an explicit slot,
/// so the apply-time emitter-contract guard accepts a matching
/// `addresses_derived` event. `new_from_script_pubkey_p2pkh` yields a
/// keyless `AddressInfo` — enough for the manifest membership check.
fn manifest_entry(
    account_type: key_wallet::account::AccountType,
    pool_type: key_wallet::managed_account::address_pool::AddressPoolType,
    index: u32,
    address: &dashcore::Address,
) -> AccountAddressPoolEntry {
    let info = AddressInfo::new_from_script_pubkey_p2pkh(
        address.script_pubkey(),
        index,
        Default::default(),
        key_wallet::Network::Testnet,
    )
    .expect("p2pkh AddressInfo");
    AccountAddressPoolEntry {
        account_type,
        pool_type,
        addresses: vec![info],
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

/// Assert a storage error is specifically a SQLite UNIQUE-constraint
/// violation (`SQLITE_CONSTRAINT_UNIQUE`, 2067) — not merely some generic
/// constraint, so a PK/CHECK/NOT-NULL failure cannot satisfy it.
fn assert_unique_violation(err: platform_wallet_storage::WalletStorageError) {
    match err {
        platform_wallet_storage::WalletStorageError::Sqlite(rusqlite::Error::SqliteFailure(
            e,
            _,
        )) => {
            assert_eq!(
                e.code,
                rusqlite::ErrorCode::ConstraintViolation,
                "expected a constraint violation, got {e:?}"
            );
            assert_eq!(
                e.extended_code,
                rusqlite::ffi::SQLITE_CONSTRAINT_UNIQUE,
                "expected SQLITE_CONSTRAINT_UNIQUE (2067), got extended_code={}",
                e.extended_code
            );
        }
        other => panic!("expected a SQLite constraint error, got {other:?}"),
    }
}

/// The whole BIP32 leaf grain: each live derivation in a multi-address pool
/// persists ONE row per derivation index — never a single collapsed row.
/// Regression guard for the 1-row collapse a non-leaf PK caused.
#[test]
fn multi_address_pool_persists_one_row_per_leaf() {
    let (persister, _tmp, _path) = fresh_persister();
    let w: WalletId = wid(0xF0);
    ensure_wallet_meta(&persister, &w);

    let (snapshots, _target) = wallet_with_pools(0x22);
    let pool = &snapshots[0];
    let pool_len = pool.addresses.len();
    assert!(pool_len >= 2, "fixture needs a multi-address pool");

    // The manifest vouches for every leaf, then each is live-derived.
    let derived: Vec<platform_wallet::DerivedAddress> = pool
        .addresses
        .iter()
        .map(|info| derived_for(pool, info))
        .collect();

    persister
        .store(
            w,
            PlatformWalletChangeSet {
                core: Some(CoreChangeSet {
                    addresses_derived: derived,
                    ..Default::default()
                }),
                account_address_pools: snapshots.clone(),
                ..Default::default()
            },
        )
        .unwrap();

    let conn = persister.lock_conn_for_test();
    let rows = core_state::list_derived_addresses_for_test(&conn, &w).unwrap();
    assert_eq!(
        rows.len(),
        pool_len,
        "every derived leaf must persist its own row (no PK collapse)"
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

    // The manifest must vouch for the derived address (emitter contract).
    persister
        .store(
            w,
            PlatformWalletChangeSet {
                account_address_pools: vec![manifest_entry(
                    acct,
                    AddressPoolType::External,
                    0,
                    &addr,
                )],
                ..Default::default()
            },
        )
        .unwrap();

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

    persister
        .store(
            w,
            PlatformWalletChangeSet {
                account_address_pools: vec![manifest_entry(
                    acct,
                    AddressPoolType::External,
                    0,
                    &addr,
                )],
                ..Default::default()
            },
        )
        .unwrap();

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

/// `used` is write-once on the AUTHORITATIVE path: a live re-derive of the
/// same leaf carries `used=false` and must NOT clear a stored `used=true`.
/// The `DO NOTHING` conflict clause is what preserves it — a `DO UPDATE SET
/// used = excluded.used` regression would clobber the flag, and this test
/// is the guard.
#[test]
fn authoritative_redrive_preserves_used_true() {
    use key_wallet::managed_account::address_pool::AddressPoolType;

    let (persister, _tmp, _path) = fresh_persister();
    let w: WalletId = wid(0xF4);
    ensure_wallet_meta(&persister, &w);

    let addr = addr_from(0x74);
    let acct = standard_account();

    persister
        .store(
            w,
            PlatformWalletChangeSet {
                account_address_pools: vec![manifest_entry(
                    acct,
                    AddressPoolType::External,
                    0,
                    &addr,
                )],
                ..Default::default()
            },
        )
        .unwrap();

    // Seed the leaf with used=true (the snapshot's real flag).
    {
        let conn = persister.lock_conn_for_test();
        conn.execute(
            "INSERT INTO core_derived_addresses \
                (wallet_id, account_type, account_index, pool_type, derivation_index, address, used) \
             VALUES (?1, 'standard_bip44', 0, 'external', 0, ?2, 1)",
            rusqlite::params![w.as_slice(), addr.to_string()],
        )
        .unwrap();
    }

    // Live re-derive of the SAME leaf — the apply path hardcodes used=false.
    {
        let mut conn = persister.lock_conn_for_test();
        let tx = conn.transaction().unwrap();
        core_state::apply(
            &tx,
            &w,
            &CoreChangeSet {
                addresses_derived: vec![derived_at(
                    acct,
                    AddressPoolType::External,
                    0,
                    addr.clone(),
                )],
                ..Default::default()
            },
        )
        .expect("same-leaf re-derive must be a no-op, not an error");
        tx.commit().unwrap();
    }

    let conn = persister.lock_conn_for_test();
    let rows = core_state::list_derived_addresses_for_test(&conn, &w).unwrap();
    let at_addr: Vec<_> = rows
        .iter()
        .filter(|r| r.address == addr.to_string())
        .collect();
    assert_eq!(at_addr.len(), 1, "re-derive must not insert a second row");
    assert!(
        at_addr[0].used,
        "a live re-derive (used=false) must NOT clear a stored used=true"
    );
}

/// Reload leaves the live cache authoritative: a pool address is ALSO held
/// as a live derived-cache row at an off-pool leaf. After `load` (no
/// reconcile), that single live row is still the only read-index for the
/// address — untouched leaf, untouched `used` — so resolution is stable
/// and `UNIQUE(address)` holds.
#[test]
fn reload_keeps_single_live_row_for_pool_address() {
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

    // Seed ONE live row claiming the pool address at an off-pool leaf.
    const LIVE_DERIVATION_INDEX: i64 = 7777;
    {
        let conn = persister.lock_conn_for_test();
        conn.execute(
            "INSERT INTO core_derived_addresses \
                (wallet_id, account_type, account_index, pool_type, derivation_index, address, used) \
             VALUES (?1, 'standard_bip44', 0, 'internal', ?2, ?3, 1)",
            rusqlite::params![w.as_slice(), LIVE_DERIVATION_INDEX, pool_addr],
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
    let at_addr: Vec<_> = rows.iter().filter(|r| r.address == pool_addr).collect();
    assert_eq!(
        at_addr.len(),
        1,
        "exactly one read-index row per address after reload (no reconcile dupe)"
    );
    let live = at_addr[0];
    assert_eq!(
        live.pool_type, "internal",
        "the live row's pool_type is untouched"
    );
    assert_eq!(
        live.derivation_index, LIVE_DERIVATION_INDEX,
        "the live row's derivation_index is untouched"
    );
    assert!(live.used, "the live row's used flag is untouched");
}

/// Derived-index invariant goes FATAL: an address this wallet's persisted
/// Emitter contract goes FATAL: a live `addresses_derived` entry whose
/// address is ABSENT from the `account_address_pools` manifest means the
/// emitter failed to attach the pool snapshot in-band. `core_state::apply`
/// aborts the flush with [`WalletStorageError::DerivedIndexInvariantViolated`]
/// (non-transient → `Fatal`), surfacing the broken contract at the storage
/// trust boundary instead of persisting a row the manifest can't vouch for.
#[test]
fn derivation_absent_from_manifest_is_fatal() {
    use key_wallet::managed_account::address_pool::AddressPoolType;
    use platform_wallet::changeset::PersistenceErrorKind;
    use platform_wallet_storage::WalletStorageError;

    let (persister, _tmp, _path) = fresh_persister();
    let w: WalletId = wid(0xE0);
    ensure_wallet_meta(&persister, &w);

    let acct = standard_account();
    let declared = addr_from(0x77);
    let orphan = addr_from(0x78);
    assert_ne!(declared, orphan, "fixture sanity");

    // Manifest vouches for `declared` only.
    persister
        .store(
            w,
            PlatformWalletChangeSet {
                account_address_pools: vec![manifest_entry(
                    acct,
                    AddressPoolType::External,
                    0,
                    &declared,
                )],
                ..Default::default()
            },
        )
        .unwrap();

    // A derivation for an address the manifest never declared must abort.
    let err = {
        let mut conn = persister.lock_conn_for_test();
        let tx = conn.transaction().unwrap();
        core_state::apply(
            &tx,
            &w,
            &CoreChangeSet {
                addresses_derived: vec![derived_at(
                    acct,
                    AddressPoolType::External,
                    1,
                    orphan.clone(),
                )],
                ..Default::default()
            },
        )
        .expect_err("a derivation absent from the manifest must be fatal")
    };

    match &err {
        WalletStorageError::DerivedIndexInvariantViolated { address } => {
            assert_eq!(
                *address,
                orphan.to_string(),
                "the violation must name the orphaned derived address"
            );
        }
        other => panic!("expected DerivedIndexInvariantViolated, got {other:?}"),
    }
    assert!(
        !err.is_transient(),
        "an emitter-contract violation is a logic regression, never retryable"
    );
    assert_eq!(
        err.persistence_kind(),
        PersistenceErrorKind::Fatal,
        "the violation must classify Fatal at the trait boundary"
    );
}

/// PK axis-1 regression: two Standard accounts with the same pool slot but
/// DIFFERENT `account_index` (index 0 and index 1, both BIP44) must each
/// persist their own row in `core_derived_addresses`. Before the fix both
/// collapsed to the same PK and the second row was silently dropped.
///
/// Also verifies that a UTXO lookup at each address resolves to the CORRECT
/// `account_index`, not the survivor's.
#[test]
fn multi_account_index_same_slot_persists_both_rows() {
    use key_wallet::account::{AccountType, StandardAccountType};
    use key_wallet::managed_account::address_pool::AddressPoolType;

    let (persister, _tmp, _path) = fresh_persister();
    let w: WalletId = wid(0x51);
    ensure_wallet_meta(&persister, &w);

    // Two BIP44 standard accounts at index 0 and 1, both deriving slot 0 of
    // their external pool at distinct addresses.
    let acct0 = AccountType::Standard {
        index: 0,
        standard_account_type: StandardAccountType::BIP44Account,
    };
    let acct1 = AccountType::Standard {
        index: 1,
        standard_account_type: StandardAccountType::BIP44Account,
    };
    let addr0 = addr_from(0xA0);
    let addr1 = addr_from(0xA1);

    // Register both pools so the emitter-contract guard accepts both events.
    persister
        .store(
            w,
            PlatformWalletChangeSet {
                account_address_pools: vec![
                    manifest_entry(acct0, AddressPoolType::External, 0, &addr0),
                    manifest_entry(acct1, AddressPoolType::External, 0, &addr1),
                ],
                ..Default::default()
            },
        )
        .unwrap();

    // Live derive both leaves in the same changeset.
    persister
        .store(
            w,
            PlatformWalletChangeSet {
                core: Some(CoreChangeSet {
                    addresses_derived: vec![
                        derived_at(acct0, AddressPoolType::External, 0, addr0.clone()),
                        derived_at(acct1, AddressPoolType::External, 0, addr1.clone()),
                    ],
                    new_utxos: vec![utxo_at(&addr0, 0, 100_000), utxo_at(&addr1, 1, 200_000)],
                    ..Default::default()
                }),
                ..Default::default()
            },
        )
        .expect("both derived rows must survive (account_index is now in the PK)");

    let conn = persister.lock_conn_for_test();
    let derived = core_state::list_derived_addresses_for_test(&conn, &w).unwrap();
    assert_eq!(
        derived.len(),
        2,
        "both derived rows must exist — no PK collapse"
    );

    let by_account = core_state::list_unspent_utxos(&conn, &w).unwrap();
    let utxo0 = by_account
        .get(&0)
        .and_then(|v| v.iter().find(|r| r.value == 100_000));
    let utxo1 = by_account
        .get(&1)
        .and_then(|v| v.iter().find(|r| r.value == 200_000));
    assert!(
        utxo0.is_some(),
        "the account-0 UTXO must resolve to account_index 0"
    );
    assert!(
        utxo1.is_some(),
        "the account-1 UTXO must resolve to account_index 1"
    );
}

/// PK axis-2 regression: a BIP32 standard acct-0 and a BIP44 standard
/// acct-0 derive to distinct addresses at the same pool slot. Before the
/// fix both collapsed to one row (both mapped to the `"standard"` label),
/// and the second write was dropped. After the fix the labels are distinct
/// (`"standard_bip32"` vs `"standard_bip44"`) so both rows coexist.
#[test]
fn bip32_and_bip44_standard_acct0_persist_both_rows() {
    use key_wallet::account::{AccountType, StandardAccountType};
    use key_wallet::managed_account::address_pool::AddressPoolType;

    let (persister, _tmp, _path) = fresh_persister();
    let w: WalletId = wid(0x52);
    ensure_wallet_meta(&persister, &w);

    let bip44 = AccountType::Standard {
        index: 0,
        standard_account_type: StandardAccountType::BIP44Account,
    };
    let bip32 = AccountType::Standard {
        index: 0,
        standard_account_type: StandardAccountType::BIP32Account,
    };
    let addr_bip44 = addr_from(0xB4);
    let addr_bip32 = addr_from(0xB2);

    persister
        .store(
            w,
            PlatformWalletChangeSet {
                account_address_pools: vec![
                    manifest_entry(bip44, AddressPoolType::External, 0, &addr_bip44),
                    manifest_entry(bip32, AddressPoolType::External, 0, &addr_bip32),
                ],
                ..Default::default()
            },
        )
        .unwrap();

    persister
        .store(
            w,
            PlatformWalletChangeSet {
                core: Some(CoreChangeSet {
                    addresses_derived: vec![
                        derived_at(bip44, AddressPoolType::External, 0, addr_bip44.clone()),
                        derived_at(bip32, AddressPoolType::External, 0, addr_bip32.clone()),
                    ],
                    new_utxos: vec![
                        utxo_at(&addr_bip44, 0, 444_000),
                        utxo_at(&addr_bip32, 1, 322_000),
                    ],
                    ..Default::default()
                }),
                ..Default::default()
            },
        )
        .expect("BIP32 and BIP44 standard acct-0 rows must coexist (distinct labels)");

    let conn = persister.lock_conn_for_test();
    let derived = core_state::list_derived_addresses_for_test(&conn, &w).unwrap();
    assert_eq!(
        derived.len(),
        2,
        "BIP32 and BIP44 derived rows must both survive"
    );
    let labels: std::collections::BTreeSet<&str> =
        derived.iter().map(|r| r.account_type.as_str()).collect();
    assert!(
        labels.contains("standard_bip44") && labels.contains("standard_bip32"),
        "both account_type labels must be present in the derived cache"
    );

    let by_account = core_state::list_unspent_utxos(&conn, &w).unwrap();
    let total: usize = by_account.values().map(|v| v.len()).sum();
    assert_eq!(
        total, 2,
        "both UTXOs must resolve via their respective accounts"
    );
}

/// `account_registrations` PK regression (todo 0e3ad26b): a BIP32 and a
/// BIP44 standard acct-0 registered with DIFFERENT xpubs must each persist
/// their own row. Before the fix both resolved to label `"standard"` so the
/// second INSERT clobbered the first row's xpub. After the fix their labels
/// differ and both rows coexist with their original xpubs intact.
#[test]
fn account_registrations_bip32_and_bip44_both_survive() {
    use key_wallet::account::{AccountType, StandardAccountType};
    use key_wallet::wallet::initialization::WalletAccountCreationOptions;
    use key_wallet::wallet::Wallet;
    use platform_wallet::changeset::AccountRegistrationEntry;
    use platform_wallet_storage::sqlite::schema::accounts;

    // Two distinct xpubs: seed them from different seed bytes.
    let xpub_bip44 = {
        let w = Wallet::from_seed_bytes(
            [0xBBu8; 64],
            key_wallet::Network::Testnet,
            WalletAccountCreationOptions::Default,
        )
        .unwrap();
        w.accounts
            .all_accounts()
            .first()
            .expect("at least one account")
            .account_xpub
    };
    let xpub_bip32 = {
        let w = Wallet::from_seed_bytes(
            [0xCCu8; 64],
            key_wallet::Network::Testnet,
            WalletAccountCreationOptions::Default,
        )
        .unwrap();
        w.accounts
            .all_accounts()
            .first()
            .expect("at least one account")
            .account_xpub
    };
    assert_ne!(
        xpub_bip44, xpub_bip32,
        "fixture: seeds must yield distinct xpubs"
    );

    let (persister, _tmp, _path) = fresh_persister();
    let w: WalletId = wid(0x53);
    ensure_wallet_meta(&persister, &w);

    let bip44_entry = AccountRegistrationEntry {
        account_type: AccountType::Standard {
            index: 0,
            standard_account_type: StandardAccountType::BIP44Account,
        },
        account_xpub: xpub_bip44,
    };
    let bip32_entry = AccountRegistrationEntry {
        account_type: AccountType::Standard {
            index: 0,
            standard_account_type: StandardAccountType::BIP32Account,
        },
        account_xpub: xpub_bip32,
    };

    persister
        .store(
            w,
            PlatformWalletChangeSet {
                account_registrations: vec![bip44_entry.clone(), bip32_entry.clone()],
                ..Default::default()
            },
        )
        .expect("both registrations must persist without clobbering each other");

    let conn = persister.lock_conn_for_test();
    let manifest = accounts::load_state(&conn, &w).expect("load_state");
    drop(conn);

    assert_eq!(
        manifest.len(),
        2,
        "both account_registrations rows must survive — no xpub clobber"
    );

    let has_bip44 = manifest.iter().any(|e| {
        matches!(
            e.account_type,
            AccountType::Standard {
                standard_account_type: StandardAccountType::BIP44Account,
                ..
            }
        ) && e.account_xpub == xpub_bip44
    });
    let has_bip32 = manifest.iter().any(|e| {
        matches!(
            e.account_type,
            AccountType::Standard {
                standard_account_type: StandardAccountType::BIP32Account,
                ..
            }
        ) && e.account_xpub == xpub_bip32
    });
    assert!(
        has_bip44,
        "the BIP44 registration with its original xpub must survive"
    );
    assert!(
        has_bip32,
        "the BIP32 registration with its original xpub must survive"
    );
}

/// The guard does not over-fire: a UTXO at a genuinely-undeclared address
/// (in NO pool, never derived) is still SKIPPED quietly — no error escapes,
/// and the rest of the batch persists. Guards against the invariant guard
/// firing on a benign SPV gap-limit miss.
#[test]
fn undeclared_unspent_utxo_at_apply_level_is_skipped_not_fatal() {
    let (persister, _tmp, _path) = fresh_persister();
    let w: WalletId = wid(0xE1);
    ensure_wallet_meta(&persister, &w);

    // Register a pool so `account_address_pools` is NON-empty: the good
    // address resolves via the manifest fallback, the undeclared one skips.
    let (snapshots, good) = wallet_with_pools(0x88);
    let good_addr = good.address.clone();
    let expected_index = match snapshots[0].account_type {
        key_wallet::account::AccountType::Standard { index, .. } => index,
        _ => unreachable!("fixture uses a Standard account"),
    };
    persister
        .store(
            w,
            PlatformWalletChangeSet {
                account_address_pools: snapshots,
                ..Default::default()
            },
        )
        .unwrap();

    // An address in NO pool and never derived.
    let undeclared = addr_from(0x9A);
    assert_ne!(undeclared, good_addr, "fixture sanity");

    {
        let mut conn = persister.lock_conn_for_test();
        let tx = conn.transaction().unwrap();
        core_state::apply(
            &tx,
            &w,
            &CoreChangeSet {
                new_utxos: vec![
                    utxo_at(&good_addr, 0, 100_000),
                    utxo_at(&undeclared, 9, 200_000),
                ],
                ..Default::default()
            },
        )
        .expect("a genuinely-undeclared address must be skipped, not fatal");
        tx.commit().unwrap();
    }

    let conn = persister.lock_conn_for_test();

    // The good pool address resolves via the manifest fallback — no mirrored
    // derived row exists for it.
    let derived = core_state::list_derived_addresses_for_test(&conn, &w).unwrap();
    assert!(
        derived.iter().all(|r| r.address != good_addr.to_string()),
        "no mirror: the good pool address is resolved from the manifest, not the cache"
    );

    let by_account = core_state::list_unspent_utxos(&conn, &w).unwrap();
    let all: Vec<_> = by_account.values().flatten().collect();
    assert_eq!(
        all.len(),
        1,
        "the declared-address UTXO commits; the undeclared one is skipped"
    );
    let survivor = all[0];
    assert_eq!(
        survivor.value, 100_000,
        "the committed UTXO is the one at the declared pool address"
    );
    // Skipping the undeclared row must leave the good row's attribution
    // intact — it resolves to the pool's account index, not a placeholder.
    assert_eq!(
        survivor.account_index, expected_index,
        "the surviving UTXO must keep its resolved account index"
    );
}
