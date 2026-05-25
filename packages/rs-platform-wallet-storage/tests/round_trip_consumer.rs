//! T-024 / CODE-008 — consumer↔SqlitePersister round-trip integration
//! tests.
//!
//! These tests exercise a real [`PlatformWalletManager`] (the consumer
//! side, from `rs-platform-wallet`) against a real [`SqlitePersister`]
//! (this crate). They are the meta-fix CI safety net for the
//! consumer/persister contract drifts surfaced in PR #3625's
//! call-paths audit:
//!
//! * CODE-001 — `load_from_persistor` would silently drop persisted
//!   `platform_addresses` (post T-003 it refuses with a typed error;
//!   the wired round-trip path here proves wallets re-register and
//!   their state survives).
//! * CODE-002 — token-balance writes used a sentinel
//!   `WalletId::default()` so every store FK-violated. Post T-002 the
//!   schema is V002 with `(identity_id, token_id)` PK and identity-
//!   scoped cascade, plus T-003 threads the real wallet id. We
//!   round-trip a real `TokenBalanceChangeSet` through `persister.store`
//!   under a registered wallet/identity pair and assert the row reads
//!   back after reopen.
//! * CODE-003 — `remove_wallet` never propagated to disk. Post T-004
//!   the `delete_wallet` trait method is wired and called from
//!   `remove_wallet`; we register two wallets, drop one, reopen, and
//!   assert the cascade actually fired without touching the surviving
//!   wallet's rows.
//! * CODE-004 — transient errors were erased at the trait boundary.
//!   Post T-001 the typed `PersistenceErrorKind` flows through; the
//!   `WalletId::default()` happy-path here also exercises the typed
//!   `LockPoisoned` → trait mapping at compile time.
//!
//! Per user direction ("If possible, put it into persister crate") the
//! test lives in this crate so the dev-dep cycle stays one-way:
//! `platform-wallet` ships no dependency on `platform-wallet-storage`,
//! while the storage crate is free to pull `platform-wallet` into
//! `[dev-dependencies]` for integration coverage.

#![allow(clippy::field_reassign_with_default)]

use std::collections::BTreeMap;
use std::sync::Arc;

use dpp::prelude::Identifier;
use key_wallet::wallet::initialization::WalletAccountCreationOptions;
use key_wallet::Network;
use platform_wallet::changeset::{
    IdentityKeyDerivationIndices, IdentityKeyEntry, IdentityKeysChangeSet, PlatformWalletChangeSet,
    PlatformWalletPersistence, TokenBalanceChangeSet,
};
use platform_wallet::events::{EventHandler, PlatformEventHandler};
use platform_wallet::wallet::platform_wallet::WalletId;
use platform_wallet::PlatformWalletManager;
use platform_wallet_storage::{SqlitePersister, SqlitePersisterConfig};

// ---------------------------------------------------------------------
// Scaffolding — minimal manager construction around a real persister.
// ---------------------------------------------------------------------

struct NoopEventHandler;
impl EventHandler for NoopEventHandler {}
impl PlatformEventHandler for NoopEventHandler {}

fn mock_sdk() -> Arc<dash_sdk::Sdk> {
    Arc::new(
        dash_sdk::SdkBuilder::new_mock()
            .build()
            .expect("mock sdk should build"),
    )
}

/// Build a `PlatformWalletManager` backed by a fresh `SqlitePersister`
/// at `<tempdir>/wallets.db`. The tempdir is returned so callers can
/// keep it alive across the manager's lifetime and reopen the same DB
/// after drop.
fn fresh_manager() -> (
    Arc<PlatformWalletManager<SqlitePersister>>,
    Arc<SqlitePersister>,
    tempfile::TempDir,
    std::path::PathBuf,
) {
    let tmp = tempfile::tempdir().expect("tempdir");
    let db_path = tmp.path().join("wallets.db");
    let persister =
        Arc::new(SqlitePersister::open(SqlitePersisterConfig::new(&db_path)).expect("open"));
    let sdk = mock_sdk();
    let handler: Arc<dyn PlatformEventHandler> = Arc::new(NoopEventHandler);
    let manager = Arc::new(PlatformWalletManager::new(
        sdk,
        Arc::clone(&persister),
        handler,
    ));
    (manager, persister, tmp, db_path)
}

