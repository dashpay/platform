#![allow(clippy::field_reassign_with_default)]

//! `schema::accounts::load_state` reads `account_registrations` rows back
//! into a keyless [`AccountRegistrationEntry`] manifest, bit-exact,
//! fail-hard on a corrupt blob, and never mints a `Wallet`.

mod common;

use common::{ensure_wallet_meta, fresh_persister, wid};
use key_wallet::account::{AccountType, StandardAccountType};
use platform_wallet::changeset::{
    AccountRegistrationEntry, PlatformWalletChangeSet, PlatformWalletPersistence,
};
use platform_wallet::wallet::platform_wallet::WalletId;
use platform_wallet_storage::sqlite::schema::{accounts, blob};
use platform_wallet_storage::{LoadCtx, LoadSite, SqlitePersister, WalletStorageError};

/// A distinct extended public key per `seed` byte, so a round-trip test can
/// tell entries apart instead of asserting against one shared xpub.
fn xpub_from_seed(seed: u8) -> key_wallet::bip32::ExtendedPubKey {
    use key_wallet::wallet::initialization::WalletAccountCreationOptions;
    use key_wallet::wallet::Wallet;
    let w = Wallet::from_seed_bytes(
        [seed; 64],
        key_wallet::Network::Testnet,
        WalletAccountCreationOptions::Default,
    )
    .expect("wallet");
    w.accounts
        .all_accounts()
        .first()
        .expect("at least one account")
        .account_xpub
}

fn reopen(path: &std::path::Path) -> platform_wallet_storage::SqlitePersister {
    platform_wallet_storage::SqlitePersister::open(
        platform_wallet_storage::SqlitePersisterConfig::new(path),
    )
    .expect("reopen persister")
}

/// Registrations round-trip bit-exact, in the reader's deterministic order
/// (`account_type` label ascending), with each entry keeping its OWN xpub.
#[test]
fn a1_account_registrations_roundtrip() {
    let (persister, _tmp, path) = fresh_persister();
    use platform_wallet::changeset::PlatformWalletPersistence;
    let w = wid(0xA1);
    ensure_wallet_meta(&persister, &w);

    // Distinct xpubs so the round-trip proves each entry keeps its own key,
    // not just that *some* xpub survives.
    let standard_xpub = xpub_from_seed(7);
    let idreg_xpub = xpub_from_seed(8);
    assert_ne!(standard_xpub, idreg_xpub, "fixtures must differ");

    let entries = vec![
        AccountRegistrationEntry {
            account_type: AccountType::Standard {
                index: 0,
                standard_account_type: key_wallet::account::StandardAccountType::BIP44Account,
            },
            account_xpub: standard_xpub,
        },
        AccountRegistrationEntry {
            account_type: AccountType::IdentityRegistration,
            account_xpub: idreg_xpub,
        },
    ];
    let cs = PlatformWalletChangeSet {
        account_registrations: entries.clone(),
        ..Default::default()
    };
    persister.store(w, cs).unwrap();
    drop(persister);

    let p2 = reopen(&path);
    let conn = p2.lock_conn_for_test();
    let manifest = accounts::load_state(&conn, &w, &platform_wallet_storage::LoadCtx::strict())
        .expect("load_state")
        .ecdsa;
    drop(conn);

    assert_eq!(manifest.len(), 2, "all rows must be returned");
    // Reader orders by `account_type` label: 'identity_registration' sorts
    // before 'standard_bip44', so the manifest is deterministically ordered.
    assert!(
        matches!(manifest[0].account_type, AccountType::IdentityRegistration),
        "identity_registration must sort first, got {:?}",
        manifest[0].account_type
    );
    assert_eq!(
        manifest[0].account_xpub, idreg_xpub,
        "IdentityRegistration must keep its own xpub"
    );
    assert!(
        matches!(
            manifest[1].account_type,
            AccountType::Standard { index: 0, .. }
        ),
        "standard_bip44 must sort second, got {:?}",
        manifest[1].account_type
    );
    assert_eq!(
        manifest[1].account_xpub, standard_xpub,
        "Standard must keep its own xpub"
    );
}

