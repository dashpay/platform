#![allow(clippy::field_reassign_with_default)]

//! Populated-V001 fixture capture.
//!
//! `regenerate_populated_v001_fixture` (`#[ignore]`) seeds a realistic
//! multi-wallet store via the persister, then lifts its V001-table rows onto a
//! destination capped at V001 only (`runner().set_target(Version(1))`), writing
//! `tests/fixtures/populated_v001.db`. Seeding runs fully-migrated because the
//! writers reference V002/V003 tables; the V001-capped copy is what the
//! migration-execution suites (TC-B-031/032/033/035/036) migrate forward.
//! Re-run it whenever V001's shape changes so the committed fixture's
//! applied-V001 checksum stays in lockstep with `V001__initial.rs` — otherwise
//! migrating the frozen store forward trips refinery's divergent-version guard.
//!
//! `populated_v001_fixture_is_present_and_openable` is the always-run guard
//! that keeps the committed fixture honest: it opens read-only, asserts the
//! store is at schema version 1, and spot-checks the seeded rows.

mod common;

use std::path::{Path, PathBuf};

use common::wid;
use dpp::prelude::Identifier;
use key_wallet::account::{AccountType, StandardAccountType};
use key_wallet::bip32::ExtendedPubKey;
use key_wallet::wallet::initialization::WalletAccountCreationOptions;
use key_wallet::wallet::managed_wallet_info::ManagedWalletInfo;
use key_wallet::wallet::Wallet;
use key_wallet::Network;
use platform_wallet::changeset::{
    AccountRegistrationEntry, ContactChangeSet, ContactRequestEntry, CoreChangeSet,
    IdentityChangeSet, IdentityEntry, PlatformWalletChangeSet, PlatformWalletPersistence,
    SentContactRequestKey, WalletMetadataEntry,
};
use platform_wallet::wallet::identity::{ContactRequest, IdentityStatus};
use platform_wallet::wallet::platform_wallet::WalletId;
use platform_wallet_storage::{SqlitePersister, SqlitePersisterConfig};
use rusqlite::OptionalExtension;

/// The two wallets the fixture carries.
const FULL_WALLET: u8 = 0xA1;
const EMPTY_WALLET: u8 = 0xB2;

/// Absolute path to the committed fixture under the crate's test tree.
fn fixture_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("populated_v001.db")
}

/// A deterministic test xpub decoded from a fixed serialized form, matching
/// the reconstruction suite so registrations round-trip reproducibly.
fn test_xpub() -> ExtendedPubKey {
    ExtendedPubKey::decode(&hex::decode(
        "0488B21E000000000000000000873DFF81C02F525623FD1FE5167EAC3A55A049DE3D314BB42EE227FFED37D5080339A36013301597DAEF41FBE593A02CC513D0B55527EC2DF1050E2E8FF49C85C2",
    ).unwrap()).unwrap()
}

/// First external address of the Standard BIP44 account 0, derived from a
/// fixed seed so the UTXO lands on a real, script-round-trippable address.
fn first_external_address(seed_byte: u8) -> dashcore::Address {
    use key_wallet::managed_account::address_pool::AddressPoolType;
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
            let mut infos: Vec<_> = pool.addresses.values().cloned().collect();
            infos.sort_by_key(|a| a.index);
            return infos.first().cloned().unwrap().address;
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
        height: 200,
        is_coinbase: false,
        is_confirmed: true,
        is_instantlocked: false,
        is_locked: false,
        is_trusted: false,
    }
}

fn one_tx_record() -> key_wallet::managed_account::transaction_record::TransactionRecord {
    use dashcore::hashes::Hash;
    use dashcore::{BlockHash, Transaction, Txid};
    use key_wallet::managed_account::transaction_record::{
        TransactionDirection, TransactionRecord,
    };
    use key_wallet::transaction_checking::{BlockInfo, TransactionContext, TransactionType};
    let mut record = TransactionRecord::new(
        Transaction {
            version: 3,
            lock_time: 0,
            input: vec![],
            output: vec![],
            special_transaction_payload: None,
        },
        AccountType::Standard {
            index: 0,
            standard_account_type: StandardAccountType::BIP44Account,
        },
        TransactionContext::InChainLockedBlock(BlockInfo::new(
            200,
            BlockHash::from_byte_array([0x03; 32]),
            1_735_689_600,
        )),
        TransactionType::Standard,
        TransactionDirection::Incoming,
        Vec::new(),
        Vec::new(),
        150_000,
    );
    record.txid = Txid::from_byte_array([0x7E; 32]);
    record
}

