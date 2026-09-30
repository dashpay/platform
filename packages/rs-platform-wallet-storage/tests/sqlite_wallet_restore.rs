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
                .any(|(account, _, _, _)| *account == required),
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
    for (account_type, pool_type, addresses, reserved_index) in expected {
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

#[test]
fn should_read_one_snapshot_despite_a_concurrent_commit() {
    use rusqlite::hooks::{AuthAction, Authorization};
    use std::sync::atomic::{AtomicBool, Ordering};

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
    let writer = rusqlite::Connection::open(path).unwrap();
    let changed = Arc::new(AtomicBool::new(false));
    let changed_callback = Arc::clone(&changed);
    persister
        .lock_conn_for_test()
        .authorizer(Some(move |ctx: rusqlite::hooks::AuthContext<'_>| {
            if matches!(
                ctx.action,
                AuthAction::Read {
                    table_name: "core_utxos",
                    ..
                }
            ) && !changed_callback.swap(true, Ordering::SeqCst)
            {
                writer
                    .execute("UPDATE core_sync_state SET synced_height = 200", [])
                    .unwrap();
            }
            Authorization::Allow
        }))
        .unwrap();
    let state = persister.load().unwrap();
    assert!(changed.load(Ordering::SeqCst));
    assert_eq!(
        state.wallets[&wallet.wallet_id]
            .wallet_info
            .metadata
            .synced_height,
        100,
        "one load must retain its original read snapshot"
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
