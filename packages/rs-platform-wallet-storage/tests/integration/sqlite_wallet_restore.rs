//! Core snapshot restoration through SQLite reopen and real manager hydration.

use crate::common;

use std::sync::Arc;

use dashcore::{Network, OutPoint, TxOut, Txid};
#[cfg(feature = "shielded")]
use key_wallet::account::StandardAccountType;
use key_wallet::{
    account::AccountType,
    managed_account::address_pool::{AddressState, KeySource},
    wallet::{initialization::WalletAccountCreationOptions, ManagedWalletInfo, Wallet},
};
use platform_wallet::{
    changeset::{
        AccountAddressPoolEntry, AccountRegistrationEntry, CoreChangeSet, PersistenceCapabilities,
        PlatformWalletChangeSet, PlatformWalletPersistence, ProviderKeyAccountEntry,
        ProviderKeyExtendedPubKey, WalletMetadataEntry,
    },
    events::EventHandler,
    PlatformEventHandler, PlatformWalletManager,
};
#[cfg(feature = "shielded")]
use platform_wallet::{
    test_support::funded_wallet_manager, wallet::asset_lock::AssetLockOperationState,
};
use platform_wallet_storage::{SqlitePersister, SqlitePersisterConfig};

struct NoopHandler;
impl EventHandler for NoopHandler {}
impl PlatformEventHandler for NoopHandler {}

fn registration(wallet: &Wallet, info: &ManagedWalletInfo) -> PlatformWalletChangeSet {
    let mut provider = Vec::new();
    if let Some(account) = wallet
        .accounts
        .bls_account_of_type(AccountType::ProviderOperatorKeys)
    {
        provider.push(ProviderKeyAccountEntry {
            account_type: AccountType::ProviderOperatorKeys,
            extended_public_key: ProviderKeyExtendedPubKey::Bls(account.bls_public_key.clone()),
        });
    }
    if let Some(account) = wallet
        .accounts
        .eddsa_account_of_type(AccountType::ProviderPlatformKeys)
    {
        provider.push(ProviderKeyAccountEntry {
            account_type: AccountType::ProviderPlatformKeys,
            extended_public_key: ProviderKeyExtendedPubKey::EdDSA(
                account.ed25519_public_key.clone(),
            ),
        });
    }
    let mut pools = Vec::new();
    for account in info.accounts.all_accounts() {
        let managed = account.managed_account_type();
        for pool in managed.address_pools() {
            if !pool.addresses.is_empty() {
                pools.push(AccountAddressPoolEntry {
                    account_type: managed.to_account_type(),
                    pool_type: pool.pool_type,
                    addresses: pool.addresses.values().cloned().collect(),
                });
            }
        }
    }
    PlatformWalletChangeSet {
        wallet_metadata: Some(WalletMetadataEntry {
            network: Network::Testnet,
            wallet_group_id: [0; 32],
            birth_height: 1,
        }),
        account_registrations: wallet
            .accounts
            .all_accounts()
            .into_iter()
            .map(|account| AccountRegistrationEntry {
                account_type: account.account_type,
                account_xpub: account.account_xpub,
            })
            .collect(),
        provider_key_account_registrations: provider,
        account_address_pools: pools,
        ..Default::default()
    }
}

async fn manager(persister: SqlitePersister) -> PlatformWalletManager<SqlitePersister> {
    let manager = PlatformWalletManager::new(
        Arc::new(
            dash_sdk::SdkBuilder::new_mock()
                .with_network(Network::Testnet)
                .build()
                .unwrap(),
        ),
        Arc::new(persister),
        Arc::new(NoopHandler),
    );
    manager
        .load_from_persistor()
        .await
        .expect("manager hydration");
    manager
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn should_restore_funding_indices_and_reserved_addresses_before_handing_out_keys() {
    let mut wallet =
        Wallet::new_random(Network::Testnet, WalletAccountCreationOptions::Default).unwrap();
    wallet
        .add_account(
            AccountType::IdentityTopUp {
                registration_index: 7,
            },
            None,
        )
        .unwrap();
    let mut info = ManagedWalletInfo::from_wallet(&wallet, 1);
    let platform_keys =
        platform_wallet::wallet::provider_key_at_index::derive_platform_node_public_keys(
            &wallet,
            Network::Testnet,
            3,
        )
        .unwrap();
    platform_wallet::wallet::provider_key_at_index::populate_platform_node_pool(
        &mut info,
        &platform_keys,
        Network::Testnet,
    )
    .unwrap();
    let mut expected = Vec::new();
    for mut account in info.accounts.all_accounts_mut() {
        let account_type = account.managed_account_type().to_account_type();
        let source = public_key_source(&wallet, account_type);
        for pool in account.managed_account_type_mut().address_pools_mut() {
            let reserved_index = if let Some(source) = &source {
                pool.generate_addresses(40, source, true).unwrap();
                for index in 0..40 {
                    assert!(pool.mark_index_used(index));
                }
                let reserved = pool.next_unused_and_reserve(source, 12345).unwrap();
                pool.address_index[&reserved]
            } else {
                if pool.addresses.is_empty() {
                    continue;
                }
                assert!(pool.mark_index_used(0));
                pool.addresses.get_mut(&1).unwrap().state = AddressState::Reserved { at: 12345 };
                1
            };
            expected.push((
                account_type,
                pool.pool_type,
                pool.addresses.clone(),
                reserved_index,
                pool.highest_used,
            ));
        }
    }
    for required in [
        AccountType::IdentityRegistration,
        AccountType::IdentityTopUp {
            registration_index: 7,
        },
        AccountType::IdentityTopUpNotBoundToIdentity,
        AccountType::IdentityInvitation,
        AccountType::AssetLockAddressTopUp,
        AccountType::AssetLockShieldedAddressTopUp,
        AccountType::ProviderPlatformKeys,
        AccountType::ProviderOperatorKeys,
    ] {
        assert!(
            expected
                .iter()
                .any(|(account, _, _, _, _)| *account == required),
            "missing required role {required:?}"
        );
    }
    let (persister, _tmp, path) = common::fresh_persister();
    persister
        .store(wallet.wallet_id, registration(&wallet, &info))
        .unwrap();
    drop(persister);
    let manager = manager(SqlitePersister::open(SqlitePersisterConfig::new(path)).unwrap()).await;
    let wm = manager.wallet_manager_arc();
    let mut wm = wm.write().await;
    let restored = wm.get_wallet_info_mut(&wallet.wallet_id).unwrap();
    for (account_type, pool_type, addresses, reserved_index, highest_used) in expected {
        let source = public_key_source(&wallet, account_type);
        let mut accounts = restored.core_wallet.accounts.all_accounts_mut();
        let account = accounts
            .iter_mut()
            .find(|account| account.managed_account_type().to_account_type() == account_type)
            .unwrap();
        let mut pools = account.managed_account_type_mut().address_pools_mut();
        let pool = pools
            .iter_mut()
            .find(|pool| pool.pool_type == pool_type)
            .unwrap();
        for (index, expected) in addresses {
            let actual = pool
                .addresses
                .get(&index)
                .expect("persisted beyond-gap address");
            assert_eq!(
                actual.state, expected.state,
                "{account_type:?} {pool_type:?} #{index}"
            );
            assert_eq!(actual.address, expected.address);
            assert_eq!(actual.path, expected.path);
            assert_eq!(actual.public_key, expected.public_key);
        }
        assert_eq!(
            pool.addresses[&reserved_index].state,
            AddressState::Reserved { at: 12345 }
        );
        assert_eq!(
            pool.highest_used, highest_used,
            "{account_type:?} {pool_type:?}"
        );
        if let Some(source) = source {
            let next = pool.next_unused(&source, true).unwrap();
            assert!(
                pool.address_index[&next] > reserved_index,
                "must not reuse funded or handed-out keys"
            );
        }
    }
    drop(wm);
    assert!(manager.shutdown().await.all_clean());
}

#[test]
fn should_advertise_full_core_wallet_restore() {
    let (persister, _tmp, _path) = common::fresh_persister();
    assert!(persister
        .persistence_capabilities()
        .contains(PersistenceCapabilities::WALLET_RESTORE));
}

#[cfg(feature = "shielded")]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn should_report_not_sent_from_a_restored_wallet_with_durable_operation_evidence() {
    let (source, wallet_id, _, _signer) =
        funded_wallet_manager(StandardAccountType::BIP44Account).await;
    let (persister, tmp, _) = common::fresh_persister();
    let snapshot = {
        let source = source.read().await;
        let wallet = source.get_wallet(&wallet_id).unwrap();
        let info = &source.get_wallet_info(&wallet_id).unwrap().core_wallet;
        let mut snapshot = registration(wallet, info);
        snapshot.core = Some(CoreChangeSet {
            new_utxos: info.accounts.standard_bip44_accounts[&0]
                .utxos
                .values()
                .cloned()
                .collect(),
            ..Default::default()
        });
        snapshot
    };
    persister.store(wallet_id, snapshot).unwrap();
    let persister = Arc::new(persister);
    let manager = PlatformWalletManager::new(
        Arc::new(
            dash_sdk::SdkBuilder::new_mock()
                .with_network(Network::Testnet)
                .build()
                .unwrap(),
        ),
        Arc::clone(&persister),
        Arc::new(NoopHandler),
    );
    manager.load_from_persistor().await.unwrap();
    manager
        .configure_shielded(tmp.path().join("shielded.db"))
        .await
        .unwrap();
    let wallet = manager.get_wallet(&wallet_id).await.unwrap();
    let changes_before = persister.lock_conn_for_test().total_changes();
    let status = wallet
        .asset_locks()
        .wallet_drain_operation_status([7; 16])
        .await
        .unwrap();
    assert_eq!(status.state, AssetLockOperationState::NotSent);
    assert_eq!(status.out_point, None);
    assert_eq!(
        persister.lock_conn_for_test().total_changes(),
        changes_before
    );
    assert!(wallet.asset_locks().list_tracked_locks().await.is_empty());
    assert!(manager.shutdown().await.all_clean());
}

