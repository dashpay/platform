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
    let core = core_state::load_state(&conn, &w, key_wallet::Network::Testnet).expect("load_state");
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
            &[],
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
    let core = core_state::load_state(&conn, &w, key_wallet::Network::Testnet).unwrap();
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
    let core = core_state::load_state(&conn, &w, key_wallet::Network::Testnet).unwrap();
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
            &[],
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
    let core = core_state::load_state(&conn, &w, key_wallet::Network::Testnet).unwrap();
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
        let loaded = core_state::load_state(&conn, &w, key_wallet::Network::Testnet)
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
    let loaded = core_state::load_state(&conn, &w, key_wallet::Network::Testnet)
        .expect("load_state must succeed");
    assert_eq!(
        loaded.last_applied_chain_lock.as_ref(),
        Some(&high),
        "a lower-height chain lock must not regress the stored higher one"
    );
}