fn identity_entry() -> IdentityEntry {
    IdentityEntry {
        id: Identifier::from([0xC1; 32]),
        balance: 42,
        revision: 1,
        identity_index: Some(0),
        last_updated_balance_block_time: None,
        last_synced_keys_block_time: None,
        dpns_names: Vec::new(),
        contested_dpns_names: Vec::new(),
        status: IdentityStatus::Active,
        wallet_id: None,
        dashpay_profile: None,
        dashpay_payments: Default::default(),
        contact_profiles: Default::default(),
        ignored_senders: Default::default(),
    }
}

/// Build the populated multi-wallet store at `path` via the real persister.
fn build_populated_store(path: &Path) {
    let cfg = SqlitePersisterConfig::new(path);
    let persister = SqlitePersister::open(cfg).expect("open persister");

    let full: WalletId = wid(FULL_WALLET);
    let empty: WalletId = wid(EMPTY_WALLET);

    // Empty wallet: registered (a `wallets` row) but never synced.
    persister
        .store(
            empty,
            PlatformWalletChangeSet {
                wallet_metadata: Some(WalletMetadataEntry {
                    network: Network::Testnet,
                    wallet_group_id: empty,
                    birth_height: 50,
                }),
                ..Default::default()
            },
        )
        .expect("store empty wallet meta");

    // Full wallet: metadata + registration + core state + identity + contact.
    persister
        .store(
            full,
            PlatformWalletChangeSet {
                wallet_metadata: Some(WalletMetadataEntry {
                    network: Network::Testnet,
                    wallet_group_id: full,
                    birth_height: 100,
                }),
                account_registrations: vec![AccountRegistrationEntry {
                    account_type: AccountType::Standard {
                        index: 0,
                        standard_account_type: StandardAccountType::BIP44Account,
                    },
                    account_xpub: test_xpub(),
                }],
                ..Default::default()
            },
        )
        .expect("store full wallet meta + registration");

    let addr = first_external_address(FULL_WALLET);
    persister
        .store(
            full,
            PlatformWalletChangeSet {
                core: Some(CoreChangeSet {
                    records: vec![one_tx_record()],
                    new_utxos: vec![utxo_at(&addr, 0, 150_000)],
                    last_processed_height: Some(200),
                    synced_height: Some(200),
                    ..Default::default()
                }),
                ..Default::default()
            },
        )
        .expect("store full wallet core state");

    let mut identities = std::collections::BTreeMap::new();
    let ident = identity_entry();
    identities.insert(ident.id, ident);
    let owner = Identifier::from([0xC1; 32]);
    let recipient = Identifier::from([0xC2; 32]);
    let mut sent = std::collections::BTreeMap::new();
    sent.insert(
        SentContactRequestKey {
            owner_id: owner,
            recipient_id: recipient,
        },
        ContactRequestEntry {
            request: ContactRequest {
                sender_id: owner,
                recipient_id: recipient,
                sender_key_index: 0,
                recipient_key_index: 0,
                account_reference: 0,
                encrypted_account_label: None,
                encrypted_public_key: Vec::new(),
                auto_accept_proof: None,
                core_height_created_at: 200,
                created_at: 0,
            },
        },
    );
    persister
        .store(
            full,
            PlatformWalletChangeSet {
                identities: Some(IdentityChangeSet {
                    identities,
                    removed: Default::default(),
                }),
                contacts: Some(ContactChangeSet {
                    sent_requests: sent,
                    ..Default::default()
                }),
                ..Default::default()
            },
        )
        .expect("store full wallet identity + contact");

    persister.flush(full).expect("flush full");
    persister.flush(empty).expect("flush empty");
}

