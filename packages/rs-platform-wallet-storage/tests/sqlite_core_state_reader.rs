#![allow(clippy::field_reassign_with_default)]

//! `schema::core_state::load_state` bulk-reconstructs the keyless
//! `CoreChangeSet` (UTXOs, records, IS-locks, sync watermarks), and the
//! full snapshots preserve Core lifecycle state while legacy rows trigger a rescan.

mod common;

use common::{ensure_wallet_meta, fresh_persister, wid};
use dashcore::hashes::Hash;
use dashcore::{OutPoint, Txid};
use key_wallet::transaction_checking::WalletTransactionChecker;
use key_wallet::wallet::initialization::WalletAccountCreationOptions;
use key_wallet::wallet::managed_wallet_info::wallet_info_interface::WalletInfoInterface;
use key_wallet::wallet::managed_wallet_info::ManagedWalletInfo;
use key_wallet::wallet::Wallet;
use key_wallet::Utxo;
use platform_wallet::changeset::AccountRegistrationEntry;
use platform_wallet::changeset::{
    CoreChangeSet, PlatformWalletChangeSet, PlatformWalletPersistence,
};
use platform_wallet_storage::sqlite::schema::core_state;
use platform_wallet_storage::LoadCtx;
use platform_wallet_storage::WalletStorageError;

/// Keyless account manifest the rehydration path resolves xpubs from.
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

fn store_snapshot(
    persister: &platform_wallet_storage::SqlitePersister,
    wallet: &Wallet,
    info: ManagedWalletInfo,
) {
    persister
        .store(
            wallet.wallet_id,
            PlatformWalletChangeSet {
                account_registrations: manifest_for(wallet),
                provider_key_account_registrations: {
                    use key_wallet::account::AccountType;
                    use platform_wallet::changeset::{
                        ProviderKeyAccountEntry, ProviderKeyExtendedPubKey,
                    };
                    let mut entries = Vec::new();
                    if let Some(account) = wallet
                        .accounts
                        .bls_account_of_type(AccountType::ProviderOperatorKeys)
                    {
                        entries.push(ProviderKeyAccountEntry {
                            account_type: AccountType::ProviderOperatorKeys,
                            extended_public_key: ProviderKeyExtendedPubKey::Bls(
                                account.bls_public_key.clone(),
                            ),
                        });
                    }
                    if let Some(account) = wallet
                        .accounts
                        .eddsa_account_of_type(AccountType::ProviderPlatformKeys)
                    {
                        entries.push(ProviderKeyAccountEntry {
                            account_type: AccountType::ProviderPlatformKeys,
                            extended_public_key: ProviderKeyExtendedPubKey::EdDSA(
                                account.ed25519_public_key.clone(),
                            ),
                        });
                    }
                    entries
                },
                core_wallet_snapshot: Some(info),
                ..Default::default()
            },
        )
        .unwrap();
}

fn snapshot_with_coins(wallet: &Wallet, coins: &[Utxo], height: u32) -> ManagedWalletInfo {
    use key_wallet::account::ManagedAccountTrait;
    let mut info = ManagedWalletInfo::from_wallet(wallet, 1);
    for coin in coins {
        let mut accounts = info.accounts.all_funding_accounts_mut();
        let account = accounts
            .iter_mut()
            .find(|account| account.contains_address(&coin.address))
            .unwrap();
        account.utxos.insert(coin.outpoint, coin.clone());
    }
    info.update_last_processed_height(height);
    info.update_synced_height(height);
    info.update_balance();
    info
}

fn reopen(path: &std::path::Path) -> platform_wallet_storage::SqlitePersister {
    platform_wallet_storage::SqlitePersister::open(
        platform_wallet_storage::SqlitePersisterConfig::new(path),
    )
    .expect("reopen persister")
}