/// A wallet is loaded inside its own read transaction, so a writer on
/// another connection cannot change the rows that load sees mid-load.
///
/// Timing is deterministic, not slept for: SQLite calls the authorizer while
/// it compiles a statement, so the write is made from inside the load's own
/// `core_utxos` read, after the transaction has already read other rows.
#[test]
fn should_load_a_wallet_from_one_snapshot_while_another_connection_writes() {
    use rusqlite::hooks::{AuthAction, Authorization};
    use std::sync::Mutex;

    const CONCURRENT_WRITE: &str = "UPDATE core_sync_state SET synced_height = 200";

    let (persister, _tmp, path) = common::fresh_persister();
    let wallet =
        Wallet::new_random(Network::Testnet, WalletAccountCreationOptions::Default).unwrap();
    let info = ManagedWalletInfo::from_wallet(&wallet, 1);
    let mut cs = registration(&wallet, &info);
    cs.core = Some(CoreChangeSet {
        synced_height: Some(100),
        ..Default::default()
    });
    persister.store(wallet.wallet_id, cs).unwrap();
    let synced_height = |persister: &SqlitePersister| {
        persister.load().unwrap().wallets[&wallet.wallet_id]
            .wallet_info
            .metadata
            .synced_height
    };

    let writer = rusqlite::Connection::open(&path).unwrap();
    // Fail at once instead of waiting out the default busy handler.
    writer.busy_timeout(std::time::Duration::ZERO).unwrap();
    // The authorizer swallows panics, so the outcome is asserted outside it.
    let attempt: Arc<Mutex<Option<rusqlite::Result<usize>>>> = Arc::new(Mutex::new(None));
    let attempt_callback = Arc::clone(&attempt);
    persister
        .lock_conn_for_test()
        .authorizer(Some(move |ctx: rusqlite::hooks::AuthContext<'_>| {
            if matches!(
                ctx.action,
                AuthAction::Read {
                    table_name: "core_utxos",
                    ..
                }
            ) {
                let mut attempt = attempt_callback.lock().unwrap();
                if attempt.is_none() {
                    *attempt = Some(writer.execute(CONCURRENT_WRITE, []));
                }
            }
            Authorization::Allow
        }))
        .unwrap();

    assert_eq!(
        synced_height(&persister),
        100,
        "one wallet load must see one snapshot"
    );
    {
        // Scoped: the authorizer takes this lock again on the next load.
        let attempt = attempt.lock().unwrap();
        let attempt = attempt
            .as_ref()
            .expect("the load must read core_utxos, which triggers the write");
        assert!(
            matches!(attempt, Ok(1)),
            "a write on another connection must land during a read-only load, got {attempt:?}"
        );
    }

    // Control: the next load reads the write, so the value above was the
    // load's snapshot, not an inert statement.
    assert_eq!(synced_height(&persister), 200);
}

/// A failure while rebuilding a wallet reaches the caller as the storage
/// error that caused it: Strict returns it typed, and Recovery names it as
/// the dropped wallet's cause rather than an anonymous `backend`.
/// A store whose only wallet has platform-node pool rows carrying a
/// well-formed BLS key, on the account that only ever holds EdDSA keys.
fn store_with_wrong_curve_platform_node_keys() -> (tempfile::TempDir, std::path::PathBuf, [u8; 32])
{
    let (persister, tmp, path) = common::fresh_persister();
    let wallet =
        Wallet::new_random(Network::Testnet, WalletAccountCreationOptions::Default).unwrap();
    let mut info = ManagedWalletInfo::from_wallet(&wallet, 1);
    let platform_keys =
        platform_wallet::wallet::provider_key_at_index::derive_platform_node_public_keys(
            &wallet,
            Network::Testnet,
            3,
        )
        .unwrap();
    platform_wallet::wallet::provider_key_at_index::populate_platform_node_pool(
        &mut info,
        &platform_keys,
        Network::Testnet,
    )
    .unwrap();
    persister
        .store(wallet.wallet_id, registration(&wallet, &info))
        .unwrap();
    let rewritten = persister
        .lock_conn_for_test()
        .execute(
            "UPDATE core_address_pool SET key_type = 2, public_key = zeroblob(48) \
             WHERE account_type = 'provider_platform'",
            [],
        )
        .unwrap();
    assert_eq!(rewritten, platform_keys.len());
    (tmp, path, wallet.wallet_id)
}

/// A typed cause rarely names its wallet, and under Strict the load returns
/// that cause as it is: the log is where the wallet is named.
#[test]
#[tracing_test::traced_test]
fn should_log_which_wallet_a_strict_load_could_not_rebuild() {
    let (_tmp, path, wallet_id) = store_with_wrong_curve_platform_node_keys();
    let wallet = hex::encode(wallet_id);
    assert!(!logs_contain(&wallet), "the fixture must not name it first");

    let error = SqlitePersister::open(SqlitePersisterConfig::new(&path))
        .unwrap()
        .load()
        .expect_err("a wrong-curve key must fail");
    assert!(
        !error.to_string().contains(&wallet),
        "the error itself does not name the wallet: {error}"
    );
    for field in [wallet.as_str(), "error_kind=\"blob_decode\""] {
        assert!(logs_contain(field), "missing from the log: {field}");
    }
}