struct V001UtxoRow {
    wallet_id: Vec<u8>,
    outpoint: Vec<u8>,
    value: i64,
    script: Vec<u8>,
    spent: i64,
}

fn copy_core_utxos_to_v001(source: &rusqlite::Connection, destination: &rusqlite::Connection) {
    let wallet_ids: Vec<Vec<u8>> = {
        let mut stmt = source
            .prepare("SELECT wallet_id FROM wallets ORDER BY wallet_id")
            .unwrap();
        stmt.query_map([], |row| row.get(0))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap()
    };
    let mut account_indices = std::collections::HashMap::new();
    for wallet_id_bytes in wallet_ids {
        let wallet_id: WalletId = wallet_id_bytes
            .as_slice()
            .try_into()
            .expect("fixture wallet_id must be 32 bytes");
        let unspent = platform_wallet_storage::sqlite::schema::core_state::list_unspent_utxos(
            source, &wallet_id,
        )
        .expect("resolve fixture UTXO owners from core_address_pool");
        for (account_index, rows) in unspent {
            for row in rows {
                let outpoint =
                    platform_wallet_storage::sqlite::schema::blob::encode_outpoint(&row.outpoint)
                        .expect("encode fixture outpoint");
                account_indices.insert((wallet_id_bytes.clone(), outpoint), account_index);
            }
        }
    }

    let rows: Vec<V001UtxoRow> = {
        let mut stmt = source
            .prepare(
                "SELECT wallet_id, outpoint, value, script, spent \
                 FROM core_utxos ORDER BY wallet_id, outpoint",
            )
            .unwrap();
        stmt.query_map([], |row| {
            Ok(V001UtxoRow {
                wallet_id: row.get(0)?,
                outpoint: row.get(1)?,
                value: row.get(2)?,
                script: row.get(3)?,
                spent: row.get(4)?,
            })
        })
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap()
    };

    for row in rows {
        let outpoint =
            platform_wallet_storage::sqlite::schema::blob::decode_outpoint(&row.outpoint)
                .expect("decode fixture outpoint");
        let height = source
            .query_row(
                "SELECT height FROM core_transactions WHERE wallet_id = ?1 AND txid = ?2",
                rusqlite::params![&row.wallet_id, AsRef::<[u8]>::as_ref(&outpoint.txid)],
                |row| row.get::<_, Option<i64>>(0),
            )
            .optional()
            .unwrap()
            .flatten();
        let account_index = account_indices
            .get(&(row.wallet_id.clone(), row.outpoint.clone()))
            .copied()
            .unwrap_or(0);
        destination
            .execute(
                "INSERT INTO main.core_utxos \
                    (wallet_id, outpoint, value, script, height, account_index, spent, spent_in_txid) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, NULL)",
                rusqlite::params![
                    row.wallet_id,
                    row.outpoint,
                    row.value,
                    row.script,
                    height,
                    account_index,
                    row.spent
                ],
            )
            .expect("copy core_utxos row into V001 fixture");
    }
}