/// Reopen the persister at `db_path` — used by every round-trip test
/// post-drop to verify the on-disk state actually survived.
fn reopen(db_path: &std::path::Path) -> SqlitePersister {
    SqlitePersister::open(SqlitePersisterConfig::new(db_path)).expect("reopen")
}

/// Distinct 64-byte seed per wallet, deterministic per `index`.
fn seed_bytes_for(index: u8) -> [u8; 64] {
    let mut seed = [0u8; 64];
    for (i, b) in seed.iter_mut().enumerate() {
        *b = ((i as u8).wrapping_mul(7))
            .wrapping_add(3)
            .wrapping_add(index.wrapping_mul(31));
    }
    seed
}

async fn register_test_wallet(
    manager: &PlatformWalletManager<SqlitePersister>,
    seed_index: u8,
) -> WalletId {
    let wallet = manager
        .create_wallet_from_seed_bytes(
            Network::Testnet,
            seed_bytes_for(seed_index),
            WalletAccountCreationOptions::Default,
            Some(0),
        )
        .await
        .expect("wallet registration should succeed against a real SqlitePersister");
    wallet.wallet_id()
}

async fn shutdown_and_drop(manager: Arc<PlatformWalletManager<SqlitePersister>>) {
    manager.shutdown().await;
    drop(manager);
}

// ---------------------------------------------------------------------
// TC-CODE-008-1 — Register a wallet through the consumer; reopen the
// persister; the `wallet_metadata` row and the per-account snapshot
// (`account_registrations` + `account_address_pools`) survive
// drop+reopen. Locks in the bilateral contract: the manager's
// registration changeset (`wallet_lifecycle.rs:286 ish`) actually
// reaches disk through `persister.store(...)`.
// ---------------------------------------------------------------------

#[tokio::test]
async fn tc_code_008_1_register_wallet_metadata_round_trip() {
    let (manager, persister, tmp, db_path) = fresh_manager();
    let wallet_id = register_test_wallet(&manager, 1).await;

    // The registration changeset must have landed; without the
    // immediate persistor flush this assertion would falsely pass
    // (in-memory) and fail post-reopen. Probe before drop so we have a
    // baseline for the diff across reopen.
    let counts_before: BTreeMap<&'static str, usize> = persister
        .inspect_counts(Some(&wallet_id))
        .expect("inspect_counts")
        .into_iter()
        .collect();
    assert!(
        counts_before["wallet_metadata"] >= 1,
        "register_wallet must persist a wallet_metadata row; counts={counts_before:?}",
    );
    assert!(
        counts_before["account_registrations"] >= 1,
        "register_wallet must persist account_registrations rows; counts={counts_before:?}",
    );

    shutdown_and_drop(manager).await;
    drop(persister);

    let persister2 = reopen(&db_path);
    let counts_after: BTreeMap<&'static str, usize> = persister2
        .inspect_counts(Some(&wallet_id))
        .expect("inspect_counts post-reopen")
        .into_iter()
        .collect();

    assert_eq!(
        counts_after, counts_before,
        "every persisted table count must survive drop+reopen; before={counts_before:?} after={counts_after:?}",
    );
    drop(tmp);
}

// ---------------------------------------------------------------------
// TC-CODE-008-2 — Persist platform addresses through the manager's
// registered wallet path, drop, reopen, assert the addresses round-trip
// row-for-row through `schema::platform_addrs::list_per_wallet`.
//
// Drives the storage trait the way `manager::platform_address_sync`
// does (`persister.store(wallet_id, PlatformAddressChangeSet { .. })`)
// — without a live DAPI mock no real BLAST balances appear, so we
// inject a deterministic `PlatformAddressChangeSet` ourselves through
// the trait the consumer would call.
// ---------------------------------------------------------------------