#[test]
fn should_attribute_a_platform_node_pool_failure_to_its_typed_cause() {
    use platform_wallet::changeset::PersistenceError;
    use platform_wallet_storage::{LoadPolicy, WalletStorageError};

    let (_tmp, path, wallet_id) = store_with_wrong_curve_platform_node_keys();

    let strict = SqlitePersister::open(SqlitePersisterConfig::new(&path)).unwrap();
    let error = strict.load().expect_err("a wrong-curve key must fail");
    let PersistenceError::Backend { source, .. } = error else {
        panic!("expected a backend error, got {error:?}")
    };
    assert!(
        matches!(
            source.downcast_ref::<WalletStorageError>(),
            Some(WalletStorageError::BlobDecode { reason })
                if *reason == "provider platform pool row does not carry an EdDSA public key"
        ),
        "the platform-node pool reader's own error must survive: {source:?}"
    );
    drop(strict);

    let recovery = SqlitePersister::open(
        SqlitePersisterConfig::new(&path).with_load_policy(LoadPolicy::Recovery),
    )
    .unwrap();
    let state = recovery.load().expect("recovery isolates the wallet");
    assert!(!state.wallets.contains_key(&wallet_id));
    assert_eq!(
        recovery
            .last_load_degradation()
            .wallets_degraded
            .get(&wallet_id)
            .copied(),
        Some("blob_decode")
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn should_reject_malformed_pool_keys_with_typed_source_before_manager_hydration() {
    use platform_wallet::changeset::PersistenceError;
    use platform_wallet_storage::WalletStorageError;

    let (persister, _tmp, path) = common::fresh_persister();
    let wallet =
        Wallet::new_random(Network::Testnet, WalletAccountCreationOptions::Default).unwrap();
    let info = ManagedWalletInfo::from_wallet(&wallet, 1);
    persister
        .store(wallet.wallet_id, registration(&wallet, &info))
        .unwrap();
    persister.lock_conn_for_test().execute(
        "UPDATE core_address_pool SET public_key = X'00' WHERE account_type = 'identity_invitation'",
        [],
    ).unwrap();
    drop(persister);
    let persister = SqlitePersister::open(SqlitePersisterConfig::new(path)).unwrap();
    let error = persister
        .load()
        .expect_err("invalid key must fail the full snapshot");
    let PersistenceError::Backend { source, .. } = error else {
        panic!("typed backend error")
    };
    assert!(source.downcast_ref::<WalletStorageError>().is_some());
    let manager = PlatformWalletManager::new(
        Arc::new(
            dash_sdk::SdkBuilder::new_mock()
                .with_network(Network::Testnet)
                .build()
                .unwrap(),
        ),
        Arc::new(persister),
        Arc::new(NoopHandler),
    );
    assert!(manager.load_from_persistor().await.is_err());
    assert!(manager
        .wallet_manager_arc()
        .read()
        .await
        .get_wallet_info(&wallet.wallet_id)
        .is_none());
    assert!(manager.shutdown().await.all_clean());
}

fn with_pool<R>(
    info: &mut ManagedWalletInfo,
    account_type: AccountType,
    pool_type: key_wallet::managed_account::address_pool::AddressPoolType,
    edit: impl FnOnce(&mut key_wallet::managed_account::address_pool::AddressPool) -> R,
) -> R {
    let mut accounts = info.accounts.all_accounts_mut();
    let account = accounts
        .iter_mut()
        .find(|account| account.managed_account_type().to_account_type() == account_type)
        .expect("account");
    let mut pools = account.managed_account_type_mut().address_pools_mut();
    let pool = pools
        .iter_mut()
        .find(|pool| pool.pool_type == pool_type)
        .expect("pool");
    edit(pool)
}

/// The public key an account's non-hardened pool derives from: an ECDSA
/// xpub, or the operator account's BLS key. Platform-node keys are hardened
/// and have none.
fn public_key_source(wallet: &Wallet, account_type: AccountType) -> Option<KeySource> {
    if let Some(account) = wallet.accounts.account_of_type(account_type) {
        return Some(KeySource::Public(account.account_xpub));
    }
    wallet
        .accounts
        .bls_account_of_type(account_type)
        .map(|account| KeySource::BLSPublic(account.bls_public_key.clone()))
}

fn account_key_source(wallet: &Wallet, account_type: AccountType) -> KeySource {
    public_key_source(wallet, account_type).expect("account keys")
}

/// Persists a wallet shaped by `prepare` next to a healthy sibling, runs the
/// statement `prepare` returns against the first wallet's rows (`?1` is its
/// id), and asserts the load's verdict: Strict returns the typed storage
/// error, Recovery drops that wallet alone and names the cause.
fn assert_pool_corruption_is_typed_and_isolated(
    prepare: impl FnOnce(&Wallet, &mut ManagedWalletInfo) -> String,
    reason: &str,
) {
    use platform_wallet_storage::WalletStorageError;

    assert_wallet_corruption_is_isolated(
        Wallet::new_random(Network::Testnet, WalletAccountCreationOptions::Default).unwrap(),
        prepare,
        |error| matches!(error, WalletStorageError::BlobDecode { reason: actual } if *actual == reason),
        reason,
    );
}

/// `prepare` edits the sick wallet before it is stored and returns the SQL
/// that damages exactly one of its rows (`?1` is the wallet id). Strict must
/// then fail with an error `strict` accepts; Recovery must drop that wallet
/// alone, as `blob_decode`, and keep the healthy one beside it.
fn assert_wallet_corruption_is_isolated(
    sick: Wallet,
    prepare: impl FnOnce(&Wallet, &mut ManagedWalletInfo) -> String,
    strict: impl Fn(&platform_wallet_storage::WalletStorageError) -> bool,
    expectation: &str,
) {
    use platform_wallet::changeset::PersistenceError;
    use platform_wallet_storage::{LoadPolicy, WalletStorageError};

    let (persister, _tmp, path) = common::fresh_persister();
    let mut sick_info = ManagedWalletInfo::from_wallet(&sick, 1);
    let corruption = prepare(&sick, &mut sick_info);
    persister
        .store(sick.wallet_id, registration(&sick, &sick_info))
        .unwrap();
    let healthy =
        Wallet::new_random(Network::Testnet, WalletAccountCreationOptions::Default).unwrap();
    persister
        .store(
            healthy.wallet_id,
            registration(&healthy, &ManagedWalletInfo::from_wallet(&healthy, 1)),
        )
        .unwrap();
    let rewritten = persister
        .lock_conn_for_test()
        .execute(&corruption, [sick.wallet_id.as_slice()])
        .unwrap();
    assert_eq!(
        rewritten, 1,
        "{expectation}: the fixture must rewrite exactly one row"
    );
    drop(persister);

    let opened = SqlitePersister::open(SqlitePersisterConfig::new(&path)).unwrap();
    let error = match opened.load() {
        Ok(_) => panic!("Strict must refuse the row: {expectation}"),
        Err(error) => error,
    };
    let PersistenceError::Backend { source, .. } = error else {
        panic!("expected a backend error, got {error:?}")
    };
    assert!(
        source
            .downcast_ref::<WalletStorageError>()
            .is_some_and(&strict),
        "expected `{expectation}`, got {source:?}"
    );
    drop(opened);

    let recovery = SqlitePersister::open(
        SqlitePersisterConfig::new(&path).with_load_policy(LoadPolicy::Recovery),
    )
    .unwrap();
    let state = recovery.load().expect("recovery isolates the wallet");
    assert!(state.wallets.contains_key(&healthy.wallet_id));
    assert!(
        !state.wallets.contains_key(&sick.wallet_id),
        "Recovery must not return the wallet: {expectation}"
    );
    let degradation = recovery.last_load_degradation();
    assert_eq!(degradation.wallets_degraded.len(), 1);
    assert_eq!(
        degradation.wallets_degraded.get(&sick.wallet_id).copied(),
        Some("blob_decode"),
        "{expectation}"
    );
}

/// Past the freshly derived window there is no derived entry to compare a
/// persisted row with, yet the row's address is what the wallet would hand
/// out. It has to match the account's own derivation there too.
#[test]
fn should_reject_a_pool_row_beyond_the_derived_window_that_its_account_cannot_derive() {
    use key_wallet::managed_account::address_pool::AddressPoolType;

    assert_pool_corruption_is_typed_and_isolated(
        |wallet, info| {
            let account_type = AccountType::IdentityInvitation;
            let source = account_key_source(wallet, account_type);
            let deep = with_pool(info, account_type, AddressPoolType::Absent, |pool| {
                pool.generate_addresses(40, &source, true).unwrap();
                pool.highest_generated.unwrap()
            });
            // A well-formed P2PKH script and a correctly sized key that are
            // not this account's child at that index.
            format!(
                "UPDATE core_address_pool \
                 SET script = X'76a914{}88ac', public_key = X'02{}' \
                 WHERE wallet_id = ?1 AND account_type = 'identity_invitation' \
                   AND address_index = {deep}",
                "42".repeat(20),
                "11".repeat(32),
            )
        },
        "core pool row conflicts with its derived address or key",
    );
}

/// Pool snapshots are not rewritten when an address is funded, so a row can
/// say "available" while `core_utxos` holds a coin on it. Past the freshly
/// derived window nothing anchors that address until the persisted pool is
/// restored, and the reuse guard has to reach it there.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn should_not_hand_out_a_funded_address_restored_beyond_the_derived_window() {
    use key_wallet::account::StandardAccountType;
    use key_wallet::managed_account::address_pool::AddressPoolType;

    let account_type = AccountType::Standard {
        index: 0,
        standard_account_type: StandardAccountType::BIP44Account,
    };
    let wallet =
        Wallet::new_random(Network::Testnet, WalletAccountCreationOptions::Default).unwrap();
    let mut info = ManagedWalletInfo::from_wallet(&wallet, 1);
    let source = account_key_source(&wallet, account_type);
    let (funded, funded_index) =
        with_pool(&mut info, account_type, AddressPoolType::External, |pool| {
            // Hand out, without funding, more addresses than one gap window.
            for _ in 0..pool.gap_limit + 10 {
                pool.next_unused_and_reserve(&source, 12345).unwrap();
            }
            let funded = pool.next_unused(&source, true).unwrap();
            let index = pool.address_index[&funded];
            assert_eq!(index, pool.gap_limit + 10);
            (funded, index)
        });
    let (persister, _tmp, path) = common::fresh_persister();
    persister
        .store(wallet.wallet_id, registration(&wallet, &info))
        .unwrap();
    // A coin with no transaction record: history replay cannot mark it.
    persister
        .store(
            wallet.wallet_id,
            PlatformWalletChangeSet {
                core: Some(CoreChangeSet {
                    new_utxos: vec![key_wallet::Utxo::new(
                        OutPoint::new(Txid::from_byte_array([9; 32]), 0),
                        TxOut {
                            value: 50_000,
                            script_pubkey: funded.script_pubkey(),
                        },
                        funded.clone(),
                        100,
                        false,
                    )],
                    synced_height: Some(100),
                    last_processed_height: Some(100),
                    ..Default::default()
                }),
                ..Default::default()
            },
        )
        .unwrap();
    drop(persister);

    let manager = manager(SqlitePersister::open(SqlitePersisterConfig::new(path)).unwrap()).await;
    let wm = manager.wallet_manager_arc();
    let mut wm = wm.write().await;
    let restored = &mut wm
        .get_wallet_info_mut(&wallet.wallet_id)
        .unwrap()
        .core_wallet;
    with_pool(restored, account_type, AddressPoolType::External, |pool| {
        assert_eq!(
            pool.addresses[&funded_index].state,
            AddressState::Used,
            "a funded address must come back used"
        );
        assert_ne!(
            pool.next_unused(&source, true).unwrap(),
            funded,
            "a funded address must not be handed out again"
        );
    });
    drop(wm);
    assert!(manager.shutdown().await.all_clean());
}

/// A row filed under a pool its account does not have is read by no pool, so
/// the reservation it carries would vanish from a load that reports success.
#[test]
fn should_reject_a_pool_row_filed_under_a_pool_its_account_does_not_have() {
    use key_wallet::managed_account::address_pool::AddressPoolType;

    assert_pool_corruption_is_typed_and_isolated(
        |wallet, info| {
            let account_type = AccountType::IdentityInvitation;
            let source = account_key_source(wallet, account_type);
            with_pool(info, account_type, AddressPoolType::Absent, |pool| {
                pool.next_unused_and_reserve(&source, 12345).unwrap();
            });
            "UPDATE core_address_pool SET pool_type = 0 \
             WHERE wallet_id = ?1 AND account_type = 'identity_invitation' \
               AND reserved_at IS NOT NULL"
                .to_string()
        },
        "core pool row names a pool its account does not have",
    );
}

/// Two contacts share an account label, index and key class; only the
/// identity pair tells their stored pools apart. They are given different
/// states at the same indices, so a row read under the wrong contact shows.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn should_restore_each_dashpay_contact_pool_under_its_own_identity_pair() {
    use key_wallet::managed_account::address_pool::AddressPoolType;

    let contact = |friend: u8| AccountType::DashpayReceivingFunds {
        index: 0,
        user_identity_id: [0x21; 32],
        friend_identity_id: [friend; 32],
    };
    let mut wallet =
        Wallet::new_random(Network::Testnet, WalletAccountCreationOptions::Default).unwrap();
    for friend in [0x31, 0x32] {
        wallet.add_account(contact(friend), None).unwrap();
    }
    let mut info = ManagedWalletInfo::from_wallet(&wallet, 1);
    let mut expected = Vec::new();
    for (friend, funded, reserved_at) in [(0x31, true, 111), (0x32, false, 222)] {
        let account_type = contact(friend);
        let source = account_key_source(&wallet, account_type);
        let addresses = with_pool(&mut info, account_type, AddressPoolType::Absent, |pool| {
            if funded {
                let first = pool.next_unused(&source, true).unwrap();
                assert!(pool.mark_used(&first));
            }
            pool.next_unused_and_reserve(&source, reserved_at).unwrap();
            pool.addresses.clone()
        });
        expected.push((account_type, addresses));
    }
    let state_at = |contact: usize, index: u32| expected[contact].1[&index].state;
    assert_eq!(state_at(0, 0), AddressState::Used);
    assert_eq!(state_at(0, 1), AddressState::Reserved { at: 111 });
    assert_eq!(state_at(1, 0), AddressState::Reserved { at: 222 });
    assert_eq!(state_at(1, 1), AddressState::Available);
    assert_ne!(expected[0].1[&0].address, expected[1].1[&0].address);

    let (persister, _tmp, path) = common::fresh_persister();
    persister
        .store(wallet.wallet_id, registration(&wallet, &info))
        .unwrap();
    drop(persister);
    let manager = manager(SqlitePersister::open(SqlitePersisterConfig::new(path)).unwrap()).await;
    let wm = manager.wallet_manager_arc();
    let mut wm = wm.write().await;
    let restored = &mut wm
        .get_wallet_info_mut(&wallet.wallet_id)
        .unwrap()
        .core_wallet;
    for (account_type, addresses) in expected {
        with_pool(restored, account_type, AddressPoolType::Absent, |pool| {
            for (index, expected) in &addresses {
                let actual = pool.addresses.get(index).expect("stored contact address");
                assert_eq!(actual.state, expected.state, "{account_type:?} #{index}");
                assert_eq!(
                    actual.address, expected.address,
                    "{account_type:?} #{index}"
                );
                assert_eq!(actual.path, expected.path, "{account_type:?} #{index}");
                assert_eq!(actual.public_key, expected.public_key);
            }
        });
    }
    drop(wm);
    assert!(manager.shutdown().await.all_clean());
}

