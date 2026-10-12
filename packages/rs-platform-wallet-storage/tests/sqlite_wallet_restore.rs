//! Core snapshot restoration through SQLite reopen and real manager hydration.

mod common;

use std::sync::Arc;

use dashcore::{hashes::Hash, BlockHash, Network, OutPoint, Transaction, TxIn, TxOut, Txid};
use key_wallet::{
    account::AccountType,
    managed_account::address_pool::{AddressState, KeySource},
    transaction_checking::{BlockInfo, TransactionContext, WalletTransactionChecker},
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
        let source = wallet
            .accounts
            .account_of_type(account_type)
            .map(|keys| KeySource::Public(keys.account_xpub));
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
        let source = wallet
            .accounts
            .account_of_type(account_type)
            .map(|keys| KeySource::Public(keys.account_xpub));
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

fn transaction(input: OutPoint, outputs: Vec<TxOut>) -> Transaction {
    Transaction {
        version: 1,
        lock_time: 0,
        input: vec![TxIn {
            previous_output: input,
            ..Default::default()
        }],
        output: outputs,
        special_transaction_payload: None,
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn should_restore_pending_sends_in_dependency_order_without_resending_incoming() {
    let mut wallet =
        Wallet::new_random(Network::Testnet, WalletAccountCreationOptions::Default).unwrap();
    let mut info = ManagedWalletInfo::from_wallet(&wallet, 1);
    let xpub = wallet.accounts.standard_bip44_accounts[&0].account_xpub;
    let address = info
        .accounts
        .standard_bip44_accounts
        .get_mut(&0)
        .unwrap()
        .next_receive_address(Some(&xpub), true)
        .unwrap();
    let output = |value| TxOut {
        value,
        script_pubkey: address.script_pubkey(),
    };
    let funding = transaction(
        OutPoint::new(Txid::from_byte_array([1; 32]), 0),
        vec![output(100_000)],
    );
    let parent = transaction(OutPoint::new(funding.txid(), 0), vec![output(90_000)]);
    let child = transaction(OutPoint::new(parent.txid(), 0), vec![output(80_000)]);
    let incoming = transaction(
        OutPoint::new(Txid::from_byte_array([2; 32]), 0),
        vec![output(5_000)],
    );
    let (persister, _tmp, path) = common::fresh_persister();
    persister
        .store(wallet.wallet_id, registration(&wallet, &info))
        .unwrap();
    for (tx, context) in [
        (
            &funding,
            TransactionContext::InBlock(BlockInfo::new(
                100,
                BlockHash::from_byte_array([3; 32]),
                1,
            )),
        ),
        (&parent, TransactionContext::Mempool),
        (&child, TransactionContext::Mempool),
        (&incoming, TransactionContext::Mempool),
    ] {
        let result = info
            .check_core_transaction(tx, context, &mut wallet, true, true)
            .await;
        assert!(result.is_relevant);
        persister
            .store(
                wallet.wallet_id,
                PlatformWalletChangeSet {
                    core: Some(CoreChangeSet {
                        records: result.new_records,
                        new_utxos: info.accounts.standard_bip44_accounts[&0]
                            .utxos
                            .values()
                            .cloned()
                            .collect(),
                        synced_height: Some(100),
                        last_processed_height: Some(100),
                        ..Default::default()
                    }),
                    ..Default::default()
                },
            )
            .unwrap();
    }
    drop(persister);
    let persister = SqlitePersister::open(SqlitePersisterConfig::new(path)).unwrap();
    let start = persister.load().unwrap();
    let pending = &start.wallets[&wallet.wallet_id].unconfirmed_outgoing_txs;
    assert!(
        pending.is_empty(),
        "restoration must not schedule transactions for rebroadcast"
    );
    let manager = manager(persister).await;
    let wm = manager.wallet_manager_arc();
    let wm = wm.read().await;
    let restored = &wm.get_wallet_info(&wallet.wallet_id).unwrap().core_wallet;
    let utxos = &restored.accounts.standard_bip44_accounts[&0].utxos;
    assert!(!utxos.contains_key(&OutPoint::new(funding.txid(), 0)));
    assert!(!utxos.contains_key(&OutPoint::new(parent.txid(), 0)));
    assert!(utxos.contains_key(&OutPoint::new(child.txid(), 0)));
    assert!(utxos.contains_key(&OutPoint::new(incoming.txid(), 0)));
    assert_eq!(utxos.values().map(|utxo| utxo.value()).sum::<u64>(), 85_000);
    use key_wallet::managed_account::managed_account_trait::ManagedAccountTrait;
    let before = info.accounts.standard_bip44_accounts[&0]
        .managed_account_type()
        .address_pools()[0]
        .addresses
        .values()
        .find(|entry| entry.address == address)
        .unwrap();
    let after = restored.accounts.standard_bip44_accounts[&0]
        .managed_account_type()
        .address_pools()[0]
        .addresses
        .values()
        .find(|entry| entry.address == address)
        .unwrap();
    assert_eq!(
        (
            after.tx_count,
            after.total_received,
            after.total_sent,
            after.balance
        ),
        (
            before.tx_count,
            before.total_received,
            before.total_sent,
            before.balance
        )
    );
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

/// A wallet is loaded inside its own `BEGIN IMMEDIATE` transaction, so a
/// writer on another connection cannot change the rows mid-load.
///
/// Timing is deterministic, not slept for: SQLite calls the authorizer while
/// it compiles a statement, so the write is attempted from inside the load's
/// own `core_utxos` read. The busy rejection is the proof that the load
/// transaction is open at that instant — in WAL mode a second connection is
/// refused the write lock only while another connection holds it, and the
/// persister holds it nowhere else during `load()`.
#[test]
fn should_reject_a_concurrent_writer_while_a_wallet_load_transaction_is_open() {
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
            matches!(
                attempt,
                Err(rusqlite::Error::SqliteFailure(error, _))
                    if error.code == rusqlite::ErrorCode::DatabaseBusy
            ),
            "a write during a wallet's load transaction must be refused as busy, got {attempt:?}"
        );
    }

    // Control: the same statement lands once the load has released its lock,
    // and the next load reads it — so the refusal above was the load's lock,
    // not an inert statement.
    let late_writer = rusqlite::Connection::open(&path).unwrap();
    assert_eq!(late_writer.execute(CONCURRENT_WRITE, []).unwrap(), 1);
    assert_eq!(synced_height(&persister), 200);
}

/// A failure while rebuilding a wallet reaches the caller as the storage
/// error that caused it: Strict returns it typed, and Recovery names it as
/// the dropped wallet's cause rather than an anonymous `backend`.
#[test]
fn should_attribute_a_platform_node_pool_failure_to_its_typed_cause() {
    use platform_wallet::changeset::PersistenceError;
    use platform_wallet_storage::{LoadPolicy, WalletStorageError};

    let (persister, _tmp, path) = common::fresh_persister();
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
    // A well-formed BLS key on the account that only ever holds EdDSA keys.
    let rewritten = persister
        .lock_conn_for_test()
        .execute(
            "UPDATE core_address_pool SET key_type = 2, public_key = zeroblob(48) \
             WHERE account_type = 'provider_platform'",
            [],
        )
        .unwrap();
    assert_eq!(rewritten, platform_keys.len());
    drop(persister);

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
    assert!(!state.wallets.contains_key(&wallet.wallet_id));
    assert_eq!(
        recovery
            .last_load_degradation()
            .wallets_degraded
            .get(&wallet.wallet_id)
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

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn should_restore_provider_key_transaction_history_and_positions() {
    use dashcore::blockdata::transaction::special_transaction::{
        provider_registration::{ProviderMasternodeType, ProviderRegistrationPayload},
        provider_update_registrar::ProviderUpdateRegistrarPayload,
        TransactionPayload,
    };
    use key_wallet::managed_account::address_pool::PublicKeyType;
    use key_wallet::managed_account::managed_account_trait::ManagedAccountTrait;

    let mut wallet =
        Wallet::new_random(Network::Testnet, WalletAccountCreationOptions::Default).unwrap();
    let mut info = ManagedWalletInfo::from_wallet(&wallet, 1);
    for role in [
        AccountType::ProviderOwnerKeys,
        AccountType::ProviderVotingKeys,
    ] {
        let source = KeySource::Public(wallet.accounts.account_of_type(role).unwrap().account_xpub);
        let mut accounts = info.accounts.all_accounts_mut();
        let account = accounts
            .iter_mut()
            .find(|account| account.managed_account_type().to_account_type() == role)
            .unwrap();
        for pool in account.managed_account_type_mut().address_pools_mut() {
            pool.generate_addresses(45, &source, true).unwrap();
        }
    }
    let hash_at = |role| {
        let accounts = info.accounts.all_accounts();
        let account = accounts
            .iter()
            .find(|account| account.managed_account_type().to_account_type() == role)
            .unwrap();
        let address = &account.managed_account_type().address_pools()[0].addresses[&40].address;
        let dashcore::address::Payload::PubkeyHash(hash) = address.payload() else {
            panic!("P2PKH")
        };
        *hash
    };
    let owner = hash_at(AccountType::ProviderOwnerKeys);
    let voting = hash_at(AccountType::ProviderVotingKeys);
    let operator_pool = info
        .accounts
        .provider_operator_keys
        .as_ref()
        .unwrap()
        .managed_account_type()
        .address_pools()[0];
    let Some(PublicKeyType::BLS(operator)) = &operator_pool.addresses[&0].public_key else {
        panic!("BLS key")
    };
    let operator = dashcore::bls_sig_utils::BLSPublicKey::from(
        <[u8; 48]>::try_from(operator.as_slice()).unwrap(),
    );
    let special = |payload| Transaction {
        version: 3,
        lock_time: 0,
        input: vec![],
        output: vec![],
        special_transaction_payload: Some(payload),
    };
    let registration_tx = special(TransactionPayload::ProviderRegistrationPayloadType(
        ProviderRegistrationPayload {
            version: 2,
            masternode_type: ProviderMasternodeType::Regular,
            masternode_mode: 0,
            collateral_outpoint: OutPoint::null(),
            service_address: "127.0.0.1:19999".parse().unwrap(),
            owner_key_hash: owner,
            operator_public_key: operator,
            voting_key_hash: voting,
            operator_reward: 0,
            script_payout: dashcore::ScriptBuf::new(),
            inputs_hash: [0; 32].into(),
            signature: vec![],
            platform_node_id: None,
            platform_p2p_port: None,
            platform_http_port: None,
        },
    ));
    let pro_tx_hash = registration_tx.txid();
    let registrar = special(TransactionPayload::ProviderUpdateRegistrarPayloadType(
        ProviderUpdateRegistrarPayload {
            version: 2,
            pro_tx_hash,
            provider_mode: 0,
            operator_public_key: operator,
            voting_key_hash: voting,
            script_payout: dashcore::ScriptBuf::new(),
            inputs_hash: [0; 32].into(),
            payload_sig: vec![],
        },
    ));
    let (persister, _tmp, path) = common::fresh_persister();
    persister
        .store(wallet.wallet_id, registration(&wallet, &info))
        .unwrap();
    let mut expected = Vec::new();
    for (position, tx) in [registration_tx, registrar].into_iter().enumerate() {
        let context = TransactionContext::InBlock(BlockInfo::new(
            100,
            BlockHash::from_byte_array([7; 32]),
            position as u32,
        ));
        let result = info
            .check_core_transaction(&tx, context, &mut wallet, true, true)
            .await;
        assert!(
            result.is_relevant,
            "provider payload #{position} must match owned keys"
        );
        assert!(!result.new_records.is_empty());
        expected.extend(result.new_records.clone());
        persister
            .store(
                wallet.wallet_id,
                PlatformWalletChangeSet {
                    core: Some(CoreChangeSet {
                        records: result.new_records,
                        ..Default::default()
                    }),
                    ..Default::default()
                },
            )
            .unwrap();
    }
    drop(persister);
    let manager = manager(SqlitePersister::open(SqlitePersisterConfig::new(path)).unwrap()).await;
    let wm = manager.wallet_manager_arc();
    let wm = wm.read().await;
    let restored = &wm.get_wallet_info(&wallet.wallet_id).unwrap().core_wallet;
    for expected in expected {
        let accounts = restored.accounts.all_accounts();
        let account = accounts
            .iter()
            .find(|account| {
                account.managed_account_type().to_account_type() == expected.account_type
            })
            .unwrap();
        let actual = account
            .transactions()
            .get(&expected.txid)
            .expect("provider history");
        assert_eq!(actual.transaction, expected.transaction);
        assert_eq!(actual.context, expected.context);
    }
    drop(wm);
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

fn account_key_source(wallet: &Wallet, account_type: AccountType) -> KeySource {
    KeySource::Public(
        wallet
            .accounts
            .account_of_type(account_type)
            .expect("account keys")
            .account_xpub,
    )
}

/// Persists a wallet shaped by `prepare` next to a healthy sibling, runs the
/// statement `prepare` returns against the first wallet's rows (`?1` is its
/// id), and asserts the load's verdict: Strict returns the typed storage
/// error, Recovery drops that wallet alone and names the cause.
fn assert_pool_corruption_is_typed_and_isolated(
    prepare: impl FnOnce(&Wallet, &mut ManagedWalletInfo) -> String,
    reason: &str,
) {
    use platform_wallet::changeset::PersistenceError;
    use platform_wallet_storage::{LoadPolicy, WalletStorageError};

    let (persister, _tmp, path) = common::fresh_persister();
    let sick = Wallet::new_random(Network::Testnet, WalletAccountCreationOptions::Default).unwrap();
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
    assert_eq!(rewritten, 1, "the fixture must rewrite exactly one row");
    drop(persister);

    let strict = SqlitePersister::open(SqlitePersisterConfig::new(&path)).unwrap();
    let error = match strict.load() {
        Ok(_) => panic!("Strict must refuse the row: {reason}"),
        Err(error) => error,
    };
    let PersistenceError::Backend { source, .. } = error else {
        panic!("expected a backend error, got {error:?}")
    };
    assert!(
        matches!(
            source.downcast_ref::<WalletStorageError>(),
            Some(WalletStorageError::BlobDecode { reason: actual }) if *actual == reason
        ),
        "expected `{reason}`, got {source:?}"
    );
    drop(strict);

    let recovery = SqlitePersister::open(
        SqlitePersisterConfig::new(&path).with_load_policy(LoadPolicy::Recovery),
    )
    .unwrap();
    let state = recovery.load().expect("recovery isolates the wallet");
    assert!(state.wallets.contains_key(&healthy.wallet_id));
    assert!(!state.wallets.contains_key(&sick.wallet_id));
    let degradation = recovery.last_load_degradation();
    assert_eq!(degradation.wallets_degraded.len(), 1);
    assert_eq!(
        degradation.wallets_degraded.get(&sick.wallet_id).copied(),
        Some("blob_decode")
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

/// A reserved row whose account coordinates match no account of the wallet
/// is read by no pool. The row cannot say whether it is damaged or belongs
/// to an account the wallet no longer rebuilds, so the load keeps the wallet
/// and reports the reservation it could not take back.
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
        AccountType::IdentityInvitation,
        AccountType::IdentityRegistration,
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
    // The invitation account's key class is always zero.
    let rewritten = persister
        .lock_conn_for_test()
        .execute(
            "UPDATE core_address_pool SET key_class = 1 \
             WHERE wallet_id = ?1 AND account_type = 'identity_invitation' \
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
            state_of(restored, AccountType::IdentityInvitation, reserved[0]),
            AddressState::Available,
            "{policy:?}: this is the loss the counter reports"
        );
        assert_eq!(
            state_of(restored, AccountType::IdentityRegistration, reserved[1]),
            AddressState::Reserved { at: 12345 },
        );
    }
}
