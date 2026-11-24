//! Asset-lock direction from credit ownership, end to end: a wallet-signed
//! lock through the live event adapter, the SQLite store and its repair, and
//! a reload.

use dashcore::transaction::special_transaction::asset_lock::AssetLockPayload;
use dashcore::transaction::TransactionPayload;
use dashcore::{Network, OutPoint, ScriptBuf, Transaction};
use key_wallet::account::AccountType;
use key_wallet::managed_account::managed_account_trait::ManagedAccountTrait;
use key_wallet::managed_account::transaction_record::TransactionDirection;
use key_wallet::wallet::initialization::WalletAccountCreationOptions;
use key_wallet::wallet::managed_wallet_info::ManagedAccountOperations;
use platform_wallet::changeset::{AccountAddressPoolEntry, PlatformWalletPersistence};
use platform_wallet::key_wallet_manager::{WalletInterface, WalletManager};
use platform_wallet::test_support::core_fixtures::{foreign_address, outside, pay, spend_of};
use platform_wallet::wallet::platform_wallet::PlatformWalletInfo;
use platform_wallet_storage::{SqlitePersister, SqlitePersisterConfig};
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::common::{self, mine, persist_events, registration, Manager};

struct Fixture {
    persister: Arc<SqlitePersister>,
    manager: Manager,
    wallet_id: [u8; 32],
    transaction: Transaction,
    initial_pool_rows: i64,
    _directory: tempfile::TempDir,
    path: std::path::PathBuf,
}

/// A wallet registered with its initial pools and a mined coin, then an
/// asset lock it signs, observed in the mempool and persisted, whose credits
/// pay `case`'s keys. `deep` first derives 80 more registration keys in
/// memory only, so the credit key is missing from the persisted pool.
async fn observed_lock(case: &str, deep: bool) -> Fixture {
    let (persister, directory, path) = common::fresh_persister();
    let persister = Arc::new(persister);
    let mut wm = WalletManager::<PlatformWalletInfo>::new(Network::Testnet);
    let mut events = wm.take_persistence_receiver().unwrap();
    let wallet_id = wm.create_wallet_from_mnemonic(
        "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about",
        0, WalletAccountCreationOptions::Default,
    ).unwrap();
    let contact_type = AccountType::DashpayExternalAccount {
        index: 0,
        user_identity_id: [3; 32],
        friend_identity_id: [4; 32],
    };
    if case == "contact" {
        wm.create_account(&wallet_id, contact_type, None).unwrap();
        let wallet = wm.get_wallet(&wallet_id).unwrap().clone();
        wm.get_wallet_info_mut(&wallet_id)
            .unwrap()
            .core_wallet
            .add_managed_account(&wallet, contact_type)
            .unwrap();
    }
    let wallet = wm.get_wallet(&wallet_id).unwrap().clone();
    let info = wm.get_wallet_info_mut(&wallet_id).unwrap();
    let address = info
        .core_wallet
        .accounts
        .standard_bip44_accounts
        .get_mut(&0)
        .unwrap()
        .next_receive_address(
            Some(&wallet.accounts.standard_bip44_accounts[&0].account_xpub),
            true,
        )
        .unwrap();
    let initial_pools: Vec<_> = info
        .core_wallet
        .accounts
        .all_accounts()
        .into_iter()
        .flat_map(|account| {
            let account_type = account.managed_account_type().to_account_type();
            account
                .managed_account_type()
                .address_pools()
                .into_iter()
                .map(move |pool| AccountAddressPoolEntry {
                    account_type,
                    pool_type: pool.pool_type,
                    addresses: pool.addresses.values().cloned().collect(),
                })
                .collect::<Vec<_>>()
        })
        .collect();
    let key_account = info
        .core_wallet
        .accounts
        .identity_registration
        .as_mut()
        .unwrap();
    if deep {
        let xpub = wallet
            .accounts
            .identity_registration
            .as_ref()
            .unwrap()
            .account_xpub;
        for _ in 0..80 {
            let used = key_account.next_address(Some(&xpub), true).unwrap();
            key_account.mark_address_used(&used);
        }
    }
    let pools = key_account.managed_account_type().address_pools();
    let last = pools[0]
        .addresses
        .iter()
        .max_by_key(|(index, _)| *index)
        .unwrap()
        .1
        .address
        .clone();
    let first = pools[0].addresses[&0].address.clone();
    let watched = info
        .core_wallet
        .accounts
        .dashpay_external_accounts
        .values()
        .next()
        .map(|account| {
            account.managed_account_type().address_pools()[0]
                .address_at_index(0)
                .unwrap()
        });
    let credits = match case {
        "own" => vec![last.clone()],
        "contact" => vec![watched.unwrap()],
        "two owned" => vec![first, last.clone()],
        "mixed" => vec![last.clone(), foreign_address(92)],
        _ => vec![foreign_address(92)],
    };
    let mut registered = registration(&wallet, None);
    registered.account_address_pools = initial_pools;
    persister.store(wallet_id, registered).unwrap();
    let initial_pool_rows = pool_rows(&persister);
    let funding = spend_of(&[outside(31)], vec![pay(100_000, &address)]);
    let manager = Arc::new(RwLock::new(wm));
    mine(&manager, wallet_id, funding.clone(), 100).await;
    persist_events(&manager, &persister, &mut events).await;
    let mut transaction = spend_of(
        &[OutPoint::new(funding.txid(), 0)],
        vec![
            dashcore::TxOut {
                value: 60_000,
                script_pubkey: ScriptBuf::new_op_return(&[]),
            },
            pay(39_000, &address),
        ],
    );
    transaction.version = 3;
    transaction.special_transaction_payload = Some(TransactionPayload::AssetLockPayloadType(
        AssetLockPayload::new(
            credits
                .iter()
                .map(|address| pay(60_000 / credits.len() as u64, address))
                .collect(),
        ),
    ));
    {
        let wm = manager.read().await;
        transaction = common::authorize(
            wm.get_wallet(&wallet_id).unwrap(),
            &wm.get_wallet_info(&wallet_id).unwrap().core_wallet,
            &address,
            transaction,
        );
    }
    assert!(
        manager
            .write()
            .await
            .process_mempool_transaction(&transaction, None)
            .await
            .is_relevant
    );
    persist_events(&manager, &persister, &mut events).await;
    Fixture {
        persister,
        manager,
        wallet_id,
        transaction,
        initial_pool_rows,
        _directory: directory,
        path,
    }
}