/// A stored key of another curve than its account's.
#[test]
fn should_reject_a_pool_row_whose_key_type_is_not_its_accounts() {
    assert_pool_corruption_is_typed_and_isolated(
        |_, _| {
            "UPDATE core_address_pool SET key_type = 2, public_key = zeroblob(48) \
             WHERE wallet_id = ?1 AND account_type = 'identity_invitation' AND address_index = 0"
                .to_string()
        },
        "core pool key type does not match its account",
    );
}

/// A stored address the account derives differently at that index.
#[test]
fn should_reject_a_pool_row_whose_address_is_not_the_derived_one() {
    assert_pool_corruption_is_typed_and_isolated(
        |_, _| {
            format!(
                "UPDATE core_address_pool SET script = X'76a914{}88ac' \
                 WHERE wallet_id = ?1 AND account_type = 'identity_invitation' \
                   AND address_index = 0",
                "42".repeat(20),
            )
        },
        "core pool row conflicts with its derived address or key",
    );
}

/// A stored key the account derives differently at that index, under the
/// right address.
#[test]
fn should_reject_a_pool_row_whose_key_is_not_the_derived_one() {
    assert_pool_corruption_is_typed_and_isolated(
        |_, _| {
            format!(
                "UPDATE core_address_pool SET public_key = X'02{}' \
                 WHERE wallet_id = ?1 AND account_type = 'identity_invitation' \
                   AND address_index = 0",
                "11".repeat(32),
            )
        },
        "core pool row conflicts with its derived address or key",
    );
}

