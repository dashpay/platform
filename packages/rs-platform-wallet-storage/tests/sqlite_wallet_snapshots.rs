//! Complete Core snapshots survive SQLite restarts and continued transaction processing.

mod common;

use dashcore::hashes::Hash;
use dashcore::{BlockHash, ChainLock, InstantLock, OutPoint, Transaction, TxIn, TxOut, Txid};
use key_wallet::account::ManagedAccountTrait;
use key_wallet::transaction_checking::{BlockInfo, TransactionContext, WalletTransactionChecker};
use key_wallet::wallet::initialization::WalletAccountCreationOptions;
use key_wallet::wallet::managed_wallet_info::wallet_info_interface::WalletInfoInterface;
use key_wallet::wallet::{ManagedWalletInfo, Wallet};
use platform_wallet::changeset::{
    AccountRegistrationEntry, CoreChangeSet, PlatformWalletChangeSet, PlatformWalletPersistence,
    WalletMetadataEntry,
};
use platform_wallet_storage::{SqlitePersister, SqlitePersisterConfig};

fn register(p: &SqlitePersister, wallet: &Wallet) {
    p.store(
        wallet.wallet_id,
        PlatformWalletChangeSet {
            wallet_metadata: Some(WalletMetadataEntry {
                network: key_wallet::Network::Testnet,
                wallet_group_id: [0; 32],
                birth_height: 10,
            }),
            provider_key_account_registrations: {
                use key_wallet::account::AccountType;
                use platform_wallet::changeset::{
                    ProviderKeyAccountEntry, ProviderKeyExtendedPubKey,
                };
                let mut entries = Vec::new();
                if let Some(account) = wallet
                    .accounts
                    .bls_account_of_type(AccountType::ProviderOperatorKeys)
                {
                    entries.push(ProviderKeyAccountEntry {
                        account_type: AccountType::ProviderOperatorKeys,
                        extended_public_key: ProviderKeyExtendedPubKey::Bls(
                            account.bls_public_key.clone(),
                        ),
                    });
                }
                if let Some(account) = wallet
                    .accounts
                    .eddsa_account_of_type(AccountType::ProviderPlatformKeys)
                {
                    entries.push(ProviderKeyAccountEntry {
                        account_type: AccountType::ProviderPlatformKeys,
                        extended_public_key: ProviderKeyExtendedPubKey::EdDSA(
                            account.ed25519_public_key.clone(),
                        ),
                    });
                }
                entries
            },
            account_registrations: wallet
                .accounts
                .all_accounts()
                .into_iter()
                .map(|a| AccountRegistrationEntry {
                    account_type: a.account_type,
                    account_xpub: a.account_xpub,
                })
                .collect(),
            ..Default::default()
        },
    )
    .unwrap();
}

fn wallet() -> (Wallet, ManagedWalletInfo, Transaction) {
    let wallet = Wallet::from_seed_bytes(
        [42; 64],
        key_wallet::Network::Testnet,
        WalletAccountCreationOptions::Default,
    )
    .unwrap();
    let mut info = ManagedWalletInfo::from_wallet(&wallet, 10);
    let xpub = wallet.accounts.standard_bip44_accounts[&0].account_xpub;
    let address = info
        .first_bip44_managed_account_mut()
        .unwrap()
        .next_receive_address(Some(&xpub), true)
        .unwrap();
    let funding = Transaction::dummy(&address, 0..1, &[100_000]);
    (wallet, info, funding)
}

fn block(height: u32) -> TransactionContext {
    TransactionContext::InBlock(BlockInfo::new(height, BlockHash::all_zeros(), 1_000))
}

fn save(p: &SqlitePersister, info: &ManagedWalletInfo) {
    p.store(
        info.wallet_id,
        PlatformWalletChangeSet {
            core_wallet_snapshot: Some(info.clone()),
            ..Default::default()
        },
    )
    .unwrap();
}