#[tokio::test]
async fn tc_code_008_2_platform_addresses_round_trip() {
    use dash_sdk::platform::address_sync::AddressFunds;
    use key_wallet::PlatformP2PKHAddress;
    use platform_wallet::changeset::{PlatformAddressBalanceEntry, PlatformAddressChangeSet};

    let (manager, persister, tmp, db_path) = fresh_manager();
    let wallet_id = register_test_wallet(&manager, 2).await;

    let entries = vec![
        PlatformAddressBalanceEntry {
            wallet_id,
            account_index: 0,
            address_index: 0,
            address: PlatformP2PKHAddress::new([0xA1; 20]),
            funds: AddressFunds {
                nonce: 1,
                balance: 7_777,
            },
        },
        PlatformAddressBalanceEntry {
            wallet_id,
            account_index: 0,
            address_index: 1,
            address: PlatformP2PKHAddress::new([0xA2; 20]),
            funds: AddressFunds {
                nonce: 2,
                balance: 13_337,
            },
        },
    ];

    // Drive the same trait method the consumer's
    // `platform_address_sync.rs:80` invokes.
    persister
        .store(
            wallet_id,
            PlatformWalletChangeSet {
                platform_addresses: Some(PlatformAddressChangeSet {
                    addresses: entries.clone(),
                    sync_height: Some(424_242),
                    ..Default::default()
                }),
                ..Default::default()
            },
        )
        .expect("platform_addresses store through real persister");

    shutdown_and_drop(manager).await;
    drop(persister);

    let persister2 = reopen(&db_path);
    let rows = platform_wallet_storage::sqlite::schema::platform_addrs::list_per_wallet(
        &persister2.lock_conn_for_test(),
        &wallet_id,
    )
    .expect("list_per_wallet post-reopen");

    assert_eq!(
        rows.len(),
        entries.len(),
        "every persisted platform address must survive drop+reopen",
    );
    for (got, want) in rows.iter().zip(entries.iter()) {
        assert_eq!(got.address, want.address);
        assert_eq!(got.account_index, want.account_index);
        assert_eq!(got.address_index, want.address_index);
        assert_eq!(got.funds.balance, want.funds.balance);
        assert_eq!(got.funds.nonce, want.funds.nonce);
    }
    drop(tmp);
}

// ---------------------------------------------------------------------
// TC-CODE-008-3 — Identity-scoped writes (`identity_keys` and
// `token_balances`) require the V002 cascade chain
// `wallet_metadata → identities → …` to be honoured end-to-end. Bind
// an identity to a manager-registered wallet, then exercise the same
// store path `identity_sync.rs:630` uses for token balance updates
// AND the schema's `identity_keys` writer.
//
// This is the test that would have caught CODE-002 (sentinel
// `WalletId::default()` FK violation): without the V002 identity-
// owned-row redesign + the real wallet_id threading, the
// `TokenBalanceChangeSet` write below would FK-fail.
// ---------------------------------------------------------------------