/// The platform-node pool is hardened, so its rows cannot be re-derived and
/// are trusted as stored. One address filed at two indices is the
/// contradiction that can still be caught there.
#[test]
fn should_reject_a_platform_node_address_stored_at_two_indices() {
    assert_pool_corruption_is_typed_and_isolated(
        |wallet, info| {
            let keys =
                platform_wallet::wallet::provider_key_at_index::derive_platform_node_public_keys(
                    wallet,
                    Network::Testnet,
                    3,
                )
                .unwrap();
            platform_wallet::wallet::provider_key_at_index::populate_platform_node_pool(
                info,
                &keys,
                Network::Testnet,
            )
            .unwrap();
            "UPDATE core_address_pool SET script = ( \
                 SELECT script FROM core_address_pool \
                 WHERE wallet_id = ?1 AND account_type = 'provider_platform' \
                   AND address_index = 0) \
             WHERE wallet_id = ?1 AND account_type = 'provider_platform' AND address_index = 1"
                .to_string()
        },
        "core pool address has multiple derivation indices",
    );
}

/// A platform-node row filed under other account coordinates belongs to no
/// account this wallet rebuilds. Importing it into the platform-node pool
/// would bring its key back without its reservation, so it is left out and
/// the reservation is reported as not taken back.
#[test]
fn should_not_import_a_platform_node_row_filed_under_another_account() {
    use key_wallet::managed_account::address_pool::AddressPoolType;
    use platform_wallet_storage::{LoadPolicy, LoadSite};

    const RESERVED: u32 = 1;
    let wallet =
        Wallet::new_random(Network::Testnet, WalletAccountCreationOptions::Default).unwrap();
    let mut info = ManagedWalletInfo::from_wallet(&wallet, 1);
    let keys = platform_wallet::wallet::provider_key_at_index::derive_platform_node_public_keys(
        &wallet,
        Network::Testnet,
        3,
    )
    .unwrap();
    platform_wallet::wallet::provider_key_at_index::populate_platform_node_pool(
        &mut info,
        &keys,
        Network::Testnet,
    )
    .unwrap();
    with_pool(
        &mut info,
        AccountType::ProviderPlatformKeys,
        AddressPoolType::AbsentHardened,
        |pool| {
            pool.addresses.get_mut(&RESERVED).unwrap().state = AddressState::Reserved { at: 12345 };
        },
    );
    let (persister, _tmp, path) = common::fresh_persister();
    persister
        .store(wallet.wallet_id, registration(&wallet, &info))
        .unwrap();
    let moved = persister
        .lock_conn_for_test()
        .execute(
            "UPDATE core_address_pool SET account_index = 1 \
             WHERE wallet_id = ?1 AND account_type = 'provider_platform' \
               AND address_index = ?2",
            rusqlite::params![wallet.wallet_id.as_slice(), RESERVED],
        )
        .unwrap();
    assert_eq!(moved, 1);
    drop(persister);

    for policy in [LoadPolicy::Strict, LoadPolicy::Recovery] {
        let persister =
            SqlitePersister::open(SqlitePersisterConfig::new(&path).with_load_policy(policy))
                .unwrap();
        let mut state = persister
            .load()
            .unwrap_or_else(|e| panic!("{policy:?}: the wallet still loads: {e:?}"));
        assert_eq!(
            persister
                .last_load_degradation()
                .by_site
                .get(&LoadSite::UnrestoredPoolReservation),
            Some(&1),
            "{policy:?}: the reservation left behind is reported"
        );
        let restored = &mut state
            .wallets
            .get_mut(&wallet.wallet_id)
            .expect("wallet")
            .wallet_info;
        with_pool(
            restored,
            AccountType::ProviderPlatformKeys,
            AddressPoolType::AbsentHardened,
            |pool| {
                assert!(
                    !pool.addresses.contains_key(&RESERVED),
                    "{policy:?}: the foreign row must not become an available key"
                );
                assert_eq!(pool.addresses.len(), keys.len() - 1, "{policy:?}");
            },
        );
    }
}

