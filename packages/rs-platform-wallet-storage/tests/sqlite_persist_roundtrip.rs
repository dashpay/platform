#![allow(clippy::field_reassign_with_default)]

//! Per-sub-changeset round-trip tests.
//!
//! Now that `platform-wallet`'s `serde` feature is active, every
//! changeset blob is a single bincode-serde payload — these tests
//! store a non-trivial entry, reopen the persister, decode the blob,
//! and assert structural equality (where the type allows) or
//! field-level equality (where it doesn't, e.g. `TransactionRecord`
//! which is `Debug + Clone` only upstream).
//!
//! TC-001 (CoreChangeSet records) is exercised through the trait
//! method in `sqlite_buffer_semantics.rs::tc001_get_core_tx_record_roundtrip`.
//! TC-015 (multi-wallet coexistence) lives there too.

mod common;

use common::{ensure_wallet_meta, fresh_persister, wid};
use key_wallet::Network;
use platform_wallet::changeset::{
    CoreChangeSet, PlatformWalletChangeSet, PlatformWalletPersistence, WalletMetadataEntry,
};
use platform_wallet_storage::{
    JournalMode, SqlitePersister, SqlitePersisterConfig, Synchronous, WalletStorageError,
};

/// TC-005: sync heights round-trip with monotonic-max merge.
#[test]
fn tc005_sync_heights_roundtrip() {
    let (persister, _tmp, _path) = fresh_persister();
    let w = wid(0xF0);
    ensure_wallet_meta(&persister, &w);
    let mut cs = PlatformWalletChangeSet::default();
    cs.core = Some(CoreChangeSet {
        last_processed_height: Some(100),
        synced_height: Some(95),
        ..Default::default()
    });
    persister.store(w, cs).unwrap();
    let mut cs = PlatformWalletChangeSet::default();
    cs.core = Some(CoreChangeSet {
        last_processed_height: Some(120),
        synced_height: Some(100),
        ..Default::default()
    });
    persister.store(w, cs).unwrap();
    let conn = persister.lock_conn_for_test();
    let (lp, sy): (i64, i64) = conn
        .query_row(
            "SELECT last_processed_height, synced_height FROM core_sync_state WHERE wallet_id = ?1",
            rusqlite::params![w.as_slice()],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    assert_eq!(lp, 120);
    assert_eq!(sy, 100);
}

/// TC-013: wallet_metadata round-trip.
#[test]
fn tc013_wallet_metadata_roundtrip() {
    let (persister, _tmp, _path) = fresh_persister();
    let w = wid(0xF1);
    ensure_wallet_meta(&persister, &w);
    let cs = PlatformWalletChangeSet {
        wallet_metadata: Some(WalletMetadataEntry {
            network: Network::Testnet,
            wallet_group_id: w,
            birth_height: 12345,
        }),
        ..Default::default()
    };
    persister.store(w, cs).unwrap();
    let conn = persister.lock_conn_for_test();
    let (network, birth_height): (String, i64) = conn
        .query_row(
            "SELECT network, birth_height FROM wallet_metadata WHERE wallet_id = ?1",
            rusqlite::params![w.as_slice()],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    assert_eq!(network, "testnet");
    assert_eq!(birth_height, 12345);
}

/// journal_mode=Memory is rejected at open with a typed
/// `ConfigInvalid` error and the DB is not created.
#[test]
fn tc_code_029_1_journal_mode_memory_rejected() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("w.db");
    let mut cfg = SqlitePersisterConfig::new(&path);
    cfg.journal_mode = JournalMode::Memory;
    let err = SqlitePersister::open(cfg);
    let matched = matches!(err.as_ref(), Err(WalletStorageError::ConfigInvalid { .. }));
    assert!(
        matched,
        "expected ConfigInvalid for journal_mode=Memory, got error = {:?}",
        err.as_ref().err()
    );
    assert!(
        !path.exists(),
        "DB should not be created when config is invalid"
    );
}

/// journal_mode=Off is rejected at open with a typed `ConfigInvalid`
/// error and the DB is not created.
#[test]
fn tc_code_029_2_journal_mode_off_rejected() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("w.db");
    let mut cfg = SqlitePersisterConfig::new(&path);
    cfg.journal_mode = JournalMode::Off;
    let err = SqlitePersister::open(cfg);
    let matched = matches!(err.as_ref(), Err(WalletStorageError::ConfigInvalid { .. }));
    assert!(
        matched,
        "expected ConfigInvalid for journal_mode=Off, got error = {:?}",
        err.as_ref().err()
    );
    assert!(
        !path.exists(),
        "DB should not be created when config is invalid"
    );
}