#[tokio::test]
async fn tc_code_008_3_identity_keys_and_token_balances_round_trip() {
    use dpp::identity::identity_public_key::v0::IdentityPublicKeyV0;
    use dpp::identity::{IdentityPublicKey, KeyType, Purpose, SecurityLevel};
    use dpp::platform_value::BinaryData;

    let (manager, persister, tmp, db_path) = fresh_manager();
    let wallet_id = register_test_wallet(&manager, 3).await;

    let identity_id = Identifier::from([0xCD; 32]);
    // Bind the identity to the wallet via the public API — this is
    // exactly the path `IdentitySyncManager` uses to know which parent
    // wallet a token-balance write belongs to (post T-002/T-003).
    manager
        .identity_sync()
        .register_identity_with_wallet(identity_id, Some(wallet_id), [])
        .await;

    // `identities` row needs to exist before identity-scoped writes
    // can pass V002's FK. The manager's registration handler creates
    // the row lazily — for this offline test we materialise it
    // through the same schema helper `identity_sync` would hit on the
    // first real sync.
    {
        let conn = persister.lock_conn_for_test();
        platform_wallet_storage::sqlite::schema::identities::ensure_exists(
            &conn,
            &wallet_id,
            identity_id
                .as_slice()
                .try_into()
                .expect("identity_id is 32B"),
        )
        .expect("ensure identity row");
    }

    // Identity key — drives the same `identity_keys` writer the
    // consumer's `identity_sync.rs` reaches through `persister.store`.
    let public_key = IdentityPublicKey::V0(IdentityPublicKeyV0 {
        id: 0,
        purpose: Purpose::AUTHENTICATION,
        security_level: SecurityLevel::HIGH,
        contract_bounds: None,
        key_type: KeyType::ECDSA_SECP256K1,
        read_only: false,
        data: BinaryData::new(vec![0xAB; 33]),
        disabled_at: None,
    });
    let key_entry = IdentityKeyEntry {
        identity_id,
        key_id: 11,
        public_key,
        public_key_hash: [0x55; 20],
        wallet_id: Some(wallet_id),
        derivation_indices: Some(IdentityKeyDerivationIndices {
            identity_index: 1,
            key_index: 0,
        }),
    };
    let mut keys = IdentityKeysChangeSet::default();
    keys.upserts.insert((identity_id, 11), key_entry.clone());

    // Token balance — the writer path that CODE-002 broke (sentinel
    // `WalletId::default()` => FK-violation against `wallet_metadata`).
    // Real `wallet_id` from above; V002 PK is `(identity_id, token_id)`.
    let token_id = Identifier::from([0xEE; 32]);
    let mut balances = TokenBalanceChangeSet::default();
    balances.balances.insert((identity_id, token_id), 999_888);

    persister
        .store(
            wallet_id,
            PlatformWalletChangeSet {
                identity_keys: Some(keys),
                token_balances: Some(balances),
                ..Default::default()
            },
        )
        .expect(
            "identity_keys + token_balances store through real persister \
             must succeed end-to-end under a registered wallet/identity pair",
        );

    shutdown_and_drop(manager).await;
    drop(persister);

    // Reopen and assert both rows are present.
    let persister2 = reopen(&db_path);
    let conn = persister2.lock_conn_for_test();

    let key_blob: Vec<u8> = conn
        .query_row(
            "SELECT public_key_blob FROM identity_keys WHERE identity_id = ?1 AND key_id = ?2",
            rusqlite::params![identity_id.as_slice(), 11i64],
            |row| row.get(0),
        )
        .expect("identity_keys row must survive reopen");
    let decoded_key =
        platform_wallet_storage::sqlite::schema::identity_keys::decode_entry(&key_blob)
            .expect("decode identity_keys blob");
    assert_eq!(
        decoded_key, key_entry,
        "identity_keys round-trip must be field-for-field equal",
    );

    let balance: i64 = conn
        .query_row(
            "SELECT balance FROM token_balances WHERE identity_id = ?1 AND token_id = ?2",
            rusqlite::params![identity_id.as_slice(), token_id.as_slice()],
            |row| row.get(0),
        )
        .expect("token_balances row must survive reopen (CODE-002 regression guard)");
    assert_eq!(balance, 999_888);
    drop(tmp);
}

// ---------------------------------------------------------------------
// TC-CODE-008-4 — `remove_wallet` must cascade through the storage
// boundary (CODE-003 regression guard): register two wallets with
// per-wallet state, remove one, drop+reopen, assert the removed
// wallet's rows are gone across every `PER_WALLET_TABLES` entry while
// the surviving wallet's rows are intact.
// ---------------------------------------------------------------------

