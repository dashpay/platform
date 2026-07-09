#![allow(clippy::field_reassign_with_default)]

//! `schema::core_state::load_state` bulk-reconstructs the keyless
//! `CoreChangeSet` (UTXOs, records, IS-locks, sync watermarks), and the
//! no-silent-zero balance contract holds end-to-end.

mod common;

use common::{ensure_wallet_meta, fresh_persister, wid};
use dashcore::hashes::Hash;
use dashcore::{OutPoint, Txid};
use key_wallet::wallet::initialization::WalletAccountCreationOptions;
use key_wallet::wallet::managed_wallet_info::wallet_info_interface::WalletInfoInterface;
use key_wallet::wallet::managed_wallet_info::ManagedWalletInfo;
use key_wallet::wallet::Wallet;
use key_wallet::Utxo;
#[cfg(feature = "rehydration-apply")]
use platform_wallet::changeset::AccountRegistrationEntry;
use platform_wallet::changeset::{
    CoreChangeSet, PlatformWalletChangeSet, PlatformWalletPersistence,
};
use platform_wallet_storage::sqlite::schema::core_state;
use platform_wallet_storage::WalletStorageError;

/// Keyless account manifest the rehydration path resolves xpubs from.
#[cfg(feature = "rehydration-apply")]
fn manifest_for(wallet: &Wallet) -> Vec<AccountRegistrationEntry> {
    wallet
        .accounts
        .all_accounts()
        .into_iter()
        .map(|a| AccountRegistrationEntry {
            account_type: a.account_type,
            account_xpub: a.account_xpub,
        })
        .collect()
}

fn reopen(path: &std::path::Path) -> platform_wallet_storage::SqlitePersister {
    platform_wallet_storage::SqlitePersister::open(
        platform_wallet_storage::SqlitePersisterConfig::new(path),
    )
    .expect("reopen persister")
}

/// Build a wallet + a UTXO paying one of its BIP44 addresses, value
/// `value`, confirmed at `height`.
fn wallet_and_utxo(seed: [u8; 64], value: u64, height: u32, vout: u32) -> (Wallet, Utxo) {
    let w = Wallet::from_seed_bytes(
        seed,
        key_wallet::Network::Testnet,
        WalletAccountCreationOptions::Default,
    )
    .unwrap();
    let info = ManagedWalletInfo::from_wallet(&w, 1);
    // Any monitored address of the wallet — what a real UTXO would pay.
    let address = WalletInfoInterface::monitored_addresses(&info)
        .into_iter()
        .next()
        .expect("at least one monitored address");
    let script = address.script_pubkey();
    let utxo = Utxo {
        outpoint: OutPoint {
            txid: Txid::from_byte_array([0x55; 32]),
            vout,
        },
        txout: dashcore::TxOut {
            value,
            script_pubkey: script,
        },
        address,
        height,
        is_coinbase: false,
        is_confirmed: true,
        is_instantlocked: false,
        is_locked: false,
        is_trusted: false,
    };
    (w, utxo)
}

