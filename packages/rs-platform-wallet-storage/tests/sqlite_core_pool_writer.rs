#![allow(clippy::field_reassign_with_default)]

//! `core_address_pool` writer + `core_utxos.account_index` attribution
//! (WS-B task B3). Covers TC-B-001 (pool rows with `used` flags), TC-B-002
//! (real account_index, not the retired `=0` constant), TC-B-010 (idempotent
//! per-changeset pool state), TC-B-015 (`key_class` survives).

mod common;

use common::{ensure_wallet_meta, fresh_persister, wid};
use key_wallet::account::{AccountType, StandardAccountType};
use key_wallet::managed_account::address_pool::AddressPoolType;
use key_wallet::wallet::initialization::WalletAccountCreationOptions;
use key_wallet::wallet::managed_wallet_info::ManagedWalletInfo;
use key_wallet::wallet::Wallet;
use key_wallet::{AddressInfo, Network, Utxo};
use platform_wallet::changeset::{
    AccountAddressPoolEntry, CoreChangeSet, PlatformWalletChangeSet, PlatformWalletPersistence,
};
use platform_wallet::wallet::platform_wallet::WalletId;

/// Real external-pool `AddressInfo`s for a wallet's Standard BIP44 account 0,
/// sorted by derivation index — genuine scripts that round-trip.
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
            if pool.pool_type != AddressPoolType::External || pool.addresses.is_empty() {
                continue;
            }
            let mut infos: Vec<AddressInfo> = pool.addresses.values().cloned().collect();
            infos.sort_by_key(|a| a.index);
            return infos;
        }
    }
    panic!("wallet must expose a non-empty Standard BIP44 external pool");
}

fn utxo_on(info: &AddressInfo, value: u64) -> Utxo {
    use dashcore::hashes::Hash;
    Utxo {
        outpoint: dashcore::OutPoint {
            txid: dashcore::Txid::from_byte_array([info.index as u8 ^ 0x5A; 32]),
            vout: 0,
        },
        txout: dashcore::TxOut {
            value,
            script_pubkey: info.script_pubkey.clone(),
        },
        address: info.address.clone(),
        height: 10,
        is_coinbase: false,
        is_confirmed: true,
        is_instantlocked: false,
        is_locked: false,
        is_trusted: false,
    }
}

fn pool_entry(
    account_type: AccountType,
    pool_type: AddressPoolType,
    addresses: Vec<AddressInfo>,
) -> AccountAddressPoolEntry {
    AccountAddressPoolEntry {
        account_type,
        pool_type,
        addresses,
    }
}

/// TC-B-001 — six pool rows with `used` set on indices {0,2,4}; the pool
/// table is a first-class row store, not a `core_utxos` derivation.
#[test]
fn tc_b_001_pool_rows_with_used_flags() {
    let (persister, _tmp, _path) = fresh_persister();
    let w: WalletId = wid(0xA0);
    ensure_wallet_meta(&persister, &w);

    let mut infos = external_infos(0x11);
    infos.truncate(6);
    assert_eq!(infos.len(), 6, "need at least six derived addresses");
    for info in infos.iter_mut() {
        info.used = matches!(info.index, 0 | 2 | 4);
    }
    let entry = pool_entry(
        AccountType::Standard {
            index: 0,
            standard_account_type: StandardAccountType::BIP44Account,
        },
        AddressPoolType::External,
        infos.clone(),
    );
    persister
        .store(
            w,
            PlatformWalletChangeSet {
                account_address_pools: vec![entry],
                ..Default::default()
            },
        )
        .unwrap();

    let conn = persister.lock_conn_for_test();
    let count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM core_address_pool \
             WHERE wallet_id = ?1 AND account_index = 0 AND key_class = 0 AND pool_type = 0",
            rusqlite::params![w.as_slice()],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(count, 6, "exactly six scoped rows");

    for info in &infos {
        let used: i64 = conn
            .query_row(
                "SELECT used FROM core_address_pool \
                 WHERE wallet_id = ?1 AND account_index = 0 AND key_class = 0 \
                   AND pool_type = 0 AND address_index = ?2",
                rusqlite::params![w.as_slice(), i64::from(info.index)],
                |r| r.get(0),
            )
            .unwrap();
        let expect = i64::from(matches!(info.index, 0 | 2 | 4));
        assert_eq!(used, expect, "used flag for index {}", info.index);
    }
}