/// busy_timeout=0 opens successfully but emits a tracing::warn so
/// operators can spot the footgun in logs.
#[test]
#[tracing_test::traced_test]
fn tc_code_029_3_busy_timeout_zero_warns() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("w.db");
    let mut cfg = SqlitePersisterConfig::new(&path);
    cfg.busy_timeout = std::time::Duration::ZERO;
    let p = SqlitePersister::open(cfg).expect("open should succeed with busy_timeout=0");
    drop(p);
    assert!(
        logs_contain("busy_timeout=0"),
        "expected a busy_timeout=0 warning in captured logs"
    );
}

/// TC-079: synchronous=Off is rejected at open with a typed error.
#[test]
fn tc079_synchronous_off_rejected() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("w.db");
    let mut cfg = SqlitePersisterConfig::new(&path);
    cfg.synchronous = Synchronous::Off;
    let err = SqlitePersister::open(cfg);
    let matched = matches!(err.as_ref(), Err(WalletStorageError::ConfigInvalid { .. }));
    assert!(
        matched,
        "expected ConfigInvalid, got error = {:?}",
        err.as_ref().err()
    );
    assert!(
        !path.exists(),
        "DB should not be created when config is invalid"
    );
}

/// TC-080: SqlitePersisterConfig::new yields sensible defaults.
#[test]
fn tc080_config_defaults() {
    let cfg = SqlitePersisterConfig::new("/tmp/some.db");
    assert!(matches!(
        cfg.flush_mode,
        platform_wallet_storage::FlushMode::Immediate
    ));
    assert_eq!(cfg.busy_timeout, std::time::Duration::from_secs(5));
    assert!(matches!(
        cfg.journal_mode,
        platform_wallet_storage::JournalMode::Wal
    ));
    assert!(matches!(cfg.synchronous, Synchronous::Normal));
    assert!(cfg.auto_backup_dir.is_some());
}

/// TC-081: LockPoisoned round-trips into PersistenceError::LockPoisoned.
#[test]
fn tc081_lock_poisoned_mapping() {
    use platform_wallet::changeset::PersistenceError;
    let err = WalletStorageError::LockPoisoned;
    let mapped: PersistenceError = err.into();
    assert!(matches!(mapped, PersistenceError::LockPoisoned));
}