/// A pool the wallet has really grown past the discovery ceiling is stored
/// row for row by the writer, and has to load again.
#[test]
fn should_restore_a_correctly_derived_pool_deeper_than_the_discovery_ceiling() {
    use key_wallet::managed_account::address_pool::AddressPoolType;

    const DEEPEST: u32 = 10_001;
    let account_type = AccountType::IdentityInvitation;
    let wallet =
        Wallet::new_random(Network::Testnet, WalletAccountCreationOptions::Default).unwrap();
    let mut info = ManagedWalletInfo::from_wallet(&wallet, 1);
    let source = account_key_source(&wallet, account_type);
    let reserved = with_pool(&mut info, account_type, AddressPoolType::Absent, |pool| {
        let derived = pool.highest_generated.map_or(0, |highest| highest + 1);
        pool.generate_addresses(DEEPEST + 1 - derived, &source, true)
            .unwrap();
        assert_eq!(pool.highest_generated, Some(DEEPEST));
        let entry = pool.addresses.get_mut(&DEEPEST).unwrap();
        entry.state = AddressState::Reserved { at: 12345 };
        entry.clone()
    });
    let (persister, _tmp, path) = common::fresh_persister();
    persister
        .store(wallet.wallet_id, registration(&wallet, &info))
        .unwrap();
    drop(persister);

    let mut state = SqlitePersister::open(SqlitePersisterConfig::new(path))
        .unwrap()
        .load()
        .expect("a pool the writer accepted must load");
    let restored = &mut state
        .wallets
        .get_mut(&wallet.wallet_id)
        .expect("wallet")
        .wallet_info;
    with_pool(restored, account_type, AddressPoolType::Absent, |pool| {
        let actual = &pool.addresses[&DEEPEST];
        assert_eq!(actual.state, AddressState::Reserved { at: 12345 });
        assert_eq!(actual.address, reserved.address);
        assert_eq!(actual.path, reserved.path);
    });
}

/// `AddressPool::prune_unused` drops available entries above the gap window
/// but keeps reserved ones, so a valid pool can hold a reservation far above
/// every other stored row. The reservation has to come back, for an ordinary
/// receive pool and for a one-time funding pool alike.
#[test]
fn should_restore_a_reservation_that_pruning_left_far_above_the_rest_of_its_pool() {
    use key_wallet::account::StandardAccountType;
    use key_wallet::managed_account::address_pool::AddressPoolType;
    use platform_wallet_storage::LoadSite;

    const RESERVED: u32 = 12_000;
    for (account_type, pool_type) in [
        (
            AccountType::Standard {
                index: 0,
                standard_account_type: StandardAccountType::BIP44Account,
            },
            AddressPoolType::External,
        ),
        (AccountType::IdentityInvitation, AddressPoolType::Absent),
    ] {
        let wallet =
            Wallet::new_random(Network::Testnet, WalletAccountCreationOptions::Default).unwrap();
        let mut info = ManagedWalletInfo::from_wallet(&wallet, 1);
        let source = account_key_source(&wallet, account_type);
        let reserved = with_pool(&mut info, account_type, pool_type, |pool| {
            pool.generate_up_to(RESERVED, &source).unwrap();
            pool.addresses.get_mut(&RESERVED).unwrap().state = AddressState::Reserved { at: 12345 };
            assert!(pool.prune_unused() > 0);
            assert!(pool.addresses.len() < 100, "the pruned pool must be sparse");
            pool.addresses[&RESERVED].clone()
        });
        let (persister, _tmp, path) = common::fresh_persister();
        persister
            .store(wallet.wallet_id, registration(&wallet, &info))
            .unwrap();
        drop(persister);

        let persister = SqlitePersister::open(SqlitePersisterConfig::new(path)).unwrap();
        let mut state = persister
            .load()
            .unwrap_or_else(|e| panic!("{account_type:?}: a pruned pool must load: {e:?}"));
        let degradation = persister.last_load_degradation();
        for site in [
            LoadSite::PoolRowBeyondDerivationBudget,
            LoadSite::UnrestoredPoolReservation,
        ] {
            assert_eq!(
                degradation.by_site.get(&site),
                None,
                "{account_type:?}: {site:?}"
            );
        }
        let restored = &mut state
            .wallets
            .get_mut(&wallet.wallet_id)
            .expect("wallet")
            .wallet_info;
        with_pool(restored, account_type, pool_type, |pool| {
            let actual = pool
                .addresses
                .get(&RESERVED)
                .unwrap_or_else(|| panic!("{account_type:?}: the reservation is restored"));
            assert_eq!(actual.state, AddressState::Reserved { at: 12345 });
            assert_eq!(actual.address, reserved.address);
            assert_eq!(actual.path, reserved.path);
            assert_eq!(pool.address_index.get(&reserved.address), Some(&RESERVED));
        });
    }
}

/// A reserved row whose account coordinates match no account of the wallet
/// is read by no pool. Outside the one-time funding accounts the row cannot
/// say whether it is damaged or belongs to an account the wallet no longer
/// rebuilds, so the load keeps the wallet and reports the reservation it
/// could not take back.
#[test]
fn should_report_a_reservation_that_no_restored_pool_takes_back() {
    use key_wallet::managed_account::address_pool::AddressPoolType;
    use platform_wallet_storage::{LoadPolicy, LoadSite};

    let (persister, _tmp, path) = common::fresh_persister();
    let wallet =
        Wallet::new_random(Network::Testnet, WalletAccountCreationOptions::Default).unwrap();
    let mut info = ManagedWalletInfo::from_wallet(&wallet, 1);
    let mut reserved = Vec::new();
    for account_type in [
        AccountType::ProviderOwnerKeys,
        AccountType::ProviderVotingKeys,
    ] {
        let source = account_key_source(&wallet, account_type);
        reserved.push(with_pool(
            &mut info,
            account_type,
            AddressPoolType::Absent,
            |pool| {
                let address = pool.next_unused_and_reserve(&source, 12345).unwrap();
                pool.address_index[&address]
            },
        ));
    }
    persister
        .store(wallet.wallet_id, registration(&wallet, &info))
        .unwrap();
    persister.load().unwrap();
    assert_eq!(
        persister
            .last_load_degradation()
            .by_site
            .get(&LoadSite::UnrestoredPoolReservation),
        None,
        "every reservation of an intact store is taken back"
    );
    // A provider key account's key class is always zero.
    let rewritten = persister
        .lock_conn_for_test()
        .execute(
            "UPDATE core_address_pool SET key_class = 1 \
             WHERE wallet_id = ?1 AND account_type = 'provider_owner' \
               AND reserved_at IS NOT NULL",
            [wallet.wallet_id.as_slice()],
        )
        .unwrap();
    assert_eq!(rewritten, 1);
    drop(persister);

    for policy in [LoadPolicy::Strict, LoadPolicy::Recovery] {
        let persister =
            SqlitePersister::open(SqlitePersisterConfig::new(&path).with_load_policy(policy))
                .unwrap();
        let mut state = persister
            .load()
            .expect("an unmatched reservation must not fail the wallet");
        assert_eq!(
            persister
                .last_load_degradation()
                .by_site
                .get(&LoadSite::UnrestoredPoolReservation),
            Some(&1),
            "{policy:?}: the lost reservation is counted, the restored one is not"
        );
        let restored = &mut state
            .wallets
            .get_mut(&wallet.wallet_id)
            .expect("the wallet still loads")
            .wallet_info;
        let state_of = |restored: &mut ManagedWalletInfo, account_type, index| {
            with_pool(restored, account_type, AddressPoolType::Absent, |pool| {
                pool.addresses[&index].state
            })
        };
        assert_eq!(
            state_of(restored, AccountType::ProviderOwnerKeys, reserved[0]),
            AddressState::Available,
            "{policy:?}: this is the loss the counter reports"
        );
        assert_eq!(
            state_of(restored, AccountType::ProviderVotingKeys, reserved[1]),
            AddressState::Reserved { at: 12345 },
        );
    }
}