#[test]
fn copy_core_utxos_to_v001_preserves_legacy_row_shape() {
    use dashcore::hashes::Hash;

    let mut source = rusqlite::Connection::open_in_memory().unwrap();
    platform_wallet_storage::sqlite::migrations::run(&mut source).unwrap();
    let wallet_id = wid(0xD1);
    let txid = dashcore::Txid::from_byte_array([0x92; 32]);
    let outpoint = dashcore::OutPoint::new(txid, 3);
    let encoded_outpoint =
        platform_wallet_storage::sqlite::schema::blob::encode_outpoint(&outpoint).unwrap();
    let script = vec![0x51];
    source
        .execute(
            "INSERT INTO wallets (wallet_id, network, birth_height) \
             VALUES (?1, 'testnet', 0)",
            rusqlite::params![wallet_id.as_slice()],
        )
        .unwrap();
    source
        .execute(
            "INSERT INTO core_transactions \
                (wallet_id, txid, height, block_hash, block_time, finalized, record_blob) \
             VALUES (?1, ?2, 222, NULL, NULL, 0, NULL)",
            rusqlite::params![wallet_id.as_slice(), AsRef::<[u8]>::as_ref(&txid)],
        )
        .unwrap();
    source
        .execute(
            "INSERT INTO core_address_pool \
                (wallet_id, account_type, account_index, pool_type, address_index, script) \
             VALUES (?1, 'standard_bip44', 7, 0, 0, ?2)",
            rusqlite::params![wallet_id.as_slice(), &script],
        )
        .unwrap();
    source
        .execute(
            "INSERT INTO core_utxos (wallet_id, outpoint, value, script, spent) \
             VALUES (?1, ?2, 1234, ?3, 0)",
            rusqlite::params![wallet_id.as_slice(), &encoded_outpoint, &script],
        )
        .unwrap();

    let mut destination = rusqlite::Connection::open_in_memory().unwrap();
    platform_wallet_storage::sqlite::migrations::runner()
        .set_target(refinery::Target::Version(1))
        .run(&mut destination)
        .unwrap();
    destination
        .execute(
            "INSERT INTO wallets (wallet_id, network, birth_height) \
             VALUES (?1, 'testnet', 0)",
            rusqlite::params![wallet_id.as_slice()],
        )
        .unwrap();
    copy_core_utxos_to_v001(&source, &destination);

    let copied: (
        Vec<u8>,
        Vec<u8>,
        i64,
        Vec<u8>,
        Option<i64>,
        i64,
        i64,
        Option<Vec<u8>>,
    ) = destination
        .query_row(
            "SELECT wallet_id, outpoint, value, script, height, account_index, spent, \
                        spent_in_txid \
                 FROM core_utxos",
            [],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                    row.get(6)?,
                    row.get(7)?,
                ))
            },
        )
        .unwrap();
    assert_eq!(copied.0, wallet_id);
    assert_eq!(copied.1, encoded_outpoint);
    assert_eq!(copied.2, 1234);
    assert_eq!(copied.3, script);
    assert_eq!(copied.4, Some(222));
    assert_eq!(copied.5, 7);
    assert_eq!(copied.6, 0);
    assert!(copied.7.is_none());
}

/// Fixture regenerator. Ignored by default — run explicitly to rebuild the
/// committed fixture:
/// `cargo test -p platform-wallet-storage --test fixture_gen -- --ignored regenerate`.
#[test]
#[ignore = "regenerator: rewrites the committed fixture on disk"]
fn regenerate_populated_v001_fixture() {
    let tmp = common::secure_tempdir().expect("tempdir");
    let src = tmp.path().join("build.db");
    // Seed on a fully-migrated store: the writers (core_state UTXO attribution,
    // the meta_data_versions bump) reference V002/V003 tables, so a V001-only
    // seed is not expressible directly. The rows the migration suite reads all
    // live in V001 tables, which we lift onto a V001-capped destination below.
    build_populated_store(&src);
    let source = rusqlite::Connection::open(&src).expect("open populated source");

    let dest = fixture_path();
    if dest.exists() {
        std::fs::remove_file(&dest).expect("remove stale fixture");
    }

    // Cap the destination at V001 only, via the `runner()` set_target helper, so
    // the committed fixture is a genuine pre-V002 store carrying the CURRENT
    // V001 schema — migratable forward by the TC-B-031/032/033/035/036 suite
    // without a refinery applied-migration checksum divergence.
    let mut conn = rusqlite::Connection::open(&dest).expect("create dest db");
    platform_wallet_storage::sqlite::migrations::runner()
        .set_target(refinery::Target::Version(1))
        .run(&mut conn)
        .expect("apply V001 only to fixture");

    // Lift populated V001 tables from the built store with FKs disabled.
    // `core_utxos` crosses a later column-shape change and is copied explicitly.
    conn.execute(
        "ATTACH DATABASE ?1 AS built",
        rusqlite::params![src.to_str().expect("utf8 src path")],
    )
    .expect("attach built store");
    conn.execute_batch("PRAGMA foreign_keys = OFF;").unwrap();
    let tables: Vec<String> = {
        let mut stmt = conn
            .prepare(
                "SELECT name FROM main.sqlite_master \
                 WHERE type = 'table' AND name NOT LIKE 'sqlite_%' \
                   AND name <> 'refinery_schema_history'",
            )
            .unwrap();
        let rows = stmt
            .query_map([], |r| r.get::<_, String>(0))
            .unwrap()
            .collect::<rusqlite::Result<Vec<_>>>()
            .unwrap();
        rows
    };
    for t in &tables {
        if t == "core_utxos" {
            copy_core_utxos_to_v001(&source, &conn);
            continue;
        }
        let count: i64 = conn
            .query_row(&format!("SELECT COUNT(*) FROM built.\"{t}\""), [], |r| {
                r.get(0)
            })
            .unwrap_or(0);
        if count > 0 {
            conn.execute_batch(&format!(
                "INSERT INTO main.\"{t}\" SELECT * FROM built.\"{t}\";"
            ))
            .unwrap_or_else(|e| panic!("copy V001 table {t}: {e}"));
        }
    }
    conn.execute_batch("DETACH DATABASE built;").unwrap();

    // Collapse to a single committed file: rollback-journal mode + VACUUM leaves
    // no side WAL/journal alongside the fixture.
    conn.pragma_update(None, "journal_mode", "DELETE").unwrap();
    conn.execute_batch("VACUUM;").unwrap();
    drop(conn);
}

