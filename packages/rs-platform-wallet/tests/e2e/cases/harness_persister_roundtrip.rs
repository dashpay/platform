//! Offline check of the harness persister's accounting observation points.
//!
//! `framework::harness_persister::HarnessPersister` backs the live and
//! after-reload asset-lock accounting checks in CR-003 / ID-002b / AL-001.
//! Those only run against a live network, so this case pins the plumbing
//! offline: a wallet registered through a manager on the persister, then
//! one wallet-level Core record stored for it, must be readable both as the
//! live record and from a reopened SQLite snapshot, unchanged.

use std::sync::Arc;

use dashcore::Transaction;
use key_wallet::account::account_type::StandardAccountType;
use key_wallet::managed_account::transaction_record::{
    InputDetail, OutputDetail, OutputRole, TransactionDirection, TransactionRecord,
};
use key_wallet::transaction_checking::transaction_router::TransactionType;
use key_wallet::transaction_checking::TransactionContext;
use key_wallet::wallet::initialization::WalletAccountCreationOptions;
use key_wallet::{AccountType, Network};
use platform_wallet::changeset::{
    CoreChangeSet, PlatformWalletChangeSet, PlatformWalletPersistence,
};
use platform_wallet::events::{EventHandler, PlatformEventHandler};
use platform_wallet::PlatformWalletManager;

use crate::framework::harness_persister::HarnessPersister;

const TEST_SEED: [u8; 64] = [11u8; 64];
const SPENT: u64 = 1_000_000;
const CHANGE: u64 = 400_000;
const SENT: u64 = 599_000;

struct NoopEventHandler;
impl EventHandler for NoopEventHandler {}
impl PlatformEventHandler for NoopEventHandler {}

// Multi-thread: `PlatformWalletManager::shutdown` joins worker threads via
// `Handle::block_on`, which panics on a current-thread runtime.
#[tokio_shared_rt::test(shared, flavor = "multi_thread", worker_threads = 4)]
async fn harness_persister_round_trips_a_wallet_record() {
    let persister = Arc::new(HarnessPersister::open());
    let manager = PlatformWalletManager::new(
        Arc::new(dash_sdk::Sdk::new_mock()),
        Arc::clone(&persister),
        vec![Arc::new(NoopEventHandler) as Arc<dyn PlatformEventHandler>],
    );
    let wallet = manager
        .create_wallet_from_seed_bytes(
            Network::Testnet,
            &TEST_SEED,
            WalletAccountCreationOptions::Default,
            Some(0),
        )
        .await
        .expect("register wallet through the harness persister");
    let wallet_id = wallet.wallet_id();
    let ours = wallet
        .core()
        .next_receive_address_for_account(0)
        .await
        .expect("derive receive address");

    let transaction = Transaction::dummy(&ours, 0..1, &[CHANGE, SENT]);
    let mut record = TransactionRecord::new(
        transaction,
        AccountType::Standard {
            index: 0,
            standard_account_type: StandardAccountType::BIP44Account,
        },
        TransactionContext::Mempool,
        TransactionType::Standard,
        TransactionDirection::Outgoing,
        vec![InputDetail {
            index: 0,
            value: SPENT,
            address: ours.clone(),
        }],
        vec![OutputDetail {
            index: 0,
            role: OutputRole::Change,
            address: Some(ours),
            value: CHANGE,
        }],
        CHANGE as i64 - SPENT as i64,
    );
    record.fee = Some(SPENT - CHANGE - SENT);
    let txid = record.txid;

    persister
        .store(
            wallet_id,
            PlatformWalletChangeSet {
                core: Some(CoreChangeSet {
                    records: vec![record.clone()],
                    ..Default::default()
                }),
                ..Default::default()
            },
        )
        .expect("the harness persister never fails a store");

    let live = persister
        .live_record(wallet_id, &txid)
        .expect("stored record is captured live");
    assert_eq!(
        (live.direction, live.net_amount),
        (record.direction, record.net_amount)
    );

    let reloaded = persister
        .reloaded_record(wallet_id, &txid)
        .expect("reload from the tee store")
        .expect("stored record has a row after a reload");
    assert_eq!(reloaded.txid, txid);
    assert_eq!(
        (reloaded.direction, reloaded.net_amount),
        (record.direction, record.net_amount),
        "a reload must return the stored accounting unchanged"
    );

    let shutdown = manager.shutdown().await;
    assert!(shutdown.all_clean(), "manager teardown: {shutdown:?}");
}
