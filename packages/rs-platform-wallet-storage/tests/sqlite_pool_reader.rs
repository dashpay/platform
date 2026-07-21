#![allow(clippy::field_reassign_with_default)]

//! Verbatim pool-snapshot reader. Covers TC-B-020 (used-set
//! comes from `core_address_pool`, not `core_utxos` re-derivation), TC-B-023
//! (deep-derivation window — no horizon-walk truncation), TC-B-025/007
//! (empty wallet loads empty-but-valid), multi-wallet isolation (TC-B-026),
//! and the pool ∪ `core_utxos` used-set union (pre-pool + mixed stores).
//!
//! These assert directly on the two shipped reader fns `load()` itself calls —
//! `core_pool::load_used_addresses` (verbatim pool `used=1`) and
//! `core_state::load_used_addresses` (`core_utxos`-derived, spent + unspent) —
//! not on `load()`'s assembled `core_wallet_info`. The reuse-guard facts pinned
//! here (deep-index no-truncation, pool ∪ UTXO dedup) live at the reader layer:
//! `load()` only marks addresses that resolve to a *registered* account's
//! derived pool, which these keyless, arbitrary-address stores deliberately
//! don't set up. The `load()` → `core_wallet_info` marking path is covered by
//! `sqlite_used_core_addresses.rs`.

mod common;

use common::{ensure_wallet_meta, fresh_persister, wid};
use dashcore::address::Payload;
use dashcore::hashes::Hash;
use dashcore::{Address, Network, PubkeyHash};
use key_wallet::account::{AccountType, StandardAccountType};
use key_wallet::managed_account::address_pool::{AddressPoolType, AddressState};
use key_wallet::wallet::initialization::WalletAccountCreationOptions;
use key_wallet::wallet::managed_wallet_info::ManagedWalletInfo;
use key_wallet::wallet::Wallet;
use key_wallet::{AddressInfo, Utxo};
use platform_wallet::changeset::{
    AccountAddressPoolEntry, CoreChangeSet, PlatformWalletChangeSet, PlatformWalletPersistence,
};
use platform_wallet::wallet::platform_wallet::WalletId;
use platform_wallet_storage::sqlite::schema::{core_pool, core_state};
use platform_wallet_storage::SqlitePersister;

/// Verbatim `core_address_pool` `used=1` addresses — the pool half of the
/// reuse-guard set `load()` reads (owner dropped; these tests assert addresses).
fn pool_used(persister: &SqlitePersister, w: &WalletId) -> Vec<Address> {
    let conn = persister.lock_conn_for_test();
    core_pool::load_used_addresses(&conn, w, Network::Testnet)
        .expect("pool used-set")
        .into_iter()
        .map(|(addr, _owner)| addr)
        .collect()
}

/// `core_utxos`-derived used addresses (spent + unspent) — the UTXO half.
fn utxo_used(persister: &SqlitePersister, w: &WalletId) -> Vec<Address> {
    let conn = persister.lock_conn_for_test();
    core_state::load_used_addresses(&conn, w, Network::Testnet)
        .expect("utxo used-set")
        .into_iter()
        .map(|(addr, _owner)| addr)
        .collect()
}

/// The assembled reuse-guard set `load()` hands the manager: pool ∪ UTXO,
/// deduped by script (mirrors `SqlitePersister::load`).
fn used_set(persister: &SqlitePersister, w: &WalletId) -> Vec<Address> {
    let conn = persister.lock_conn_for_test();
    let pool = core_pool::load_used_addresses(&conn, w, Network::Testnet).expect("pool used-set");
    let utxo = core_state::load_used_addresses(&conn, w, Network::Testnet).expect("utxo used-set");
    drop(conn);
    let mut seen = std::collections::HashSet::new();
    let mut union = Vec::new();
    for addr in pool
        .into_iter()
        .map(|(addr, _owner)| addr)
        .chain(utxo.into_iter().map(|(addr, _owner)| addr))
    {
        if seen.insert(addr.script_pubkey().to_bytes()) {
            union.push(addr);
        }
    }
    union
}

fn external_infos(seed_byte: u8) -> Vec<AddressInfo> {
    let wallet = Wallet::from_seed_bytes(
        [seed_byte; 64],
        Network::Testnet,
        WalletAccountCreationOptions::Default,
    )
    .unwrap();
    let info = ManagedWalletInfo::from_wallet(&wallet, 0);
    for managed in info.all_managed_accounts() {
        if !matches!(
            managed.managed_account_type().to_account_type(),
            AccountType::Standard { index: 0, .. }
        ) {
            continue;
        }
        for pool in managed.managed_account_type().address_pools() {
            if pool.pool_type == AddressPoolType::External && !pool.addresses.is_empty() {
                let mut infos: Vec<AddressInfo> = pool.addresses.values().cloned().collect();
                infos.sort_by_key(|a| a.index);
                return infos;
            }
        }
    }
    panic!("no external pool");
}