/// TC-007: IdentityKeysChangeSet stores public-only material that
/// round-trips through `identity_keys.public_key_blob`.
#[test]
fn tc007_identity_key_entry_roundtrip() {
    use dpp::identity::identity_public_key::v0::IdentityPublicKeyV0;
    use dpp::identity::{IdentityPublicKey, KeyType, Purpose, SecurityLevel};
    use dpp::platform_value::BinaryData;
    use dpp::prelude::Identifier;
    use platform_wallet::changeset::{
        IdentityKeyDerivationIndices, IdentityKeyEntry, IdentityKeysChangeSet,
    };

    let (persister, tmp, path) = fresh_persister();
    let w = wid(0xF7);
    ensure_wallet_meta(&persister, &w);

    let identity_id = Identifier::from([0xAA; 32]);
    platform_wallet_storage::sqlite::schema::identities::ensure_exists(
        &persister.lock_conn_for_test(),
        &w,
        identity_id.as_slice().try_into().unwrap(),
    )
    .unwrap();

    let public_key = IdentityPublicKey::V0(IdentityPublicKeyV0 {
        id: 0,
        purpose: Purpose::AUTHENTICATION,
        security_level: SecurityLevel::HIGH,
        contract_bounds: None,
        key_type: KeyType::ECDSA_SECP256K1,
        read_only: false,
        data: BinaryData::new(vec![2u8; 33]),
        disabled_at: None,
    });
    let entry = IdentityKeyEntry {
        identity_id,
        key_id: 7,
        public_key: public_key.clone(),
        public_key_hash: [3u8; 20],
        wallet_id: Some(w),
        derivation_indices: Some(IdentityKeyDerivationIndices {
            identity_index: 1,
            key_index: 2,
        }),
    };
    let mut keys = IdentityKeysChangeSet::default();
    keys.upserts.insert((identity_id, 7), entry.clone());
    persister
        .store(
            w,
            PlatformWalletChangeSet {
                identity_keys: Some(keys),
                ..Default::default()
            },
        )
        .unwrap();
    drop(persister);

    let p2 = SqlitePersister::open(SqlitePersisterConfig::new(&path)).unwrap();
    let conn = p2.lock_conn_for_test();
    // identity_keys is keyed by (identity_id, key_id); the wallet_id
    // column is not part of the schema.
    let blob_bytes: Vec<u8> = conn
        .query_row(
            "SELECT public_key_blob FROM identity_keys WHERE identity_id = ?1 AND key_id = ?2",
            rusqlite::params![identity_id.as_slice(), 7i64],
            |row| row.get(0),
        )
        .unwrap();
    let decoded =
        platform_wallet_storage::sqlite::schema::identity_keys::decode_entry(&blob_bytes).unwrap();
    assert_eq!(decoded, entry);
    // The load-bearing NFR-10 check is `tests/secrets_scan.rs`,
    // which greps every file under `src/sqlite/schema/` and
    // `migrations/` for forbidden secret-material substrings —
    // bincode wire bytes carry no field names, so any runtime
    // substring scan against the blob would be a false-confidence
    // smoke test.
    drop(tmp);
}

/// TC-009: PlatformAddressChangeSet round-trips through
/// `platform_addresses`. The typed columns (account_index,
/// address_index, address, balance, nonce) carry the entire
/// `PlatformAddressBalanceEntry` shape — no blob column needed for
/// this table, but we exercise the schema writer + a direct probe.
#[test]
fn tc009_platform_address_roundtrip() {
    use dash_sdk::platform::address_sync::AddressFunds;
    use key_wallet::PlatformP2PKHAddress;
    use platform_wallet::changeset::{PlatformAddressBalanceEntry, PlatformAddressChangeSet};

    let (persister, tmp, path) = fresh_persister();
    let w = wid(0xF8);
    ensure_wallet_meta(&persister, &w);

    let addr1 = PlatformP2PKHAddress::new([0x11; 20]);
    let addr2 = PlatformP2PKHAddress::new([0x22; 20]);
    let entries = vec![
        PlatformAddressBalanceEntry {
            wallet_id: w,
            account_index: 0,
            address_index: 0,
            address: addr1,
            funds: AddressFunds {
                nonce: 1,
                balance: 500,
                as_of_height: 111,
            },
        },
        PlatformAddressBalanceEntry {
            wallet_id: w,
            account_index: 0,
            address_index: 1,
            address: addr2,
            funds: AddressFunds {
                nonce: 2,
                balance: 1500,
                as_of_height: 222,
            },
        },
    ];
    persister
        .store(
            w,
            PlatformWalletChangeSet {
                platform_addresses: Some(PlatformAddressChangeSet {
                    addresses: entries.clone(),
                    sync_height: Some(99),
                    ..Default::default()
                }),
                ..Default::default()
            },
        )
        .unwrap();
    drop(persister);
    let p2 = SqlitePersister::open(SqlitePersisterConfig::new(&path)).unwrap();
    let rows = platform_wallet_storage::sqlite::schema::platform_addrs::list_per_wallet(
        &p2.lock_conn_for_test(),
        &w,
    )
    .unwrap();
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].address, addr1);
    assert_eq!(rows[0].funds.balance, 500);
    assert_eq!(rows[0].funds.nonce, 1);
    assert_eq!(rows[1].address, addr2);
    assert_eq!(rows[1].funds.balance, 1500);
    drop(tmp);
}

