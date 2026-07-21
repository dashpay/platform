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
    AccountRegistrationEntry, PersistenceError, PlatformWalletChangeSet, PlatformWalletPersistence,
    ProviderKeyAccountEntry, ProviderKeyExtendedPubKey, WalletMetadataEntry,
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

fn operator_entry(seed: u8) -> ProviderKeyAccountEntry {
    ProviderKeyAccountEntry {
        account_type: AccountType::ProviderOperatorKeys,
        extended_public_key: bls_xpub(seed),
    }
}

fn platform_entry(seed: u8) -> ProviderKeyAccountEntry {
    ProviderKeyAccountEntry {
        account_type: AccountType::ProviderPlatformKeys,
        extended_public_key: eddsa_xpub(seed),
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

fn wallet_storage_error(err: PersistenceError) -> Box<WalletStorageError> {
    let source = match err {
        PersistenceError::Backend { source, .. } => source,
        other => panic!("expected Backend {{ .. }}, got {other:?}"),
    };
    source
        .downcast::<WalletStorageError>()
        .unwrap_or_else(|source| panic!("expected WalletStorageError, got {source}"))
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
                    platform_entry(0x21),
                ],
                ..Default::default()
            },
        )
        .expect("store");
    drop(persister);

    let persister = reopen(&path);
    let state = persister.load().expect("load");
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
                provider_key_account_registrations: vec![platform_entry(0x24)],
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

    // Negative half, on a BLS row. Two corruptions the writer cannot produce
    // but a schema bug or a tampered DB can, each rejected by a different
    // guard:
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
            ProviderKeyAccountEntry {
                account_type: AccountType::ProviderPlatformKeys,
                extended_public_key: eddsa_xpub(0x24),
            },
        ),
        (
            "blob carries the wrong curve for its account type",
            ProviderKeyAccountEntry {
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

/// TC-PKA-015 — a retried `store()` updates the account row in place.
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
                        platform_entry(0x2A),
                    ],
                    ..Default::default()
                },
            )
            .expect("store");
    }

    let conn = persister.lock_conn_for_test();
    let manifest = accounts::load_state(&conn, &w).expect("load_state");
    assert_eq!(manifest.provider.len(), 2, "re-persist must not duplicate");
}

/// SEC-001 — the writer enforces the curve/type pairing the reader enforces.
/// The two writers share one table, one PK space and one blob column,
/// discriminated only by `account_type`: a `ProviderOperatorKeys` entry
/// carrying an EdDSA key would upsert onto the operator account's row with a
/// payload the fail-hard reader then rejects — bricking `load()` for the whole
/// wallet. Reject it at write time instead of storing a landmine.
#[test]
fn sec_001_writer_rejects_mispaired_curve_and_account_type() {
    let (persister, _tmp, _path) = fresh_persister();
    let w: WalletId = wid(0xD1);
    ensure_wallet_meta(&persister, &w);

    // ProviderOperatorKeys (BLS by contract) carrying an EdDSA key.
    let mispaired = ProviderKeyAccountEntry {
        account_type: AccountType::ProviderOperatorKeys,
        extended_public_key: eddsa_xpub(0x31),
    };
    let err = persister
        .store(
            w,
            PlatformWalletChangeSet {
                provider_key_account_registrations: vec![mispaired],
                ..Default::default()
            },
        )
        .expect_err("a mis-paired provider entry must be rejected at write time");
    assert!(
        format!("{err:?}").contains("ProviderKeyAccountEntryMismatch"),
        "expected a curve/type mismatch, got {err:?}"
    );

    let conn = persister.lock_conn_for_test();
    assert_eq!(
        account_row_count(&conn, &w),
        0,
        "a rejected entry must leave no row behind"
    );
}

/// Two entries for one account that disagree about its extended public key are
/// a contradiction no merge semantic can resolve — one is wrong and the store
/// cannot tell which. Fail closed rather than let write order pick the winner.
#[test]
fn conflicting_duplicate_provider_entries_are_rejected() {
    let (persister, _tmp, _path) = fresh_persister();
    let w: WalletId = wid(0xD2);
    ensure_wallet_meta(&persister, &w);

    // Same account type, two different seeds → two different xpubs.
    let err = persister
        .store(
            w,
            PlatformWalletChangeSet {
                provider_key_account_registrations: vec![
                    platform_entry(0x32),
                    platform_entry(0x33),
                ],
                ..Default::default()
            },
        )
        .expect_err("conflicting entries for one account must be rejected");
    assert!(
        format!("{err:?}").contains("ProviderKeyAccountConflict"),
        "expected ProviderKeyAccountConflict, got {err:?}"
    );

    let conn = persister.lock_conn_for_test();
    assert_eq!(
        account_row_count(&conn, &w),
        0,
        "neither entry may be written"
    );
}

/// A BLS provider account cannot change xpub across flushes.
#[test]
fn conflicting_provider_xpub_against_persisted_row_is_rejected() {
    let (persister, _tmp, _path) = fresh_persister();
    let w: WalletId = wid(0xD6);
    ensure_wallet_meta(&persister, &w);

    persister
        .store(
            w,
            PlatformWalletChangeSet {
                provider_key_account_registrations: vec![operator_entry(0x36)],
                ..Default::default()
            },
        )
        .expect("store original provider account");

    let err = persister
        .store(
            w,
            PlatformWalletChangeSet {
                provider_key_account_registrations: vec![operator_entry(0x37)],
                ..Default::default()
            },
        )
        .expect_err("a persisted provider account must reject a different xpub");
    let storage_error = wallet_storage_error(err);
    assert!(matches!(
        *storage_error,
        WalletStorageError::ProviderKeyAccountConflict {
            account_type: "provider_operator"
        }
    ));

    let conn = persister.lock_conn_for_test();
    let provider = accounts::load_state(&conn, &w).expect("load original provider account");
    assert_eq!(provider.provider.len(), 1);
    assert_eq!(
        bls_bytes(&provider.provider[0].extended_public_key),
        bls_bytes(&bls_xpub(0x36)),
        "the rejected store must leave the original xpub untouched"
    );
}

