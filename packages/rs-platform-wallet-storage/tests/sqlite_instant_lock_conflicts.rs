//! A separately persisted lock must retain the live wallet's conflict outcome.

mod common;

use common::{ensure_wallet_meta, fresh_persister};
use dashcore::hashes::Hash;
use dashcore::{InstantLock, OutPoint, Transaction, TxIn, TxOut, Txid};
use key_wallet::account::ManagedAccountTrait;
use key_wallet::transaction_checking::{TransactionContext, WalletTransactionChecker};
use key_wallet::wallet::initialization::WalletAccountCreationOptions;
use key_wallet::wallet::managed_wallet_info::wallet_info_interface::WalletInfoInterface;
use key_wallet::wallet::managed_wallet_info::ManagedWalletInfo;
use key_wallet::wallet::Wallet;
use platform_wallet::changeset::{
    AccountRegistrationEntry, CoreChangeSet, PlatformWalletChangeSet, PlatformWalletPersistence,
};
use platform_wallet_storage::{SqlitePersister, SqlitePersisterConfig};

async fn separate_lock_round_trip(with_descendant: bool) {
    let (persister, _tmp, path) = fresh_persister();
    let mut wallet = Wallet::from_seed_bytes(
        [0xC8; 64],
        dashcore::Network::Testnet,
        WalletAccountCreationOptions::Default,
    )
    .unwrap();
    let mut live = ManagedWalletInfo::from_wallet(&wallet, 1);
    let wallet_id = live.wallet_id;
    ensure_wallet_meta(&persister, &wallet_id);
    persister
        .store(
            wallet_id,
            PlatformWalletChangeSet {
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
                    .map(|account| AccountRegistrationEntry {
                        account_type: account.account_type,
                        account_xpub: account.account_xpub,
                    })
                    .collect(),
                ..Default::default()
            },
        )
        .unwrap();
    let address = live
        .monitored_addresses()
        .into_iter()
        .find(|address| {
            live.first_bip44_managed_account()
                .unwrap()
                .contains_address(address)
        })
        .unwrap();
    let external_input = OutPoint {
        txid: Txid::from_byte_array([0xC9; 32]),
        vout: 0,
    };
    let transaction = |value| Transaction {
        version: 2,
        lock_time: 0,
        input: vec![TxIn {
            previous_output: external_input,
            ..Default::default()
        }],
        output: vec![TxOut {
            value,
            script_pubkey: address.script_pubkey(),
        }],
        special_transaction_payload: None,
    };
    let loser = transaction(7_000);
    let winner = transaction(5_000);
    let mut records = Vec::new();
    for transaction in [&loser, &winner] {
        let result = live
            .check_core_transaction(
                transaction,
                TransactionContext::Mempool,
                &mut wallet,
                true,
                true,
            )
            .await;
        records.extend(result.new_records);
    }
    let descendant = with_descendant.then(|| Transaction {
        input: vec![TxIn {
            previous_output: OutPoint::new(loser.txid(), 0),
            ..Default::default()
        }],
        output: vec![TxOut {
            value: 6_000,
            script_pubkey: address.script_pubkey(),
        }],
        ..loser.clone()
    });
    if let Some(descendant) = &descendant {
        let result = live
            .check_core_transaction(
                descendant,
                TransactionContext::Mempool,
                &mut wallet,
                true,
                true,
            )
            .await;
        records.extend(result.new_records);
    }
    live.update_balance();
    assert_eq!(
        live.balance.total(),
        if with_descendant { 11_000 } else { 12_000 }
    );
    let coins: Vec<_> = live
        .accounts
        .all_funding_accounts()
        .into_iter()
        .flat_map(|account| {
            account
                .utxos
                .values()
                .cloned()
                .map(move |coin| (account.managed_account_type().to_account_type(), coin))
        })
        .collect();
    persister
        .store(
            wallet_id,
            PlatformWalletChangeSet {
                core_wallet_snapshot: Some(live.clone()),
                core: Some(CoreChangeSet {
                    records: records.clone(),
                    account_records: records.clone(),
                    new_utxos: coins.iter().map(|(_, coin)| coin.clone()).collect(),
                    ..Default::default()
                }),
                ..Default::default()
            },
        )
        .unwrap();
    let lock = InstantLock {
        txid: winner.txid(),
        inputs: vec![external_input],
        ..Default::default()
    };
    assert!(live.mark_instant_send_utxos(&winner.txid(), &lock));
    assert_eq!(live.balance.total(), 5_000);
    assert!(!live
        .first_bip44_managed_account()
        .unwrap()
        .transactions()
        .contains_key(&loser.txid()));

    // The bridge projects TransactionInstantLocked as a lock-only changeset.
    persister
        .store(
            wallet_id,
            PlatformWalletChangeSet {
                core_wallet_snapshot: Some(live.clone()),
                core: Some(CoreChangeSet {
                    instant_locks_for_non_final_records: [(winner.txid(), lock.clone())].into(),
                    ..Default::default()
                }),
                ..Default::default()
            },
        )
        .unwrap();
    drop(persister);

    let reopened = SqlitePersister::open(SqlitePersisterConfig::new(&path)).unwrap();
    let restored = reopened
        .load()
        .unwrap()
        .wallets
        .remove(&wallet_id)
        .unwrap()
        .wallet_info;
    assert_eq!(
        restored.balance.total(),
        live.balance.total(),
        "a separately persisted winner lock must not resurrect the swept loser"
    );
    assert!(!restored
        .first_bip44_managed_account()
        .unwrap()
        .transactions()
        .contains_key(&loser.txid()));
    if let Some(descendant) = descendant {
        assert!(!restored
            .first_bip44_managed_account()
            .unwrap()
            .transactions()
            .contains_key(&descendant.txid()));
    }
}

#[tokio::test]
async fn should_match_live_balance_after_separate_instant_lock_sweeps_conflict() {
    separate_lock_round_trip(false).await;
}

#[tokio::test]
async fn should_remove_conflicting_descendants_after_separate_instant_lock() {
    separate_lock_round_trip(true).await;
}
