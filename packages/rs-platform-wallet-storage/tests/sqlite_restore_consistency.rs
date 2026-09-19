//! Provider pools, full lifecycle snapshots, and concurrent SQLite read consistency.

mod common;

use dashcore::hashes::Hash;
use dashcore::{BlockHash, OutPoint, Transaction, TxIn, TxOut, Txid};
use key_wallet::account::AccountType;
use key_wallet::managed_account::managed_account_trait::ManagedAccountTrait;
use key_wallet::transaction_checking::{BlockInfo, TransactionContext, WalletTransactionChecker};
use key_wallet::wallet::managed_wallet_info::wallet_info_interface::WalletInfoInterface;
use key_wallet::wallet::managed_wallet_info::ManagedWalletInfo;
use key_wallet::wallet::Wallet;
use platform_wallet::changeset::provider_key_account::provider_key_test_wallet;
use platform_wallet::changeset::{
    AccountAddressPoolEntry, AccountRegistrationEntry, CoreChangeSet, PlatformWalletChangeSet,
    PlatformWalletPersistence, ProviderKeyAccountEntry, ProviderKeyExtendedPubKey,
    WalletMetadataEntry,
};
use platform_wallet_storage::sqlite::schema::core_state;
use platform_wallet_storage::SqlitePersister;