fn p2pkh(byte: u8) -> Address {
    Address::new(
        Network::Testnet,
        Payload::PubkeyHash(PubkeyHash::from_byte_array([byte; 20])),
    )
}

/// TC-B-020 — the used-set is the verbatim pool `used=1` state, computed
/// without touching `core_utxos`: no UTXO is stored, yet the used addresses
/// surface (a projection-derived reader would return an empty set).
#[test]
fn tc_b_020_used_set_from_pool_not_utxos() {
    let (persister, _tmp, _path) = fresh_persister();
    let w: WalletId = wid(0x20);
    ensure_wallet_meta(&persister, &w);

    let mut infos = external_infos(0x20);
    infos.truncate(10);
    assert_eq!(infos.len(), 10);
    let used_indices = [0u32, 3, 7];
    for info in infos.iter_mut() {
        info.state = if used_indices.contains(&info.index) {
            AddressState::Used
        } else {
            AddressState::Available
        };
    }
    persister
        .store(
            w,
            PlatformWalletChangeSet {
                account_address_pools: vec![AccountAddressPoolEntry {
                    account_type: AccountType::Standard {
                        index: 0,
                        standard_account_type: StandardAccountType::BIP44Account,
                    },
                    pool_type: AddressPoolType::External,
                    addresses: infos.clone(),
                }],
                ..Default::default()
            },
        )
        .unwrap();

    let got: std::collections::BTreeSet<String> = pool_used(&persister, &w)
        .iter()
        .map(|a| a.to_string())
        .collect();
    let expected: std::collections::BTreeSet<String> = infos
        .iter()
        .filter(|i| used_indices.contains(&i.index))
        .map(|i| i.address.to_string())
        .collect();
    assert_eq!(got, expected, "used-set must equal the pool's used=1 rows");
    assert!(
        utxo_used(&persister, &w).is_empty(),
        "no UTXO stored: the used-set is pool-derived, not core_utxos-derived"
    );
}

/// TC-B-023 — a wallet whose pool advanced past the old horizon-walk window
/// (used up to index 45, then 30 unused) restores its full used-set: the
/// index-45 address is present, never truncated at 30.
#[test]
fn tc_b_023_deep_derivation_window_not_truncated() {
    let (persister, _tmp, _path) = fresh_persister();
    let w: WalletId = wid(0x23);
    ensure_wallet_meta(&persister, &w);
    {
        let conn = persister.lock_conn_for_test();
        for i in 0u32..=75 {
            let used = i32::from(i <= 45);
            conn.execute(
                "INSERT INTO core_address_pool \
                    (wallet_id, account_type, account_index, key_class, pool_type, \
                     address_index, script, used) \
                 VALUES (?1, 'standard_bip44', 0, 0, 0, ?2, ?3, ?4)",
                rusqlite::params![
                    w.as_slice(),
                    i64::from(i),
                    p2pkh(i as u8).script_pubkey().as_bytes(),
                    used
                ],
            )
            .unwrap();
        }
    }

    let used = pool_used(&persister, &w);
    assert_eq!(
        used.len(),
        46,
        "indices 0..=45 are used and must all restore"
    );
    let want = p2pkh(45).to_string();
    assert!(
        used.iter().any(|a| a.to_string() == want),
        "the index-45 used address must survive (no gap-limit-30 truncation)"
    );
}

/// TC-B-025/007 — an empty wallet (a `wallets` row, no pool rows, no UTXOs)
/// loads as empty-but-valid: present with an empty used-set, not corrupt.
#[test]
fn tc_b_025_empty_wallet_is_empty_but_valid() {
    let (persister, _tmp, _path) = fresh_persister();
    let w: WalletId = wid(0x25);
    ensure_wallet_meta(&persister, &w);

    assert!(
        used_set(&persister, &w).is_empty(),
        "empty wallet has an empty used-set"
    );
}

fn utxo_on(addr: &Address, byte: u8, value: u64) -> Utxo {
    Utxo::new(
        dashcore::OutPoint::new(dashcore::Txid::from_byte_array([byte; 32]), 0),
        dashcore::TxOut {
            value,
            script_pubkey: addr.script_pubkey(),
        },
        addr.clone(),
        10,
        false,
    )
}

