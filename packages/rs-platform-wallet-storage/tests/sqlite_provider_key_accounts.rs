#![allow(clippy::field_reassign_with_default)]

//! Provider key-material account persistence (dashpay/platform#4113): the BLS
//! operator-key and EdDSA platform-node-key accounts survive `store()` →
//! `load()` on par with the ECDSA `account_registrations` manifest.
//!
//! Test-case IDs follow `docs/testspec-4113.md` (`TC-PKA-0NN`).

mod common;

use common::{ensure_wallet_meta, fresh_persister, wid};
use key_wallet::account::AccountType;
use key_wallet::wallet::initialization::WalletAccountCreationOptions;
use key_wallet::wallet::Wallet;
use key_wallet::Network;
use platform_wallet::changeset::{
    AccountRegistrationEntry, PlatformWalletChangeSet, PlatformWalletPersistence,
    ProviderKeyAccountEntry, ProviderKeyExtendedPubKey, ProviderKeyRegistrationBlob,
    ProviderPlatformNodePubKey, WalletMetadataEntry,
};
use platform_wallet::wallet::platform_wallet::WalletId;
use platform_wallet_storage::sqlite::schema::versions::{self, Domain};
use platform_wallet_storage::sqlite::schema::{accounts, blob};
use platform_wallet_storage::{SqlitePersister, SqlitePersisterConfig, WalletStorageError};

/// Deterministic seed wallet carrying both provider key-material accounts
/// (`WalletAccountCreationOptions::Default` creates every special-purpose
/// account, BLS operator + EdDSA platform node included).
fn seed_wallet(seed: u8) -> Wallet {
    Wallet::from_seed_bytes(
        [seed; 64],
        Network::Testnet,
        WalletAccountCreationOptions::Default,
    )
    .expect("seed wallet")
}

/// The wallet's BLS operator-key account xpub.
fn bls_xpub(seed: u8) -> ProviderKeyExtendedPubKey {
    ProviderKeyExtendedPubKey::Bls(
        seed_wallet(seed)
            .accounts
            .bls_account_of_type(AccountType::ProviderOperatorKeys)
            .expect("BLS operator account")
            .bls_public_key
            .clone(),
    )
}

/// The wallet's EdDSA platform-node-key account xpub.
fn eddsa_xpub(seed: u8) -> ProviderKeyExtendedPubKey {
    ProviderKeyExtendedPubKey::EdDSA(
        seed_wallet(seed)
            .accounts
            .eddsa_account_of_type(AccountType::ProviderPlatformKeys)
            .expect("EdDSA platform account")
            .ed25519_public_key
            .clone(),
    )
}

/// Pre-derived platform-node keys are carried public bytes; distinct per
/// `index` so a round-trip cannot pass by accident.
fn node_key(index: u32) -> ProviderPlatformNodePubKey {
    let b = u8::try_from(index & 0xFF).expect("masked to a byte");
    ProviderPlatformNodePubKey {
        index,
        public_key: [b.wrapping_add(1); 32],
        node_id: [b.wrapping_add(2); 20],
    }
}

fn operator_entry(seed: u8) -> ProviderKeyAccountEntry {
    ProviderKeyAccountEntry {
        account_type: AccountType::ProviderOperatorKeys,
        extended_public_key: bls_xpub(seed),
        // The BLS operator pool re-derives on demand from the account xpub.
        derived_platform_node_keys: Vec::new(),
    }
}

fn platform_entry(seed: u8, node_keys: Vec<ProviderPlatformNodePubKey>) -> ProviderKeyAccountEntry {
    ProviderKeyAccountEntry {
        account_type: AccountType::ProviderPlatformKeys,
        extended_public_key: eddsa_xpub(seed),
        derived_platform_node_keys: node_keys,
    }
}

fn metadata() -> WalletMetadataEntry {
    WalletMetadataEntry {
        network: Network::Testnet,
        wallet_group_id: [0; 32],
        birth_height: 1,
    }
}

fn reopen(path: &std::path::Path) -> SqlitePersister {
    SqlitePersister::open(SqlitePersisterConfig::new(path)).expect("reopen persister")
}

/// Raw Ed25519 public-key bytes of an extended key, for cross-curve
/// comparison without leaning on a `PartialEq` the BLS type does not have.
fn eddsa_bytes(key: &ProviderKeyExtendedPubKey) -> Vec<u8> {
    match key {
        ProviderKeyExtendedPubKey::EdDSA(k) => k.encode().to_vec(),
        other => panic!("expected an EdDSA key, got {other:?}"),
    }
}