#[test]
fn should_reject_instant_lock_with_mismatched_row_txid() {
    let (persister, _tmp, _path) = fresh_persister();
    let wallet_id = wid(0xE9);
    ensure_wallet_meta(&persister, &wallet_id);
    let txid = Txid::from_byte_array([0xEA; 32]);
    let other = Txid::from_byte_array([0xEB; 32]);
    let lock = dashcore::InstantLock {
        txid,
        ..Default::default()
    };
    persister
        .store(
            wallet_id,
            PlatformWalletChangeSet {
                core: Some(CoreChangeSet {
                    instant_locks_for_non_final_records: [(txid, lock.clone())].into(),
                    ..Default::default()
                }),
                ..Default::default()
            },
        )
        .unwrap();
    let conn = persister.lock_conn_for_test();
    let (loaded, _, _) = core_state::load_state(
        &conn,
        &wallet_id,
        key_wallet::Network::Testnet,
        &LoadCtx::strict(),
    )
    .unwrap();
    assert_eq!(
        loaded.instant_locks_for_non_final_records.get(&txid),
        Some(&lock)
    );
    assert_eq!(
        conn.execute(
            "UPDATE core_instant_locks SET txid = ?1 WHERE wallet_id = ?2",
            rusqlite::params![other.as_byte_array().as_slice(), wallet_id.as_slice()],
        )
        .unwrap(),
        1
    );
    let error = core_state::load_state(
        &conn,
        &wallet_id,
        key_wallet::Network::Testnet,
        &LoadCtx::strict(),
    )
    .expect_err("Strict load must reject a lock whose blob txid differs from its row key");
    assert!(matches!(error, WalletStorageError::BlobDecode { .. }));
}

#[tokio::test]
async fn should_restore_coinbase_maturity_from_funding_record() {
    use dashcore::{BlockHash, ScriptBuf, Transaction, TxIn, Witness};
    use key_wallet::transaction_checking::{BlockInfo, TransactionContext};

    let (persister, _tmp, _path) = fresh_persister();
    let (mut wallet, coin) = wallet_and_utxo([0xD4; 64], 5_000_000, 100, 0);
    let wallet_id = wallet.wallet_id;
    ensure_wallet_meta(&persister, &wallet_id);
    let mut live = ManagedWalletInfo::from_wallet(&wallet, 1);
    let transaction = Transaction {
        version: 2,
        lock_time: 0,
        input: vec![TxIn {
            previous_output: OutPoint::null(),
            script_sig: ScriptBuf::new(),
            sequence: u32::MAX,
            witness: Witness::new(),
        }],
        output: vec![coin.txout],
        special_transaction_payload: None,
    };
    live.update_last_processed_height(100);
    let result = live
        .check_core_transaction(
            &transaction,
            TransactionContext::InBlock(BlockInfo::new(100, BlockHash::all_zeros(), 0)),
            &mut wallet,
            true,
            true,
        )
        .await;
    let coins = live
        .accounts
        .all_funding_accounts()
        .into_iter()
        .flat_map(|account| account.utxos.values().cloned())
        .collect();
    persister
        .store(
            wallet_id,
            PlatformWalletChangeSet {
                core: Some(CoreChangeSet {
                    records: result.new_records,
                    new_utxos: coins,
                    last_processed_height: Some(100),
                    ..Default::default()
                }),
                ..Default::default()
            },
        )
        .unwrap();
    store_snapshot(&persister, &wallet, live.clone());
    let (core, _owners, _spent) = {
        let conn = persister.lock_conn_for_test();
        core_state::load_state(
            &conn,
            &wallet_id,
            key_wallet::Network::Testnet,
            &LoadCtx::strict(),
        )
        .unwrap()
    };
    assert_eq!(core.new_utxos.len(), 1);
    assert!(core.new_utxos[0].is_coinbase);
    let state = persister.load().unwrap();
    let restored = &state.wallets.get(&wallet_id).unwrap().wallet_info;
    assert_eq!(restored.balance.immature(), 5_000_000);
    assert_eq!(restored.balance.confirmed(), 0);
    assert_eq!(restored.balance, live.balance);
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

    let seed = [0x42; 64];
    let (wallet, utxo) = wallet_and_utxo(seed, 1_234_500, 100, 0);

    let w = wallet.wallet_id;
    ensure_wallet_meta(&persister, &w);
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
    store_snapshot(
        &persister,
        &wallet,
        snapshot_with_coins(&wallet, std::slice::from_ref(&utxo), 200),
    );
    drop(persister);

    let p2 = reopen(&path);
    let conn = p2.lock_conn_for_test();
    let (core, _utxo_accounts, _restored_spends) =
        core_state::load_state(&conn, &w, key_wallet::Network::Testnet, &LoadCtx::strict())
            .expect("load_state");
    drop(conn);

    // The persisted UTXO round-trips by outpoint + value.
    assert_eq!(core.new_utxos.len(), 1);
    assert_eq!(core.new_utxos[0].outpoint, utxo.outpoint);
    assert_eq!(core.new_utxos[0].value(), 1_234_500);
    assert_eq!(core.last_processed_height, Some(200));
    assert_eq!(core.synced_height, Some(200));

    let state = p2.load().unwrap();
    let info = &state.wallets.get(&w).unwrap().wallet_info;
    let bal = WalletInfoInterface::balance(info);
    let total = bal.confirmed() + bal.unconfirmed() + bal.immature() + bal.locked();
    assert_eq!(
        total, 1_234_500,
        "reconstructed wallet balance must be exact"
    );
    assert!(total > 0, "silent zero balance is a FAIL");
    // Height-bearing UTXO lands in the confirmed bucket.
    assert_eq!(bal.confirmed(), 1_234_500);
}