fn register(persister: &SqlitePersister, wallet: &Wallet) {
    let mut providers = Vec::new();
    if let Some(account) = wallet
        .accounts
        .bls_account_of_type(AccountType::ProviderOperatorKeys)
    {
        providers.push(ProviderKeyAccountEntry {
            account_type: AccountType::ProviderOperatorKeys,
            extended_public_key: ProviderKeyExtendedPubKey::Bls(account.bls_public_key.clone()),
        });
    }
    if let Some(account) = wallet
        .accounts
        .eddsa_account_of_type(AccountType::ProviderPlatformKeys)
    {
        providers.push(ProviderKeyAccountEntry {
            account_type: AccountType::ProviderPlatformKeys,
            extended_public_key: ProviderKeyExtendedPubKey::EdDSA(
                account.ed25519_public_key.clone(),
            ),
        });
    }
    persister
        .store(
            wallet.wallet_id,
            PlatformWalletChangeSet {
                provider_key_account_registrations: providers,
                wallet_metadata: Some(WalletMetadataEntry {
                    network: key_wallet::Network::Testnet,
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
                ..Default::default()
            },
        )
        .unwrap();
}

fn block(height: u32) -> TransactionContext {
    TransactionContext::InBlock(BlockInfo::new(height, BlockHash::all_zeros(), 0))
}

async fn provider_round_trip(with_funding: bool, untyped_ecdsa: bool, with_snapshot: bool) {
    use dashcore::blockdata::transaction::special_transaction::provider_registration::{
        ProviderMasternodeType, ProviderRegistrationPayload,
    };
    use dashcore::blockdata::transaction::special_transaction::TransactionPayload;

    let (persister, _tmp, _path) = common::fresh_persister();
    let mut wallet = provider_key_test_wallet();
    register(&persister, &wallet);
    let mut live = ManagedWalletInfo::from_wallet(&wallet, 1);
    let owner_source = key_wallet::managed_account::address_pool::KeySource::Public(
        wallet
            .accounts
            .all_accounts()
            .into_iter()
            .find(|account| account.account_type == AccountType::ProviderOwnerKeys)
            .unwrap()
            .account_xpub,
    );
    live.accounts
        .provider_owner_keys
        .as_mut()
        .unwrap()
        .managed_account_type_mut()
        .address_pools_mut()[0]
        .generate_addresses(40, &owner_source, true)
        .unwrap();
    let owner = live
        .accounts
        .provider_owner_keys
        .as_ref()
        .unwrap()
        .managed_account_type()
        .address_pools()[0]
        .address_at_index(35)
        .unwrap();
    let voting = live
        .accounts
        .provider_voting_keys
        .as_ref()
        .unwrap()
        .all_addresses()[0]
        .clone();
    let platform_keys =
        platform_wallet::wallet::provider_key_at_index::derive_platform_node_public_keys(
            &wallet,
            key_wallet::Network::Testnet,
            2,
        )
        .unwrap();
    platform_wallet::wallet::provider_key_at_index::populate_platform_node_pool(
        &mut live,
        &platform_keys,
        key_wallet::Network::Testnet,
    )
    .unwrap();
    let payout = if with_funding {
        live.accounts.standard_bip44_accounts[&0].all_addresses()[0].clone()
    } else {
        dashcore::Address::new(
            key_wallet::Network::Testnet,
            dashcore::address::Payload::PubkeyHash(dashcore::PubkeyHash::from_byte_array([9; 20])),
        )
    };
    let transaction = Transaction {
        version: 3,
        lock_time: 0,
        input: vec![TxIn {
            previous_output: OutPoint::new(Txid::from_byte_array([7; 32]), 0),
            ..Default::default()
        }],
        output: vec![TxOut {
            value: 5_000,
            script_pubkey: payout.script_pubkey(),
        }],
        special_transaction_payload: Some(TransactionPayload::ProviderRegistrationPayloadType(
            ProviderRegistrationPayload {
                version: 2,
                masternode_type: ProviderMasternodeType::HighPerformance,
                masternode_mode: 0,
                collateral_outpoint: OutPoint::new(Txid::from_byte_array([8; 32]), 0),
                service_address: "127.0.0.1:19999".parse().unwrap(),
                owner_key_hash: *owner.payload().as_pubkey_hash().unwrap(),
                operator_public_key: [0; 48].into(),
                voting_key_hash: *voting.payload().as_pubkey_hash().unwrap(),
                operator_reward: 0,
                script_payout: payout.script_pubkey(),
                inputs_hash: dashcore::hash_types::InputsHash::all_zeros(),
                signature: vec![],
                platform_node_id: Some(dashcore::PlatformNodeId::from_byte_array(
                    platform_keys[1].node_id,
                )),
                platform_p2p_port: Some(26656),
                platform_http_port: Some(443),
            },
        )),
    };
    let result = live
        .check_core_transaction(&transaction, block(100), &mut wallet, true, true)
        .await;
    for account in [
        &live.accounts.provider_owner_keys,
        &live.accounts.provider_voting_keys,
        &live.accounts.provider_platform_keys,
    ] {
        assert!(account
            .as_ref()
            .unwrap()
            .has_transaction(&transaction.txid()));
    }
    let record = if with_funding {
        result
            .new_records
            .iter()
            .find(|record| matches!(record.account_type, AccountType::Standard { .. }))
            .unwrap()
    } else {
        result
            .new_records
            .iter()
            .find(|record| record.account_type == AccountType::ProviderOwnerKeys)
            .unwrap()
    };
    let chain_lock = dashcore::ChainLock {
        block_height: 100,
        block_hash: BlockHash::all_zeros(),
        signature: [0; 96].into(),
    };
    live.update_last_processed_height(100);
    live.update_synced_height(100);
    live.apply_chain_lock(chain_lock.clone());
    persister
        .store(
            wallet.wallet_id,
            PlatformWalletChangeSet {
                core_wallet_snapshot: with_snapshot.then(|| live.clone()),
                account_address_pools: [
                    &live.accounts.provider_owner_keys,
                    &live.accounts.provider_voting_keys,
                    &live.accounts.provider_platform_keys,
                ]
                .into_iter()
                .flat_map(|account| {
                    let account = account.as_ref().unwrap();
                    account
                        .managed_account_type()
                        .address_pools()
                        .into_iter()
                        .map(|pool| AccountAddressPoolEntry {
                            account_type: account.managed_account_type().to_account_type(),
                            pool_type: pool.pool_type,
                            addresses: pool.addresses.values().cloned().collect(),
                        })
                        .collect::<Vec<_>>()
                })
                .collect(),
                core: Some(CoreChangeSet {
                    records: vec![record.clone()],
                    new_utxos: live
                        .accounts
                        .all_funding_accounts()
                        .into_iter()
                        .flat_map(|a| a.utxos.values().cloned())
                        .collect(),
                    last_processed_height: Some(100),
                    synced_height: Some(100),
                    last_applied_chain_lock: Some(chain_lock.clone()),
                    ..Default::default()
                }),
                ..Default::default()
            },
        )
        .unwrap();
    if untyped_ecdsa {
        persister.lock_conn_for_test().execute(
            "UPDATE core_address_pool SET public_key = NULL, key_type = NULL WHERE key_type = 0",
            [],
        ).unwrap();
    }
    let loaded = persister.load().unwrap();
    let restored = &loaded.wallets[&wallet.wallet_id].wallet_info;
    for (before, after) in [
        (
            &live.accounts.provider_owner_keys,
            &restored.accounts.provider_owner_keys,
        ),
        (
            &live.accounts.provider_voting_keys,
            &restored.accounts.provider_voting_keys,
        ),
        (
            &live.accounts.provider_platform_keys,
            &restored.accounts.provider_platform_keys,
        ),
    ] {
        let before = before.as_ref().unwrap();
        let after = after.as_ref().unwrap();
        assert_eq!(
            after.tx_count(),
            if with_snapshot { before.tx_count() } else { 0 },
            "only full snapshots restore provider transaction history"
        );
        assert_eq!(
            after.transactions().contains_key(&transaction.txid()),
            with_snapshot,
            "a full snapshot preserves provider payloads across ChainLock finality"
        );
        for pool in before.managed_account_type().address_pools() {
            for info in pool.addresses.values() {
                let restored_address = after.get_address_info(&info.address).unwrap();
                assert_eq!(restored_address.public_key, info.public_key);
                assert_eq!(restored_address.is_used(), info.is_used());
            }
        }
    }
    assert_eq!(
        restored.balance.total(),
        if with_snapshot {
            live.balance.total()
        } else {
            0
        }
    );
    assert_eq!(
        restored.metadata.synced_height,
        if with_snapshot { 100 } else { 0 }
    );
    if untyped_ecdsa {
        persister.lock_conn_for_test().execute(
            "UPDATE core_address_pool SET script = ?1 WHERE account_type = ?2 AND address_index = 35",
            rusqlite::params![payout.script_pubkey().as_bytes(), "provider_owner"],
        ).unwrap();
        assert!(
            persister.load().is_err(),
            "untyped provider addresses still require xpub ownership verification"
        );
    }
}

#[tokio::test]
async fn should_restore_provider_snapshot_with_funding() {
    provider_round_trip(true, false, true).await;
}

#[tokio::test]
async fn should_restore_provider_snapshot_without_funding() {
    provider_round_trip(false, false, true).await;
}

#[tokio::test]
async fn should_rescan_legacy_core_without_losing_untyped_provider_pools() {
    provider_round_trip(true, true, false).await;
}

#[tokio::test]
async fn should_verify_untyped_provider_pool_rows_over_a_full_snapshot() {
    provider_round_trip(true, true, true).await;
}

#[tokio::test]
async fn should_load_one_snapshot_while_another_connection_commits_a_spend() {
    use rusqlite::hooks::{AuthAction, AuthContext, Authorization};
    use std::sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    };

    let (persister, _tmp, path) = common::fresh_persister();
    let mut wallet = provider_key_test_wallet();
    register(&persister, &wallet);
    let mut live = ManagedWalletInfo::from_wallet(&wallet, 1);
    let address = live.accounts.standard_bip44_accounts[&0].all_addresses()[0].clone();
    let funding = Transaction {
        version: 2,
        lock_time: 0,
        input: vec![TxIn {
            previous_output: OutPoint::new(Txid::from_byte_array([11; 32]), 0),
            ..Default::default()
        }],
        output: vec![TxOut {
            value: 5_000,
            script_pubkey: address.script_pubkey(),
        }],
        special_transaction_payload: None,
    };
    let result = live
        .check_core_transaction(&funding, block(100), &mut wallet, true, true)
        .await;
    let coin = live.accounts.standard_bip44_accounts[&0]
        .utxos
        .values()
        .next()
        .unwrap()
        .clone();
    live.update_last_processed_height(100);
    live.update_synced_height(100);
    persister
        .store(
            wallet.wallet_id,
            PlatformWalletChangeSet {
                core_wallet_snapshot: Some(live.clone()),
                core: Some(CoreChangeSet {
                    records: result.new_records.clone(),
                    account_records: result.new_records,
                    new_utxos: vec![coin.clone()],
                    last_processed_height: Some(100),
                    synced_height: Some(100),
                    ..Default::default()
                }),
                ..Default::default()
            },
        )
        .unwrap();
    let spend = Transaction {
        version: 2,
        lock_time: 0,
        input: vec![TxIn {
            previous_output: coin.outpoint,
            ..Default::default()
        }],
        output: vec![TxOut {
            value: 4_000,
            script_pubkey: dashcore::ScriptBuf::new(),
        }],
        special_transaction_payload: None,
    };
    let result = live
        .check_core_transaction(&spend, block(101), &mut wallet, true, true)
        .await;
    live.update_last_processed_height(101);
    live.update_synced_height(101);
    let spending_snapshot =
        bincode::serde::encode_to_vec(&live, bincode::config::standard()).unwrap();
    let spending = CoreChangeSet {
        records: result.new_records.clone(),
        account_records: result.new_records,
        spent_utxos: vec![coin],
        last_processed_height: Some(101),
        synced_height: Some(101),
        ..Default::default()
    };
    let mut writer = rusqlite::Connection::open(path).unwrap();
    let wallet_id = wallet.wallet_id;
    let fired = Arc::new(AtomicBool::new(false));
    let callback_fired = fired.clone();
    persister
        .lock_conn_for_test()
        .authorizer(Some(move |ctx: AuthContext<'_>| {
            if matches!(
                ctx.action,
                AuthAction::Read {
                    table_name: "core_wallet_snapshots",
                    column_name: "snapshot_blob"
                }
            ) && !callback_fired.swap(true, Ordering::SeqCst)
            {
                let tx = writer.transaction().unwrap();
                core_state::apply(&tx, &wallet_id, &spending).unwrap();
                tx.execute(
                    "UPDATE core_wallet_snapshots SET snapshot_blob = ?1 WHERE wallet_id = ?2",
                    rusqlite::params![&spending_snapshot, wallet_id.as_slice()],
                )
                .unwrap();
                tx.commit().unwrap();
            }
            Authorization::Allow
        }))
        .unwrap();
    let loaded = persister.load().unwrap();
    assert!(
        fired.load(Ordering::SeqCst),
        "second connection committed after the load transaction began but before its Core snapshot read"
    );
    let restored = &loaded.wallets[&wallet_id].wallet_info;
    assert_eq!(
        restored.balance.total(),
        5_000,
        "load must retain the committed snapshot it began reading"
    );
    assert_eq!(restored.metadata.synced_height, 100);
    let mut latest = persister
        .load()
        .unwrap()
        .wallets
        .remove(&wallet_id)
        .unwrap();
    assert_eq!(latest.wallet_info.balance.total(), 0);
    assert_eq!(latest.wallet_info.metadata.synced_height, 101);
    latest
        .wallet_info
        .check_core_transaction(&funding, block(100), &mut latest.wallet, true, true)
        .await;
    assert_eq!(
        latest.wallet_info.balance.total(),
        0,
        "new snapshot retains spend evidence on funding replay"
    );
}