/// A non-zero balance survives store → drop → reopen → load, guarding
/// against a silent-zero-balance reconstruction.
#[test]
fn rt2_nonzero_balance_survives_reopen() {
    let (persister, _tmp, path) = fresh_persister();
    let w = wid(0xB1);
    ensure_wallet_meta(&persister, &w);

    let seed = [0x42; 64];
    let (wallet, utxo) = wallet_and_utxo(seed, 1_234_500, 100, 0);

    let cs = PlatformWalletChangeSet {
        core: Some(CoreChangeSet {
            new_utxos: vec![utxo.clone()],
            last_processed_height: Some(200),
            synced_height: Some(200),
            ..Default::default()
        }),
        ..Default::default()
    };
    persister.store(w, cs).unwrap();
    drop(persister);

    let p2 = reopen(&path);
    let conn = p2.lock_conn_for_test();
    #[cfg_attr(not(feature = "rehydration-apply"), allow(unused_variables))]
    let (core, utxo_accounts) =
        core_state::load_state(&conn, &w, key_wallet::Network::Testnet).expect("load_state");
    drop(conn);

    // The persisted UTXO round-trips by outpoint + value.
    assert_eq!(core.new_utxos.len(), 1);
    assert_eq!(core.new_utxos[0].outpoint, utxo.outpoint);
    assert_eq!(core.new_utxos[0].value(), 1_234_500);
    assert_eq!(core.last_processed_height, Some(200));
    assert_eq!(core.synced_height, Some(200));

    // End-to-end: apply the loaded state onto a freshly minted skeleton and
    // assert the wallet balance is the persisted amount — NOT a silent zero.
    // The apply leg drives `apply_persisted_core_state`, gated behind
    // `rehydration-apply`; the storage `load_state` assertions above run
    // standalone regardless.
    #[cfg(feature = "rehydration-apply")]
    {
        let mut info = ManagedWalletInfo::from_wallet(&wallet, 1);
        platform_wallet_storage::sqlite::util::apply_persisted_core_state(
            &mut info,
            &manifest_for(&wallet),
            &core,
            &utxo_accounts,
            &Default::default(),
        )
        .expect("BIP44 reconstruction must not error");
        let bal = WalletInfoInterface::balance(&info);
        let total = bal.confirmed() + bal.unconfirmed() + bal.immature() + bal.locked();
        assert_eq!(
            total, 1_234_500,
            "reconstructed wallet balance must be exact"
        );
        assert!(total > 0, "silent zero balance is a FAIL");
        // Height-bearing UTXO lands in the confirmed bucket.
        assert_eq!(bal.confirmed(), 1_234_500);
    }
    // `wallet` only feeds the gated manager-apply leg above.
    #[cfg(not(feature = "rehydration-apply"))]
    let _ = &wallet;
}

/// Spent UTXOs are excluded from the reconstructed feed.
#[test]
fn b2_spent_utxo_excluded() {
    let (persister, _tmp, path) = fresh_persister();
    let w = wid(0xB2);
    ensure_wallet_meta(&persister, &w);
    let seed = [0x07; 64];
    let (_w, u_unspent) = wallet_and_utxo(seed, 1000, 10, 0);
    let (_w2, u_spent) = wallet_and_utxo(seed, 9999, 10, 1);

    persister
        .store(
            w,
            PlatformWalletChangeSet {
                core: Some(CoreChangeSet {
                    new_utxos: vec![u_unspent.clone()],
                    spent_utxos: vec![u_spent.clone()],
                    ..Default::default()
                }),
                ..Default::default()
            },
        )
        .unwrap();
    drop(persister);
    let p2 = reopen(&path);
    let conn = p2.lock_conn_for_test();
    let (core, _utxo_accounts) =
        core_state::load_state(&conn, &w, key_wallet::Network::Testnet).unwrap();
    drop(conn);
    let ops: Vec<_> = core.new_utxos.iter().map(|u| u.outpoint).collect();
    assert!(ops.contains(&u_unspent.outpoint));
    assert!(
        !ops.contains(&u_spent.outpoint),
        "spent UTXO must not resurrect on reload"
    );
}

/// A corrupt `record_blob` is a typed hard error.
#[test]
fn b3_corrupt_record_blob_is_hard_error() {
    let (persister, _tmp, path) = fresh_persister();
    let w = wid(0xB3);
    ensure_wallet_meta(&persister, &w);
    {
        let conn = persister.lock_conn_for_test();
        conn.execute(
            "INSERT INTO core_transactions \
                (wallet_id, txid, height, block_hash, block_time, finalized, record_blob) \
             VALUES (?1, ?2, NULL, NULL, NULL, 0, X'00')",
            rusqlite::params![w.as_slice(), &[0x11u8; 32][..]],
        )
        .unwrap();
    }
    drop(persister);
    let p2 = reopen(&path);
    let conn = p2.lock_conn_for_test();
    let result = core_state::load_state(&conn, &w, key_wallet::Network::Testnet);
    drop(conn);
    assert!(
        matches!(result, Err(WalletStorageError::BincodeDecode { .. })),
        "corrupt record_blob must be a typed BincodeDecode; got {result:?}"
    );
}