/// Spent UTXOs are excluded from the balance feed and restored as claims.
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
    let (core, _utxo_accounts, restored_spends) =
        core_state::load_state(&conn, &w, key_wallet::Network::Testnet, &LoadCtx::strict())
            .unwrap();
    drop(conn);
    let ops: Vec<_> = core.new_utxos.iter().map(|u| u.outpoint).collect();
    assert!(ops.contains(&u_unspent.outpoint));
    assert!(
        !ops.contains(&u_spent.outpoint),
        "spent UTXO must not resurrect on reload"
    );
    assert_eq!(restored_spends.get(&u_spent.outpoint), Some(&None));
}

async fn assert_spent_utxo_is_not_resurrected(records_spend_first: bool, record_only: bool) {
    use dashcore::{BlockHash, ScriptBuf, Transaction, TxIn, TxOut, Witness};
    use key_wallet::managed_account::transaction_record::{
        InputDetail, OutputDetail, OutputRole, TransactionDirection, TransactionRecord,
    };
    use key_wallet::transaction_checking::{BlockInfo, TransactionContext, TransactionType};

    let (persister, _tmp, path) = fresh_persister();
    let mut wallet = Wallet::from_seed_bytes(
        [0x73; 64],
        key_wallet::Network::Testnet,
        WalletAccountCreationOptions::Default,
    )
    .expect("wallet");
    let wallet_id = wallet.wallet_id;
    ensure_wallet_meta(&persister, &wallet_id);
    let info = ManagedWalletInfo::from_wallet(&wallet, 1);
    let address = WalletInfoInterface::monitored_addresses(&info)
        .into_iter()
        .next()
        .expect("monitored address");
    let account_type = key_wallet::account::AccountType::Standard {
        index: 0,
        standard_account_type: key_wallet::account::StandardAccountType::BIP44Account,
    };

    let funding = Transaction {
        version: 2,
        lock_time: 0,
        input: vec![TxIn {
            previous_output: OutPoint {
                txid: Txid::from_byte_array([0x10; 32]),
                vout: 0,
            },
            script_sig: ScriptBuf::new(),
            sequence: u32::MAX,
            witness: Witness::new(),
        }],
        output: vec![TxOut {
            value: 5_000_000,
            script_pubkey: address.script_pubkey(),
        }],
        special_transaction_payload: None,
    };
    let funding_outpoint = OutPoint {
        txid: funding.txid(),
        vout: 0,
    };
    let funding_context = TransactionContext::InBlock(BlockInfo::new(
        100,
        BlockHash::from_byte_array([0x20; 32]),
        1_700_000_000,
    ));
    let funding_record = TransactionRecord::new(
        funding.clone(),
        account_type,
        funding_context.clone(),
        TransactionType::Standard,
        TransactionDirection::Incoming,
        Vec::new(),
        vec![OutputDetail {
            index: 0,
            role: OutputRole::Received,
            address: Some(address.clone()),
            value: 5_000_000,
        }],
        5_000_000,
    );
    let utxo = Utxo {
        outpoint: funding_outpoint,
        txout: funding.output[0].clone(),
        address: address.clone(),
        height: 100,
        is_coinbase: false,
        is_confirmed: true,
        is_instantlocked: false,
        is_locked: false,
        is_trusted: false,
    };

    let spend = Transaction {
        version: 2,
        lock_time: 0,
        input: vec![TxIn {
            previous_output: funding_outpoint,
            script_sig: ScriptBuf::new(),
            sequence: u32::MAX,
            witness: Witness::new(),
        }],
        output: vec![TxOut {
            value: 4_999_000,
            script_pubkey: ScriptBuf::new(),
        }],
        special_transaction_payload: None,
    };
    let spend_record = TransactionRecord::new(
        spend,
        account_type,
        if record_only {
            TransactionContext::Mempool
        } else {
            TransactionContext::InBlock(BlockInfo::new(
                101,
                BlockHash::from_byte_array([0x21; 32]),
                1_700_000_100,
            ))
        },
        TransactionType::Standard,
        TransactionDirection::Outgoing,
        vec![InputDetail {
            index: 0,
            value: 5_000_000,
            address: address.clone(),
        }],
        vec![OutputDetail {
            index: 0,
            role: OutputRole::Sent,
            address: None,
            value: 4_999_000,
        }],
        -5_000_000,
    );

    let mut live = info.clone();
    live.check_core_transaction(&funding, funding_context.clone(), &mut wallet, true, true)
        .await;
    live.check_core_transaction(
        &spend_record.transaction,
        spend_record.context.clone(),
        &mut wallet,
        true,
        true,
    )
    .await;
    live.update_last_processed_height(101);
    live.update_synced_height(101);
    let spend_txid = spend_record.txid;
    let records = if records_spend_first {
        vec![spend_record, funding_record]
    } else {
        vec![funding_record, spend_record]
    };
    persister
        .store(
            wallet_id,
            PlatformWalletChangeSet {
                core: Some(CoreChangeSet {
                    records,
                    new_utxos: if record_only {
                        vec![]
                    } else {
                        vec![utxo.clone()]
                    },
                    spent_utxos: if record_only { vec![] } else { vec![utxo] },
                    last_processed_height: Some(101),
                    synced_height: Some(101),
                    ..Default::default()
                }),
                ..Default::default()
            },
        )
        .expect("persist history");
    store_snapshot(&persister, &wallet, live.clone());
    drop(persister);

    let reopened = reopen(&path);
    let (mut core, _utxo_accounts, _restored_spends) = {
        let conn = reopened.lock_conn_for_test();
        core_state::load_state(
            &conn,
            &wallet_id,
            key_wallet::Network::Testnet,
            &LoadCtx::strict(),
        )
        .expect("load state")
    };

    core.records.sort_by_key(|record| {
        record.txid
            != if records_spend_first {
                spend_txid
            } else {
                funding_outpoint.txid
            }
    });
    assert_eq!(
        core.records.first().map(|record| record.txid),
        Some(if records_spend_first {
            spend_txid
        } else {
            funding_outpoint.txid
        })
    );

    let mut state = reopened.load().unwrap();
    let mut restored_info = state.wallets.remove(&wallet_id).unwrap().wallet_info;
    assert!(
        !restored_info
            .first_bip44_managed_account()
            .expect("BIP44 account")
            .utxos
            .contains_key(&funding_outpoint),
        "spent output must start absent"
    );

    let mut restored_wallet = wallet.clone();
    restored_info
        .check_core_transaction(
            &funding,
            funding_context.clone(),
            &mut restored_wallet,
            true,
            true,
        )
        .await;

    assert!(
        !restored_info
            .first_bip44_managed_account()
            .expect("BIP44 account")
            .utxos
            .contains_key(&funding_outpoint),
        "persisted spend must prevent historical funding redelivery from resurrecting the coin"
    );
    if record_only {
        let abandoned = restored_info.abandon_transaction(spend_txid);
        assert!(abandoned.abandoned.contains(&spend_txid));
        restored_info
            .check_core_transaction(
                &funding,
                funding_context.clone(),
                &mut restored_wallet,
                true,
                true,
            )
            .await;
        let live_abandoned = live.abandon_transaction(spend_txid);
        assert_eq!(abandoned.abandoned, live_abandoned.abandoned);
        live.check_core_transaction(&funding, funding_context, &mut wallet, true, true)
            .await;
        assert_eq!(
            restored_info.balance, live.balance,
            "snapshot must retain the live engine's abandon behavior"
        );
        assert!(!restored_info
            .transaction_history()
            .iter()
            .any(|record| record.txid == spend_txid));
    }
}

