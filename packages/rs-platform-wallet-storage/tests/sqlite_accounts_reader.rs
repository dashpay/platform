#![allow(clippy::field_reassign_with_default)]

//! `schema::accounts::load_state` reads `account_registrations` rows back
//! into a keyless [`AccountRegistrationEntry`] manifest, bit-exact,
//! fail-hard on a corrupt blob, and never mints a `Wallet`.

mod common;

use common::{ensure_wallet_meta, fresh_persister, wid};
use key_wallet::account::AccountType;
use platform_wallet::changeset::{AccountRegistrationEntry, PlatformWalletChangeSet};
use platform_wallet_storage::sqlite::schema::accounts;
use platform_wallet_storage::WalletStorageError;

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
    let manifest = accounts::load_state(&conn, &w).expect("load_state").ecdsa;
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
    let manifest = accounts::load_state(&conn, &w).expect("load_state").ecdsa;
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
    let result = accounts::load_state(&conn, &w);
    drop(conn);
    assert!(
        matches!(result, Err(WalletStorageError::BincodeDecode { .. })),
        "corrupt account_xpub_bytes must be a typed BincodeDecode; got {result:?}"
    );
}
