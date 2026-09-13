//! Populated pre-rehydration databases must retain their typed state on upgrade.

mod common;

use key_wallet::account::{AccountType, StandardAccountType};
use key_wallet::managed_account::address_pool::{AddressPoolType, AddressState};
use key_wallet::wallet::initialization::WalletAccountCreationOptions;
use key_wallet::wallet::managed_wallet_info::ManagedWalletInfo;
use key_wallet::wallet::Wallet;
use key_wallet::{AddressInfo, Network};
use platform_wallet::changeset::{
    AccountAddressPoolEntry, AccountRegistrationEntry, PlatformWalletPersistence,
};
use platform_wallet_storage::sqlite::{migrations, schema::blob};
use platform_wallet_storage::{SqlitePersister, SqlitePersisterConfig};
use rusqlite::{params, Connection};

const WALLET_ID: [u8; 32] = [0x61; 32];

type StoredPoolRow = (String, u32, Vec<u8>, bool, Option<Vec<u8>>, Option<i64>);

fn standard_type() -> AccountType {
    AccountType::Standard {
        index: 0,
        standard_account_type: StandardAccountType::BIP44Account,
    }
}

fn legacy_database(
    path: &std::path::Path,
) -> (Connection, AccountRegistrationEntry, Vec<AddressInfo>) {
    let mut conn = Connection::open(path).unwrap();
    conn.pragma_update(None, "foreign_keys", true).unwrap();
    migrations::runner()
        .set_target(refinery::Target::Version(7))
        .run(&mut conn)
        .unwrap();
    conn.execute(
        "INSERT INTO wallet_metadata (wallet_id, network, birth_height) VALUES (?1, 'testnet', 0)",
        params![WALLET_ID.as_slice()],
    )
    .unwrap();
    let wallet = Wallet::from_seed_bytes(
        [0x61; 64],
        Network::Testnet,
        WalletAccountCreationOptions::Default,
    )
    .unwrap();
    let entry = AccountRegistrationEntry {
        account_type: standard_type(),
        account_xpub: wallet
            .accounts
            .account_of_type(standard_type())
            .unwrap()
            .account_xpub,
    };
    insert_registration(&conn, &WALLET_ID, "standard", 0, &entry);
    let managed = ManagedWalletInfo::from_wallet(&wallet, 0);
    let mut addresses = managed
        .all_managed_accounts()
        .into_iter()
        .find(|account| account.managed_account_type().to_account_type() == standard_type())
        .unwrap()
        .managed_account_type()
        .address_pools()
        .into_iter()
        .find(|pool| pool.pool_type == AddressPoolType::External)
        .unwrap()
        .addresses
        .values()
        .cloned()
        .collect::<Vec<_>>();
    addresses.sort_by_key(|info| info.index);
    (conn, entry, addresses)
}

fn insert_registration(
    conn: &Connection,
    wallet_id: &[u8; 32],
    label: &str,
    index: u32,
    entry: &AccountRegistrationEntry,
) {
    conn.execute(
        "INSERT INTO account_registrations (wallet_id, account_type, account_index, account_xpub_bytes) VALUES (?1, ?2, ?3, ?4)",
        params![wallet_id.as_slice(), label, index, blob::encode(entry).unwrap()],
    ).unwrap();
}

fn snapshot_bytes(entry: &AccountAddressPoolEntry) -> Vec<u8> {
    // The base backend encoded this public-only type with bincode-serde.
    bincode::serde::encode_to_vec(entry, bincode::config::standard()).unwrap()
}

fn insert_pool(conn: &Connection, addresses: Vec<AddressInfo>) -> Vec<u8> {
    let bytes = snapshot_bytes(&AccountAddressPoolEntry {
        account_type: standard_type(),
        pool_type: AddressPoolType::External,
        addresses,
    });
    conn.execute(
        "INSERT INTO account_address_pools (wallet_id, account_type, account_index, pool_type, snapshot_blob) VALUES (?1, 'standard', 0, 'external', ?2)",
        params![WALLET_ID.as_slice(), &bytes],
    ).unwrap();
    bytes
}

fn history(conn: &Connection) -> Vec<(i64, String, String, String)> {
    conn.prepare(
        "SELECT version, name, applied_on, checksum FROM refinery_schema_history ORDER BY version",
    )
    .unwrap()
    .query_map([], |row| {
        Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
    })
    .unwrap()
    .collect::<Result<_, _>>()
    .unwrap()
}