/// TC-014: AccountRegistrationEntry round-trips through
/// `account_registrations` via the bincode-serde blob.
#[test]
fn tc014_account_registration_roundtrip() {
    use key_wallet::account::{AccountType, StandardAccountType};
    use key_wallet::bip32::ExtendedPubKey;
    use platform_wallet::changeset::AccountRegistrationEntry;

    // Synthesise a deterministic xpub from a fixed test seed so the
    // test is reproducible without external fixtures.
    let xpub = ExtendedPubKey::decode(&hex::decode(
        "0488B21E000000000000000000873DFF81C02F525623FD1FE5167EAC3A55A049DE3D314BB42EE227FFED37D5080339A36013301597DAEF41FBE593A02CC513D0B55527EC2DF1050E2E8FF49C85C2",
    ).unwrap()).unwrap();
    let entry = AccountRegistrationEntry {
        account_type: AccountType::Standard {
            index: 0,
            standard_account_type: StandardAccountType::BIP44Account,
        },
        account_xpub: xpub,
    };

    let (persister, tmp, path) = fresh_persister();
    let w = wid(0xFE);
    ensure_wallet_meta(&persister, &w);
    persister
        .store(
            w,
            PlatformWalletChangeSet {
                account_registrations: vec![entry.clone()],
                ..Default::default()
            },
        )
        .unwrap();
    drop(persister);

    let p2 = SqlitePersister::open(SqlitePersisterConfig::new(&path)).unwrap();
    let conn = p2.lock_conn_for_test();
    let blob_bytes: Vec<u8> = conn
        .query_row(
            "SELECT account_xpub_bytes FROM account_registrations WHERE wallet_id = ?1",
            rusqlite::params![w.as_slice()],
            |row| row.get(0),
        )
        .unwrap();
    let decoded: AccountRegistrationEntry =
        platform_wallet_storage::sqlite::schema::blob::decode(&blob_bytes).unwrap();
    assert_eq!(decoded.account_type, entry.account_type);
    assert_eq!(decoded.account_xpub, xpub);
    drop(tmp);
}

/// TC-010: AssetLockChangeSet round-trips lifecycle data — including
/// the embedded `Transaction` and optional `AssetLockProof` — through
/// the bincode-serde payload in `asset_locks.lifecycle_blob`.
#[test]
fn tc010_asset_lock_roundtrip() {
    use dashcore::hashes::Hash;
    use dashcore::{OutPoint, Transaction, Txid};
    use key_wallet::wallet::managed_wallet_info::asset_lock_builder::AssetLockFundingType;
    use platform_wallet::changeset::{AssetLockChangeSet, AssetLockEntry};
    use platform_wallet::wallet::asset_lock::tracked::AssetLockStatus;

    let txid = Txid::from_byte_array([0x42; 32]);
    let outpoint = OutPoint { txid, vout: 1 };
    let transaction = Transaction {
        version: 3,
        lock_time: 0,
        input: vec![],
        output: vec![],
        special_transaction_payload: None,
    };
    let entry = AssetLockEntry {
        out_point: outpoint,
        transaction: transaction.clone(),
        account_index: 5,
        funding_type: AssetLockFundingType::IdentityTopUp,
        identity_index: 9,
        amount_duffs: 12_345,
        status: AssetLockStatus::Built,
        proof: None,
    };
    let mut locks = AssetLockChangeSet::default();
    locks.asset_locks.insert(outpoint, entry.clone());

    let (persister, tmp, path) = fresh_persister();
    let w = wid(0xFD);
    ensure_wallet_meta(&persister, &w);
    persister
        .store(
            w,
            PlatformWalletChangeSet {
                asset_locks: Some(locks),
                ..Default::default()
            },
        )
        .unwrap();
    drop(persister);

    let p2 = SqlitePersister::open(SqlitePersisterConfig::new(&path)).unwrap();
    let bucketed = platform_wallet_storage::sqlite::schema::asset_locks::load_state(
        &p2.lock_conn_for_test(),
        &w,
    )
    .unwrap();
    let by_outpoint = &bucketed[&5];
    let tracked = &by_outpoint[&outpoint];
    assert_eq!(tracked.amount, entry.amount_duffs);
    assert_eq!(tracked.account_index, entry.account_index);
    assert_eq!(tracked.identity_index, entry.identity_index);
    assert_eq!(tracked.funding_type, entry.funding_type);
    assert_eq!(tracked.status, entry.status);
    assert_eq!(tracked.transaction.version, transaction.version);
    drop(tmp);
}