/// BLS public-key bytes of an extended key (the G2 element), same purpose.
fn bls_bytes(key: &ProviderKeyExtendedPubKey) -> Vec<u8> {
    match key {
        ProviderKeyExtendedPubKey::Bls(k) => k.public_key.to_bytes().to_vec(),
        other => panic!("expected a BLS key, got {other:?}"),
    }
}

/// TC-PKA-001 — a reloaded wallet gets its BLS operator and EdDSA
/// platform-node accounts back. The end-to-end contract #4113 exists for:
/// a seedless/external-signable wallet can list its provider key accounts
/// after a restart without the mnemonic.
#[test]
fn tc_pka_001_provider_accounts_survive_store_load() {
    let (persister, _tmp, path) = fresh_persister();
    let w: WalletId = wid(0xC1);

    persister
        .store(
            w,
            PlatformWalletChangeSet {
                wallet_metadata: Some(metadata()),
                provider_key_account_registrations: vec![
                    operator_entry(0x21),
                    platform_entry(0x21, vec![node_key(0)]),
                ],
                ..Default::default()
            },
        )
        .expect("store");
    drop(persister);

    let state = reopen(&path).load().expect("load");
    let restored = &state.wallets.get(&w).expect("wallet rehydrated").wallet;

    let bls = restored
        .accounts
        .bls_account_of_type(AccountType::ProviderOperatorKeys)
        .expect("BLS operator account must come back from persistence");
    assert_eq!(
        bls.bls_public_key.public_key.to_bytes().to_vec(),
        bls_bytes(&bls_xpub(0x21)),
        "the restored BLS account must carry the persisted operator xpub"
    );
    assert!(bls.is_watch_only, "a rehydrated account is watch-only");

    let eddsa = restored
        .accounts
        .eddsa_account_of_type(AccountType::ProviderPlatformKeys)
        .expect("EdDSA platform account must come back from persistence");
    assert_eq!(
        eddsa.ed25519_public_key.encode().to_vec(),
        eddsa_bytes(&eddsa_xpub(0x21)),
        "the restored EdDSA account must carry the persisted platform-node xpub"
    );
    assert!(eddsa.is_watch_only, "a rehydrated account is watch-only");
}

/// TC-PKA-008 — a changeset carrying only provider-key registrations bumps
/// the `account_registrations` domain seq and no other. The forgotten-domain
/// guard (R8): a field that reaches the DB must invalidate a cache.
#[test]
fn tc_pka_008_provider_only_changeset_bumps_account_registrations_domain() {
    let (persister, _tmp, _path) = fresh_persister();
    let w: WalletId = wid(0xC2);
    ensure_wallet_meta(&persister, &w);

    let cs = PlatformWalletChangeSet {
        provider_key_account_registrations: vec![operator_entry(0x22)],
        ..Default::default()
    };
    assert_eq!(
        versions::touched_domains(&cs),
        vec![Domain::AccountRegistrations],
        "provider-key registrations must map to the account-registrations domain"
    );

    persister.store(w, cs).expect("store");

    let conn = persister.lock_conn_for_test();
    for domain in Domain::ALL {
        let seq = versions::read_seq(&conn, &w, domain).expect("read_seq");
        let expected = i64::from(domain == Domain::AccountRegistrations);
        assert_eq!(
            seq, expected,
            "{domain:?} seq must be {expected} after a provider-only store"
        );
    }
}

/// TC-PKA-002 — a BLS operator account decodes back as BLS, never as the
/// other curve, and its xpub bytes survive verbatim.
#[test]
fn tc_pka_002_bls_decodes_as_bls() {
    let (persister, _tmp, _path) = fresh_persister();
    let w: WalletId = wid(0xC3);
    ensure_wallet_meta(&persister, &w);
    persister
        .store(
            w,
            PlatformWalletChangeSet {
                provider_key_account_registrations: vec![operator_entry(0x23)],
                ..Default::default()
            },
        )
        .expect("store");

    let conn = persister.lock_conn_for_test();
    let provider = accounts::load_state(&conn, &w)
        .expect("load_state")
        .provider;
    assert_eq!(provider.len(), 1);
    assert!(
        matches!(
            provider[0].extended_public_key,
            ProviderKeyExtendedPubKey::Bls(_)
        ),
        "a ProviderOperatorKeys account must decode as BLS, got {:?}",
        provider[0].extended_public_key
    );
    assert_eq!(
        bls_bytes(&provider[0].extended_public_key),
        bls_bytes(&bls_xpub(0x23)),
        "BLS xpub must survive byte-for-byte"
    );
    assert!(provider[0].derived_platform_node_keys.is_empty());
}

