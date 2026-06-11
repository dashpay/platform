#![allow(clippy::field_reassign_with_default)]

//! Marvin's verification-gate scratch tests (PR #3828 Phase-2 gate).
//!
//! These prove the single load-bearing claim the retirement plan rests on:
//! a UTXO landing on a freshly-derived address resolves to the correct
//! account_index when the pool snapshot rides the SAME `PlatformWalletChangeSet`
//! as the UTXO — purely because `apply_changeset_to_tx` applies
//! `account_address_pools` (persister.rs:1073) BEFORE the core UTXO delta
//! (:1077) inside one transaction. If this holds, the manifest is a
//! sufficient resolution source and the eager-mirror is redundant.

mod common;

use common::{ensure_wallet_meta, fresh_persister, fresh_persister_with_mode, wid};

use key_wallet::wallet::initialization::WalletAccountCreationOptions;
use key_wallet::wallet::managed_wallet_info::ManagedWalletInfo;
use key_wallet::wallet::Wallet;
use key_wallet::AddressInfo;
use platform_wallet::changeset::{
    AccountAddressPoolEntry, CoreChangeSet, PlatformWalletChangeSet, PlatformWalletPersistence,
};
use platform_wallet::wallet::platform_wallet::WalletId;
use platform_wallet_storage::sqlite::schema::core_state;
use platform_wallet_storage::FlushMode;

/// Snapshot the wallet's Standard BIP44 external pool plus the expected
/// `account_index` (0 for BIP44 index-0) and the LAST address in the pool —
/// the gap-limit-edge address, the one most likely to be a fresh extension.
fn pool_and_edge_address(seed_byte: u8) -> (AccountAddressPoolEntry, u32, AddressInfo) {
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
            let edge = infos.last().cloned().unwrap();
            let account_index = account_index_of(&account_type);
            return (
                AccountAddressPoolEntry {
                    account_type,
                    pool_type: pool.pool_type,
                    addresses: infos,
                },
                account_index,
                edge,
            );
        }
    }
    panic!("wallet must expose a non-empty Standard BIP44 external pool");
}