/// TC-010b: a `RecoveredFromChain` lock — the restore-scan
/// reconstruction's status, admitted by the V004 CHECK widening —
/// round-trips through the writer's TEXT status mapping and the
/// lifecycle blob, chain proof included.
#[test]
fn tc010b_recovered_from_chain_lock_roundtrip() {
    use dashcore::hashes::Hash;
    use dashcore::{OutPoint, Transaction, Txid};
    use dpp::identity::state_transition::asset_lock_proof::chain::ChainAssetLockProof;
    use key_wallet::wallet::managed_wallet_info::asset_lock_builder::AssetLockFundingType;
    use platform_wallet::changeset::{AssetLockChangeSet, AssetLockEntry};
    use platform_wallet::wallet::asset_lock::tracked::AssetLockStatus;

    let txid = Txid::from_byte_array([0x43; 32]);
    let outpoint = OutPoint { txid, vout: 0 };
    let entry = AssetLockEntry {
        out_point: outpoint,
        transaction: Transaction {
            version: 3,
            lock_time: 0,
            input: vec![],
            output: vec![],
            special_transaction_payload: None,
        },
        account_index: 0,
        funding_type: AssetLockFundingType::AssetLockShieldedAddressTopUp,
        identity_index: 0,
        amount_duffs: 500_000,
        status: AssetLockStatus::RecoveredFromChain,
        proof: Some(dpp::prelude::AssetLockProof::Chain(ChainAssetLockProof {
            core_chain_locked_height: 411_495,
            out_point: outpoint,
        })),
    };
    let mut locks = AssetLockChangeSet::default();
    locks.asset_locks.insert(outpoint, entry.clone());

    let (persister, tmp, path) = fresh_persister();
    let w = wid(0xFB);
    ensure_wallet_meta(&persister, &w);
    persister
        .store(
            w,
            PlatformWalletChangeSet {
                asset_locks: Some(locks),
                ..Default::default()
            },
        )
        .expect("recovered_from_chain must satisfy the widened CHECK");
    drop(persister);

    let p2 = SqlitePersister::open(SqlitePersisterConfig::new(&path)).unwrap();
    let bucketed = platform_wallet_storage::sqlite::schema::asset_locks::load_state(
        &p2.lock_conn_for_test(),
        &w,
    )
    .unwrap();
    let tracked = &bucketed[&0][&outpoint];
    assert_eq!(tracked.status, AssetLockStatus::RecoveredFromChain);
    match &tracked.proof {
        Some(dpp::prelude::AssetLockProof::Chain(chain)) => {
            assert_eq!(chain.core_chain_locked_height, 411_495);
        }
        other => panic!("chain proof must survive the roundtrip, got {other:?}"),
    }
    drop(tmp);
}