/// An empty wallet yields an empty manifest, not an error.
#[test]
fn a1_empty_manifest_is_ok() {
    let (persister, _tmp, path) = fresh_persister();
    let w = wid(0xA2);
    ensure_wallet_meta(&persister, &w);
    drop(persister);
    let p2 = reopen(&path);
    let conn = p2.lock_conn_for_test();
    let manifest = accounts::load_state(&conn, &w, &platform_wallet_storage::LoadCtx::strict())
        .expect("load_state")
        .ecdsa;
    drop(conn);
    assert!(manifest.is_empty());
}

/// A corrupt `account_xpub_bytes` blob is a typed hard error, never a
/// silent skip.
#[test]
fn a1_corrupt_blob_is_hard_error() {
    let (persister, _tmp, path) = fresh_persister();
    let w = wid(0xA3);
    ensure_wallet_meta(&persister, &w);
    {
        let conn = persister.lock_conn_for_test();
        conn.execute(
            "INSERT INTO account_registrations \
                (wallet_id, account_type, account_index, account_xpub_bytes) \
             VALUES (?1, 'standard_bip44', 0, X'00')",
            rusqlite::params![w.as_slice()],
        )
        .unwrap();
    }
    drop(persister);
    let p2 = reopen(&path);
    let conn = p2.lock_conn_for_test();
    let result = accounts::load_state(&conn, &w, &platform_wallet_storage::LoadCtx::strict());
    drop(conn);
    assert!(
        matches!(result, Err(WalletStorageError::BincodeDecode { .. })),
        "corrupt account_xpub_bytes must be a typed BincodeDecode; got {result:?}"
    );
}

/// A standard-account registration with a distinguishable xpub.
fn standard(index: u32, variant: StandardAccountType, xpub_seed: u8) -> AccountRegistrationEntry {
    AccountRegistrationEntry {
        account_type: AccountType::Standard {
            index,
            standard_account_type: variant,
        },
        account_xpub: xpub_from_seed(xpub_seed),
    }
}

fn store_registrations(
    persister: &SqlitePersister,
    wallet: WalletId,
    entries: &[AccountRegistrationEntry],
) {
    let cs = PlatformWalletChangeSet {
        account_registrations: entries.to_vec(),
        ..Default::default()
    };
    persister.store(wallet, cs).expect("store registrations");
}

/// Plant a raw row under an arbitrary `account_type` label. This is how a
/// pre-split `standard` row exists in a database migrated past the split: no
/// writer path can produce one, because the label is derived from the typed
/// `AccountType`.
fn plant_registration(
    persister: &SqlitePersister,
    wallet: &WalletId,
    label: &str,
    index: i64,
    entry: &AccountRegistrationEntry,
) {
    let payload = blob::encode(entry).expect("encode registration");
    let conn = persister.lock_conn_for_test();
    conn.execute(
        "INSERT INTO account_registrations \
            (wallet_id, account_type, account_index, account_xpub_bytes) \
         VALUES (?1, ?2, ?3, ?4)",
        rusqlite::params![wallet.as_slice(), label, index, payload],
    )
    .expect("plant a registration row");
}

fn ecdsa_manifest(
    persister: &SqlitePersister,
    wallet: &WalletId,
    ctx: &LoadCtx,
) -> Vec<AccountRegistrationEntry> {
    let conn = persister.lock_conn_for_test();
    accounts::load_state(&conn, wallet, ctx)
        .expect("load_state")
        .ecdsa
}

/// A pre-split `standard` row and the precise row a later save inserted beside
/// it are ONE account, and the reader returns it once. The writer cannot merge
/// them — its upsert keys on `account_type`, so the precise label is a
/// different primary key — so the reader owns the reconciliation. Emitting the
/// account twice would make this crate's manifest depend on a consumer it does
/// not own to dedup.
#[test]
fn a1_legacy_standard_row_collapses_into_its_precise_sibling() {
    let (persister, _tmp, _path) = fresh_persister();
    let w = wid(0xA4);
    ensure_wallet_meta(&persister, &w);

    let entry = standard(0, StandardAccountType::BIP44Account, 7);
    store_registrations(&persister, w, std::slice::from_ref(&entry));
    plant_registration(&persister, &w, "standard", 0, &entry);

    assert_eq!(
        ecdsa_manifest(&persister, &w, &LoadCtx::strict()),
        vec![entry],
        "the forked pair is one account and must collapse to the precise row"
    );
}