fn pool_rows(persister: &SqlitePersister) -> i64 {
    persister
        .lock_conn_for_test()
        .query_row("SELECT COUNT(*) FROM core_address_pool", [], |row| {
            row.get(0)
        })
        .unwrap()
}

/// Live projection, repair and reload agree: `Internal` only when every
/// credit pays a key of the wallet's own (non-contact) accounts.
#[tokio::test]
async fn should_persist_and_restore_asset_lock_credit_direction() {
    for case in ["own", "foreign", "mixed", "two owned", "contact"] {
        let fixture = observed_lock(case, false).await;
        let expected = if matches!(case, "own" | "two owned") {
            TransactionDirection::Internal
        } else {
            TransactionDirection::Outgoing
        };
        let row = fixture
            .persister
            .get_core_tx_record(fixture.wallet_id, &fixture.transaction.txid())
            .unwrap()
            .unwrap();
        assert_eq!(
            (row.direction, row.net_amount),
            (expected, -61_000),
            "{case}"
        );
        fixture.persister.load().unwrap();
        let restored = fixture
            .persister
            .get_core_tx_record(fixture.wallet_id, &fixture.transaction.txid())
            .unwrap()
            .unwrap();
        assert_eq!(
            (restored.direction, restored.net_amount),
            (expected, -61_000),
            "{case} reload"
        );
    }
}

/// A credit key the wallet derived past its persisted pool snapshot is
/// stored with the lock, so the repair and a reopened store still read the
/// wallet's own lock as `Internal`.
#[tokio::test]
async fn should_keep_a_lock_internal_when_its_credit_key_is_past_the_persisted_pool() {
    let fixture = observed_lock("own", true).await;
    assert!(
        pool_rows(&fixture.persister) > fixture.initial_pool_rows,
        "new credit-key evidence must persist with its record"
    );
    let Fixture {
        persister,
        manager,
        wallet_id,
        transaction,
        _directory,
        path,
        ..
    } = fixture;
    drop(manager);
    drop(persister);
    let persister = SqlitePersister::open(SqlitePersisterConfig::new(path)).unwrap();
    let loaded = persister.load().unwrap();
    let Some(TransactionPayload::AssetLockPayloadType(payload)) =
        &transaction.special_transaction_payload
    else {
        panic!("asset lock fixture")
    };
    assert!(
        loaded.wallets[&wallet_id]
            .wallet_info
            .accounts
            .all_accounts()
            .iter()
            .any(|account| account
                .managed_account_type()
                .contains_script_pub_key(&payload.credit_outputs[0].script_pubkey)),
        "the deep credit key is restored into the real managed wallet"
    );
    let row = persister
        .get_core_tx_record(wallet_id, &transaction.txid())
        .unwrap()
        .unwrap();
    assert_eq!(
        (row.direction, row.net_amount),
        (TransactionDirection::Internal, -61_000)
    );
}
