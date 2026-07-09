#![allow(clippy::field_reassign_with_default)]

//! Genesis-rescan regression for the account-attribution fallback default
//! (PR #3828).
//!
//! Before: a UTXO landing on a freshly-derived gap-limit-edge address could
//! race address-derivation persistence and be mis-attributed or dropped, so
//! the bridge smuggled a full pool snapshot in-band to resolve it. Now the
//! storage writer resolves each UTXO's owning account by matching its script
//! against the `core_address_pool` lookup table, falling back to account
//! index 0 when no pool row covers the script — no in-band snapshot. A fresh
//! gap-limit-edge address has no pool row yet, so this test pins that such a
//! UTXO persists directly under the `account_index == 0` fallback, contributes
//! the exact balance, and never aborts the flush.

mod common;

use common::{ensure_wallet_meta, fresh_persister, wid};

use key_wallet::wallet::initialization::WalletAccountCreationOptions;
use key_wallet::wallet::managed_wallet_info::ManagedWalletInfo;
use key_wallet::wallet::Wallet;
use key_wallet::AddressInfo;
use platform_wallet::changeset::{
    CoreChangeSet, PlatformWalletChangeSet, PlatformWalletPersistence,
};
use platform_wallet::wallet::platform_wallet::WalletId;
use platform_wallet_storage::sqlite::schema::core_state;

/// The LAST address in the wallet's Standard BIP44 external pool — the
/// gap-limit-edge address, the one most likely to be a fresh extension and
/// thus the worst case for the retired attribution race.
fn gap_limit_edge_address(seed_byte: u8) -> AddressInfo {
    use key_wallet::account::AccountType;
    use key_wallet::managed_account::address_pool::AddressPoolType;

    let wallet = Wallet::from_seed_bytes(
        [seed_byte; 64],
        key_wallet::Network::Testnet,
        WalletAccountCreationOptions::Default,
    )
    .unwrap();
    let info = ManagedWalletInfo::from_wallet(&wallet, 0);

    for managed in info.all_managed_accounts() {
        let account_type = managed.managed_account_type().to_account_type();
        if !matches!(account_type, AccountType::Standard { index: 0, .. }) {
            continue;
        }
        for pool in managed.managed_account_type().address_pools() {
            if pool.pool_type != AddressPoolType::External || pool.addresses.is_empty() {
                continue;
            }
            let infos: Vec<AddressInfo> = pool.addresses.values().cloned().collect();
            return infos.last().cloned().unwrap();
        }
    }
    panic!("wallet must expose a non-empty Standard BIP44 external pool");
}

fn utxo_at(addr: &dashcore::Address, vout: u32, value: u64) -> key_wallet::Utxo {
    use dashcore::hashes::Hash;
    key_wallet::Utxo {
        outpoint: dashcore::OutPoint {
            txid: dashcore::Txid::from_byte_array([0x7E; 32]),
            vout,
        },
        txout: dashcore::TxOut {
            value,
            script_pubkey: addr.script_pubkey(),
        },
        address: addr.clone(),
        height: 7,
        is_coinbase: false,
        is_confirmed: true,
        is_instantlocked: false,
        is_locked: false,
        is_trusted: false,
    }
}

/// A UTXO on a freshly-derived gap-limit-edge address (no `core_address_pool`
/// row) persists directly under the `account_index == 0` fallback: no snapshot,
/// no flush abort, and the unspent balance is exact.
#[test]
fn utxo_on_fresh_gap_limit_address_persists_under_account_zero() {
    let (persister, _tmp, _path) = fresh_persister();
    let w: WalletId = wid(0xD1);
    ensure_wallet_meta(&persister, &w);

    let edge = gap_limit_edge_address(0x55);
    let addr = edge.address.clone();

    persister
        .store(
            w,
            PlatformWalletChangeSet {
                core: Some(CoreChangeSet {
                    new_utxos: vec![utxo_at(&addr, 0, 777_000)],
                    ..Default::default()
                }),
                ..Default::default()
            },
        )
        .expect("a UTXO on a fresh gap-limit address must persist, not abort");

    let conn = persister.lock_conn_for_test();
    let by_account = core_state::list_unspent_utxos(&conn, &w).unwrap();

    let total: usize = by_account.values().map(|v| v.len()).sum();
    assert_eq!(total, 1, "the edge-address UTXO must persist");

    let rows = by_account
        .get(&0)
        .expect("UTXO must be attributed to the default account (index 0)");
    assert_eq!(
        rows.len(),
        1,
        "exactly the one edge-address UTXO under account 0"
    );
    assert_eq!(rows[0].value, 777_000, "value preserved");
}