/// TC-012: DashPay profile + payment overlay round-trip through the
/// dashpay_* tables via bincode-serde blobs.
#[test]
fn tc012_dashpay_overlay_roundtrip() {
    use dpp::prelude::Identifier;
    use platform_wallet::wallet::identity::{DashPayProfile, PaymentEntry};

    let (persister, tmp, path) = fresh_persister();
    let w = wid(0xFC);
    ensure_wallet_meta(&persister, &w);
    let identity_id = Identifier::from([0x55; 32]);
    platform_wallet_storage::sqlite::schema::identities::ensure_exists(
        &persister.lock_conn_for_test(),
        &w,
        identity_id.as_slice().try_into().unwrap(),
    )
    .unwrap();

    let profile = DashPayProfile {
        display_name: Some("alice".into()),
        bio: Some("hello world".into()),
        avatar_url: None,
        avatar_hash: None,
        avatar_fingerprint: None,
        public_message: Some("public".into()),
    };
    let payment = PaymentEntry::new_sent(Identifier::from([0x66; 32]), 7_500, Some("lunch".into()));

    let mut profiles = std::collections::BTreeMap::new();
    profiles.insert(identity_id, Some(profile.clone()));
    let mut by_tx = std::collections::BTreeMap::new();
    by_tx.insert("tx-aaaa".to_string(), payment.clone());
    let mut payments = std::collections::BTreeMap::new();
    payments.insert(identity_id, by_tx);

    persister
        .store(
            w,
            PlatformWalletChangeSet {
                dashpay_profiles: Some(profiles),
                dashpay_payments_overlay: Some(payments),
                ..Default::default()
            },
        )
        .unwrap();
    drop(persister);

    let p2 = SqlitePersister::open(SqlitePersisterConfig::new(&path)).unwrap();
    let conn = p2.lock_conn_for_test();
    // dashpay_profiles is keyed by identity_id only.
    let profile_blob: Vec<u8> = conn
        .query_row(
            "SELECT profile_blob FROM dashpay_profiles WHERE identity_id = ?1",
            rusqlite::params![identity_id.as_slice()],
            |row| row.get(0),
        )
        .unwrap();
    let decoded_profile: DashPayProfile =
        platform_wallet_storage::sqlite::schema::blob::decode(&profile_blob).unwrap();
    assert_eq!(decoded_profile, profile);

    let payment_blob: Vec<u8> = conn
        .query_row(
            "SELECT overlay_blob FROM dashpay_payments_overlay WHERE identity_id = ?1 AND payment_id = ?2",
            rusqlite::params![identity_id.as_slice(), "tx-aaaa"],
            |row| row.get(0),
        )
        .unwrap();
    let decoded_payment: PaymentEntry =
        platform_wallet_storage::sqlite::schema::blob::decode(&payment_blob).unwrap();
    assert_eq!(decoded_payment, payment);
    drop(tmp);
}

/// TC-082 (lint): grep for `Box<dyn Error>` in the crate's sources.
#[test]
fn tc082_no_box_dyn_error_in_src() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut offenders = Vec::new();
    visit(&root, &mut offenders);
    assert!(
        offenders.is_empty(),
        "Box<dyn Error> found in: {offenders:?}"
    );

    fn visit(dir: &std::path::Path, out: &mut Vec<String>) {
        for entry in std::fs::read_dir(dir).unwrap().flatten() {
            let p = entry.path();
            if p.is_dir() {
                visit(&p, out);
            } else if p.extension().is_some_and(|e| e == "rs") {
                let s = std::fs::read_to_string(&p).unwrap();
                if s.contains("Box<dyn Error") || s.contains("Box<dyn std::error::Error") {
                    out.push(p.display().to_string());
                }
            }
        }
    }
}

/// TC-P1-004: prepared-statement cache survives 60 sequential
/// store+flush cycles. SQLite's default statement cache holds 16
/// statements; running well past that exercises LRU eviction and
/// confirms `prepare_cached`'s borrow-checker-enforced lifecycle.
#[test]
fn tc_p1_004_cache_scope_under_heavy_reuse() {
    let (persister, _tmp, _path) = fresh_persister();
    let w = wid(0xC0);
    ensure_wallet_meta(&persister, &w);
    for i in 0u32..60 {
        let mut cs = PlatformWalletChangeSet::default();
        cs.core = Some(CoreChangeSet {
            synced_height: Some(i),
            ..Default::default()
        });
        persister.store(w, cs).expect("store");
        persister.flush(w).expect("flush");
    }
    let conn = persister.lock_conn_for_test();
    let synced: i64 = conn
        .query_row(
            "SELECT synced_height FROM core_sync_state WHERE wallet_id = ?1",
            rusqlite::params![w.as_slice()],
            |row| row.get(0),
        )
        .expect("read final synced");
    assert_eq!(synced, 59);
}
