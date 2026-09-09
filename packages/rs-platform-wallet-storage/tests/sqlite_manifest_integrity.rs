#![allow(clippy::field_reassign_with_default)]

//! Manifest corruption aborts strict loading and isolates the affected wallet
//! during recovery, including after reopening. Backup restores remain valid.

mod common;

use common::{ensure_wallet_meta, fresh_persister, wid};
use key_wallet::account::AccountType;
use key_wallet::wallet::initialization::WalletAccountCreationOptions;
use key_wallet::wallet::Wallet;
use platform_wallet::changeset::{
    AccountRegistrationEntry, PlatformWalletChangeSet, PlatformWalletPersistence,
};
use platform_wallet::wallet::platform_wallet::WalletId;
use platform_wallet_storage::{
    LoadPolicy, SqlitePersister, SqlitePersisterConfig, WalletStorageError,
};
use rusqlite::params;

/// A distinct, real extended public key per `seed` byte.
fn xpub_from_seed(seed: u8) -> key_wallet::bip32::ExtendedPubKey {
    Wallet::from_seed_bytes(
        [seed; 64],
        key_wallet::Network::Testnet,
        WalletAccountCreationOptions::Default,
    )
    .expect("wallet")
    .accounts
    .all_accounts()
    .first()
    .expect("account")
    .account_xpub
}

/// Persist one valid `platform_payment` registration under `w` (with its
/// `wallets` parent row), through the production writer so the checksum lands.
fn store_valid_manifest(persister: &SqlitePersister, w: WalletId) {
    ensure_wallet_meta(persister, &w);
    let manifest = vec![AccountRegistrationEntry {
        account_type: AccountType::PlatformPayment {
            account: 0,
            key_class: 0,
        },
        account_xpub: xpub_from_seed(w[0]),
    }];
    persister
        .store(
            w,
            PlatformWalletChangeSet {
                account_registrations: manifest,
                ..Default::default()
            },
        )
        .expect("store manifest");
}

fn assert_manifest_skip(persister: &SqlitePersister, wallet: WalletId) {
    let degradation = persister.last_load_degradation();
    assert_eq!(degradation.wallets_degraded.len(), 1);
    assert_eq!(
        degradation.wallets_degraded.get(&wallet),
        Some(&"manifest_integrity_mismatch")
    );
}

fn recover(persister: SqlitePersister, path: &std::path::Path) -> SqlitePersister {
    let err = persister
        .load()
        .expect_err("strict load must reject corruption");
    let platform_wallet::changeset::PersistenceError::Backend { source, .. } = err else {
        panic!("expected typed backend error");
    };
    assert!(matches!(
        source.downcast_ref::<WalletStorageError>(),
        Some(WalletStorageError::ManifestIntegrityMismatch)
    ));
    drop(persister);
    SqlitePersister::open(SqlitePersisterConfig::new(path).with_load_policy(LoadPolicy::Recovery))
        .expect("open recovery")
}

fn reopen(path: &std::path::Path) -> SqlitePersister {
    SqlitePersister::open(SqlitePersisterConfig::new(path)).expect("reopen")
}

/// A valid checksum loads its wallet without degradation.
#[test]
fn tc_c_002_valid_checksum_loads() {
    let (persister, _tmp, path) = fresh_persister();
    let w = wid(0x02);
    store_valid_manifest(&persister, w);
    drop(persister);

    let p2 = reopen(&path);
    let state = p2.load().expect("load");
    assert!(state.wallets.contains_key(&w), "clean wallet must load");
    assert!(
        !p2.last_load_degradation().degraded,
        "no skip on a valid checksum"
    );
    assert!(
        !state.wallets[&w]
            .wallet_info
            .accounts
            .platform_payment_accounts
            .is_empty(),
        "manifest round-trips"
    );
}