/// Always-run guard: the committed fixture opens, is at schema version 1,
/// and carries the seeded rows (full wallet populated, empty wallet bare).
#[test]
fn populated_v001_fixture_is_present_and_openable() {
    let path = fixture_path();
    assert!(
        path.exists(),
        "committed fixture missing at {}; regenerate with the #[ignore] test",
        path.display()
    );

    let conn = rusqlite::Connection::open_with_flags(
        &path,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_URI,
    )
    .expect("open fixture read-only");

    // The fixture is a V001-only store: refinery history tops out at 1.
    let max_version: i64 = conn
        .query_row(
            "SELECT MAX(version) FROM refinery_schema_history",
            [],
            |r| r.get(0),
        )
        .expect("read schema history");
    assert_eq!(
        max_version, 1,
        "fixture must be at V001, not a later schema"
    );

    let full = wid(FULL_WALLET);
    let empty = wid(EMPTY_WALLET);

    let wallet_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM wallets", [], |r| r.get(0))
        .unwrap();
    assert_eq!(wallet_count, 2, "two wallets: one full, one empty");

    let reg_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM account_registrations WHERE wallet_id = ?1",
            rusqlite::params![full.as_slice()],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(reg_count, 1, "full wallet has one account registration");

    let utxo_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM core_utxos WHERE wallet_id = ?1",
            rusqlite::params![full.as_slice()],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(utxo_count, 1, "full wallet has one unspent UTXO");
    let account_index: i64 = conn
        .query_row(
            "SELECT account_index FROM core_utxos WHERE wallet_id = ?1",
            rusqlite::params![full.as_slice()],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(
        account_index, 0,
        "V001 hardcodes account_index=0 — the pre-redirect writer gap"
    );

    let tx_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM core_transactions WHERE wallet_id = ?1",
            rusqlite::params![full.as_slice()],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(tx_count, 1, "full wallet has one transaction record");

    let ident_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM identities WHERE wallet_id = ?1",
            rusqlite::params![full.as_slice()],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(ident_count, 1, "full wallet has one identity");

    let contact_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM contacts WHERE wallet_id = ?1",
            rusqlite::params![full.as_slice()],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(contact_count, 1, "full wallet has one contact");

    // The empty wallet is bare: a `wallets` row and nothing else.
    let empty_utxos: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM core_utxos WHERE wallet_id = ?1",
            rusqlite::params![empty.as_slice()],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(empty_utxos, 0, "empty wallet has no core state");
}