/// TC-PKA-003 — an EdDSA account decodes as EdDSA, and a row whose
/// `account_type` column claims the other curve's account is rejected: the
/// column is the decode discriminator, so cross-curve confusion must be a
/// hard error, never a silently-wrong account.
#[test]
fn tc_pka_003_eddsa_decodes_as_eddsa_and_cross_curve_row_is_rejected() {
    let (persister, _tmp, _path) = fresh_persister();
    let w: WalletId = wid(0xC4);
    ensure_wallet_meta(&persister, &w);
    persister
        .store(
            w,
            PlatformWalletChangeSet {
                provider_key_account_registrations: vec![platform_entry(0x24, vec![node_key(0)])],
                ..Default::default()
            },
        )
        .expect("store");

    let conn = persister.lock_conn_for_test();
    let provider = accounts::load_state(&conn, &w)
        .expect("load_state")
        .provider;
    assert_eq!(provider.len(), 1);
    assert!(
        matches!(
            provider[0].extended_public_key,
            ProviderKeyExtendedPubKey::EdDSA(_)
        ),
        "a ProviderPlatformKeys account must decode as EdDSA"
    );
    assert_eq!(
        eddsa_bytes(&provider[0].extended_public_key),
        eddsa_bytes(&eddsa_xpub(0x24)),
        "EdDSA xpub must survive byte-for-byte"
    );

    drop(conn);

    // Negative half, on a BLS row (no node keys, so nothing to orphan). Two
    // corruptions the writer cannot produce but a schema bug or a tampered DB
    // can, each rejected by a different guard:
    //   (a) the blob's account type contradicts the `account_type` column;
    //   (b) the column and the blob agree on the type, but the blob carries
    //       the other curve's key — only the curve check catches this one.
    let w2: WalletId = wid(0xD4);
    ensure_wallet_meta(&persister, &w2);
    persister
        .store(
            w2,
            PlatformWalletChangeSet {
                provider_key_account_registrations: vec![operator_entry(0x24)],
                ..Default::default()
            },
        )
        .expect("store");

    let conn = persister.lock_conn_for_test();
    let plant = |payload: Vec<u8>| {
        conn.execute(
            "UPDATE account_registrations SET account_xpub_bytes = ?2 WHERE wallet_id = ?1",
            rusqlite::params![w2.as_slice(), payload],
        )
        .expect("plant corrupt provider row");
    };

    for (case, payload) in [
        (
            "blob type contradicts the account_type column",
            ProviderKeyRegistrationBlob {
                account_type: AccountType::ProviderPlatformKeys,
                extended_public_key: eddsa_xpub(0x24),
            },
        ),
        (
            "blob carries the wrong curve for its account type",
            ProviderKeyRegistrationBlob {
                account_type: AccountType::ProviderOperatorKeys,
                extended_public_key: eddsa_xpub(0x24),
            },
        ),
    ] {
        plant(blob::encode(&payload).expect("encode"));
        let err = accounts::load_state(&conn, &w2).expect_err("cross-curve row must hard-error");
        assert!(
            matches!(err, WalletStorageError::ProviderKeyAccountEntryMismatch),
            "{case}: expected ProviderKeyAccountEntryMismatch, got {err:?}"
        );
    }
}

/// TC-PKA-004 — every pre-derived platform-node key comes back, identity
/// intact, ordered by index. This pool is hardened-only: a key dropped here
/// is unrecoverable without the seed.
#[test]
fn tc_pka_004_node_keys_round_trip_ordered_and_complete() {
    let (persister, _tmp, _path) = fresh_persister();
    let w: WalletId = wid(0xC5);
    ensure_wallet_meta(&persister, &w);

    // Non-sequential, deliberately unsorted on the way in.
    let keys: Vec<ProviderPlatformNodePubKey> =
        [9u32, 0, 5, 2, 1].into_iter().map(node_key).collect();
    persister
        .store(
            w,
            PlatformWalletChangeSet {
                provider_key_account_registrations: vec![platform_entry(0x25, keys.clone())],
                ..Default::default()
            },
        )
        .expect("store");

    let conn = persister.lock_conn_for_test();
    let provider = accounts::load_state(&conn, &w)
        .expect("load_state")
        .provider;
    let loaded = &provider[0].derived_platform_node_keys;

    let mut expected = keys;
    expected.sort_by_key(|k| k.index);
    assert_eq!(
        loaded, &expected,
        "node keys must come back whole, identity intact, ordered by index"
    );
}