/// A CoinJoin-only wallet (no BIP44 account) with non-zero persisted
/// UTXOs reconstructs to the correct non-zero total, never a silent
/// `Ok` + 0.
#[test]
fn f2_no_bip44_wallet_nonzero_balance_survives_reopen() {
    use std::collections::BTreeSet;

    let (persister, _tmp, path) = fresh_persister();
    let w = wid(0xBF);
    ensure_wallet_meta(&persister, &w);

    // CoinJoin-only topology: empty BIP44/BIP32 sets, one CoinJoin
    // account, no special accounts.
    let mut coinjoin = BTreeSet::new();
    coinjoin.insert(0u32);
    let opts = WalletAccountCreationOptions::SpecificAccounts(
        BTreeSet::new(),
        BTreeSet::new(),
        coinjoin,
        BTreeSet::new(),
        BTreeSet::new(),
        None,
    );
    let seed = [0x4F; 64];
    let wallet = Wallet::from_seed_bytes(seed, key_wallet::Network::Testnet, opts).unwrap();
    assert!(
        wallet.accounts.standard_bip44_accounts.is_empty(),
        "fixture must be BIP44-free to exercise F2"
    );
    let info = ManagedWalletInfo::from_wallet(&wallet, 1);
    assert!(
        info.accounts.standard_bip44_accounts.is_empty()
            && !info.accounts.coinjoin_accounts.is_empty(),
        "managed info must be CoinJoin-only"
    );
    let address = WalletInfoInterface::monitored_addresses(&info)
        .into_iter()
        .next()
        .expect("CoinJoin-only wallet still has monitored addresses");

    let utxo = Utxo {
        outpoint: OutPoint {
            txid: Txid::from_byte_array([0x77; 32]),
            vout: 0,
        },
        txout: dashcore::TxOut {
            value: 9_000_000,
            script_pubkey: address.script_pubkey(),
        },
        address,
        height: 50,
        is_coinbase: false,
        is_confirmed: true,
        is_instantlocked: false,
        is_locked: false,
        is_trusted: false,
    };
    persister
        .store(
            w,
            PlatformWalletChangeSet {
                core: Some(CoreChangeSet {
                    new_utxos: vec![utxo.clone()],
                    last_processed_height: Some(60),
                    synced_height: Some(60),
                    ..Default::default()
                }),
                ..Default::default()
            },
        )
        .unwrap();
    drop(persister);

    let p2 = reopen(&path);
    let conn = p2.lock_conn_for_test();
    #[cfg_attr(not(feature = "rehydration-apply"), allow(unused_variables))]
    let (core, utxo_accounts) =
        core_state::load_state(&conn, &w, key_wallet::Network::Testnet).unwrap();
    drop(conn);
    assert_eq!(core.new_utxos.len(), 1);

    // Apply leg (`apply_persisted_core_state`) gated behind
    // `rehydration-apply`; the storage `load_state` assertions above run
    // standalone regardless.
    #[cfg(feature = "rehydration-apply")]
    {
        let mut info = ManagedWalletInfo::from_wallet(&wallet, 1);
        platform_wallet_storage::sqlite::util::apply_persisted_core_state(
            &mut info,
            &manifest_for(&wallet),
            &core,
            &utxo_accounts,
            &Default::default(),
        )
        .expect("CoinJoin-only reconstruction must not error");
        let bal = WalletInfoInterface::balance(&info);
        let total = bal.confirmed() + bal.unconfirmed() + bal.immature() + bal.locked();
        assert_eq!(
            total, 9_000_000,
            "CoinJoin-only wallet must reconstruct the exact non-zero total — \
             a silent zero is a FAIL"
        );
    }
}

/// Empty wallet → empty core state, no error.
#[test]
fn b4_empty_core_state_is_ok() {
    let (persister, _tmp, path) = fresh_persister();
    let w = wid(0xB4);
    ensure_wallet_meta(&persister, &w);
    drop(persister);
    let p2 = reopen(&path);
    let conn = p2.lock_conn_for_test();
    let (core, _utxo_accounts) =
        core_state::load_state(&conn, &w, key_wallet::Network::Testnet).unwrap();
    drop(conn);
    assert!(core.new_utxos.is_empty());
    assert!(core.records.is_empty());
    assert_eq!(core.last_processed_height, None);
}