#[tokio::test]
async fn should_keep_spent_coins_unavailable_after_snapshot_restart() {
    for finalized in [false, true] {
        let (p, _tmp, path) = common::fresh_persister();
        let (mut wallet, mut info, funding) = wallet();
        register(&p, &wallet);
        let parent = OutPoint::new(funding.txid(), 0);
        let spend = Transaction {
            version: 2,
            lock_time: 0,
            input: vec![TxIn {
                previous_output: parent,
                ..Default::default()
            }],
            output: vec![TxOut {
                value: 99_000,
                script_pubkey: Default::default(),
            }],
            special_transaction_payload: None,
        };
        info.check_core_transaction(&funding, block(120), &mut wallet, true, true)
            .await;
        info.check_core_transaction(
            &spend,
            if finalized {
                block(150)
            } else {
                TransactionContext::Mempool
            },
            &mut wallet,
            true,
            true,
        )
        .await;
        if finalized {
            info.apply_chain_lock(ChainLock {
                block_height: 200,
                block_hash: BlockHash::all_zeros(),
                signature: [0; 96].into(),
            });
            info.update_synced_height(200);
        }
        save(&p, &info);
        drop(p);
        let p = SqlitePersister::open(SqlitePersisterConfig::new(&path)).unwrap();
        let mut restored = p.load().unwrap().wallets.remove(&wallet.wallet_id).unwrap();
        assert_eq!(restored.wallet_info.balance, info.balance);
        assert_eq!(
            restored.wallet_info.metadata.synced_height,
            info.metadata.synced_height
        );
        restored
            .wallet_info
            .check_core_transaction(&funding, block(120), &mut restored.wallet, true, true)
            .await;
        assert!(restored
            .wallet_info
            .first_bip44_managed_account()
            .unwrap()
            .utxos
            .is_empty());
        assert_eq!(restored.wallet_info.balance.spendable(), 0);
        if !finalized {
            restored.wallet_info.abandon_transaction(spend.txid());
            restored
                .wallet_info
                .check_core_transaction(&funding, block(120), &mut restored.wallet, true, true)
                .await;
            info.abandon_transaction(spend.txid());
            info.check_core_transaction(&funding, block(120), &mut wallet, true, true)
                .await;
            assert_eq!(restored.wallet_info.balance, info.balance);
        }
    }
}

#[test]
fn should_reject_corrupted_or_future_snapshot_without_legacy_fallback() {
    for corruption in [
        "UPDATE core_wallet_snapshots SET snapshot_blob = X'00'",
        "UPDATE core_wallet_snapshots SET format_version = 999",
        "UPDATE core_wallet_snapshots SET layout_marker = X'00'",
        "UPDATE core_wallet_snapshots SET snapshot_blob = zeroblob(16777217)",
    ] {
        let (p, _tmp, _path) = common::fresh_persister();
        let (wallet, info, _) = wallet();
        register(&p, &wallet);
        save(&p, &info);
        p.lock_conn_for_test().execute_batch(corruption).unwrap();
        assert!(p.load().is_err());
    }
}

async fn snapshot_with_mismatched_lock() -> (Wallet, ManagedWalletInfo) {
    let (mut wallet, mut info, funding) = wallet();
    info.check_core_transaction(&funding, block(120), &mut wallet, true, true)
        .await;
    let record = info
        .first_bip44_managed_account_mut()
        .unwrap()
        .transactions_mut()
        .get_mut(&funding.txid())
        .unwrap();
    record.context = TransactionContext::InstantSend(InstantLock {
        txid: Txid::from_byte_array([91; 32]),
        ..Default::default()
    });
    (wallet, info)
}

#[tokio::test]
async fn should_reject_snapshot_write_with_mismatched_embedded_lock() {
    let (p, _tmp, _) = common::fresh_persister();
    let (wallet, info) = snapshot_with_mismatched_lock().await;
    register(&p, &wallet);
    assert!(p
        .store(
            wallet.wallet_id,
            PlatformWalletChangeSet {
                core_wallet_snapshot: Some(info),
                ..Default::default()
            },
        )
        .is_err());
}

#[tokio::test]
async fn should_reject_snapshot_load_with_mismatched_embedded_lock() {
    let (p, _tmp, _) = common::fresh_persister();
    let (wallet, info) = snapshot_with_mismatched_lock().await;
    register(&p, &wallet);
    save(&p, &ManagedWalletInfo::from_wallet(&wallet, 10));
    let bytes = bincode::serde::encode_to_vec(&info, bincode::config::standard()).unwrap();
    p.lock_conn_for_test()
        .execute(
            "UPDATE core_wallet_snapshots SET snapshot_blob = ?1",
            [bytes],
        )
        .unwrap();
    assert!(p.load().is_err());
}