/// The pair is only a duplicate while both rows agree. The legacy row is never
/// updated — `DO UPDATE` targets the precise row alone — so a persisted xpub
/// change leaves two DIFFERENT accounts at one index. That is drift, not a
/// duplicate, and it goes through the same typed, policy-governed site as
/// every other blob-versus-column disagreement instead of being silently
/// resolved by preference.
#[test]
fn a1_legacy_standard_row_that_contradicts_its_sibling_is_typed_drift() {
    let (persister, _tmp, _path) = fresh_persister();
    let w = wid(0xA5);
    ensure_wallet_meta(&persister, &w);

    let current = standard(0, StandardAccountType::BIP44Account, 7);
    let stale = standard(0, StandardAccountType::BIP44Account, 8);
    assert_ne!(
        current.account_xpub, stale.account_xpub,
        "fixtures must differ"
    );
    store_registrations(&persister, w, std::slice::from_ref(&current));
    plant_registration(&persister, &w, "standard", 0, &stale);

    let err = {
        let conn = persister.lock_conn_for_test();
        accounts::load_state(&conn, &w, &LoadCtx::strict())
            .expect_err("a contradicting pair must be fatal under Strict")
    };
    assert!(
        matches!(err, WalletStorageError::AccountRegistrationEntryMismatch),
        "expected AccountRegistrationEntryMismatch, got {err:?}"
    );

    let ctx = LoadCtx::recovery();
    assert_eq!(
        ecdsa_manifest(&persister, &w, &ctx),
        vec![current],
        "recovery keeps the row the writer maintains, not the stale projection"
    );
    let degradation = ctx.degradation();
    assert_eq!(
        degradation.by_site.get(&LoadSite::AccountRegistrationDrift),
        Some(&1),
        "the tolerated pair must be counted at its own site"
    );
    assert_eq!(
        degradation.by_site.len(),
        1,
        "nothing else may be tolerated: {:?}",
        degradation.by_site
    );
}

/// The common case — no pre-split row anywhere — keeps every registration and
/// its order. Reconciliation is an edge case and must not leak into the path
/// every post-split database takes.
#[test]
fn a1_wallet_without_a_legacy_row_keeps_every_registration() {
    let (persister, _tmp, _path) = fresh_persister();
    let w = wid(0xA6);
    ensure_wallet_meta(&persister, &w);

    let bip44 = standard(0, StandardAccountType::BIP44Account, 7);
    let bip32 = standard(0, StandardAccountType::BIP32Account, 8);
    let idreg = AccountRegistrationEntry {
        account_type: AccountType::IdentityRegistration,
        account_xpub: xpub_from_seed(9),
    };
    store_registrations(
        &persister,
        w,
        &[bip44.clone(), bip32.clone(), idreg.clone()],
    );

    // Label order: 'identity_registration' < 'standard_bip32' < 'standard_bip44'.
    assert_eq!(
        ecdsa_manifest(&persister, &w, &LoadCtx::strict()),
        vec![idreg, bip32, bip44],
        "a wallet with no legacy row must be returned unchanged"
    );
}

/// BIP44 index 0 and BIP32 index 0 are DIFFERENT accounts that the pre-split
/// schema could not tell apart — splitting the label is what stopped them
/// sharing a row. A legacy row must therefore be matched to its sibling by the
/// full typed account, not by `(account_index, key_class, identity ids)`:
/// that coarser key fuses this pair, drops a real account, and reports a fatal
/// drift for a wallet that has none.
#[test]
fn a1_legacy_standard_row_does_not_absorb_the_other_standard_variant() {
    let (persister, _tmp, _path) = fresh_persister();
    let w = wid(0xA7);
    ensure_wallet_meta(&persister, &w);

    let bip32 = standard(0, StandardAccountType::BIP32Account, 9);
    let legacy_bip44 = standard(0, StandardAccountType::BIP44Account, 7);
    store_registrations(&persister, w, std::slice::from_ref(&bip32));
    plant_registration(&persister, &w, "standard", 0, &legacy_bip44);

    assert_eq!(
        ecdsa_manifest(&persister, &w, &LoadCtx::strict()),
        vec![legacy_bip44, bip32],
        "distinct standard variants at one index must both survive"
    );
}
