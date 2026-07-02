#![allow(clippy::field_reassign_with_default)]

//! Verbatim pool-snapshot reader (WS-B task B6). Covers TC-B-020 (used-set
//! comes from `core_address_pool`, not `core_utxos` re-derivation), TC-B-023
//! (deep-derivation window — no horizon-walk truncation), TC-B-025/007
//! (empty wallet loads empty-but-valid), plus the pre-pool fallback.

mod common;

use common::{ensure_wallet_meta, fresh_persister, wid};
use dashcore::address::Payload;
use dashcore::hashes::Hash;
use dashcore::{Address, Network, PubkeyHash};
use key_wallet::account::{AccountType, StandardAccountType};
use key_wallet::managed_account::address_pool::AddressPoolType;
use key_wallet::wallet::initialization::WalletAccountCreationOptions;
use key_wallet::wallet::managed_wallet_info::ManagedWalletInfo;
use key_wallet::wallet::Wallet;
use key_wallet::{AddressInfo, Utxo};
use platform_wallet::changeset::{
    AccountAddressPoolEntry, CoreChangeSet, PlatformWalletChangeSet, PlatformWalletPersistence,
};
use platform_wallet::wallet::platform_wallet::WalletId;

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
        info.used = used_indices.contains(&info.index);
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

    let state = persister.load().unwrap();
    let slice = state.wallets.get(&w).expect("wallet surfaces in load");
    let got: std::collections::BTreeSet<String> = slice
        .used_core_addresses
        .iter()
        .map(|a| a.to_string())
        .collect();
    let expected: std::collections::BTreeSet<String> = infos
        .iter()
        .filter(|i| used_indices.contains(&i.index))
        .map(|i| i.address.to_string())
        .collect();
    assert_eq!(got, expected, "used-set must equal the pool's used=1 rows");
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
                    (wallet_id, account_index, key_class, pool_type, address_index, script, used) \
                 VALUES (?1, 0, 0, 0, ?2, ?3, ?4)",
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

    let state = persister.load().unwrap();
    let slice = state.wallets.get(&w).expect("wallet surfaces");
    assert_eq!(
        slice.used_core_addresses.len(),
        46,
        "indices 0..=45 are used and must all restore"
    );
    let want = p2pkh(45).to_string();
    assert!(
        slice
            .used_core_addresses
            .iter()
            .any(|a| a.to_string() == want),
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

    let state = persister.load().unwrap();
    let slice = state
        .wallets
        .get(&w)
        .expect("empty wallet must still surface");
    assert!(
        slice.used_core_addresses.is_empty(),
        "empty wallet has an empty used-set"
    );
}

/// Fallback — a pre-pool store (UTXOs, no `core_address_pool` rows) still
/// yields the reuse-guard set from the `core_utxos`-derived path.
#[test]
fn pre_pool_store_falls_back_to_utxo_derived_used_set() {
    let (persister, _tmp, _path) = fresh_persister();
    let w: WalletId = wid(0x26);
    ensure_wallet_meta(&persister, &w);

    let addr = p2pkh(0x99);
    let utxo = Utxo::new(
        dashcore::OutPoint::new(dashcore::Txid::from_byte_array([0x11; 32]), 0),
        dashcore::TxOut {
            value: 1000,
            script_pubkey: addr.script_pubkey(),
        },
        addr.clone(),
        10,
        false,
    );
    persister
        .store(
            w,
            PlatformWalletChangeSet {
                core: Some(CoreChangeSet {
                    new_utxos: vec![utxo],
                    ..Default::default()
                }),
                ..Default::default()
            },
        )
        .unwrap();

    let state = persister.load().unwrap();
    let slice = state.wallets.get(&w).expect("wallet surfaces");
    assert_eq!(
        slice.used_core_addresses.len(),
        1,
        "no pool rows → fall back to the UTXO-derived used-set"
    );
    assert_eq!(slice.used_core_addresses[0].to_string(), addr.to_string());
}