/// A pre-pool store (UTXOs, no `core_address_pool` rows) yields the
/// reuse-guard set from the `core_utxos`-derived half of the union.
#[test]
fn pre_pool_store_yields_utxo_derived_used_set() {
    let (persister, _tmp, _path) = fresh_persister();
    let w: WalletId = wid(0x26);
    ensure_wallet_meta(&persister, &w);

    let addr = p2pkh(0x99);
    persister
        .store(
            w,
            PlatformWalletChangeSet {
                core: Some(CoreChangeSet {
                    new_utxos: vec![utxo_on(&addr, 0x11, 1000)],
                    ..Default::default()
                }),
                ..Default::default()
            },
        )
        .unwrap();

    let used = utxo_used(&persister, &w);
    assert_eq!(used.len(), 1);
    assert_eq!(used[0].to_string(), addr.to_string());
}

/// TC-B-026 — reader multi-wallet isolation: two wallets seeded with
/// distinct, distinguishable used addresses (and balances) load such that
/// neither wallet's snapshot shows the other's — no cross-wallet leakage.
#[test]
fn tc_b_026_reader_isolates_two_wallets() {
    let (persister, _tmp, _path) = fresh_persister();
    let a: WalletId = wid(0x2A);
    let b: WalletId = wid(0x2B);
    ensure_wallet_meta(&persister, &a);
    ensure_wallet_meta(&persister, &b);

    let addr_a = p2pkh(0xA1);
    let addr_b = p2pkh(0xB1);
    persister
        .store(
            a,
            PlatformWalletChangeSet {
                core: Some(CoreChangeSet {
                    new_utxos: vec![utxo_on(&addr_a, 0x01, 111)],
                    ..Default::default()
                }),
                ..Default::default()
            },
        )
        .unwrap();
    persister
        .store(
            b,
            PlatformWalletChangeSet {
                core: Some(CoreChangeSet {
                    new_utxos: vec![utxo_on(&addr_b, 0x02, 222)],
                    ..Default::default()
                }),
                ..Default::default()
            },
        )
        .unwrap();

    let a_used: Vec<String> = used_set(&persister, &a)
        .iter()
        .map(|x| x.to_string())
        .collect();
    let b_used: Vec<String> = used_set(&persister, &b)
        .iter()
        .map(|x| x.to_string())
        .collect();
    assert_eq!(
        a_used,
        vec![addr_a.to_string()],
        "A sees only its own address"
    );
    assert_eq!(
        b_used,
        vec![addr_b.to_string()],
        "B sees only its own address"
    );
    assert!(
        !a_used.contains(&addr_b.to_string()),
        "A must not see B's address"
    );
    assert!(
        !b_used.contains(&addr_a.to_string()),
        "B must not see A's address"
    );
}

/// Mixed-store regression — a historical `core_utxos` address that
/// a later partial pool snapshot never enumerates must surface BOTH the
/// historical UTXO address and the pool used address. The union must never
/// let the pool set shadow the historical one (address-reuse / funds safety).
#[test]
fn mixed_store_unions_utxo_and_pool_used_sets() {
    let (persister, _tmp, _path) = fresh_persister();
    let w: WalletId = wid(0x27);
    ensure_wallet_meta(&persister, &w);

    // Historical UTXO on address X, written before any pool snapshot exists.
    let historical = p2pkh(0xAA);
    persister
        .store(
            w,
            PlatformWalletChangeSet {
                core: Some(CoreChangeSet {
                    new_utxos: vec![utxo_on(&historical, 0x12, 500)],
                    ..Default::default()
                }),
                ..Default::default()
            },
        )
        .unwrap();

    // A later pool snapshot marks a DIFFERENT address Y used and does not
    // enumerate the historical address at all.
    let mut infos = external_infos(0x27);
    infos.truncate(1);
    infos[0].state = AddressState::Used;
    let pool_used = infos[0].address.clone();
    persister
        .store(
            w,
            PlatformWalletChangeSet {
                account_address_pools: vec![AccountAddressPoolEntry {
                    account_type: AccountType::Standard {
                        index: 0,
                        standard_account_type: StandardAccountType::BIP44Account,
                    },
                    pool_type: AddressPoolType::External,
                    addresses: infos.clone(),
                }],
                ..Default::default()
            },
        )
        .unwrap();

    let got: std::collections::BTreeSet<String> = used_set(&persister, &w)
        .iter()
        .map(|a| a.to_string())
        .collect();
    assert!(
        got.contains(&historical.to_string()),
        "historical UTXO address must survive a later partial pool snapshot"
    );
    assert!(
        got.contains(&pool_used.to_string()),
        "pool used address must be present"
    );
    assert_eq!(got.len(), 2, "exactly the union of both sources, deduped");
}