/// A confirmed spend remains authoritative after reload when its funding
/// transaction is delivered again by a historical scan.
#[tokio::test]
async fn should_not_resurrect_spent_utxo_after_rehydration() {
    assert_spent_utxo_is_not_resurrected(false, false).await;
}

/// Rehydration is independent of the persisted transaction iteration order.
#[tokio::test]
async fn should_not_resurrect_spent_utxo_when_spend_record_loads_first() {
    assert_spent_utxo_is_not_resurrected(true, false).await;
}

/// Transaction inputs remain authoritative when no materialized spent row exists.
#[tokio::test]
async fn should_restore_and_release_mempool_claim_from_record_without_utxo_row() {
    assert_spent_utxo_is_not_resurrected(true, true).await;
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
    let result =
        core_state::load_state(&conn, &w, key_wallet::Network::Testnet, &LoadCtx::strict());
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
    let w = wallet.wallet_id;
    ensure_wallet_meta(&persister, &w);
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
    store_snapshot(
        &persister,
        &wallet,
        snapshot_with_coins(&wallet, std::slice::from_ref(&utxo), 60),
    );
    drop(persister);

    let p2 = reopen(&path);
    let conn = p2.lock_conn_for_test();
    let (core, _utxo_accounts, _restored_spends) =
        core_state::load_state(&conn, &w, key_wallet::Network::Testnet, &LoadCtx::strict())
            .unwrap();
    drop(conn);
    assert_eq!(core.new_utxos.len(), 1);

    let state = p2.load().unwrap();
    let info = &state.wallets.get(&w).unwrap().wallet_info;
    let bal = WalletInfoInterface::balance(info);
    let total = bal.confirmed() + bal.unconfirmed() + bal.immature() + bal.locked();
    assert_eq!(
        total, 9_000_000,
        "CoinJoin-only wallet must reconstruct the exact non-zero total — \
         a silent zero is a FAIL"
    );
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
    let (core, _utxo_accounts, _restored_spends) =
        core_state::load_state(&conn, &w, key_wallet::Network::Testnet, &LoadCtx::strict())
            .unwrap();
    drop(conn);
    assert!(core.new_utxos.is_empty());
    assert!(core.records.is_empty());
    assert_eq!(core.last_processed_height, None);
}