/// `last_applied_chain_lock` persists through flush → reopen → `load_state`
/// and through the higher-level `PlatformWalletPersistence::load()` path.
///
/// Adversarial confirmation: the assertion at the end fails if the reader
/// `load_state` does NOT populate `cs.last_applied_chain_lock` (i.e. if
/// the old code path "left None" is still in place).
#[test]
fn b5_last_applied_chain_lock_round_trips() {
    use dashcore::ephemerealdata::chain_lock::ChainLock;
    use dashcore::hashes::Hash;
    use dashcore::BlockHash;

    let (persister, _tmp, path) = fresh_persister();
    let w = wid(0xB5);
    ensure_wallet_meta(&persister, &w);

    // Construct a deterministic ChainLock.
    let cl = ChainLock {
        block_height: 88_888,
        block_hash: BlockHash::from_byte_array([0xCAu8; 32]),
        signature: [0xBBu8; 96].into(),
    };

    // Persist via the normal store → flush path.
    let cs = PlatformWalletChangeSet {
        core: Some(CoreChangeSet {
            last_applied_chain_lock: Some(cl.clone()),
            synced_height: Some(88_888),
            ..Default::default()
        }),
        ..Default::default()
    };
    persister.store(w, cs).expect("store");
    PlatformWalletPersistence::flush(&persister, w).expect("flush");
    drop(persister);

    // Reopen and read via `core_state::load_state` directly.
    let p2 = reopen(&path);
    {
        let conn = p2.lock_conn_for_test();
        let (loaded, _utxo_accounts) =
            core_state::load_state(&conn, &w, key_wallet::Network::Testnet)
                .expect("load_state must succeed");
        assert_eq!(
            loaded.last_applied_chain_lock.as_ref(),
            Some(&cl),
            "core_state::load_state must populate last_applied_chain_lock from disk"
        );
        // Other fields carried by the same row must also survive.
        assert_eq!(loaded.synced_height, Some(88_888));
    }
    drop(p2);

    // Adversarial path: `PlatformWalletPersistence::load()` must also surface
    // the chain lock through the assembled `core_wallet_info` metadata.
    let p3 = reopen(&path);
    let start_state = PlatformWalletPersistence::load(&p3).expect("load must succeed");
    let wallet_start = start_state
        .wallets
        .get(&w)
        .expect("wallet must be in load output");
    assert_eq!(
        wallet_start
            .wallet_info
            .metadata
            .last_applied_chain_lock
            .as_ref(),
        Some(&cl),
        "PlatformWalletPersistence::load must carry last_applied_chain_lock \
         into the assembled core_wallet_info metadata"
    );
}

/// A lower-height chain lock arriving AFTER a higher one must not regress the
/// stored `last_applied_chain_lock`: heights monotonic-max merge just like the
/// sync watermarks, so an out-of-order update can't roll the finalized
/// checkpoint backwards.
#[test]
fn chain_lock_does_not_regress_on_lower_height_update() {
    use dashcore::ephemerealdata::chain_lock::ChainLock;
    use dashcore::BlockHash;

    let (persister, _tmp, path) = fresh_persister();
    let w = wid(0xB6);
    ensure_wallet_meta(&persister, &w);

    let high = ChainLock {
        block_height: 100_000,
        block_hash: BlockHash::from_byte_array([0xAAu8; 32]),
        signature: [0x11u8; 96].into(),
    };
    let low = ChainLock {
        block_height: 90_000,
        block_hash: BlockHash::from_byte_array([0xBBu8; 32]),
        signature: [0x22u8; 96].into(),
    };

    let store_cl = |cl: ChainLock| {
        let cs = PlatformWalletChangeSet {
            core: Some(CoreChangeSet {
                last_applied_chain_lock: Some(cl),
                ..Default::default()
            }),
            ..Default::default()
        };
        persister.store(w, cs).expect("store");
        PlatformWalletPersistence::flush(&persister, w).expect("flush");
    };
    store_cl(high.clone());
    store_cl(low); // out-of-order, lower height — must not win
    drop(persister);

    let p2 = reopen(&path);
    let conn = p2.lock_conn_for_test();
    let (loaded, _utxo_accounts) = core_state::load_state(&conn, &w, key_wallet::Network::Testnet)
        .expect("load_state must succeed");
    assert_eq!(
        loaded.last_applied_chain_lock.as_ref(),
        Some(&high),
        "a lower-height chain lock must not regress the stored higher one"
    );
}