#[test]
fn should_backfill_legacy_account_discriminators_and_load_strictly() {
    let dir = common::secure_tempdir().unwrap();
    let path = dir.path().join("legacy.db");
    let (conn, _, _) = legacy_database(&path);
    let mut wallet = Wallet::from_seed_bytes(
        [0x61; 64],
        Network::Testnet,
        WalletAccountCreationOptions::Default,
    )
    .unwrap();
    let contact_type = AccountType::DashpayReceivingFunds {
        index: 4,
        user_identity_id: [0x21; 32],
        friend_identity_id: [0x22; 32],
    };
    let platform_type = AccountType::PlatformPayment {
        account: 3,
        key_class: 1,
    };
    wallet.add_account(contact_type, None).unwrap();
    wallet.add_account(platform_type, None).unwrap();
    let contact = AccountRegistrationEntry {
        account_type: contact_type,
        account_xpub: wallet
            .accounts
            .account_of_type(contact_type)
            .unwrap()
            .account_xpub,
    };
    let platform = AccountRegistrationEntry {
        account_type: platform_type,
        account_xpub: wallet
            .accounts
            .account_of_type(platform_type)
            .unwrap()
            .account_xpub,
    };
    insert_registration(&conn, &WALLET_ID, "dashpay_receiving", 4, &contact);
    insert_registration(&conn, &WALLET_ID, "platform_payment", 3, &platform);
    let original_history = history(&conn);
    drop(conn);

    let persister = SqlitePersister::open(SqlitePersisterConfig::new(&path)).unwrap();
    let conn = persister.lock_conn_for_test();
    let (user, friend): (Vec<u8>, Vec<u8>) = conn.query_row(
        "SELECT user_identity_id, friend_identity_id FROM account_registrations WHERE account_type = 'dashpay_receiving'",
        [], |row| Ok((row.get(0)?, row.get(1)?)),
    ).unwrap();
    assert_eq!(
        user, [0x21; 32],
        "migration must recover the actual contact owner"
    );
    assert_eq!(friend, [0x22; 32]);
    let class: u32 = conn
        .query_row(
            "SELECT key_class FROM account_registrations WHERE account_type = 'platform_payment'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(class, 1);
    assert_eq!(&history(&conn)[..7], original_history);
    drop(conn);
    assert!(persister.load().unwrap().wallets.contains_key(&WALLET_ID));
}

#[test]
fn should_preserve_legacy_used_reserved_public_keys_and_derived_only_rows() {
    let dir = common::secure_tempdir().unwrap();
    let path = dir.path().join("legacy.db");
    let (conn, _, mut addresses) = legacy_database(&path);
    addresses[0].state = AddressState::Used;
    addresses[1].state = AddressState::Reserved { at: 1_700_000_000 };
    insert_pool(&conn, addresses[..2].to_vec());
    conn.execute(
        "INSERT INTO core_derived_addresses (wallet_id, account_type, account_index, address, derivation_path, used) VALUES (?1, 'standard', 0, ?2, 'external/2', 1)",
        params![WALLET_ID.as_slice(), addresses[2].address.to_string()],
    ).unwrap();
    drop(conn);
    let persister = SqlitePersister::open(SqlitePersisterConfig::new(&path)).unwrap();
    {
        let conn = persister.lock_conn_for_test();
        let count: u32 = conn
            .query_row("SELECT count(*) FROM core_address_pool", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(
            count, 3,
            "used snapshots and derived-only rows must survive"
        );
        for index in 0u32..3 {
            let (label, account, script, used, key, reserved): StoredPoolRow = conn.query_row(
                "SELECT account_type, account_index, script, used, public_key, reserved_at FROM core_address_pool WHERE wallet_id = ?1 AND address_index = ?2",
                params![WALLET_ID.as_slice(), index], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?, row.get(5)?)),
            ).unwrap();
            assert_eq!(label, "standard_bip44");
            assert_eq!(account, 0);
            assert_eq!(script, addresses[index as usize].script_pubkey.to_bytes());
            assert_eq!(used, index != 1);
            if index < 2 {
                let key_wallet::managed_account::address_pool::PublicKeyType::ECDSA(expected) =
                    addresses[index as usize].public_key.as_ref().unwrap()
                else {
                    panic!("ECDSA fixture")
                };
                assert_eq!(key.as_ref(), Some(expected));
            }
            assert_eq!(reserved, (index == 1).then_some(1_700_000_000));
        }
    }
    drop(persister);
    let reopened = SqlitePersister::open(SqlitePersisterConfig::new(&path)).unwrap();
    let conn = reopened.lock_conn_for_test();
    let count: u32 = conn
        .query_row("SELECT count(*) FROM core_address_pool", [], |row| {
            row.get(0)
        })
        .unwrap();
    assert_eq!(count, 3, "reopening must be idempotent");
}

