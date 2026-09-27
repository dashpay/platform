//! Provenance and guard for the `v4.2-dev` database fixture.
//!
//! `tests/fixtures/v4_2_dev_migrated.db` is deliberately NOT produced by this
//! crate's migration runner. The whole point of the fixture is that a database
//! this branch did not create still opens, so generating it here would assume
//! the thing under test. It is created by `v4.2-dev`'s OWN shipped
//! default-feature binary and only then seeded, by the `#[ignore]` test below.
//!
//! Hand-writing a `refinery_schema_history` table would be worse than useless:
//! the checksum is SipHasher13 over `(name, version, rendered SQL)`, so a
//! forgery can pass where a real database fails -- which is the exact failure
//! this fixture exists to catch.
//!
//! Regenerating it:
//! ```text
//! git worktree add --detach /data/git-worktrees/<name>-base origin/v4.2-dev
//! cargo build -p platform-wallet-storage --bin platform-wallet-storage --features cli
//! <target>/debug/platform-wallet-storage --db <dir>/v42.db migrate --no-auto-backup
//! V4_2_DEV_DB=<dir>/v42.db cargo test -p platform-wallet-storage --test fixture_gen -- --ignored
//! ```
//! The parent directory chain of `<dir>/v42.db` must not be group- or
//! world-writable, or the persister refuses it as an insecure parent.

mod common;

use std::path::{Path, PathBuf};

use common::wid;
use dpp::prelude::Identifier;
use key_wallet::account::{AccountType, StandardAccountType};
use key_wallet::wallet::initialization::WalletAccountCreationOptions;
use key_wallet::wallet::managed_wallet_info::ManagedWalletInfo;
use key_wallet::wallet::Wallet;
use key_wallet::Network;
use platform_wallet::changeset::{AccountRegistrationEntry, IdentityEntry};
use platform_wallet::wallet::identity::{ContactRequest, IdentityStatus};
use platform_wallet_storage::sqlite::schema::blob;

/// The two wallets the fixture carries: one populated, one bare.
const FIXTURE_WALLET: u8 = 0xA1;
const EMPTY_WALLET: u8 = 0xB2;
/// The identity the fixture carries, owned by that wallet.
const FIXTURE_IDENTITY: [u8; 32] = [0xC1; 32];

/// Historical base fixture version. It predates the subsequently published
/// V007; both that migration and these V001-V006 bodies remain immutable.
const V4_2_DEV_SCHEMA_VERSION: i64 = 6;

/// `PRAGMA application_id` of a database created before `V008` stamped it:
/// SQLite's default for a file nobody stamped.
const UNSTAMPED_APPLICATION_ID: i64 = 0;

fn fixture_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("v4_2_dev_migrated.db")
}

/// First external address of the Standard BIP44 account 0, derived from fixed
/// bytes so the UTXO lands on a real, script-round-trippable address.
fn first_external_info(byte: u8) -> key_wallet::AddressInfo {
    use key_wallet::managed_account::address_pool::AddressPoolType;
    let wallet = Wallet::from_seed_bytes(
        [byte; 64],
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
            return infos.first().cloned().unwrap();
        }
    }
    panic!("wallet must expose a non-empty Standard BIP44 external pool");
}

/// A chain-locked transaction record at height 200, so the migrated store can
/// be asserted to preserve both the height column and the blob's own context.
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
        id: Identifier::from(FIXTURE_IDENTITY),
        balance: 42,
        revision: 1,
        identity_index: Some(0),
        last_updated_balance_block_time: None,
        last_synced_keys_block_time: None,
        dpns_names: Vec::new(),
        contested_dpns_names: Vec::new(),
        status: IdentityStatus::Active,
        wallet_id: Some(wid(FIXTURE_WALLET)),
        dashpay_profile: None,
        dashpay_payments: Default::default(),
        contact_profiles: Default::default(),
        ignored_senders: Default::default(),
    }
}