/// The saved pool's own `Used` rows have to drive the gap refill the load
/// runs for used addresses: a coin past the saved window, with no transaction
/// record and no pool row, is only derived by that refill.
#[test]
fn should_refill_the_gap_window_past_a_used_address_restored_from_its_pool_row() {
    use key_wallet::account::StandardAccountType;
    use key_wallet::managed_account::address_pool::AddressPoolType;

    const USED: u32 = 20;
    let account_type = AccountType::Standard {
        index: 0,
        standard_account_type: StandardAccountType::BIP44Account,
    };
    let wallet =
        Wallet::new_random(Network::Testnet, WalletAccountCreationOptions::Default).unwrap();
    let mut info = ManagedWalletInfo::from_wallet(&wallet, 1);
    let source = account_key_source(&wallet, account_type);
    let (funded, funded_index, gap) =
        with_pool(&mut info, account_type, AddressPoolType::External, |pool| {
            let gap = pool.gap_limit;
            assert_eq!(pool.highest_generated, Some(gap - 1));
            assert!(USED < gap);
            // Past the saved window and past the discovery horizon, but inside
            // the gap window of the used address.
            let funded_index = gap + 15;
            assert!(funded_index <= USED + gap);
            let mut deeper = pool.clone();
            deeper
                .generate_addresses(funded_index + 1 - gap, &source, true)
                .unwrap();
            let funded = deeper.addresses[&funded_index].address.clone();
            // Saved as used, through the initial window only.
            assert!(pool.mark_index_used(USED));
            (funded, funded_index, gap)
        });
    let (persister, _tmp, path) = common::fresh_persister();
    persister
        .store(wallet.wallet_id, registration(&wallet, &info))
        .unwrap();
    // A coin with no transaction record: history replay cannot mark it.
    persister
        .store(
            wallet.wallet_id,
            PlatformWalletChangeSet {
                core: Some(CoreChangeSet {
                    new_utxos: vec![key_wallet::Utxo::new(
                        OutPoint::new(Txid::from_byte_array([9; 32]), 0),
                        TxOut {
                            value: 50_000,
                            script_pubkey: funded.script_pubkey(),
                        },
                        funded.clone(),
                        100,
                        false,
                    )],
                    synced_height: Some(100),
                    last_processed_height: Some(100),
                    ..Default::default()
                }),
                ..Default::default()
            },
        )
        .unwrap();
    drop(persister);

    let mut state = SqlitePersister::open(SqlitePersisterConfig::new(path))
        .unwrap()
        .load()
        .unwrap();
    let restored = &mut state
        .wallets
        .get_mut(&wallet.wallet_id)
        .expect("wallet")
        .wallet_info;
    assert_eq!(restored.balance.total(), 50_000);
    with_pool(restored, account_type, AddressPoolType::External, |pool| {
        assert!(
            pool.highest_generated >= Some(USED + gap),
            "the window past the used address must be derived, got {:?}",
            pool.highest_generated
        );
        assert_eq!(
            pool.addresses[&funded_index].state,
            AddressState::Used,
            "a funded address must come back used"
        );
        assert_ne!(
            pool.next_unused(&source, true).unwrap(),
            funded,
            "a funded address must not be handed out again"
        );
    });
}