/// TC-C-003 — a blob mutated in place leaves the stored checksum stale; the
/// wallet is skipped (with `ManifestIntegrityMismatch`) and no panic occurs.
#[test]
fn tc_c_003_tampered_blob_is_skipped() {
    let (persister, _tmp, path) = fresh_persister();
    let w = wid(0x03);
    store_valid_manifest(&persister, w);

    // Replace the blob with different BLOB-typed bytes, leaving `checksum`
    // stale. `x'..'` keeps BLOB affinity (unlike `||`, which coerces to TEXT).
    {
        let conn = persister.lock_conn_for_test();
        conn.execute(
            "UPDATE account_registrations \
             SET account_xpub_bytes = x'00112233445566778899' WHERE wallet_id = ?1",
            params![w.as_slice()],
        )
        .unwrap();
    }

    let persister = recover(persister, &path);
    let state = persister
        .load()
        .expect("load must not error on a tampered row");
    assert!(
        !state.wallets.contains_key(&w),
        "tampered wallet must not load"
    );
    assert_manifest_skip(&persister, w);
}

/// TC-C-004 (Risk-6 core) — a row copied verbatim (blob + checksum) under a
/// DIFFERENT `wallet_id` fails the recompute over the new id and is skipped,
/// while the original wallet still loads.
#[test]
fn tc_c_004_wrong_wallet_row_is_skipped() {
    let (persister, _tmp, path) = fresh_persister();
    let w1 = wid(0x41);
    let w2 = wid(0x42);
    store_valid_manifest(&persister, w1);

    {
        let conn = persister.lock_conn_for_test();
        // w2's parent row.
        conn.execute(
            "INSERT INTO wallets (wallet_id, network, birth_height) VALUES (?1, 'testnet', 0)",
            params![w2.as_slice()],
        )
        .unwrap();
        // Copy w1's row under w2, blob + checksum verbatim: the checksum was
        // bound to w1, so recompute over w2 mismatches.
        conn.execute(
            "INSERT INTO account_registrations \
                (wallet_id, account_type, account_index, key_class, \
                 user_identity_id, friend_identity_id, account_xpub_bytes, checksum) \
             SELECT ?1, account_type, account_index, key_class, \
                 user_identity_id, friend_identity_id, account_xpub_bytes, checksum \
             FROM account_registrations WHERE wallet_id = ?2",
            params![w2.as_slice(), w1.as_slice()],
        )
        .unwrap();
    }

    let persister = recover(persister, &path);
    let state = persister.load().expect("load");
    assert!(state.wallets.contains_key(&w1), "w1 must load");
    assert!(
        !state.wallets.contains_key(&w2),
        "w2 (wrong-wallet) skipped"
    );
    assert_manifest_skip(&persister, w2);
}

/// A missing checksum fails strict loading and excludes the wallet in recovery,
/// including after the database is reopened.
#[test]
fn tc_c_005_null_checksum_is_skipped() {
    let (persister, _tmp, path) = fresh_persister();
    let w = wid(0x05);
    store_valid_manifest(&persister, w);

    {
        let conn = persister.lock_conn_for_test();
        conn.execute(
            "UPDATE account_registrations SET checksum = NULL WHERE wallet_id = ?1",
            params![w.as_slice()],
        )
        .unwrap();
    }

    let persister = recover(persister, &path);
    let state = persister.load().expect("load");
    assert!(
        !state.wallets.contains_key(&w),
        "NULL-checksum wallet skipped"
    );
    assert_manifest_skip(&persister, w);
}

/// TC-C-006 — batch isolation: one tampered wallet + one clean wallet; the
/// clean one loads, the tampered one is skipped, the batch does not abort.
#[test]
fn tc_c_006_combined_batch_isolates_the_bad_wallet() {
    let (persister, _tmp, path) = fresh_persister();
    let clean = wid(0x61);
    let bad = wid(0x62);
    store_valid_manifest(&persister, clean);
    store_valid_manifest(&persister, bad);

    {
        let conn = persister.lock_conn_for_test();
        conn.execute(
            "UPDATE account_registrations \
             SET account_xpub_bytes = x'00112233445566778899' WHERE wallet_id = ?1",
            params![bad.as_slice()],
        )
        .unwrap();
    }

    let persister = recover(persister, &path);
    let state = persister.load().expect("load");
    assert!(state.wallets.contains_key(&clean), "clean wallet loads");
    assert!(!state.wallets.contains_key(&bad), "bad wallet skipped");
    assert_manifest_skip(&persister, bad);
}