#[test]
fn conflicting_provider_account_rejects_whole_batch_before_any_write() {
    let (persister, _tmp, _path) = fresh_persister();
    let w: WalletId = wid(0xD8);
    ensure_wallet_meta(&persister, &w);
    persister
        .store(
            w,
            PlatformWalletChangeSet {
                provider_key_account_registrations: vec![operator_entry(0x3A)],
                ..Default::default()
            },
        )
        .expect("store original operator account");

    let err = persister
        .store(
            w,
            PlatformWalletChangeSet {
                provider_key_account_registrations: vec![
                    operator_entry(0x3B),
                    platform_entry(0x3C),
                ],
                ..Default::default()
            },
        )
        .expect_err("a conflicting account must reject the whole provider batch");
    assert!(matches!(
        *wallet_storage_error(err),
        WalletStorageError::ProviderKeyAccountConflict {
            account_type: "provider_operator"
        }
    ));

    let conn = persister.lock_conn_for_test();
    let platform_rows: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM account_registrations \
             WHERE wallet_id = ?1 AND account_type = 'provider_platform'",
            rusqlite::params![w.as_slice()],
            |row| row.get(0),
        )
        .expect("count platform accounts");
    assert_eq!(
        platform_rows, 0,
        "the rejected batch must not partially write its new platform account"
    );
    let provider = accounts::load_state(&conn, &w).expect("load original operator account");
    assert_eq!(provider.provider.len(), 1);
    assert_eq!(
        bls_bytes(&provider.provider[0].extended_public_key),
        bls_bytes(&bls_xpub(0x3A)),
        "the rejected batch must leave the original operator xpub untouched"
    );
}

/// Provider-only labels cannot bypass the parent guard through the ordinary
/// secp256k1 account-registration field.
#[test]
fn ecdsa_registration_path_rejects_provider_account_labels() {
    let (persister, _tmp, _path) = fresh_persister();
    let w: WalletId = wid(0xD7);
    ensure_wallet_meta(&persister, &w);
    persister
        .store(
            w,
            PlatformWalletChangeSet {
                provider_key_account_registrations: vec![operator_entry(0x38)],
                ..Default::default()
            },
        )
        .expect("store original provider account");

    let ecdsa_xpub = seed_wallet(0x39)
        .accounts
        .all_accounts()
        .first()
        .expect("an ECDSA account")
        .account_xpub;
    let err = persister
        .store(
            w,
            PlatformWalletChangeSet {
                account_registrations: vec![AccountRegistrationEntry {
                    account_type: AccountType::ProviderOperatorKeys,
                    account_xpub: ecdsa_xpub,
                }],
                ..Default::default()
            },
        )
        .expect_err("provider labels must be rejected by the ECDSA writer");
    assert!(matches!(
        *wallet_storage_error(err),
        WalletStorageError::ProviderKeyAccountEntryMismatch
    ));

    let conn = persister.lock_conn_for_test();
    let provider = accounts::load_state(&conn, &w).expect("load original provider account");
    assert_eq!(provider.provider.len(), 1);
    assert_eq!(
        bls_bytes(&provider.provider[0].extended_public_key),
        bls_bytes(&bls_xpub(0x38)),
        "the rejected alternate-path store must leave the original xpub untouched"
    );
}

/// QA-002 — the provider write path rides the flush transaction: when a later
/// writer in the same `store()` fails, the account row and domain-seq bump roll
/// back together. Mirrors `tc_b_012`, which pins the same invariant for the
/// pool writer.
#[test]
fn qa_002_partial_failure_rolls_back_provider_rows_and_bump() {
    let (persister, _tmp, _path) = fresh_persister();
    let w: WalletId = wid(0xD3);
    ensure_wallet_meta(&persister, &w);

    // No `identities` row for this id → the token-balances writer trips an FK
    // violation after the provider rows have already been written in this tx.
    let mut balances = std::collections::BTreeMap::new();
    balances.insert(
        (
            dpp::prelude::Identifier::from([0xEE; 32]),
            dpp::prelude::Identifier::from([0xEF; 32]),
        ),
        1u64,
    );
    let result = persister.store(
        w,
        PlatformWalletChangeSet {
            provider_key_account_registrations: vec![platform_entry(0x34)],
            token_balances: Some(platform_wallet::changeset::TokenBalanceChangeSet {
                balances,
                ..Default::default()
            }),
            ..Default::default()
        },
    );
    assert!(result.is_err(), "the FK violation must fail the flush");

    let conn = persister.lock_conn_for_test();
    assert_eq!(
        account_row_count(&conn, &w),
        0,
        "the provider account row must roll back with the failed flush"
    );
    assert_eq!(
        versions::read_seq(&conn, &w, Domain::AccountRegistrations).expect("read_seq"),
        0,
        "the domain bump must roll back with the data it marks"
    );
}

/// Provider account-registration rows for one wallet, counted straight from SQL.
fn account_row_count(conn: &rusqlite::Connection, wallet_id: &WalletId) -> i64 {
    conn.query_row(
        "SELECT COUNT(*) FROM account_registrations WHERE wallet_id = ?1 \
           AND account_type IN ('provider_operator', 'provider_platform')",
        rusqlite::params![wallet_id.as_slice()],
        |row| row.get(0),
    )
    .expect("count provider account rows")
}