/// TC-B-002 — a UTXO whose owning account is index 1 stores
/// `account_index = 1`, not the retired hardcoded 0.
#[test]
fn tc_b_002_account_index_is_real_not_zero() {
    let (persister, _tmp, _path) = fresh_persister();
    let w: WalletId = wid(0xA2);
    ensure_wallet_meta(&persister, &w);

    let infos = external_infos(0x22);
    let addr0 = infos[0].clone();
    let addr1 = infos[1].clone();

    // Pools declaring the address' owning account: addr0 -> account 0,
    // addr1 -> account 1 (non-default).
    let pools = vec![
        pool_entry(
            AccountType::Standard {
                index: 0,
                standard_account_type: StandardAccountType::BIP44Account,
            },
            AddressPoolType::External,
            vec![addr0.clone()],
        ),
        pool_entry(
            AccountType::Standard {
                index: 1,
                standard_account_type: StandardAccountType::BIP44Account,
            },
            AddressPoolType::External,
            vec![addr1.clone()],
        ),
    ];
    persister
        .store(
            w,
            PlatformWalletChangeSet {
                account_address_pools: pools,
                core: Some(CoreChangeSet {
                    new_utxos: vec![utxo_on(&addr0, 111), utxo_on(&addr1, 222)],
                    ..Default::default()
                }),
                ..Default::default()
            },
        )
        .unwrap();

    let conn = persister.lock_conn_for_test();
    let account_for = |script: &[u8]| -> i64 {
        conn.query_row(
            "SELECT account_index FROM core_utxos WHERE wallet_id = ?1 AND script = ?2",
            rusqlite::params![w.as_slice(), script],
            |r| r.get(0),
        )
        .unwrap()
    };
    assert_eq!(
        account_for(addr1.script_pubkey.as_bytes()),
        1,
        "UTXO on account 1's address must store account_index = 1"
    );
    assert_eq!(
        account_for(addr0.script_pubkey.as_bytes()),
        0,
        "UTXO on account 0's address must store account_index = 0"
    );
}

/// A UTXO whose script matches no pool row falls back to account 0 — the
/// one-way historical-attribution default (R7), funds never dropped.
#[test]
fn utxo_without_pool_row_defaults_to_account_zero() {
    let (persister, _tmp, _path) = fresh_persister();
    let w: WalletId = wid(0xA3);
    ensure_wallet_meta(&persister, &w);

    let infos = external_infos(0x33);
    persister
        .store(
            w,
            PlatformWalletChangeSet {
                core: Some(CoreChangeSet {
                    new_utxos: vec![utxo_on(&infos[0], 500)],
                    ..Default::default()
                }),
                ..Default::default()
            },
        )
        .unwrap();

    let conn = persister.lock_conn_for_test();
    let account: i64 = conn
        .query_row(
            "SELECT account_index FROM core_utxos WHERE wallet_id = ?1",
            rusqlite::params![w.as_slice()],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(account, 0, "unattributed UTXO defaults to account 0");
}

/// TC-B-010 — a used-flag flip persists and a second no-op flush leaves the
/// pool rows unchanged; `used` is monotonic and never reverts.
#[test]
fn tc_b_010_pool_state_idempotent_and_monotonic() {
    let (persister, _tmp, _path) = fresh_persister();
    let w: WalletId = wid(0xA4);
    ensure_wallet_meta(&persister, &w);

    let mut infos = external_infos(0x44);
    infos.truncate(3);
    let mk = |infos: &[AddressInfo]| {
        pool_entry(
            AccountType::Standard {
                index: 0,
                standard_account_type: StandardAccountType::BIP44Account,
            },
            AddressPoolType::External,
            infos.to_vec(),
        )
    };
    persister
        .store(
            w,
            PlatformWalletChangeSet {
                account_address_pools: vec![mk(&infos)],
                ..Default::default()
            },
        )
        .unwrap();

    // Flip index 1 to used.
    let mut flipped = infos.clone();
    flipped[1].used = true;
    persister
        .store(
            w,
            PlatformWalletChangeSet {
                account_address_pools: vec![mk(&flipped)],
                ..Default::default()
            },
        )
        .unwrap();

    let used_of = |conn: &rusqlite::Connection, idx: u32| -> i64 {
        conn.query_row(
            "SELECT used FROM core_address_pool \
             WHERE wallet_id = ?1 AND account_index = 0 AND pool_type = 0 AND address_index = ?2",
            rusqlite::params![w.as_slice(), i64::from(idx)],
            |r| r.get(0),
        )
        .unwrap()
    };
    {
        let conn = persister.lock_conn_for_test();
        assert_eq!(used_of(&conn, 1), 1, "flip must persist");
        assert_eq!(used_of(&conn, 0), 0, "unrelated row unchanged");
    }

    // A stale snapshot with used=false for index 1 must NOT un-use it.
    persister
        .store(
            w,
            PlatformWalletChangeSet {
                account_address_pools: vec![mk(&infos)],
                ..Default::default()
            },
        )
        .unwrap();
    let conn = persister.lock_conn_for_test();
    assert_eq!(
        used_of(&conn, 1),
        1,
        "used is monotonic — a stale snapshot never reverts it"
    );
}

/// TC-B-015 — a non-default `key_class` round-trips into the pool row's PK.
#[test]
fn tc_b_015_key_class_survives() {
    let (persister, _tmp, _path) = fresh_persister();
    let w: WalletId = wid(0xA5);
    ensure_wallet_meta(&persister, &w);

    let infos = external_infos(0x55);
    let entry = pool_entry(
        AccountType::PlatformPayment {
            account: 2,
            key_class: 1,
        },
        AddressPoolType::External,
        vec![infos[0].clone()],
    );
    persister
        .store(
            w,
            PlatformWalletChangeSet {
                account_address_pools: vec![entry],
                ..Default::default()
            },
        )
        .unwrap();

    let conn = persister.lock_conn_for_test();
    let (account_index, key_class): (i64, i64) = conn
        .query_row(
            "SELECT account_index, key_class FROM core_address_pool WHERE wallet_id = ?1",
            rusqlite::params![w.as_slice()],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .unwrap();
    assert_eq!(account_index, 2, "PlatformPayment account index");
    assert_eq!(key_class, 1, "non-default key_class must survive");
}