#[test]
fn should_leave_legacy_data_and_history_unchanged_on_malformed_pool() {
    let dir = common::secure_tempdir().unwrap();
    let path = dir.path().join("legacy.db");
    let (conn, _, addresses) = legacy_database(&path);
    let good_blob = insert_pool(&conn, addresses[..2].to_vec());
    conn.execute("UPDATE account_address_pools SET snapshot_blob = X'00'", [])
        .unwrap();
    let original_history = history(&conn);
    drop(conn);
    assert!(
        SqlitePersister::open(SqlitePersisterConfig::new(&path)).is_err(),
        "a malformed legacy pool must not be silently discarded"
    );
    let conn = Connection::open(&path).unwrap();
    assert_eq!(history(&conn), original_history);
    let stored: Vec<u8> = conn
        .query_row(
            "SELECT snapshot_blob FROM account_address_pools",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(stored, [0]);
    let backups = std::fs::read_dir(dir.path().join("backups/auto"))
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .collect::<Vec<_>>();
    assert_eq!(backups.len(), 1);
    let backup = Connection::open(&backups[0]).unwrap();
    assert_eq!(history(&backup), original_history);
    assert_eq!(
        backup
            .query_row(
                "SELECT snapshot_blob FROM account_address_pools",
                [],
                |row| row.get::<_, Vec<u8>>(0)
            )
            .unwrap(),
        [0]
    );
    conn.execute(
        "UPDATE account_address_pools SET snapshot_blob = ?1",
        params![good_blob],
    )
    .unwrap();
    drop(conn);
    // The failed attempt's backup remains intact. Use a separate destination
    // for the repaired retry, which may occur within the same timestamp second.
    let repaired = SqlitePersister::open(
        SqlitePersisterConfig::new(&path)
            .with_auto_backup_dir(Some(dir.path().join("retry-backups"))),
    )
    .unwrap();
    let conn = repaired.lock_conn_for_test();
    let count: u32 = conn
        .query_row("SELECT count(*) FROM core_address_pool", [], |row| {
            row.get(0)
        })
        .unwrap();
    assert_eq!(count, 2);
}

#[test]
fn should_resume_intermediate_targets_after_reopen_without_losing_staged_pools() {
    for target in [8, 9, 10, 11] {
        let dir = common::secure_tempdir().unwrap();
        let path = dir.path().join("legacy.db");
        let (mut conn, _, mut addresses) = legacy_database(&path);
        addresses[0].state = AddressState::Reserved { at: 1_700_000_001 };
        let original = insert_pool(&conn, addresses[..1].to_vec());
        let report = migrations::runner()
            .set_target(refinery::Target::Version(target))
            .run(&mut conn)
            .unwrap();
        assert_eq!(
            report
                .applied_migrations()
                .iter()
                .map(|m| m.version())
                .collect::<Vec<_>>(),
            (8..=target).collect::<Vec<_>>()
        );
        assert_eq!(history(&conn).last().unwrap().0, i64::from(target));
        if target < 11 {
            assert_eq!(
                conn.query_row(
                    "SELECT snapshot_blob FROM account_address_pools",
                    [],
                    |row| row.get::<_, Vec<u8>>(0)
                )
                .unwrap(),
                original
            );
        }
        drop(conn);
        let persister = SqlitePersister::open(SqlitePersisterConfig::new(&path)).unwrap();
        let conn = persister.lock_conn_for_test();
        let (count, reserved, key): (u32, i64, Vec<u8>) = conn
            .query_row(
                "SELECT count(*), reserved_at, public_key FROM core_address_pool",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .unwrap();
        assert_eq!((count, reserved), (1, 1_700_000_001));
        assert_eq!(key.len(), 33);
        assert_eq!(conn.query_row("SELECT count(*) FROM sqlite_master WHERE name IN ('account_address_pools', 'core_derived_addresses')", [], |row| row.get::<_, u32>(0)).unwrap(), 0);
    }
}

#[test]
fn should_preserve_derived_only_hardened_address_with_unambiguous_owner() {
    use key_wallet::account::derivation::AccountDerivation;
    let dir = common::secure_tempdir().unwrap();
    let path = dir.path().join("legacy.db");
    let (mut conn, _, _) = legacy_database(&path);
    let wallet = Wallet::from_seed_bytes(
        [0x61; 64],
        Network::Testnet,
        WalletAccountCreationOptions::Default,
    )
    .unwrap();
    let account = wallet
        .accounts
        .account_of_type(AccountType::IdentityRegistration)
        .unwrap();
    let xpriv = wallet
        .derive_extended_private_key(&account.derivation_path().unwrap())
        .unwrap();
    let address = account
        .derive_address_at(AddressPoolType::AbsentHardened, 3, Some(xpriv))
        .unwrap();
    assert_ne!(
        account
            .derive_address_at(AddressPoolType::AbsentHardened, 3, None)
            .unwrap(),
        address,
        "public derivation does not reproduce the hardened child"
    );
    insert_registration(
        &conn,
        &WALLET_ID,
        "identity_registration",
        0,
        &AccountRegistrationEntry {
            account_type: AccountType::IdentityRegistration,
            account_xpub: account.account_xpub,
        },
    );
    conn.execute("INSERT INTO core_derived_addresses VALUES (?1, 'identity_registration', 0, ?2, 'absent_hardened/3', 1)", params![WALLET_ID.as_slice(), address.to_string()]).unwrap();
    migrations::run(&mut conn).unwrap();
    let (label, index, script, used): (String, u32, Vec<u8>, bool) = conn
        .query_row(
            "SELECT account_type, address_index, script, used FROM core_address_pool",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .unwrap();
    assert_eq!(
        (label.as_str(), index, used),
        ("identity_registration", 3, true)
    );
    assert_eq!(script, address.script_pubkey().to_bytes());
}

#[test]
fn should_preserve_provider_public_key_snapshots_without_ecdsa_registrations() {
    use platform_wallet::wallet::provider_key_at_index::{
        derive_platform_node_public_keys, populate_platform_node_pool,
    };
    let dir = common::secure_tempdir().unwrap();
    let path = dir.path().join("legacy.db");
    let (mut conn, _, _) = legacy_database(&path);
    let wallet = Wallet::from_seed_bytes(
        [0x61; 64],
        Network::Testnet,
        WalletAccountCreationOptions::Default,
    )
    .unwrap();
    let keys = derive_platform_node_public_keys(&wallet, Network::Testnet, 3).unwrap();
    let mut managed = ManagedWalletInfo::from_wallet(&wallet, 0);
    populate_platform_node_pool(&mut managed, &keys, Network::Testnet).unwrap();
    let account = managed
        .all_managed_accounts()
        .into_iter()
        .find(|account| {
            account.managed_account_type().to_account_type() == AccountType::ProviderPlatformKeys
        })
        .unwrap();
    let pool = account
        .managed_account_type()
        .address_pools()
        .into_iter()
        .find(|pool| pool.pool_type == AddressPoolType::AbsentHardened)
        .unwrap();
    let mut addresses = pool.addresses.values().cloned().collect::<Vec<_>>();
    addresses.sort_by_key(|info| info.index);
    addresses[0].state = AddressState::Used;
    addresses[1].state = AddressState::Reserved { at: 1_700_000_002 };
    let bytes = snapshot_bytes(&AccountAddressPoolEntry {
        account_type: AccountType::ProviderPlatformKeys,
        pool_type: AddressPoolType::AbsentHardened,
        addresses,
    });
    conn.execute("INSERT INTO account_address_pools VALUES (?1, 'provider_platform', 0, 'absent_hardened', ?2)", params![WALLET_ID.as_slice(), bytes]).unwrap();
    migrations::run(&mut conn).unwrap();
    for key in keys {
        let (stored, kind): (Vec<u8>, u32) = conn.query_row("SELECT public_key, key_type FROM core_address_pool WHERE account_type = 'provider_platform' AND address_index = ?1", [key.index], |row| Ok((row.get(0)?, row.get(1)?))).unwrap();
        assert_eq!(stored, key.public_key);
        assert_eq!(kind, 1);
    }
    assert_eq!(
        conn.query_row(
            "SELECT reserved_at FROM core_address_pool WHERE address_index = 1",
            [],
            |row| row.get::<_, i64>(0)
        )
        .unwrap(),
        1_700_000_002
    );
}

#[test]
fn should_reject_inconsistent_registration_without_changing_history() {
    let dir = common::secure_tempdir().unwrap();
    let path = dir.path().join("legacy.db");
    let (mut conn, _, _) = legacy_database(&path);
    conn.execute("UPDATE account_registrations SET account_index = 1", [])
        .unwrap();
    let original = history(&conn);
    assert!(migrations::run(&mut conn).is_err());
    assert_eq!(history(&conn), original);
    assert_eq!(
        conn.query_row(
            "SELECT account_index FROM account_registrations",
            [],
            |row| row.get::<_, u32>(0)
        )
        .unwrap(),
        1
    );
}

#[test]
fn should_roll_back_converted_state_and_history_when_later_ddl_fails() {
    let dir = common::secure_tempdir().unwrap();
    let path = dir.path().join("legacy.db");
    let (mut conn, _, addresses) = legacy_database(&path);
    let original_blob = insert_pool(&conn, addresses[..2].to_vec());
    let original_history = history(&conn);
    conn.execute_batch("CREATE TABLE identity_scan_states (sentinel INTEGER)")
        .unwrap();
    let error = migrations::run(&mut conn).unwrap_err();
    assert!(
        error.report().is_none(),
        "rolled-back work must not be reported as applied"
    );
    assert_eq!(
        history(&conn),
        original_history,
        "later DDL failure must roll back the entire pending upgrade"
    );
    let stored: Vec<u8> = conn
        .query_row(
            "SELECT snapshot_blob FROM account_address_pools",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(stored, original_blob);
    conn.execute_batch("DROP TABLE identity_scan_states")
        .unwrap();
    migrations::run(&mut conn).unwrap();
    let count: u32 = conn
        .query_row("SELECT count(*) FROM core_address_pool", [], |row| {
            row.get(0)
        })
        .unwrap();
    assert_eq!(count, 2);
}

#[test]
fn should_preserve_orphan_sweep_policy_without_decoding_unreachable_blobs() {
    let dir = common::secure_tempdir().unwrap();
    let path = dir.path().join("legacy.db");
    let (mut conn, _, addresses) = legacy_database(&path);
    insert_pool(&conn, addresses[..1].to_vec());
    conn.pragma_update(None, "foreign_keys", false).unwrap();
    conn.execute(
        "INSERT INTO account_address_pools VALUES (?1, 'standard', 0, 'external', X'00')",
        params![[0x99u8; 32].as_slice()],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO account_registrations VALUES (?1, 'standard', 0, X'00')",
        params![[0x99u8; 32].as_slice()],
    )
    .unwrap();
    conn.pragma_update(None, "foreign_keys", true).unwrap();
    migrations::run(&mut conn).unwrap();
    assert_eq!(
        conn.query_row("SELECT count(*) FROM core_address_pool", [], |row| row
            .get::<_, u32>(0))
            .unwrap(),
        1
    );
}

#[test]
fn should_reject_conflicting_snapshot_slot_and_derived_row_atomically() {
    let dir = common::secure_tempdir().unwrap();
    let path = dir.path().join("legacy.db");
    let (mut conn, _, addresses) = legacy_database(&path);
    let bytes = insert_pool(&conn, addresses[..1].to_vec());
    conn.execute(
        "INSERT INTO core_derived_addresses VALUES (?1, 'standard', 0, ?2, 'external/0', 1)",
        params![WALLET_ID.as_slice(), addresses[1].address.to_string()],
    )
    .unwrap();
    let original = history(&conn);
    assert!(migrations::run(&mut conn).is_err());
    assert_eq!(history(&conn), original);
    assert_eq!(
        conn.query_row(
            "SELECT snapshot_blob FROM account_address_pools",
            [],
            |row| row.get::<_, Vec<u8>>(0)
        )
        .unwrap(),
        bytes
    );
}

#[test]
fn should_hold_writer_exclusion_before_validating_and_applying_history() {
    let dir = common::secure_tempdir().unwrap();
    let path = dir.path().join("legacy.db");
    let (mut conn, _, _) = legacy_database(&path);
    let mut other = Connection::open(&path).unwrap();
    other.busy_timeout(std::time::Duration::ZERO).unwrap();
    let tx = conn
        .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
        .unwrap();
    assert!(migrations::run(&mut other).is_err());
    assert_eq!(history(&tx).last().unwrap().0, 7);
    tx.commit().unwrap();
    let report = migrations::run(&mut other).unwrap();
    assert_eq!(report.applied_migrations().first().unwrap().version(), 8);
    assert!(migrations::run(&mut conn)
        .unwrap()
        .applied_migrations()
        .is_empty());
}