/// Mirror the persister's own `account_index` derivation so the test asserts
/// against the SAME value the writer computes, not a hand-picked constant.
fn account_index_of(account_type: &key_wallet::account::AccountType) -> u32 {
    use key_wallet::account::AccountType;
    match account_type {
        AccountType::Standard { index, .. } | AccountType::CoinJoin { index } => *index,
        _ => 0,
    }
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

/// THE GATE: a single changeset carrying the pool snapshot AND a UTXO on the
/// gap-limit-edge address resolves the UTXO to the correct account_index. The
/// pool snapshot is the ONLY thing that makes the address resolvable in this
/// changeset — there is no prior `addresses_derived` event and no prior round.
/// This is the gap-limit race (design §2.3) closed in-band.
#[test]
fn in_band_pool_snapshot_resolves_utxo_on_fresh_address_same_changeset() {
    let (persister, _tmp, _path) = fresh_persister();
    let w: WalletId = wid(0xD1);
    ensure_wallet_meta(&persister, &w);

    let (pool, expected_index, edge) = pool_and_edge_address(0x55);
    let addr = edge.address.clone();

    // ONE changeset: snapshot + UTXO on the edge address, exactly as the
    // Phase-1 emitter ships them (build_platform_changeset attaches the pool
    // to the same PlatformWalletChangeSet as the core delta).
    persister
        .store(
            w,
            PlatformWalletChangeSet {
                account_address_pools: vec![pool],
                core: Some(CoreChangeSet {
                    new_utxos: vec![utxo_at(&addr, 0, 777_000)],
                    ..Default::default()
                }),
                ..Default::default()
            },
        )
        .expect("in-band snapshot+UTXO changeset must persist");

    let conn = persister.lock_conn_for_test();
    let by_account = core_state::list_unspent_utxos(&conn, &w).unwrap();

    let total: usize = by_account.values().map(|v| v.len()).sum();
    assert_eq!(total, 1, "exactly the one edge-address UTXO must persist");

    // The load-bearing assertion: the UTXO is attributed to the RIGHT account,
    // not the spent-only `0` placeholder and not dropped.
    let rows = by_account
        .get(&expected_index)
        .unwrap_or_else(|| panic!("UTXO must resolve to account_index {expected_index}"));
    assert_eq!(
        rows.len(),
        1,
        "the edge-address UTXO under the right account"
    );
    assert_eq!(rows[0].value, 777_000, "value preserved");
}

/// Adversarial buffering: the snapshot and the UTXO arrive in TWO separate
/// `store` calls (Manual flush mode batches them), forcing the buffer's
/// `Merge` to combine them before a single flush. The merged changeset must
/// STILL apply pools before core — so the UTXO resolves. This proves the
/// in-band guarantee survives the merge path, not just the single-store path.
#[test]
fn merged_buffer_preserves_pool_before_core_ordering() {
    let (persister, _tmp, _path) = fresh_persister_with_mode(FlushMode::Manual);
    let w: WalletId = wid(0xD2);
    ensure_wallet_meta(&persister, &w);

    let (pool, expected_index, edge) = pool_and_edge_address(0x66);
    let addr = edge.address.clone();

    // Store the UTXO-bearing changeset FIRST, snapshot SECOND — the adversarial
    // arrival order. Merge must still order pools-before-core at apply time.
    persister
        .store(
            w,
            PlatformWalletChangeSet {
                core: Some(CoreChangeSet {
                    new_utxos: vec![utxo_at(&addr, 0, 888_000)],
                    ..Default::default()
                }),
                ..Default::default()
            },
        )
        .unwrap();
    persister
        .store(
            w,
            PlatformWalletChangeSet {
                account_address_pools: vec![pool],
                ..Default::default()
            },
        )
        .unwrap();

    persister.flush(w).expect("merged flush must commit");

    let conn = persister.lock_conn_for_test();
    let by_account = core_state::list_unspent_utxos(&conn, &w).unwrap();
    let rows = by_account.get(&expected_index).unwrap_or_else(|| {
        panic!("merged changeset must resolve the UTXO to account_index {expected_index}")
    });
    assert_eq!(rows.len(), 1, "edge-address UTXO resolved after merge");
    assert_eq!(rows[0].value, 888_000);
}

/// Negative control: a UTXO on a genuinely-undeclared address (NOT in any
/// snapshot) is skipped non-fatally — it does NOT abort the flush and does NOT
/// appear in the unspent set. This is the benign SPV-gap branch the guard
/// relaxation relies on (design §3.3 step 3, non-fatal skip).
#[test]
fn utxo_on_undeclared_address_skips_non_fatally() {
    use dashcore::address::Payload;
    use dashcore::hashes::Hash;
    use dashcore::PubkeyHash;

    let (persister, _tmp, _path) = fresh_persister();
    let w: WalletId = wid(0xD3);
    ensure_wallet_meta(&persister, &w);

    // An address the wallet never declared — a fixed hash160 not in any pool.
    let undeclared = dashcore::Address::new(
        key_wallet::Network::Testnet,
        Payload::PubkeyHash(PubkeyHash::from_byte_array([0xEE; 20])),
    );

    persister
        .store(
            w,
            PlatformWalletChangeSet {
                core: Some(CoreChangeSet {
                    new_utxos: vec![utxo_at(&undeclared, 0, 123_000)],
                    ..Default::default()
                }),
                ..Default::default()
            },
        )
        .expect("an undeclared-address UTXO must skip, not abort the flush");

    let conn = persister.lock_conn_for_test();
    let by_account = core_state::list_unspent_utxos(&conn, &w).unwrap();
    let total: usize = by_account.values().map(|v| v.len()).sum();
    assert_eq!(
        total, 0,
        "the undeclared-address UTXO is skipped, not persisted"
    );
}