/// TC-C-008 (lead-mandated) — a backup restore must NOT false-positive as
/// tampered. The manifest rows (blob + checksum) copy verbatim while only the
/// store generation rotates, so the checksum — which ignores the generation —
/// still verifies and the wallet loads cleanly.
#[test]
fn tc_c_008_restore_does_not_false_positive() {
    let (persister, tmp, path) = fresh_persister();
    let w = wid(0x08);
    store_valid_manifest(&persister, w);

    let backup_path = persister.backup_to(tmp.path()).expect("backup");
    drop(persister);

    SqlitePersister::restore_from_skip_backup(&path, &backup_path).expect("restore");

    let p2 = reopen(&path);
    let state = p2.load().expect("load after restore");
    assert!(
        state.wallets.contains_key(&w),
        "restored wallet must load — no false positive"
    );
    assert!(
        !p2.last_load_degradation().degraded,
        "restore must not trip the integrity checksum (generation rotation is ignored)"
    );
    drop(p2);
    drop(tmp);
}

#[test]
fn migration_backfills_existing_manifests_atomically() {
    use platform_wallet_storage::sqlite::{migrations, schema::accounts};
    use sha2::{Digest, Sha256};
    let mut conn = rusqlite::Connection::open_in_memory().unwrap();
    migrations::runner()
        .set_target(refinery::Target::Version(17))
        .run(&mut conn)
        .unwrap();
    let w = wid(0x19);
    conn.execute(
        "INSERT INTO wallets (wallet_id, network, birth_height) VALUES (?1, 'testnet', 0)",
        params![w.as_slice()],
    )
    .unwrap();
    let payload = b"legacy manifest bytes";
    conn.execute("INSERT INTO account_registrations (wallet_id, account_type, account_index, key_class, user_identity_id, friend_identity_id, account_xpub_bytes) VALUES (?1, 'platform_payment', 0, 0, zeroblob(32), zeroblob(32), ?2)", params![w.as_slice(), payload.as_slice()]).unwrap();
    migrations::run(&mut conn).unwrap();
    let checksum: Vec<u8> = conn
        .query_row("SELECT checksum FROM account_registrations", [], |row| {
            row.get(0)
        })
        .unwrap();
    let expected = Sha256::new()
        .chain_update(w)
        .chain_update(payload)
        .finalize();
    assert_eq!(checksum.as_slice(), expected.as_slice());
    accounts::verify_manifest_checksums(&conn, &w).unwrap();
    migrations::run(&mut conn).unwrap();
    accounts::verify_manifest_checksums(&conn, &w).unwrap();
}

#[test]
fn provider_manifest_checksum_is_written_and_corruption_is_isolated() {
    use platform_wallet::changeset::{ProviderKeyAccountEntry, ProviderKeyExtendedPubKey};
    let (persister, _tmp, path) = fresh_persister();
    let w = wid(0x20);
    let wallet = Wallet::from_seed_bytes(
        [0x20; 64],
        key_wallet::Network::Testnet,
        WalletAccountCreationOptions::Default,
    )
    .unwrap();
    let entry = ProviderKeyAccountEntry {
        account_type: AccountType::ProviderOperatorKeys,
        extended_public_key: ProviderKeyExtendedPubKey::Bls(
            wallet
                .accounts
                .bls_account_of_type(AccountType::ProviderOperatorKeys)
                .unwrap()
                .bls_public_key
                .clone(),
        ),
    };
    ensure_wallet_meta(&persister, &w);
    persister
        .store(
            w,
            PlatformWalletChangeSet {
                provider_key_account_registrations: vec![entry],
                ..Default::default()
            },
        )
        .unwrap();
    {
        let conn = persister.lock_conn_for_test();
        platform_wallet_storage::sqlite::schema::accounts::verify_manifest_checksums(&conn, &w)
            .unwrap();
        conn.execute(
            "UPDATE account_registrations SET checksum = zeroblob(32) WHERE wallet_id = ?1",
            params![w.as_slice()],
        )
        .unwrap();
    }
    let persister = recover(persister, &path);
    let state = persister.load().unwrap();
    assert!(!state.wallets.contains_key(&w));
    assert_manifest_skip(&persister, w);
}