/// TC-PKA-005 — no provider accounts is a clean round-trip: no rows, no
/// error, and the ECDSA manifest beside it is untouched.
#[test]
fn tc_pka_005_empty_provider_set_round_trips() {
    let (persister, _tmp, _path) = fresh_persister();
    let w: WalletId = wid(0xC6);
    ensure_wallet_meta(&persister, &w);

    let ecdsa = AccountRegistrationEntry {
        account_type: AccountType::IdentityRegistration,
        account_xpub: seed_wallet(0x26)
            .accounts
            .all_accounts()
            .first()
            .expect("an account")
            .account_xpub,
    };
    persister
        .store(
            w,
            PlatformWalletChangeSet {
                account_registrations: vec![ecdsa],
                provider_key_account_registrations: vec![],
                ..Default::default()
            },
        )
        .expect("store");

    let conn = persister.lock_conn_for_test();
    let manifest = accounts::load_state(&conn, &w).expect("load_state");
    assert!(manifest.provider.is_empty(), "no provider accounts stored");
    assert_eq!(manifest.ecdsa.len(), 1, "the ECDSA entry is unaffected");
    assert_eq!(node_key_row_count(&conn, &w), 0, "no child rows written");
}

/// TC-PKA-009 — a corrupt provider blob fails the whole load. Skipping the
/// row would hand back a wallet silently missing its operator account.
#[test]
fn tc_pka_009_corrupt_provider_blob_hard_errors() {
    let (persister, _tmp, _path) = fresh_persister();
    let w: WalletId = wid(0xC7);
    ensure_wallet_meta(&persister, &w);
    persister
        .store(
            w,
            PlatformWalletChangeSet {
                provider_key_account_registrations: vec![operator_entry(0x27)],
                ..Default::default()
            },
        )
        .expect("store");

    let conn = persister.lock_conn_for_test();
    conn.execute(
        "UPDATE account_registrations SET account_xpub_bytes = X'00' WHERE wallet_id = ?1",
        rusqlite::params![w.as_slice()],
    )
    .expect("corrupt the blob");

    let err = accounts::load_state(&conn, &w).expect_err("a corrupt provider blob must hard-error");
    assert!(
        matches!(err, WalletStorageError::BincodeDecode { .. }),
        "expected a typed BincodeDecode, got {err:?}"
    );
}

/// TC-PKA-010 — an oversize provider blob is rejected by the `length()` gate
/// before the `Vec<u8>` is materialized.
#[test]
fn tc_pka_010_oversize_provider_blob_is_rejected() {
    let (persister, _tmp, _path) = fresh_persister();
    let w: WalletId = wid(0xC8);
    ensure_wallet_meta(&persister, &w);

    let oversize = vec![0u8; blob::BLOB_SIZE_LIMIT_BYTES + 1];
    let conn = persister.lock_conn_for_test();
    conn.execute(
        "INSERT INTO account_registrations \
            (wallet_id, account_type, account_index, account_xpub_bytes) \
         VALUES (?1, 'provider_operator', 0, ?2)",
        rusqlite::params![w.as_slice(), oversize],
    )
    .expect("insert oversize provider blob");

    let err = accounts::load_state(&conn, &w).expect_err("oversize blob must be rejected");
    assert!(
        matches!(err, WalletStorageError::BlobTooLarge { .. }),
        "expected BlobTooLarge, got {err:?}"
    );
}