/// Legacy ChainLocks remain queryable but do not advance a fresh Core rescan.
#[test]
fn b5_legacy_chain_lock_remains_archived_while_core_rescans() {
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
        let (loaded, _utxo_accounts, _restored_spends) =
            core_state::load_state(&conn, &w, key_wallet::Network::Testnet, &LoadCtx::strict())
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

    let p3 = reopen(&path);
    let start_state = PlatformWalletPersistence::load(&p3).expect("load must succeed");
    let wallet_start = start_state
        .wallets
        .get(&w)
        .expect("wallet must be in load output");
    assert!(wallet_start
        .wallet_info
        .metadata
        .last_applied_chain_lock
        .is_none());
    assert_eq!(wallet_start.wallet_info.metadata.synced_height, 0);
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
    let (loaded, _utxo_accounts, _restored_spends) =
        core_state::load_state(&conn, &w, key_wallet::Network::Testnet, &LoadCtx::strict())
            .expect("load_state must succeed");
    assert_eq!(
        loaded.last_applied_chain_lock.as_ref(),
        Some(&high),
        "a lower-height chain lock must not regress the stored higher one"
    );
}

/// First external `AddressInfo` of the account matching `pred` in `wallet`,
/// sorted by derivation index — a genuine script that round-trips.
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
/// drives `load_state` → `restore_core_address_pools`. Unlike the hand-built
/// unit test, the owning-account side channel here is produced by
/// `owning_account_for_script` (column order, `ORDER BY` tie-break, `[u8;32]`
/// identity decode), so a broken query would be caught. Each UTXO must land in
/// its TRUE account with exact per-account balances, not just the wallet total.
#[test]
fn legacy_rows_resolve_each_utxos_owning_account() {
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
    let (core, utxo_accounts, _restored_spends) =
        core_state::load_state(&conn, &w, key_wallet::Network::Testnet, &LoadCtx::strict())
            .expect("load_state");
    drop(conn);

    // The real resolver populated the side channel from `core_address_pool`.
    assert_eq!(
        utxo_accounts.len(),
        2,
        "owning_account_for_script must resolve both UTXOs from the persisted pool"
    );

    assert_eq!(core.new_utxos.len(), 2);
    assert_eq!(
        utxo_accounts.get(&bip44_op).unwrap().account_type,
        "standard_bip44"
    );
    assert_eq!(
        utxo_accounts.get(&coinjoin_op).unwrap().account_type,
        "coinjoin"
    );
    assert_eq!(core.new_utxos.iter().map(Utxo::value).sum::<u64>(), 12_000);
}