/// First external `AddressInfo` of the account matching `pred` in `wallet`,
/// sorted by derivation index — a genuine script that round-trips.
#[cfg(feature = "rehydration-apply")]
fn first_external_info(
    wallet: &Wallet,
    pred: impl Fn(&key_wallet::account::AccountType) -> bool,
) -> key_wallet::AddressInfo {
    use key_wallet::managed_account::address_pool::AddressPoolType;
    let info = ManagedWalletInfo::from_wallet(wallet, 0);
    for managed in info.all_managed_accounts() {
        if !pred(&managed.managed_account_type().to_account_type()) {
            continue;
        }
        for pool in managed.managed_account_type().address_pools() {
            if pool.pool_type != AddressPoolType::External || pool.addresses.is_empty() {
                continue;
            }
            let mut infos: Vec<key_wallet::AddressInfo> =
                pool.addresses.values().cloned().collect();
            infos.sort_by_key(|a| a.index);
            return infos.into_iter().next().unwrap();
        }
    }
    panic!("wallet must expose the requested account with a non-empty external pool");
}

/// End-to-end regression (dashpay/platform#3968) exercising the REAL SQL
/// resolver. Persists a `Default` wallet through the actual writer with unspent
/// UTXOs owned by Standard BIP44[0] and CoinJoin[0] — colliding on numeric
/// index 0 — plus their `core_address_pool` snapshots, reopens the DB, then
/// drives `load_state` → `apply_persisted_core_state`. Unlike the hand-built
/// unit test, the owning-account side channel here is produced by
/// `owning_account_for_script` (column order, `ORDER BY` tie-break, `[u8;32]`
/// identity decode), so a broken query would be caught. Each UTXO must land in
/// its TRUE account with exact per-account balances, not just the wallet total.
#[cfg(feature = "rehydration-apply")]
#[test]
fn rehydration_routes_via_real_sql_resolver() {
    use key_wallet::account::{AccountType, StandardAccountType};
    use key_wallet::managed_account::address_pool::AddressPoolType;
    use platform_wallet::changeset::AccountAddressPoolEntry;

    let (persister, _tmp, path) = fresh_persister();
    let w = wid(0xC1);
    ensure_wallet_meta(&persister, &w);

    let wallet = Wallet::from_seed_bytes(
        [0x9A; 64],
        key_wallet::Network::Testnet,
        WalletAccountCreationOptions::Default,
    )
    .unwrap();

    let bip44_info = first_external_info(&wallet, |at| {
        matches!(
            at,
            AccountType::Standard {
                index: 0,
                standard_account_type: StandardAccountType::BIP44Account,
            }
        )
    });
    let coinjoin_info = first_external_info(&wallet, |at| {
        matches!(at, AccountType::CoinJoin { index: 0 })
    });
    assert_ne!(
        bip44_info.script_pubkey, coinjoin_info.script_pubkey,
        "BIP44[0] and CoinJoin[0] must derive distinct scripts"
    );

    let utxo_on = |info: &key_wallet::AddressInfo, value: u64, n: u8| Utxo {
        outpoint: OutPoint {
            txid: Txid::from_byte_array([n; 32]),
            vout: 0,
        },
        txout: dashcore::TxOut {
            value,
            script_pubkey: info.script_pubkey.clone(),
        },
        address: info.address.clone(),
        height: 5,
        is_coinbase: false,
        is_confirmed: true,
        is_instantlocked: false,
        is_locked: false,
        is_trusted: false,
    };
    let bip44_utxo = utxo_on(&bip44_info, 5_000, 1);
    let coinjoin_utxo = utxo_on(&coinjoin_info, 7_000, 2);
    let bip44_op = bip44_utxo.outpoint;
    let coinjoin_op = coinjoin_utxo.outpoint;

    let pool_entry = |account_type, info: &key_wallet::AddressInfo| AccountAddressPoolEntry {
        account_type,
        pool_type: AddressPoolType::External,
        addresses: vec![info.clone()],
    };
    persister
        .store(
            w,
            PlatformWalletChangeSet {
                account_address_pools: vec![
                    pool_entry(
                        AccountType::Standard {
                            index: 0,
                            standard_account_type: StandardAccountType::BIP44Account,
                        },
                        &bip44_info,
                    ),
                    pool_entry(AccountType::CoinJoin { index: 0 }, &coinjoin_info),
                ],
                core: Some(CoreChangeSet {
                    new_utxos: vec![bip44_utxo, coinjoin_utxo],
                    last_processed_height: Some(5),
                    synced_height: Some(5),
                    ..Default::default()
                }),
                ..Default::default()
            },
        )
        .expect("store must persist UTXOs + pool snapshots");
    drop(persister);

    let p2 = reopen(&path);
    let conn = p2.lock_conn_for_test();
    let (core, utxo_accounts) =
        core_state::load_state(&conn, &w, key_wallet::Network::Testnet).expect("load_state");
    drop(conn);

    // The real resolver populated the side channel from `core_address_pool`.
    assert_eq!(
        utxo_accounts.len(),
        2,
        "owning_account_for_script must resolve both UTXOs from the persisted pool"
    );

    let mut managed = ManagedWalletInfo::from_wallet(&wallet, 1);
    platform_wallet_storage::sqlite::util::apply_persisted_core_state(
        &mut managed,
        &manifest_for(&wallet),
        &core,
        &utxo_accounts,
        &Default::default(),
    )
    .expect("apply must not error");

    let bip44 = managed.accounts.standard_bip44_accounts.get(&0).unwrap();
    let coinjoin = managed.accounts.coinjoin_accounts.get(&0).unwrap();
    assert!(
        bip44.utxos.contains_key(&bip44_op),
        "BIP44 UTXO must route to the BIP44 account"
    );
    assert!(
        !bip44.utxos.contains_key(&coinjoin_op),
        "CoinJoin UTXO must NOT collapse onto the first (BIP44) account"
    );
    assert!(
        coinjoin.utxos.contains_key(&coinjoin_op),
        "CoinJoin UTXO must route to the CoinJoin account"
    );
    assert!(!coinjoin.utxos.contains_key(&bip44_op));
    assert_eq!(
        bip44.balance.total(),
        5_000,
        "per-account BIP44 balance exact"
    );
    assert_eq!(
        coinjoin.balance.total(),
        7_000,
        "per-account CoinJoin balance exact, not zero"
    );
    assert_eq!(managed.balance.total(), 12_000, "wallet total is the sum");
}