/// Insert rows shaped the way a `v4.2-dev` writer would have left them.
///
/// Raw SQL against the V001-V006 schema: the branch's writers target tables
/// that do not exist yet at this point in history (`wallets`,
/// `core_address_pool`), so they cannot be used. The blob encoders CAN --
/// `key-wallet` is pinned to the same revision on both sides and
/// `AccountRegistrationEntry` is unchanged, so these bytes are exactly what a
/// `v4.2-dev` build would have written.
fn seed_base_shaped_rows(conn: &rusqlite::Connection) {
    use rusqlite::params;

    let wallet = wid(FIXTURE_WALLET);
    // The legacy label is retained; its blob identifies the exact variant.
    let key_wallet = Wallet::from_seed_bytes(
        [FIXTURE_WALLET; 64],
        Network::Testnet,
        WalletAccountCreationOptions::Default,
    )
    .unwrap();
    let registration = AccountRegistrationEntry {
        account_type: AccountType::Standard {
            index: 0,
            standard_account_type: StandardAccountType::BIP44Account,
        },
        account_xpub: key_wallet
            .accounts
            .account_of_type(AccountType::Standard {
                index: 0,
                standard_account_type: StandardAccountType::BIP44Account,
            })
            .unwrap()
            .account_xpub,
    };
    let registration_blob = blob::encode(&registration).expect("encode registration");
    let identity_blob = blob::encode(&identity_entry()).expect("encode identity");

    conn.execute(
        "INSERT INTO wallet_metadata (wallet_id, network, birth_height) \
         VALUES (?1, 'testnet', 0)",
        params![wallet.as_slice()],
    )
    .expect("insert wallet_metadata");
    conn.execute(
        "INSERT INTO account_registrations \
             (wallet_id, account_type, account_index, account_xpub_bytes) \
         VALUES (?1, 'standard', 0, ?2)",
        params![wallet.as_slice(), registration_blob],
    )
    .expect("insert account_registrations");
    // Real public pool state from the published writer's bincode-serde codec.
    let mut info = first_external_info(FIXTURE_WALLET);
    info.state = key_wallet::managed_account::address_pool::AddressState::Used;
    let snapshot = platform_wallet::changeset::AccountAddressPoolEntry {
        account_type: registration.account_type,
        pool_type: key_wallet::managed_account::address_pool::AddressPoolType::External,
        addresses: vec![info.clone()],
    };
    let snapshot = bincode::serde::encode_to_vec(snapshot, bincode::config::standard()).unwrap();
    conn.execute(
        "INSERT INTO account_address_pools \
             (wallet_id, account_type, account_index, pool_type, snapshot_blob) \
         VALUES (?1, 'standard', 0, 'external', ?2)",
        params![wallet.as_slice(), snapshot],
    )
    .expect("insert account_address_pools");
    conn.execute(
        "INSERT INTO core_derived_addresses \
             (wallet_id, account_type, account_index, address, derivation_path, used) \
         VALUES (?1, 'standard', 0, ?2, 'external/0', 1)",
        params![wallet.as_slice(), info.address.to_string()],
    )
    .expect("insert core_derived_addresses");
    conn.execute(
        "INSERT INTO core_sync_state (wallet_id, last_processed_height, synced_height) \
         VALUES (?1, 100, 100)",
        params![wallet.as_slice()],
    )
    .expect("insert core_sync_state");
    // No `identity_keys` row here: a valid `public_key_blob` is an encoded
    // `IdentityKeyWire`, and a placeholder would fail the load path rather
    // than the migration. V008's `identity_keys` rebuild and its wallet-scope
    // backfill are covered directly by
    // `tc048_v007_backfills_identity_key_wallet_scope`.
    conn.execute(
        "INSERT INTO identities (identity_id, wallet_id, wallet_index, entry_blob, tombstoned) \
         VALUES (?1, ?2, 0, ?3, 0)",
        params![
            FIXTURE_IDENTITY.as_slice(),
            wallet.as_slice(),
            identity_blob
        ],
    )
    .expect("insert identities");

    // A second, bare wallet: cross-wallet isolation during the reshape is only
    // observable when more than one wallet is present.
    let empty = wid(EMPTY_WALLET);
    conn.execute(
        "INSERT INTO wallet_metadata (wallet_id, network, birth_height) \
         VALUES (?1, 'testnet', 0)",
        params![empty.as_slice()],
    )
    .expect("insert empty wallet");

    // A confirmed transaction and the UTXO it paid, on a real derived address
    // so the migrated store's used-address set resolves a script to an address.
    let address = first_external_info(FIXTURE_WALLET).address;
    let record = one_tx_record();
    let record_blob = blob::encode(&record).expect("encode transaction record");
    let txid = [0x7Eu8; 32];
    conn.execute(
        "INSERT INTO core_transactions \
             (wallet_id, txid, height, block_hash, block_time, finalized, record_blob) \
         VALUES (?1, ?2, 200, ?3, 1735689600, 1, ?4)",
        params![
            wallet.as_slice(),
            txid.as_slice(),
            [0x03u8; 32].as_slice(),
            record_blob
        ],
    )
    .expect("insert core_transactions");
    let outpoint = blob::encode_outpoint(&dashcore::OutPoint {
        txid: <dashcore::Txid as dashcore::hashes::Hash>::from_byte_array(txid),
        vout: 0,
    })
    .expect("encode outpoint");
    conn.execute(
        "INSERT INTO core_utxos \
             (wallet_id, outpoint, value, script, height, account_index, spent, spent_in_txid) \
         VALUES (?1, ?2, 150000, ?3, 200, 0, 0, NULL)",
        params![
            wallet.as_slice(),
            outpoint,
            address.script_pubkey().as_bytes()
        ],
    )
    .expect("insert core_utxos");

    // An established contact carries BOTH request blobs; the reader decodes
    // each one, so a NULL would fail the load rather than the migration.
    let contact_id = [0xD2u8; 32];
    let request = |sender: [u8; 32], recipient: [u8; 32]| ContactRequest {
        sender_id: Identifier::from(sender),
        recipient_id: Identifier::from(recipient),
        sender_key_index: 0,
        recipient_key_index: 0,
        account_reference: 0,
        encrypted_account_label: None,
        encrypted_public_key: Vec::new(),
        auto_accept_proof: None,
        core_height_created_at: 200,
        created_at: 0,
    };
    let outgoing = blob::encode(&request(FIXTURE_IDENTITY, contact_id)).expect("encode outgoing");
    let incoming = blob::encode(&request(contact_id, FIXTURE_IDENTITY)).expect("encode incoming");
    conn.execute(
        "INSERT INTO contacts \
             (wallet_id, owner_id, contact_id, state, outgoing_request, incoming_request) \
         VALUES (?1, ?2, ?3, 'established', ?4, ?5)",
        params![
            wallet.as_slice(),
            FIXTURE_IDENTITY.as_slice(),
            contact_id.as_slice(),
            outgoing,
            incoming
        ],
    )
    .expect("insert contacts");
}