/// TC-PKA-011 — the child table's typed columns are cross-checked by width:
/// a `public_key` that is not 32 bytes is a decode error, not a truncated or
/// zero-padded key silently handed to the caller.
#[test]
fn tc_pka_011_malformed_node_key_column_is_rejected() {
    let (persister, _tmp, _path) = fresh_persister();
    let w: WalletId = wid(0xC9);
    ensure_wallet_meta(&persister, &w);
    persister
        .store(
            w,
            PlatformWalletChangeSet {
                provider_key_account_registrations: vec![platform_entry(0x29, vec![node_key(0)])],
                ..Default::default()
            },
        )
        .expect("store");

    let conn = persister.lock_conn_for_test();
    conn.execute(
        "UPDATE provider_platform_node_keys SET public_key = X'DEADBEEF' WHERE wallet_id = ?1",
        rusqlite::params![w.as_slice()],
    )
    .expect("plant a short public_key");

    let err = accounts::load_state(&conn, &w).expect_err("a short public_key must hard-error");
    assert!(
        matches!(err, WalletStorageError::BlobDecode { .. }),
        "expected BlobDecode, got {err:?}"
    );
}

/// TC-PKA-015 — a retried `store()` updates in place. A bare INSERT would
/// duplicate the account row (and every node key) on every flush.
#[test]
fn tc_pka_015_idempotent_repersist_does_not_duplicate() {
    let (persister, _tmp, _path) = fresh_persister();
    let w: WalletId = wid(0xCA);
    ensure_wallet_meta(&persister, &w);

    for _ in 0..2 {
        persister
            .store(
                w,
                PlatformWalletChangeSet {
                    provider_key_account_registrations: vec![
                        operator_entry(0x2A),
                        platform_entry(0x2A, vec![node_key(0), node_key(1)]),
                    ],
                    ..Default::default()
                },
            )
            .expect("store");
    }

    let conn = persister.lock_conn_for_test();
    let manifest = accounts::load_state(&conn, &w).expect("load_state");
    assert_eq!(manifest.provider.len(), 2, "re-persist must not duplicate");
    assert_eq!(node_key_row_count(&conn, &w), 2, "nor duplicate node keys");
}

/// TC-PKA-016 — re-persisting an extended node-key batch replaces the whole
/// set: the observable result is exactly the new batch, no duplicate indices,
/// no key from the first batch dropped.
#[test]
fn tc_pka_016_node_key_batch_is_replaced_wholesale() {
    let (persister, _tmp, _path) = fresh_persister();
    let w: WalletId = wid(0xCB);
    ensure_wallet_meta(&persister, &w);

    let store = |keys: Vec<ProviderPlatformNodePubKey>| {
        persister
            .store(
                w,
                PlatformWalletChangeSet {
                    provider_key_account_registrations: vec![platform_entry(0x2B, keys)],
                    ..Default::default()
                },
            )
            .expect("store");
    };
    store(vec![node_key(0), node_key(1), node_key(2)]);
    store((0..5).map(node_key).collect());

    let conn = persister.lock_conn_for_test();
    let provider = accounts::load_state(&conn, &w)
        .expect("load_state")
        .provider;
    let indices: Vec<u32> = provider[0]
        .derived_platform_node_keys
        .iter()
        .map(|k| k.index)
        .collect();
    assert_eq!(
        indices,
        vec![0, 1, 2, 3, 4],
        "the extended batch must be observable whole, with no duplicate index"
    );
    assert_eq!(node_key_row_count(&conn, &w), 5);
}

/// Deleting the wallet reaps the node keys through the account row — the
/// child table must not outlive its parent.
#[test]
fn wallet_delete_cascades_to_node_keys() {
    let (persister, _tmp, _path) = fresh_persister();
    let w: WalletId = wid(0xCC);
    ensure_wallet_meta(&persister, &w);
    persister
        .store(
            w,
            PlatformWalletChangeSet {
                provider_key_account_registrations: vec![platform_entry(0x2C, vec![node_key(0)])],
                ..Default::default()
            },
        )
        .expect("store");

    let conn = persister.lock_conn_for_test();
    assert_eq!(node_key_row_count(&conn, &w), 1);
    conn.execute(
        "DELETE FROM wallets WHERE wallet_id = ?1",
        rusqlite::params![w.as_slice()],
    )
    .expect("delete wallet");
    assert_eq!(
        node_key_row_count(&conn, &w),
        0,
        "node keys must cascade away with their wallet"
    );
}

/// Node-key rows for one wallet, counted straight from SQL.
fn node_key_row_count(conn: &rusqlite::Connection, wallet_id: &WalletId) -> i64 {
    conn.query_row(
        "SELECT COUNT(*) FROM provider_platform_node_keys WHERE wallet_id = ?1",
        rusqlite::params![wallet_id.as_slice()],
        |row| row.get(0),
    )
    .expect("count node keys")
}