/// End-to-end regression (dashpay/platform#3968) for the address-reuse guard,
/// exercising the REAL SQL resolver. Persists a `Default` wallet with a *used*
/// address (via a `core_address_pool` snapshot with `used = true`) owned by
/// CoinJoin[0] — which is NOT the first funds account (Standard BIP44[0] is) —
/// with no unspent UTXO anchoring it. Reopens the DB, unions the two
/// used-address sources exactly as the persister does (so
/// `core_pool::load_used_addresses` carries the owner), then drives
/// `restore_core_address_pools`. The used address must land `used` on the
/// CoinJoin pool specifically — never collapsed onto BIP44 — or it stays
/// "unused" on CoinJoin and could be re-issued as a fresh receive address.
#[test]
fn rehydration_routes_used_addresses_to_owning_account() {
    use key_wallet::account::{AccountType, StandardAccountType};
    use key_wallet::managed_account::address_pool::{AddressPoolType, AddressState};
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
    coinjoin_used.state = AddressState::Used;
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
    // Union the two used-address sources exactly as the persister does — the
    // pool source carries the known owner (CoinJoin), authoritative on conflict.
    let used: HashMap<key_wallet::Address, Option<OwningAccount>> = {
        let mut map: HashMap<key_wallet::Address, Option<OwningAccount>> = HashMap::new();
        for (addr, owner) in core_pool::load_used_addresses_with_ctx(
            &conn,
            &w,
            key_wallet::Network::Testnet,
            &LoadCtx::strict(),
        )
        .expect("core_pool used addresses")
        {
            map.entry(addr).or_insert(Some(owner));
        }
        for (addr, owner) in core_state::load_used_addresses_with_ctx(
            &conn,
            &w,
            key_wallet::Network::Testnet,
            &LoadCtx::strict(),
        )
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
    platform_wallet_storage::sqlite::rehydrate::restore_core_address_pools(
        &mut managed,
        &p2.lock_conn_for_test(),
        &w,
        &platform_wallet_storage::sqlite::schema::accounts::AccountManifest {
            ecdsa: manifest_for(&wallet),
            provider: Vec::new(),
        },
        &used,
        &LoadCtx::strict(),
    )
    .expect("address restoration must not error");

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
            .is_used(),
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

/// A sweep can leave only a spent outpoint: no script, owning account or winner record.
#[tokio::test]
async fn legacy_sweep_placeholder_preserves_sql_evidence_and_starts_fresh_rescan() {
    use dashcore::{ScriptBuf, Transaction, TxIn, TxOut, Witness};
    use platform_wallet_storage::sqlite::schema::blob;

    for height in [None, Some(101u32)] {
        let (persister, _tmp, path) = fresh_persister();
        let wallet_id = wid(0xC4);
        ensure_wallet_meta(&persister, &wallet_id);
        let (wallet, template) = wallet_and_utxo([0x78; 64], 50_000, 100, 0);
        let funding = Transaction {
            version: 2,
            lock_time: 0,
            input: vec![TxIn {
                previous_output: OutPoint::new(Txid::from_byte_array([0x77; 32]), 1),
                script_sig: ScriptBuf::new(),
                sequence: u32::MAX,
                witness: Witness::new(),
            }],
            output: vec![TxOut {
                value: 50_000,
                script_pubkey: template.address.script_pubkey(),
            }],
            special_transaction_payload: None,
        };
        let outpoint = OutPoint::new(funding.txid(), 0);
        {
            let conn = persister.lock_conn_for_test();
            conn.execute(
                "INSERT INTO core_utxos (wallet_id, outpoint, value, script, spent, is_sweep_placeholder, winner_mined_height) VALUES (?1, ?2, 0, X'', 1, 1, ?3)",
                rusqlite::params![wallet_id.as_slice(), blob::encode_outpoint(&outpoint).unwrap(), height],
            ).unwrap();
        }
        persister
            .store(
                wallet_id,
                PlatformWalletChangeSet {
                    account_registrations: manifest_for(&wallet),
                    ..Default::default()
                },
            )
            .unwrap();
        drop(persister);
        let reopened = reopen(&path);
        let (core, owners, spent) = {
            let conn = reopened.lock_conn_for_test();
            core_state::load_state(
                &conn,
                &wallet_id,
                key_wallet::Network::Testnet,
                &LoadCtx::strict(),
            )
            .unwrap()
        };
        assert_eq!(spent.get(&outpoint), Some(&height));
        assert!(!owners.contains_key(&outpoint));
        assert!(core.records.is_empty());
        let state = reopened.load().unwrap();
        let info = &state.wallets.get(&wallet_id).unwrap().wallet_info;
        assert!(info.utxos().is_empty());
        assert!(info.transaction_history().is_empty());
        assert!(info.observed_spent_outpoints().is_empty());
        assert_eq!(info.metadata.synced_height, 0);
    }
}