/// Rebuild the committed fixture from a database created by `v4.2-dev`'s own
/// binary. Ignored by default: it needs that database, named by `V4_2_DEV_DB`.
#[test]
#[ignore]
fn regenerate_v4_2_dev_fixture() {
    let source = std::env::var("V4_2_DEV_DB").expect(
        "set V4_2_DEV_DB to a database created by v4.2-dev's own shipped binary \
         (see this file's module docs); it must NOT be generated by this crate",
    );
    let source = PathBuf::from(source);

    let conn = rusqlite::Connection::open(&source).expect("open source database");
    let max_version: i64 = conn
        .query_row(
            "SELECT MAX(version) FROM refinery_schema_history",
            [],
            |r| r.get(0),
        )
        .expect("read schema history");
    assert_eq!(
        max_version, V4_2_DEV_SCHEMA_VERSION,
        "V4_2_DEV_DB is not a v4.2-dev database: schema history tops out at {max_version}"
    );
    seed_base_shaped_rows(&conn);
    conn.execute_batch("VACUUM;").expect("vacuum");
    drop(conn);

    std::fs::copy(&source, fixture_path()).expect("copy fixture into place");
}

/// Always-run guard keeping the committed fixture honest: it must still look
/// like a `v4.2-dev` database rather than something this branch produced.
#[test]
fn v4_2_dev_fixture_is_present_and_shaped_like_base() {
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

    let max_version: i64 = conn
        .query_row(
            "SELECT MAX(version) FROM refinery_schema_history",
            [],
            |r| r.get(0),
        )
        .expect("read schema history");
    assert_eq!(
        max_version, V4_2_DEV_SCHEMA_VERSION,
        "fixture must sit at the published v4.2-dev schema, not a later one"
    );

    let application_id: i64 = conn
        .query_row("PRAGMA application_id", [], |r| r.get(0))
        .expect("read application_id");
    assert_eq!(
        application_id, UNSTAMPED_APPLICATION_ID,
        "fixture must be unstamped; a stamped file did not come from v4.2-dev"
    );

    // The three tables V008 retires must still be present, or the fixture is
    // not exercising the reshape at all.
    for table in [
        "wallet_metadata",
        "account_address_pools",
        "core_derived_addresses",
    ] {
        let found: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = ?1",
                [table],
                |r| r.get(0),
            )
            .expect("probe sqlite_master");
        assert_eq!(
            found, 1,
            "fixture must still carry the pre-reshape `{table}`"
        );
    }

    let legacy_standard_rows: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM account_registrations WHERE account_type = 'standard'",
            [],
            |r| r.get(0),
        )
        .expect("count legacy standard rows");
    assert_eq!(
        legacy_standard_rows, 1,
        "fixture must carry the legacy `standard` row the remap rewrites"
    );
}