/// End-to-end regression (dashpay/platform#3968) for the address-reuse guard,
/// exercising the REAL SQL resolver. Persists a `Default` wallet with a *used*
/// address (via a `core_address_pool` snapshot with `used = true`) owned by
/// CoinJoin[0] — which is NOT the first funds account (Standard BIP44[0] is) —
/// with no unspent UTXO anchoring it. Reopens the DB, unions the two
/// used-address sources exactly as the persister does (so
/// `core_pool::load_used_addresses` carries the owner), then drives
/// `apply_persisted_core_state`. The used address must land `used` on the
/// CoinJoin pool specifically — never collapsed onto BIP44 — or it stays
/// "unused" on CoinJoin and could be re-issued as a fresh receive address.
#[cfg(feature = "rehydration-apply")]
#[test]
fn rehydration_routes_used_addresses_to_owning_account() {
    use key_wallet::account::{AccountType, StandardAccountType};
    use key_wallet::managed_account::address_pool::AddressPoolType;
    use key_wallet::managed_account::managed_account_trait::ManagedAccountTrait;
    use platform_wallet::changeset::AccountAddressPoolEntry;
    use platform_wallet_storage::sqlite::schema::core_pool::OwningAccount;
    use platform_wallet_storage::sqlite::schema::{core_pool, core_state};
    use std::collections::HashMap;

    let (persister, _tmp, path) = fresh_persister();
    let w = wid(0xC2);
    ensure_wallet_meta(&persister, &w);

    let wallet = Wallet::from_seed_bytes(
        [0x9B; 64],
        key_wallet::Network::Testnet,
        WalletAccountCreationOptions::Default,
    )
    .unwrap();

    // CoinJoin[0] external index-0 address, marked used in the snapshot.
    let mut coinjoin_used = first_external_info(&wallet, |at| {
        matches!(at, AccountType::CoinJoin { index: 0 })
    });
    coinjoin_used.used = true;
    let bip44_info = first_external_info(&wallet, |at| {
        matches!(
            at,
            AccountType::Standard {
                index: 0,
                standard_account_type: StandardAccountType::BIP44Account,
            }
        )
    });
    assert_ne!(
        bip44_info.script_pubkey, coinjoin_used.script_pubkey,
        "BIP44[0] and CoinJoin[0] must derive distinct scripts"
    );

    let pool_entry = |account_type, info: &key_wallet::AddressInfo| AccountAddressPoolEntry {
        account_type,
        pool_type: AddressPoolType::External,
        addresses: vec![info.clone()],
    };
    persister
        .store(
            w,
            PlatformWalletChangeSet {
                account_address_pools: vec![
                    pool_entry(
                        AccountType::Standard {
                            index: 0,
                            standard_account_type: StandardAccountType::BIP44Account,
                        },
                        &bip44_info,
                    ),
                    pool_entry(AccountType::CoinJoin { index: 0 }, &coinjoin_used),
                ],
                ..Default::default()
            },
        )
        .expect("store must persist the pool snapshot");
    drop(persister);

    let p2 = reopen(&path);
    let conn = p2.lock_conn_for_test();
    let (core, utxo_accounts) =
        core_state::load_state(&conn, &w, key_wallet::Network::Testnet).expect("load_state");

    // Union the two used-address sources exactly as the persister does — the
    // pool source carries the known owner (CoinJoin), authoritative on conflict.
    let used: HashMap<key_wallet::Address, Option<OwningAccount>> = {
        let mut map: HashMap<key_wallet::Address, Option<OwningAccount>> = HashMap::new();
        for (addr, owner) in core_pool::load_used_addresses(&conn, &w, key_wallet::Network::Testnet)
            .expect("core_pool used addresses")
        {
            map.entry(addr).or_insert(Some(owner));
        }
        for (addr, owner) in
            core_state::load_used_addresses(&conn, &w, key_wallet::Network::Testnet)
                .expect("core_utxos used addresses")
        {
            map.entry(addr).or_insert(owner);
        }
        map
    };
    drop(conn);

    // The real pool resolver attributed the used address to CoinJoin[0].
    assert_eq!(
        used.get(&coinjoin_used.address),
        Some(&Some(OwningAccount {
            account_type: "coinjoin".to_string(),
            account_index: 0,
            user_identity_id: [0u8; 32],
            friend_identity_id: [0u8; 32],
        })),
        "the used address must resolve to the CoinJoin owner from the pool"
    );

    let mut managed = ManagedWalletInfo::from_wallet(&wallet, 1);
    platform_wallet_storage::sqlite::util::apply_persisted_core_state(
        &mut managed,
        &manifest_for(&wallet),
        &core,
        &utxo_accounts,
        &used,
    )
    .expect("apply must not error");

    // The used address is marked used on the CoinJoin pool specifically.
    let coinjoin = managed.accounts.coinjoin_accounts.get(&0).unwrap();
    let cj_external = coinjoin
        .managed_account_type()
        .address_pools()
        .into_iter()
        .find(|p| p.pool_type == AddressPoolType::External)
        .expect("CoinJoin External pool");
    assert!(
        cj_external
            .address_info(&coinjoin_used.address)
            .expect("used address present in the CoinJoin pool")
            .used,
        "used CoinJoin address must be marked used on the CoinJoin pool, not BIP44"
    );

    // It must NOT have been (mis)routed onto the first (BIP44) account.
    let bip44 = managed.accounts.standard_bip44_accounts.get(&0).unwrap();
    for pool in bip44.managed_account_type().address_pools() {
        assert!(
            pool.address_info(&coinjoin_used.address).is_none(),
            "the CoinJoin used address must not appear in any BIP44 pool"
        );
    }
}
