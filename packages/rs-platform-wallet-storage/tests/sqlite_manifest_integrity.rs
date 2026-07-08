#![allow(clippy::field_reassign_with_default)]

//! Manifest integrity checksum (#3968): a tampered / mis-bound / NULL-checksum
//! `account_registrations` row becomes a per-wallet SKIP at `load()` (recorded
//! in `ClientStartState.skipped`), never a batch abort — while a clean wallet
//! in the same batch still loads. A backup restore must NOT false-positive.

mod common;

use common::{ensure_wallet_meta, fresh_persister, wid};
use key_wallet::account::AccountType;
use key_wallet::wallet::initialization::WalletAccountCreationOptions;
use key_wallet::wallet::Wallet;
use platform_wallet::changeset::{
    AccountRegistrationEntry, PlatformWalletChangeSet, PlatformWalletPersistence,
};
use platform_wallet::manager::load_outcome::{CorruptKind, SkipReason};
use platform_wallet::wallet::platform_wallet::WalletId;
use platform_wallet_storage::{SqlitePersister, SqlitePersisterConfig};
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

fn is_manifest_skip(reason: &SkipReason) -> bool {
    matches!(
        reason,
        SkipReason::CorruptPersistedRow {
            kind: CorruptKind::ManifestIntegrityMismatch,
        }
    )
}

fn reopen(path: &std::path::Path) -> SqlitePersister {
    SqlitePersister::open(SqlitePersisterConfig::new(path)).expect("reopen")
}

/// TC-C-002 — a valid checksum loads normally: the wallet appears in `wallets`
/// and `skipped` is empty.
#[test]
fn tc_c_002_valid_checksum_loads() {
    let (persister, _tmp, path) = fresh_persister();
    let w = wid(0x02);
    store_valid_manifest(&persister, w);
    drop(persister);

    let p2 = reopen(&path);
    let state = p2.load().expect("load");
    assert!(state.wallets.contains_key(&w), "clean wallet must load");
    assert!(state.skipped.is_empty(), "no skip on a valid checksum");
    assert!(
        !state.wallets[&w].account_manifest.is_empty(),
        "manifest round-trips"
    );
}

/// TC-C-003 — a blob mutated in place leaves the stored checksum stale; the
/// wallet is skipped (with `ManifestIntegrityMismatch`) and no panic occurs.
#[test]
fn tc_c_003_tampered_blob_is_skipped() {
    let (persister, _tmp, _path) = fresh_persister();
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

    let state = persister
        .load()
        .expect("load must not error on a tampered row");
    assert!(
        !state.wallets.contains_key(&w),
        "tampered wallet must not load"
    );
    assert_eq!(state.skipped.len(), 1);
    assert_eq!(state.skipped[0].0, w);
    assert!(is_manifest_skip(&state.skipped[0].1));
}

/// TC-C-004 (Risk-6 core) — a row copied verbatim (blob + checksum) under a
/// DIFFERENT `wallet_id` fails the recompute over the new id and is skipped,
/// while the original wallet still loads.
#[test]
fn tc_c_004_wrong_wallet_row_is_skipped() {
    let (persister, _tmp, _path) = fresh_persister();
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

    let state = persister.load().expect("load");
    assert!(state.wallets.contains_key(&w1), "w1 must load");
    assert!(
        !state.wallets.contains_key(&w2),
        "w2 (wrong-wallet) skipped"
    );
    assert_eq!(state.skipped.len(), 1);
    assert_eq!(state.skipped[0].0, w2);
    assert!(is_manifest_skip(&state.skipped[0].1));
}

/// TC-C-005 — a NULL checksum on a V004+ store is treated as corruption
/// (fail-closed) and skipped. The backfill guarantees NULLs never survive
/// `open()`, so this forces one post-open on the live connection.
#[test]
fn tc_c_005_null_checksum_is_skipped() {
    let (persister, _tmp, _path) = fresh_persister();
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

    let state = persister.load().expect("load");
    assert!(
        !state.wallets.contains_key(&w),
        "NULL-checksum wallet skipped"
    );
    assert_eq!(state.skipped.len(), 1);
    assert!(is_manifest_skip(&state.skipped[0].1));
}

/// TC-C-006 — batch isolation: one tampered wallet + one clean wallet; the
/// clean one loads, the tampered one is skipped, the batch does not abort.
#[test]
fn tc_c_006_combined_batch_isolates_the_bad_wallet() {
    let (persister, _tmp, _path) = fresh_persister();
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

    let state = persister.load().expect("load");
    assert!(state.wallets.contains_key(&clean), "clean wallet loads");
    assert!(!state.wallets.contains_key(&bad), "bad wallet skipped");
    assert_eq!(state.skipped.len(), 1);
    assert_eq!(state.skipped[0].0, bad);
    assert!(is_manifest_skip(&state.skipped[0].1));
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
        state.skipped.is_empty(),
        "restore must not trip the integrity checksum (generation rotation is ignored)"
    );
    drop(p2);
    drop(tmp);
}