#[test]
fn should_invalidate_snapshot_after_core_delta_without_snapshot() {
    let (p, _tmp, _path) = common::fresh_persister();
    let (wallet, mut info, _) = wallet();
    register(&p, &wallet);
    info.update_synced_height(100);
    save(&p, &info);
    p.store(
        wallet.wallet_id,
        PlatformWalletChangeSet {
            core: Some(CoreChangeSet {
                synced_height: Some(110),
                ..Default::default()
            }),
            ..Default::default()
        },
    )
    .unwrap();
    let restored = p.load().unwrap().wallets.remove(&wallet.wallet_id).unwrap();
    assert_eq!(restored.wallet_info.metadata.synced_height, 9);
    assert_eq!(restored.wallet_info.metadata.last_processed_height, 9);
}

#[test]
fn should_roll_back_snapshot_and_rows_on_wrong_wallet() {
    let (p, _tmp, _path) = common::fresh_persister();
    let (wallet, info, _) = wallet();
    register(&p, &wallet);
    save(&p, &info);
    let mut wrong = info.clone();
    wrong.wallet_id = [99; 32];
    assert!(p
        .store(
            wallet.wallet_id,
            PlatformWalletChangeSet {
                core: Some(CoreChangeSet {
                    synced_height: Some(200),
                    ..Default::default()
                }),
                core_wallet_snapshot: Some(wrong),
                ..Default::default()
            }
        )
        .is_err());
    let restored = p.load().unwrap().wallets.remove(&wallet.wallet_id).unwrap();
    assert_eq!(
        restored.wallet_info.metadata.synced_height,
        info.metadata.synced_height
    );
}

#[test]
fn should_reject_snapshot_for_another_network() {
    let (p, _tmp, _) = common::fresh_persister();
    let (wallet, mut info, _) = wallet();
    register(&p, &wallet);
    info.network = key_wallet::Network::Mainnet;
    assert!(p
        .store(
            wallet.wallet_id,
            PlatformWalletChangeSet {
                core_wallet_snapshot: Some(info),
                ..Default::default()
            }
        )
        .is_err());
}

#[test]
fn should_rescan_when_old_snapshot_arrives_after_account_registration() {
    let (p, _tmp, _) = common::fresh_persister();
    let (wallet, mut old_snapshot, _) = wallet();
    register(&p, &wallet);
    old_snapshot.accounts.standard_bip44_accounts.remove(&0);
    old_snapshot.update_synced_height(100);
    save(&p, &old_snapshot);
    let restored = p.load().unwrap().wallets.remove(&wallet.wallet_id).unwrap();
    assert!(restored.wallet_info.first_bip44_managed_account().is_some());
    assert_eq!(restored.wallet_info.metadata.synced_height, 9);
}

#[test]
fn should_flush_newest_buffered_snapshot_and_invalidate_later_delta() {
    let (p, _tmp, _) = common::fresh_persister_with_mode(common::FlushMode::Manual);
    let (wallet, mut info, _) = wallet();
    register(&p, &wallet);
    info.update_synced_height(100);
    save(&p, &info);
    info.update_synced_height(200);
    save(&p, &info);
    p.flush(wallet.wallet_id).unwrap();
    assert_eq!(
        p.load().unwrap().wallets[&wallet.wallet_id]
            .wallet_info
            .metadata
            .synced_height,
        200
    );
    save(&p, &info);
    p.store(
        wallet.wallet_id,
        PlatformWalletChangeSet {
            core: Some(CoreChangeSet {
                synced_height: Some(300),
                ..Default::default()
            }),
            ..Default::default()
        },
    )
    .unwrap();
    p.flush(wallet.wallet_id).unwrap();
    assert_eq!(
        p.load().unwrap().wallets[&wallet.wallet_id]
            .wallet_info
            .metadata
            .synced_height,
        9
    );
}