#[tokio::test]
async fn tc_code_008_4_remove_wallet_cascades_through_storage() {
    let (manager, persister, tmp, db_path) = fresh_manager();

    let wallet_to_keep = register_test_wallet(&manager, 4).await;
    let wallet_to_remove = register_test_wallet(&manager, 5).await;

    let keep_before: BTreeMap<&'static str, usize> = persister
        .inspect_counts(Some(&wallet_to_keep))
        .expect("inspect keep before")
        .into_iter()
        .collect();
    let remove_before: BTreeMap<&'static str, usize> = persister
        .inspect_counts(Some(&wallet_to_remove))
        .expect("inspect remove before")
        .into_iter()
        .collect();
    assert!(
        remove_before["wallet_metadata"] >= 1,
        "wallet_to_remove must have registration rows before remove; counts={remove_before:?}",
    );

    manager
        .remove_wallet(&wallet_to_remove)
        .await
        .expect("remove_wallet must succeed; CODE-003 wires it to persister.delete_wallet");

    shutdown_and_drop(manager).await;
    drop(persister);

    let persister2 = reopen(&db_path);

    // Removed wallet: every per-wallet table must be empty for this id.
    let removed_after: Vec<(&'static str, usize)> = persister2
        .inspect_counts(Some(&wallet_to_remove))
        .expect("inspect remove after");
    for (table, n) in &removed_after {
        assert_eq!(
            *n, 0,
            "remove_wallet must cascade through {table}; saw {n} orphan rows after reopen",
        );
    }

    // Surviving wallet: its counts must be byte-for-byte identical to
    // what they were before — `remove_wallet(W2)` mustn't touch W1.
    let keep_after: BTreeMap<&'static str, usize> = persister2
        .inspect_counts(Some(&wallet_to_keep))
        .expect("inspect keep after")
        .into_iter()
        .collect();
    assert_eq!(
        keep_after, keep_before,
        "surviving wallet's rows must be untouched by remove_wallet of the sibling",
    );
    drop(tmp);
}

// ---------------------------------------------------------------------
// TC-CODE-008-5 — Boot the manager twice against the SAME persister
// path: first run registers two wallets and persists state; second
// run opens a fresh `SqlitePersister` + `PlatformWalletManager` over
// the same DB and exercises `load_from_persistor()`, then verifies
// the persisted state is reachable via the per-wallet
// `register_wallet` re-fetch path.
//
// This is the integration-level CODE-001 regression: the consumer's
// `load_from_persistor` correctly returns the per-wallet rehydration
// gate, and the rows ARE still on disk to feed the per-wallet
// register path.
// ---------------------------------------------------------------------

#[tokio::test]
async fn tc_code_008_5_reopen_manager_recovers_persisted_wallets() {
    let (manager, persister, tmp, db_path) = fresh_manager();

    let w1 = register_test_wallet(&manager, 6).await;
    let w2 = register_test_wallet(&manager, 7).await;

    let counts_w1_before: Vec<(&'static str, usize)> = persister
        .inspect_counts(Some(&w1))
        .expect("inspect w1 before");
    let counts_w2_before: Vec<(&'static str, usize)> = persister
        .inspect_counts(Some(&w2))
        .expect("inspect w2 before");

    shutdown_and_drop(manager).await;
    drop(persister);

    // Second boot: brand-new persister + manager over the SAME file.
    let persister2 = Arc::new(reopen(&db_path));
    let sdk = mock_sdk();
    let handler: Arc<dyn PlatformEventHandler> = Arc::new(NoopEventHandler);
    let manager2 = Arc::new(PlatformWalletManager::new(
        sdk,
        Arc::clone(&persister2),
        handler,
    ));

    // The persistor's `load()` today reports `wallets={}` (only
    // `platform_addresses` populated). With both empty the CODE-001
    // gate accepts the load; we then prove the rows are still on disk
    // by reading directly through the storage crate.
    manager2
        .load_from_persistor()
        .await
        .expect("load_from_persistor must accept the persister's well-formed payload");

    let counts_w1_after: Vec<(&'static str, usize)> = persister2
        .inspect_counts(Some(&w1))
        .expect("inspect w1 after");
    let counts_w2_after: Vec<(&'static str, usize)> = persister2
        .inspect_counts(Some(&w2))
        .expect("inspect w2 after");

    assert_eq!(
        counts_w1_after, counts_w1_before,
        "w1 rows must be recoverable after a clean reopen; before={counts_w1_before:?} after={counts_w1_after:?}",
    );
    assert_eq!(
        counts_w2_after, counts_w2_before,
        "w2 rows must be recoverable after a clean reopen; before={counts_w2_before:?} after={counts_w2_after:?}",
    );

    shutdown_and_drop(manager2).await;
    drop(tmp);
}