/// The accounts whose keys fund one asset lock each, with the label their
/// pool rows are stored under. The top-up account is the one with an account
/// index of its own.
fn one_time_funding_accounts() -> [(AccountType, &'static str); 6] {
    [
        (AccountType::IdentityRegistration, "identity_registration"),
        (
            AccountType::IdentityTopUp {
                registration_index: 3,
            },
            "identity_topup",
        ),
        (
            AccountType::IdentityTopUpNotBoundToIdentity,
            "identity_topup_unbound",
        ),
        (AccountType::IdentityInvitation, "identity_invitation"),
        (
            AccountType::AssetLockAddressTopUp,
            "asset_lock_address_topup",
        ),
        (
            AccountType::AssetLockShieldedAddressTopUp,
            "asset_lock_shielded_topup",
        ),
    ]
}

/// A wallet holding `account_type`, which a default wallet lacks only for the
/// on-demand top-up account.
fn wallet_with_funding_account(account_type: AccountType) -> Wallet {
    let mut wallet =
        Wallet::new_random(Network::Testnet, WalletAccountCreationOptions::Default).unwrap();
    if matches!(account_type, AccountType::IdentityTopUp { .. }) {
        wallet.add_account(account_type, None).unwrap();
    }
    wallet
}

/// Burn the first two one-time keys of `account_type` the way the asset-lock
/// builder does, or burn the first and only reserve the second.
fn burn_two_funding_keys(info: &mut ManagedWalletInfo, account_type: AccountType, reserve: bool) {
    use key_wallet::managed_account::address_pool::AddressPoolType;

    with_pool(info, account_type, AddressPoolType::Absent, |pool| {
        assert!(pool.mark_index_used(0));
        if reserve {
            pool.addresses.get_mut(&1).unwrap().state = AddressState::Reserved { at: 12345 };
        } else {
            assert!(pool.mark_index_used(1));
        }
    });
}

/// The index the asset-lock builder would fund its next credit output from.
fn next_funding_index(info: &mut ManagedWalletInfo, account_type: AccountType) -> u32 {
    let mut accounts = info.accounts.all_accounts_mut();
    let account = accounts
        .iter_mut()
        .find(|account| account.managed_account_type().to_account_type() == account_type)
        .expect("funding account");
    let pool = account
        .managed_account_type_mut()
        .address_pools_mut()
        .into_iter()
        .next()
        .expect("funding pool");
    pool.next_unused_with_info(&KeySource::NoKeySource, false)
        .expect("an unused funding key")
        .index
}

/// Damage the row of a burned (or only reserved) one-time funding key with
/// `damage`, for every such account, and expect the wallet not to load.
fn assert_damaged_one_time_funding_row_fails_the_wallet(
    damage: &str,
    reserve: bool,
    only: impl Fn(&AccountType) -> bool,
    strict: impl Fn(&platform_wallet_storage::WalletStorageError) -> bool + Copy,
) {
    for (account_type, label) in one_time_funding_accounts() {
        if !only(&account_type) {
            continue;
        }
        assert_wallet_corruption_is_isolated(
            wallet_with_funding_account(account_type),
            |_, info| {
                burn_two_funding_keys(info, account_type, reserve);
                format!(
                    "UPDATE core_address_pool SET {damage} \
                     WHERE wallet_id = ?1 AND account_type = '{label}' AND address_index = 1"
                )
            },
            strict,
            &format!("{label}: {damage} (reserved: {reserve})"),
        );
    }
}

fn is_blob_decode(
    expected: &'static str,
) -> impl Fn(&platform_wallet_storage::WalletStorageError) -> bool + Copy {
    move |error| {
        matches!(
            error,
            platform_wallet_storage::WalletStorageError::BlobDecode { reason } if *reason == expected
        )
    }
}

/// A one-time funding key is handed to someone else: an invitation exports
/// it. Its pool row is the only record that it is gone, so a wallet that
/// cannot take such a row back must not load, or the same key funds the next
/// asset lock too. Here the row's account coordinates match no account.
#[test]
fn should_refuse_a_wallet_whose_one_time_funding_key_row_matches_no_account() {
    let unmatched = is_blob_decode("one-time funding key row matches no restored account");
    for reserve in [false, true] {
        assert_damaged_one_time_funding_row_fails_the_wallet(
            "key_class = 1",
            reserve,
            |_| true,
            unmatched,
        );
    }
    // The top-up account is the one whose account index tells accounts apart.
    assert_damaged_one_time_funding_row_fails_the_wallet(
        "account_index = account_index + 1",
        false,
        |account| matches!(account, AccountType::IdentityTopUp { .. }),
        unmatched,
    );
}

/// The row sits under the right account, but above the derivation budget of
/// its pool, and it is not the entry derived at its own index.
#[test]
fn should_refuse_a_wallet_whose_one_time_funding_key_row_lies_beyond_the_derivation_budget() {
    assert_damaged_one_time_funding_row_fails_the_wallet(
        "address_index = 2147483647",
        false,
        |_| true,
        is_blob_decode("one-time funding key row could not be restored"),
    );
}

/// The row's script is not an address. On a used row that already fails
/// Strict in the used-address reader; Recovery tolerates it there, and must
/// still not return the wallet.
#[test]
fn should_refuse_a_wallet_whose_one_time_funding_key_row_has_an_unreadable_script() {
    assert_damaged_one_time_funding_row_fails_the_wallet(
        "script = X'6a'",
        false,
        |_| true,
        |error| {
            matches!(
                error,
                platform_wallet_storage::WalletStorageError::AddressDecode { .. }
            )
        },
    );
    // A reserved row is not read by the used-address reader at all.
    assert_damaged_one_time_funding_row_fails_the_wallet(
        "script = X'6a'",
        true,
        |_| true,
        is_blob_decode("one-time funding key row could not be restored"),
    );
}

/// The control for the refusals above: burned and reserved one-time keys of
/// an intact store come back as they were, and the next key is a new one.
#[test]
fn should_load_intact_one_time_funding_key_rows_and_hand_out_the_next_key() {
    use key_wallet::managed_account::address_pool::AddressPoolType;
    use platform_wallet_storage::LoadPolicy;

    for (account_type, label) in one_time_funding_accounts() {
        for reserve in [false, true] {
            let (persister, _tmp, path) = common::fresh_persister();
            let wallet = wallet_with_funding_account(account_type);
            let mut info = ManagedWalletInfo::from_wallet(&wallet, 1);
            burn_two_funding_keys(&mut info, account_type, reserve);
            persister
                .store(wallet.wallet_id, registration(&wallet, &info))
                .unwrap();
            drop(persister);

            for policy in [LoadPolicy::Strict, LoadPolicy::Recovery] {
                let mut state = SqlitePersister::open(
                    SqlitePersisterConfig::new(&path).with_load_policy(policy),
                )
                .unwrap()
                .load()
                .unwrap_or_else(|error| panic!("{label}, {policy:?}: {error}"));
                let restored = &mut state
                    .wallets
                    .get_mut(&wallet.wallet_id)
                    .unwrap_or_else(|| panic!("{label}, {policy:?}: wallet dropped"))
                    .wallet_info;
                let second = if reserve {
                    AddressState::Reserved { at: 12345 }
                } else {
                    AddressState::Used
                };
                with_pool(restored, account_type, AddressPoolType::Absent, |pool| {
                    assert_eq!(pool.addresses[&0].state, AddressState::Used, "{label}");
                    assert_eq!(pool.addresses[&1].state, second, "{label}");
                });
                assert_eq!(next_funding_index(restored, account_type), 2, "{label}");
            }
        }
    }
}

/// A burned one-time key is restored on its own pool, so an intact store
/// loads clean: its used address is not also routed to a funds account it
/// does not belong to, where it would be counted as an orphaned owner and an
/// address that account's key cannot derive.
#[test]
fn should_load_burned_one_time_funding_keys_without_reporting_degradation() {
    use platform_wallet_storage::LoadPolicy;

    for (account_type, label) in one_time_funding_accounts() {
        let (persister, _tmp, path) = common::fresh_persister();
        let wallet = wallet_with_funding_account(account_type);
        let mut info = ManagedWalletInfo::from_wallet(&wallet, 1);
        burn_two_funding_keys(&mut info, account_type, false);
        persister
            .store(wallet.wallet_id, registration(&wallet, &info))
            .unwrap();
        drop(persister);

        for policy in [LoadPolicy::Strict, LoadPolicy::Recovery] {
            let persister =
                SqlitePersister::open(SqlitePersisterConfig::new(&path).with_load_policy(policy))
                    .unwrap();
            persister
                .load()
                .unwrap_or_else(|error| panic!("{label}, {policy:?}: {error}"));
            let degradation = persister.last_load_degradation();
            assert!(
                !degradation.degraded,
                "{label}, {policy:?}: an intact store reported {:?}",
                degradation.by_site
            );
        }
    }
}

/// The limit of that guarantee. The row is the only record that a key was
/// burned: once the row itself is gone nothing in the store says so, the
/// wallet loads, and the key is handed out again.
#[test]
fn should_hand_out_again_a_one_time_funding_key_whose_row_was_deleted() {
    let account_type = AccountType::IdentityInvitation;
    let (persister, _tmp, path) = common::fresh_persister();
    let wallet = wallet_with_funding_account(account_type);
    let mut info = ManagedWalletInfo::from_wallet(&wallet, 1);
    burn_two_funding_keys(&mut info, account_type, false);
    persister
        .store(wallet.wallet_id, registration(&wallet, &info))
        .unwrap();
    let deleted = persister
        .lock_conn_for_test()
        .execute(
            "DELETE FROM core_address_pool \
             WHERE account_type = 'identity_invitation' AND address_index = 1",
            [],
        )
        .unwrap();
    assert_eq!(deleted, 1);
    drop(persister);

    let persister = SqlitePersister::open(SqlitePersisterConfig::new(path)).unwrap();
    let mut state = persister
        .load()
        .expect("nothing left in the store contradicts the wallet");
    let restored = &mut state
        .wallets
        .get_mut(&wallet.wallet_id)
        .unwrap()
        .wallet_info;
    assert_eq!(next_funding_index(restored, account_type), 1);
}

/// `v4.2` registered both Standard variants under the label `standard`. That
/// label lives in the account registration; the pool rows of such a store
/// carry the split label, so the account gets its pools back.
#[test]
fn should_restore_pools_of_a_standard_account_registered_under_the_legacy_label() {
    use key_wallet::account::StandardAccountType;
    use key_wallet::managed_account::address_pool::AddressPoolType;

    let account_type = AccountType::Standard {
        index: 0,
        standard_account_type: StandardAccountType::BIP44Account,
    };
    let (persister, _tmp, path) = common::fresh_persister();
    let wallet =
        Wallet::new_random(Network::Testnet, WalletAccountCreationOptions::Default).unwrap();
    let mut info = ManagedWalletInfo::from_wallet(&wallet, 1);
    let source = account_key_source(&wallet, account_type);
    let reserved = with_pool(&mut info, account_type, AddressPoolType::External, |pool| {
        pool.next_unused_and_reserve(&source, 12345).unwrap()
    });
    persister
        .store(wallet.wallet_id, registration(&wallet, &info))
        .unwrap();
    {
        let conn = persister.lock_conn_for_test();
        let relabelled = conn
            .execute(
                "UPDATE account_registrations SET account_type = 'standard' \
                 WHERE account_type = 'standard_bip44'",
                [],
            )
            .unwrap();
        assert_eq!(relabelled, 1);
        let legacy_pool_rows: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM core_address_pool WHERE account_type = 'standard'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(legacy_pool_rows, 0, "no writer files a pool row under it");
    }
    drop(persister);

    let persister = SqlitePersister::open(SqlitePersisterConfig::new(path)).unwrap();
    let mut state = persister.load().expect("a v4.2-labelled account loads");
    assert!(!persister.last_load_degradation().degraded);
    let restored = &mut state
        .wallets
        .get_mut(&wallet.wallet_id)
        .unwrap()
        .wallet_info;
    with_pool(restored, account_type, AddressPoolType::External, |pool| {
        assert_eq!(
            pool.addresses[&pool.address_index[&reserved]].state,
            AddressState::Reserved { at: 12345 }
        );
    });
}
