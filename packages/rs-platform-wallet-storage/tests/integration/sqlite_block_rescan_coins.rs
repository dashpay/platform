//! Coins a block credits past an address pool's gap, in its own pass or
//! when dash-spv re-scans it for addresses a later block derives, reach the
//! SQLite store through the event adapter, and a reload restores every one
//! of them.

use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::time::Duration;

use crate::common::fresh_persister;
use dashcore::OutPoint;
use key_wallet::account::{AccountType, StandardAccountType};
use key_wallet::managed_account::address_pool::AddressPoolType;
use key_wallet::managed_account::managed_account_trait::ManagedAccountTrait;
use key_wallet::wallet::initialization::WalletAccountCreationOptions;
use key_wallet::wallet::ManagedWalletInfo;
use key_wallet::Wallet;
use platform_wallet::changeset::{
    spawn_wallet_event_adapter, AccountAddressPoolEntry, AccountRegistrationEntry, CoreChangeSet,
    PlatformWalletChangeSet, PlatformWalletPersistence, WalletMetadataEntry,
};
use platform_wallet::key_wallet_manager::WalletEvent;
use platform_wallet::test_support::{
    block_paying, external_addresses, process_block_with_rescans, transaction_paying,
    wallet_manager_with_core_wallet, wallet_manager_with_persistence,
};
use platform_wallet::wallet::platform_wallet::{PlatformWalletInfo, WalletId};
use platform_wallet_storage::{SqlitePersister, SqlitePersisterConfig};
use tokio::sync::{mpsc, RwLock};

const VALUE: u64 = 10_000_000;
const PAST_GAP: u32 = 25;
const HEIGHT: u32 = 100;

fn external_gap(wallet: &Wallet, account: AccountType) -> u32 {
    ManagedWalletInfo::from_wallet(wallet, 0)
        .accounts
        .funds_account(&account)
        .expect("account")
        .managed_account_type()
        .address_pools()
        .into_iter()
        .find(|p| p.pool_type() == AddressPoolType::External)
        .expect("external pool")
        .gap_limit()
}

fn register(persister: &SqlitePersister, wallet_id: WalletId, wallet: &Wallet) {
    let manifest = wallet
        .accounts
        .all_accounts()
        .into_iter()
        .map(|a| AccountRegistrationEntry {
            account_type: a.account_type,
            account_xpub: a.account_xpub,
        })
        .collect();
    let pools = ManagedWalletInfo::from_wallet(wallet, 0)
        .accounts
        .all_funding_accounts()
        .into_iter()
        .flat_map(|account| {
            let account_type = account.managed_account_type().to_account_type();
            account
                .managed_account_type()
                .address_pools()
                .into_iter()
                .map(move |pool| AccountAddressPoolEntry {
                    account_type,
                    pool_type: pool.pool_type(),
                    addresses: pool.addresses().values().cloned().collect(),
                })
                .collect::<Vec<_>>()
        })
        .collect();
    persister
        .store(
            wallet_id,
            PlatformWalletChangeSet {
                wallet_metadata: Some(WalletMetadataEntry {
                    network: key_wallet::Network::Testnet,
                    wallet_group_id: [0u8; 32],
                    birth_height: 0,
                }),
                account_registrations: manifest,
                account_address_pools: pools,
                ..Default::default()
            },
        )
        .expect("register the wallet");
}

/// Runs the event adapter over every event the manager queued, into
/// `persister`, until it has stored them all.
async fn persist_queued(
    manager: &Arc<RwLock<platform_wallet::key_wallet_manager::WalletManager<PlatformWalletInfo>>>,
    queued: &mut mpsc::UnboundedReceiver<WalletEvent>,
    persister: &Arc<SqlitePersister>,
) {
    let (tx, rx) = mpsc::unbounded_channel();
    while let Ok(event) = queued.try_recv() {
        tx.send(event).expect("adapter channel");
    }
    drop(tx);
    let adapter = spawn_wallet_event_adapter(
        Arc::clone(manager),
        Arc::downgrade(persister),
        rx,
        Arc::new(AtomicBool::new(false)),
        tokio_util::sync::CancellationToken::new(),
    );
    tokio::time::timeout(Duration::from_secs(60), adapter)
        .await
        .expect("the adapter drains")
        .expect("the adapter task");
}

/// The coins every funds account of the wallet holds after a reload of the
/// store at `path`. Which account a coin past the registered pool snapshot
/// lands in is the rehydrate's placement, not this persistence path.
fn reloaded_coins(path: &std::path::Path, wallet_id: WalletId) -> Vec<OutPoint> {
    let reopened = SqlitePersister::open(SqlitePersisterConfig::new(path)).expect("reopen");
    let state = reopened.load().expect("load");
    let slice = state.wallets.get(&wallet_id).expect("the wallet loads");
    slice
        .wallet_info
        .accounts
        .all_funding_accounts()
        .into_iter()
        .flat_map(|account| account.utxos.keys().copied())
        .collect()
}

/// The unspent `core_utxos` rows the store holds for each of `addresses`.
fn stored_unspent_rows(path: &std::path::Path, addresses: &[dashcore::Address]) -> Vec<i64> {
    let conn = rusqlite::Connection::open(path).expect("open the store");
    addresses
        .iter()
        .map(|address| {
            conn.query_row(
                "SELECT COUNT(*) FROM core_utxos WHERE spent = 0 AND script = ?1",
                [address.script_pubkey().as_bytes()],
                |row| row.get(0),
            )
            .expect("count rows")
        })
        .collect()
}

async fn every_coin_of_one_transaction_past_the_gap_reloads(account: AccountType) {
    let (persister, _tmp, path) = fresh_persister();
    let persister = Arc::new(persister);
    let wallet = Wallet::new_random(
        key_wallet::Network::Testnet,
        WalletAccountCreationOptions::Default,
    )
    .expect("wallet");
    let count = external_gap(&wallet, account) + PAST_GAP;
    let addresses = external_addresses(&wallet, account, count);
    let manifest_wallet = wallet.clone();
    let (manager, wallet_id, mut queued) = wallet_manager_with_persistence(wallet);
    register(&persister, wallet_id, &manifest_wallet);

    let block = block_paying(&addresses, VALUE, HEIGHT);
    process_block_with_rescans(&manager, wallet_id, &block, HEIGHT).await;
    persist_queued(&manager, &mut queued, &persister).await;
    persister.flush(wallet_id).expect("flush");
    drop(persister);

    assert_eq!(
        stored_unspent_rows(&path, &addresses),
        vec![1; addresses.len()],
        "the store holds one unspent row for every coin the block credited"
    );

    let txid = block.txdata[0].txid();
    let mut reloaded: Vec<u32> = reloaded_coins(&path, wallet_id)
        .into_iter()
        .filter(|o| o.txid == txid)
        .map(|o| o.vout)
        .collect();
    reloaded.sort_unstable();
    assert_eq!(
        reloaded,
        (0..count).collect::<Vec<_>>(),
        "a reload restores every coin the block credited"
    );
}

#[tokio::test]
async fn coinjoin_coins_past_the_gap_in_one_transaction_survive_a_reload() {
    every_coin_of_one_transaction_past_the_gap_reloads(AccountType::CoinJoin { index: 0 }).await;
}

#[tokio::test]
async fn bip44_coins_past_the_gap_in_one_transaction_survive_a_reload() {
    every_coin_of_one_transaction_past_the_gap_reloads(AccountType::Standard {
        index: 0,
        standard_account_type: StandardAccountType::BIP44Account,
    })
    .await;
}

type Manager = platform_wallet::key_wallet_manager::WalletManager<PlatformWalletInfo>;

/// One wallet registered in a fresh store, and two CoinJoin transactions:
/// `paying_far` pays external index 0 (vout 0) and a far index past the gap
/// (vout 1), and `extending` pays a near index that extends the pool over
/// the far one. With both in one block, key-wallet's in-block re-check
/// (rust-dashcore #1368) credits the far output on the block's first call.
/// With `extending` in a later block, only dash-spv's re-scan of the earlier
/// block credits it, to the record the wallet already holds.
struct FarCoins {
    persister: Option<Arc<SqlitePersister>>,
    _tmp: tempfile::TempDir,
    path: std::path::PathBuf,
    manager: Arc<RwLock<Manager>>,
    wallet_id: WalletId,
    queued: mpsc::UnboundedReceiver<WalletEvent>,
    paying_far: dashcore::Transaction,
    extending: dashcore::Transaction,
    /// The addresses `paying_far` vout 0, its far output and `extending` pay.
    addresses: [dashcore::Address; 3],
    /// BIP44 receive addresses, for change and a second account's coins.
    bip44: Vec<dashcore::Address>,
}

/// After a restart: every coin the funds accounts hold, and the balance.
#[derive(Debug, PartialEq)]
struct Restarted {
    coins: std::collections::BTreeSet<OutPoint>,
    balance: u64,
}

impl FarCoins {
    async fn new() -> Self {
        let (persister, tmp, path) = fresh_persister();
        let persister = Arc::new(persister);
        let wallet = Wallet::new_random(
            key_wallet::Network::Testnet,
            WalletAccountCreationOptions::Default,
        )
        .expect("wallet");
        let coinjoin = AccountType::CoinJoin { index: 0 };
        let gap = external_gap(&wallet, coinjoin);
        let (near, far) = (2 * gap / 3, gap + gap / 3);
        let addresses = external_addresses(&wallet, coinjoin, far + 1);
        let bip44 = external_addresses(&wallet, bip44_account_0(), 5);
        let paying_far = transaction_paying(
            &[addresses[0].clone(), addresses[far as usize].clone()],
            VALUE,
            1,
        );
        let extending = transaction_paying(&[addresses[near as usize].clone()], VALUE, 2);
        let manifest_wallet = wallet.clone();
        let (manager, wallet_id, queued) = wallet_manager_with_persistence(wallet);
        register(&persister, wallet_id, &manifest_wallet);
        FarCoins {
            persister: Some(persister),
            _tmp: tmp,
            path,
            manager,
            wallet_id,
            queued,
            paying_far,
            extending,
            addresses: [
                addresses[0].clone(),
                addresses[far as usize].clone(),
                addresses[near as usize].clone(),
            ],
            bip44,
        }
    }

    fn coin(tx: &dashcore::Transaction, vout: u32) -> OutPoint {
        OutPoint::new(tx.txid(), vout)
    }

    fn far(&self) -> OutPoint {
        Self::coin(&self.paying_far, 1)
    }

    async fn process(&self, height: u32, block: &dashcore::Block) {
        process_block_with_rescans(&self.manager, self.wallet_id, block, height).await;
    }

    async fn holds(&self, outpoint: OutPoint) -> bool {
        self.manager
            .read()
            .await
            .get_wallet_info(&self.wallet_id)
            .expect("wallet")
            .core_wallet
            .accounts
            .all_funding_accounts()
            .into_iter()
            .any(|account| account.utxos.contains_key(&outpoint))
    }

    /// Stores every queued event through the adapter, as one drain.
    async fn persist(&mut self) {
        let persister = self.persister.as_ref().expect("open");
        persist_queued(&self.manager, &mut self.queued, persister).await;
        persister.flush(self.wallet_id).expect("flush");
    }

    /// Closes the store, reopens it and loads the wallet.
    fn restart(&mut self) -> Restarted {
        drop(self.persister.take());
        let reopened =
            SqlitePersister::open(SqlitePersisterConfig::new(&self.path)).expect("reopen");
        let reloaded = reopened
            .load()
            .expect("load")
            .wallets
            .remove(&self.wallet_id)
            .expect("the wallet loads")
            .wallet_info;
        Restarted {
            coins: reloaded
                .accounts
                .all_funding_accounts()
                .into_iter()
                .flat_map(|account| account.utxos.keys().copied())
                .collect(),
            balance: reloaded.balance.total(),
        }
    }
}

fn block_at(height: u32, txdata: Vec<dashcore::Transaction>) -> dashcore::Block {
    dashcore::Block::dummy(height, txdata)
}

fn coins(outpoints: impl IntoIterator<Item = OutPoint>) -> std::collections::BTreeSet<OutPoint> {
    outpoints.into_iter().collect()
}

/// A spend of `outpoint` paying half to an outside address and the rest,
/// less a fee, to `back` (a wallet address).
fn spend_with_change(outpoint: OutPoint, back: &dashcore::Address) -> dashcore::Transaction {
    use platform_wallet::test_support::core_fixtures::{foreign_address, pay, spend_of};
    spend_of(
        &[outpoint],
        vec![pay(VALUE / 2, &foreign_address(7)), pay(CHANGE, back)],
    )
}

const CHANGE: u64 = VALUE / 2 - 1_000;

/// A spend of `outpoint` paying everything, less a fee, to an outside address.
fn spend_away(outpoint: OutPoint) -> dashcore::Transaction {
    use platform_wallet::test_support::core_fixtures::{foreign_address, pay, spend_of};
    spend_of(&[outpoint], vec![pay(VALUE - 1_000, &foreign_address(7))])
}

/// One block pays both transactions, and the next spends the far coin; one
/// drain stores both. After a restart the far coin is spent: it is not a
/// coin and not in the balance. Vout 0 and the near coin are.
#[tokio::test]
async fn in_block_far_coin_spent_in_the_next_block_stays_spent_after_a_restart() {
    let mut w = FarCoins::new().await;
    let far = w.far();
    w.process(
        HEIGHT,
        &block_at(HEIGHT, vec![w.paying_far.clone(), w.extending.clone()]),
    )
    .await;
    assert!(
        w.holds(far).await,
        "the block's first call credits the far output"
    );
    w.process(HEIGHT + 1, &block_at(HEIGHT + 1, vec![spend_away(far)]))
        .await;
    w.persist().await;

    let rows = stored_unspent_rows(&w.path, &w.addresses);
    assert_eq!(
        w.restart(),
        Restarted {
            coins: coins([
                FarCoins::coin(&w.paying_far, 0),
                FarCoins::coin(&w.extending, 0)
            ]),
            balance: 2 * VALUE,
        }
    );
    assert_eq!(rows, vec![1, 0, 1], "the far coin has no unspent row");
}

/// One block pays both transactions; the next spends vout 0 with change to
/// the wallet; then the first block is delivered again. Each block is stored
/// as it arrives. After a restart vout 0 is spent, and the far coin, the near
/// coin and the change are coins.
#[tokio::test]
async fn in_block_coin_spent_later_stays_spent_when_its_block_is_redelivered() {
    let mut w = FarCoins::new().await;
    let funding = block_at(HEIGHT, vec![w.paying_far.clone(), w.extending.clone()]);
    let spend = spend_with_change(FarCoins::coin(&w.paying_far, 0), &w.bip44[0]);
    w.process(HEIGHT, &funding).await;
    w.persist().await;
    w.process(HEIGHT + 1, &block_at(HEIGHT + 1, vec![spend.clone()]))
        .await;
    w.persist().await;
    w.process(HEIGHT, &funding).await;
    w.persist().await;

    let rows = stored_unspent_rows(&w.path, &w.addresses);
    assert_eq!(
        w.restart(),
        Restarted {
            coins: coins([
                w.far(),
                FarCoins::coin(&w.extending, 0),
                FarCoins::coin(&spend, 1)
            ]),
            balance: 2 * VALUE + CHANGE,
        }
    );
    assert_eq!(rows, vec![0, 1, 1], "vout 0 has no unspent row");
}

/// One block pays both CoinJoin transactions and five BIP44 receive
/// addresses. After a restart every coin is held, the BIP44 ones in the
/// BIP44 account, and the balance counts all eight.
#[tokio::test]
async fn in_block_coins_of_two_accounts_survive_a_restart_in_their_accounts() {
    let mut w = FarCoins::new().await;
    let bip44_pay = transaction_paying(&w.bip44, VALUE, 3);
    w.process(
        HEIGHT,
        &block_at(
            HEIGHT,
            vec![w.paying_far.clone(), w.extending.clone(), bip44_pay.clone()],
        ),
    )
    .await;
    w.persist().await;

    let bip44_coins: Vec<OutPoint> = (0..5).map(|v| FarCoins::coin(&bip44_pay, v)).collect();
    let mut expected = coins(bip44_coins.clone());
    expected.extend([
        FarCoins::coin(&w.paying_far, 0),
        w.far(),
        FarCoins::coin(&w.extending, 0),
    ]);
    assert_eq!(
        w.restart(),
        Restarted {
            coins: expected,
            balance: 8 * VALUE,
        }
    );
    let held = reloaded_account_coins(&w.path, w.wallet_id, bip44_account_0());
    assert!(bip44_coins.iter().all(|c| held.contains(c)));
}

/// Block 100 pays `paying_far`, so its own pass credits only vout 0. Block
/// 101 pays the near index, and dash-spv re-scans block 100, which credits
/// the far coin to the record the wallet already holds. After a restart the
/// far coin is held and in the balance.
#[tokio::test]
async fn a_coin_a_rescan_of_an_earlier_block_credits_survives_a_reload() {
    let mut w = FarCoins::new().await;
    let funding = block_at(HEIGHT, vec![w.paying_far.clone()]);
    w.process(HEIGHT, &funding).await;
    assert!(
        !w.holds(w.far()).await,
        "block 100's own pass does not reach the far output"
    );
    w.process(HEIGHT + 1, &block_at(HEIGHT + 1, vec![w.extending.clone()]))
        .await;
    w.process(HEIGHT, &funding).await;
    assert!(w.holds(w.far()).await, "the re-scan credits the far output");
    w.persist().await;

    let rows = stored_unspent_rows(&w.path, &w.addresses);
    assert_eq!(
        w.restart(),
        Restarted {
            coins: coins([
                FarCoins::coin(&w.paying_far, 0),
                w.far(),
                FarCoins::coin(&w.extending, 0)
            ]),
            balance: 3 * VALUE,
        }
    );
    assert_eq!(
        rows,
        vec![1, 1, 1],
        "the store holds an unspent row for every coin, the far one included"
    );
}

/// `paying_far` is seen in the mempool, where only vout 0 is the wallet's,
/// and is stored. The block that confirms it also pays the near index, so
/// the block credits the far coin to the record the wallet already holds.
/// After a restart all three coins are held and in the balance.
#[tokio::test]
async fn a_mempool_transaction_confirmed_after_its_pools_grew_keeps_every_coin_after_a_restart() {
    use platform_wallet::key_wallet_manager::WalletInterface;

    let mut w = FarCoins::new().await;
    w.manager
        .write()
        .await
        .process_mempool_transaction(&w.paying_far, None)
        .await;
    assert!(
        !w.holds(w.far()).await,
        "the mempool sighting misses the far output"
    );
    w.persist().await;
    w.process(
        HEIGHT,
        &block_at(HEIGHT, vec![w.paying_far.clone(), w.extending.clone()]),
    )
    .await;
    assert!(w.holds(w.far()).await, "the block credits the far output");
    w.persist().await;

    let rows = stored_unspent_rows(&w.path, &w.addresses);
    assert_eq!(
        w.restart(),
        Restarted {
            coins: coins([
                FarCoins::coin(&w.paying_far, 0),
                w.far(),
                FarCoins::coin(&w.extending, 0)
            ]),
            balance: 3 * VALUE,
        }
    );
    assert_eq!(
        rows,
        vec![1, 1, 1],
        "the store holds an unspent row for every coin, the far one included"
    );
}

/// The re-scan of block 100 credits the far coin, and block 102 spends it
/// before any of it is stored; one drain stores everything. After a restart
/// the far coin is spent: not a coin and not in the balance.
#[tokio::test]
async fn a_far_coin_spent_before_its_rescan_is_stored_stays_spent_after_a_restart() {
    let mut w = FarCoins::new().await;
    let funding = block_at(HEIGHT, vec![w.paying_far.clone()]);
    w.process(HEIGHT, &funding).await;
    w.process(HEIGHT + 1, &block_at(HEIGHT + 1, vec![w.extending.clone()]))
        .await;
    w.process(HEIGHT, &funding).await;
    assert!(w.holds(w.far()).await, "the re-scan credits the far output");
    w.process(HEIGHT + 2, &block_at(HEIGHT + 2, vec![spend_away(w.far())]))
        .await;
    w.persist().await;

    let rows = stored_unspent_rows(&w.path, &w.addresses);
    assert_eq!(
        w.restart(),
        Restarted {
            coins: coins([
                FarCoins::coin(&w.paying_far, 0),
                FarCoins::coin(&w.extending, 0)
            ]),
            balance: 2 * VALUE,
        }
    );
    assert_eq!(rows, vec![1, 0, 1], "the far coin has no unspent row");
}

/// Block 100 pays `paying_far` and is stored; block 101 spends vout 0 with
/// change to the wallet and pays the near index, and is stored; then the
/// re-scan of block 100 credits the far coin and is stored. After a restart
/// vout 0 is spent, and the far coin, the near coin and the change are coins.
#[tokio::test]
async fn a_rescan_after_a_later_spend_keeps_the_spent_coin_spent_after_a_restart() {
    let mut w = FarCoins::new().await;
    let funding = block_at(HEIGHT, vec![w.paying_far.clone()]);
    let spend = spend_with_change(FarCoins::coin(&w.paying_far, 0), &w.bip44[0]);
    w.process(HEIGHT, &funding).await;
    w.persist().await;
    w.process(
        HEIGHT + 1,
        &block_at(HEIGHT + 1, vec![spend.clone(), w.extending.clone()]),
    )
    .await;
    w.persist().await;
    w.process(HEIGHT, &funding).await;
    assert!(w.holds(w.far()).await, "the re-scan credits the far output");
    w.persist().await;

    let rows = stored_unspent_rows(&w.path, &w.addresses);
    assert_eq!(
        w.restart(),
        Restarted {
            coins: coins([
                w.far(),
                FarCoins::coin(&w.extending, 0),
                FarCoins::coin(&spend, 1)
            ]),
            balance: 2 * VALUE + CHANGE,
        }
    );
    assert_eq!(
        rows,
        vec![0, 1, 1],
        "vout 0 has no unspent row, the far coin has one"
    );
}

/// Block 100 pays `paying_far` and five BIP44 receive addresses, and is
/// stored; block 101 pays the near index and is stored; then the re-scan of
/// block 100 credits the far coin, re-sends the BIP44 coins the store already
/// holds, and is stored. After a restart every coin is held, the BIP44 ones
/// in the BIP44 account and the far one in CoinJoin, and the balance counts
/// all eight.
#[tokio::test]
async fn a_rescan_keeps_a_second_accounts_coins_in_their_account_after_a_restart() {
    let mut w = FarCoins::new().await;
    let bip44_pay = transaction_paying(&w.bip44, VALUE, 3);
    let funding = block_at(HEIGHT, vec![w.paying_far.clone(), bip44_pay.clone()]);
    w.process(HEIGHT, &funding).await;
    assert!(
        !w.holds(w.far()).await,
        "block 100's own pass does not reach the far output"
    );
    w.persist().await;
    w.process(HEIGHT + 1, &block_at(HEIGHT + 1, vec![w.extending.clone()]))
        .await;
    w.persist().await;
    w.process(HEIGHT, &funding).await;
    assert!(w.holds(w.far()).await, "the re-scan credits the far output");
    w.persist().await;

    let bip44_coins: Vec<OutPoint> = (0..5).map(|v| FarCoins::coin(&bip44_pay, v)).collect();
    let mut expected = coins(bip44_coins.clone());
    expected.extend([
        FarCoins::coin(&w.paying_far, 0),
        w.far(),
        FarCoins::coin(&w.extending, 0),
    ]);
    assert_eq!(
        w.restart(),
        Restarted {
            coins: expected,
            balance: 8 * VALUE,
        }
    );
    let bip44_held = reloaded_account_coins(&w.path, w.wallet_id, bip44_account_0());
    assert!(bip44_coins.iter().all(|c| bip44_held.contains(c)));
    let coinjoin_held =
        reloaded_account_coins(&w.path, w.wallet_id, AccountType::CoinJoin { index: 0 });
    assert!(coinjoin_held.contains(&w.far()));
}

/// The store already holds the far coin as spent (a spend it recorded, or a
/// row the host marked spent) while the wallet still holds it, and then the
/// re-scan's events reach the store. A re-scan only fills rows the store
/// lacks: the far coin's row stays spent, and after a restart the far coin
/// is neither held nor in the balance.
#[tokio::test]
async fn a_rescan_never_unspends_a_coin_the_store_holds_spent() {
    let mut w = FarCoins::new().await;
    let funding = block_at(HEIGHT, vec![w.paying_far.clone()]);
    w.process(HEIGHT, &funding).await;
    w.process(HEIGHT + 1, &block_at(HEIGHT + 1, vec![w.extending.clone()]))
        .await;
    w.process(HEIGHT, &funding).await;
    let far_utxo = w
        .manager
        .read()
        .await
        .get_wallet_info(&w.wallet_id)
        .expect("wallet")
        .core_wallet
        .accounts
        .all_funding_accounts()
        .into_iter()
        .find_map(|account| account.utxos.get(&w.far()).cloned())
        .expect("the wallet holds the far coin");
    w.persister
        .as_ref()
        .expect("open")
        .store(
            w.wallet_id,
            PlatformWalletChangeSet {
                core: Some(CoreChangeSet {
                    spent_utxos: vec![far_utxo],
                    ..Default::default()
                }),
                ..Default::default()
            },
        )
        .expect("store the far coin as spent");
    w.persist().await;

    let rows = stored_unspent_rows(&w.path, &w.addresses);
    assert_eq!(
        w.restart(),
        Restarted {
            coins: coins([
                FarCoins::coin(&w.paying_far, 0),
                FarCoins::coin(&w.extending, 0)
            ]),
            balance: 2 * VALUE,
        }
    );
    assert_eq!(rows, vec![1, 0, 1], "the far coin's row stays spent");
}

/// The store holds the far coin as a stamped sweep placeholder (a spent
/// row with no value or script, which the collector deletes once the
/// finality boundary passes its stamp unless a delivery materialises it)
/// while the wallet holds the coin, and the re-scan's events reach the
/// store. The re-send materialises the row and keeps it spent, so when the
/// ChainLock and the synced height pass the stamp the row survives. A later
/// re-scan reaching the store then finds the spent row and leaves it, and
/// after a restart the far coin is neither held nor in the balance.
#[tokio::test]
async fn a_rescan_materialises_a_spent_sweep_placeholder_without_unspending_it() {
    use dashcore::bls_sig_utils::BLSSignature;
    use dashcore::ephemerealdata::chain_lock::ChainLock;

    let mut w = FarCoins::new().await;
    let funding = block_at(HEIGHT, vec![w.paying_far.clone()]);
    w.process(HEIGHT, &funding).await;
    w.process(HEIGHT + 1, &block_at(HEIGHT + 1, vec![w.extending.clone()]))
        .await;
    w.process(HEIGHT, &funding).await;
    let far_utxo = w
        .manager
        .read()
        .await
        .get_wallet_info(&w.wallet_id)
        .expect("wallet")
        .core_wallet
        .accounts
        .all_funding_accounts()
        .into_iter()
        .find_map(|account| account.utxos.get(&w.far()).cloned())
        .expect("the wallet holds the far coin");
    let persister = w.persister.as_ref().expect("open");
    persister
        .store(
            w.wallet_id,
            PlatformWalletChangeSet {
                core: Some(CoreChangeSet {
                    spent_utxos: vec![far_utxo.clone()],
                    ..Default::default()
                }),
                ..Default::default()
            },
        )
        .expect("store the far coin as spent");
    let far_script = w.addresses[1].script_pubkey();
    let placed = rusqlite::Connection::open(&w.path)
        .expect("open the store")
        .execute(
            "UPDATE core_utxos SET is_sweep_placeholder = 1, winner_mined_height = ?1, \
             value = 0, script = X'' WHERE script = ?2 AND spent = 1",
            rusqlite::params![i64::from(HEIGHT + 2), far_script.as_bytes()],
        )
        .expect("make the row a stamped placeholder");
    assert_eq!(placed, 1);
    w.persist().await;
    let past = HEIGHT + 5;
    w.persister
        .as_ref()
        .expect("open")
        .store(
            w.wallet_id,
            PlatformWalletChangeSet {
                core: Some(CoreChangeSet {
                    last_processed_height: Some(past),
                    synced_height: Some(past),
                    last_applied_chain_lock: Some(ChainLock {
                        block_height: past,
                        block_hash: dashcore::BlockHash::all_zeros(),
                        signature: BLSSignature::from([0u8; 96]),
                    }),
                    ..Default::default()
                }),
                ..Default::default()
            },
        )
        .expect("move the boundary past the stamp");

    let kept: i64 = rusqlite::Connection::open(&w.path)
        .expect("open the store")
        .query_row(
            "SELECT COUNT(*) FROM core_utxos \
             WHERE script = ?1 AND spent = 1 AND is_sweep_placeholder = 0",
            [far_script.as_bytes()],
            |row| row.get(0),
        )
        .expect("count rows");
    assert_eq!(
        kept, 1,
        "the far coin's row is materialised and stays spent"
    );
    w.persister
        .as_ref()
        .expect("open")
        .store(
            w.wallet_id,
            PlatformWalletChangeSet {
                core: Some(CoreChangeSet {
                    rescan_utxos: vec![far_utxo],
                    ..Default::default()
                }),
                ..Default::default()
            },
        )
        .expect("a later re-scan reaches the store");
    let rows = stored_unspent_rows(&w.path, &w.addresses);
    assert_eq!(
        w.restart(),
        Restarted {
            coins: coins([
                FarCoins::coin(&w.paying_far, 0),
                FarCoins::coin(&w.extending, 0)
            ]),
            balance: 2 * VALUE,
        }
    );
    assert_eq!(rows, vec![1, 0, 1], "the far coin's row stays spent");
}

/// Transaction records the store holds that spend an output of `txid`.
fn stored_spenders_of(path: &std::path::Path, txid: dashcore::Txid) -> i64 {
    rusqlite::Connection::open(path)
        .expect("open the store")
        .query_row(
            "SELECT COUNT(*) FROM core_transaction_inputs WHERE substr(outpoint, 2, 32) = ?1",
            [AsRef::<[u8]>::as_ref(&txid)],
            |row| row.get(0),
        )
        .expect("count spenders")
}

fn sweep_placeholders(path: &std::path::Path) -> i64 {
    rusqlite::Connection::open(path)
        .expect("open the store")
        .query_row(
            "SELECT COUNT(*) FROM core_utxos WHERE is_sweep_placeholder = 1",
            [],
            |row| row.get(0),
        )
        .expect("count placeholders")
}

/// The re-scan credits the far coin, and the store records it spent: with
/// `spender`, block 102 spends it and the store keeps that transaction;
/// without, only the spent row records the spend. The row becomes a stamped
/// sweep placeholder, and the ChainLock and synced height pass its stamp, so
/// the collector deletes it. Then a later re-scan delivers the far coin to a
/// store that holds no row for it. Returns the restart and the unspent rows
/// for vout 0, the far coin and the near coin.
async fn rescan_after_the_far_coins_row_is_collected(
    spender: bool,
) -> (FarCoins, Restarted, Vec<i64>) {
    use dashcore::bls_sig_utils::BLSSignature;
    use dashcore::ephemerealdata::chain_lock::ChainLock;

    let mut w = FarCoins::new().await;
    let funding = block_at(HEIGHT, vec![w.paying_far.clone()]);
    w.process(HEIGHT, &funding).await;
    w.process(HEIGHT + 1, &block_at(HEIGHT + 1, vec![w.extending.clone()]))
        .await;
    w.process(HEIGHT, &funding).await;
    let far_utxo = w
        .manager
        .read()
        .await
        .get_wallet_info(&w.wallet_id)
        .expect("wallet")
        .core_wallet
        .accounts
        .all_funding_accounts()
        .into_iter()
        .find_map(|account| account.utxos.get(&w.far()).cloned())
        .expect("the wallet holds the far coin");
    if spender {
        w.process(HEIGHT + 2, &block_at(HEIGHT + 2, vec![spend_away(w.far())]))
            .await;
        assert!(!w.holds(w.far()).await, "block 102 spends the far coin");
        w.persist().await;
    } else {
        w.persist().await;
        w.persister
            .as_ref()
            .expect("open")
            .store(
                w.wallet_id,
                PlatformWalletChangeSet {
                    core: Some(CoreChangeSet {
                        spent_utxos: vec![far_utxo.clone()],
                        ..Default::default()
                    }),
                    ..Default::default()
                },
            )
            .expect("store the far coin as spent");
    }
    assert_eq!(
        stored_spenders_of(&w.path, w.paying_far.txid()),
        i64::from(spender)
    );
    let placed = rusqlite::Connection::open(&w.path)
        .expect("open the store")
        .execute(
            "UPDATE core_utxos SET is_sweep_placeholder = 1, winner_mined_height = ?1, \
             value = 0, script = X'' WHERE script = ?2 AND spent = 1",
            rusqlite::params![
                i64::from(HEIGHT + 2),
                w.addresses[1].script_pubkey().as_bytes()
            ],
        )
        .expect("make the row a stamped placeholder");
    assert_eq!(placed, 1, "the store holds the far coin spent");
    let past = HEIGHT + 5;
    w.persister
        .as_ref()
        .expect("open")
        .store(
            w.wallet_id,
            PlatformWalletChangeSet {
                core: Some(CoreChangeSet {
                    last_processed_height: Some(past),
                    synced_height: Some(past),
                    last_applied_chain_lock: Some(ChainLock {
                        block_height: past,
                        block_hash: dashcore::BlockHash::all_zeros(),
                        signature: BLSSignature::from([0u8; 96]),
                    }),
                    ..Default::default()
                }),
                ..Default::default()
            },
        )
        .expect("move the boundary past the stamp");
    assert_eq!(
        sweep_placeholders(&w.path),
        0,
        "the collector deleted the far coin's row"
    );

    w.persister
        .as_ref()
        .expect("open")
        .store(
            w.wallet_id,
            PlatformWalletChangeSet {
                core: Some(CoreChangeSet {
                    rescan_utxos: vec![far_utxo],
                    ..Default::default()
                }),
                ..Default::default()
            },
        )
        .expect("a later re-scan reaches the store");
    let rows = stored_unspent_rows(&w.path, &w.addresses);
    let restarted = w.restart();
    (w, restarted, rows)
}

/// Block 102 spends the far coin, and the store keeps that transaction but
/// no row for the coin. A re-scan then delivers the coin. After a restart
/// the far coin is neither held nor in the balance.
#[tokio::test]
async fn a_rescan_fills_a_coin_a_stored_transaction_spends_as_spent() {
    let (w, restarted, rows) = rescan_after_the_far_coins_row_is_collected(true).await;
    assert_eq!(
        restarted,
        Restarted {
            coins: coins([
                FarCoins::coin(&w.paying_far, 0),
                FarCoins::coin(&w.extending, 0)
            ]),
            balance: 2 * VALUE,
        }
    );
    assert_eq!(rows, vec![1, 0, 1], "the far coin's row is spent");
}

/// The store keeps no transaction spending the far coin and no row for it.
/// A re-scan then delivers the coin. After a restart the far coin is held
/// and in the balance.
#[tokio::test]
async fn a_rescan_fills_a_coin_no_stored_transaction_spends_as_unspent() {
    let (w, restarted, rows) = rescan_after_the_far_coins_row_is_collected(false).await;
    assert_eq!(
        restarted,
        Restarted {
            coins: coins([
                FarCoins::coin(&w.paying_far, 0),
                w.far(),
                FarCoins::coin(&w.extending, 0)
            ]),
            balance: 3 * VALUE,
        }
    );
    assert_eq!(rows, vec![1, 1, 1], "the far coin's row is unspent");
}

fn bip44_account_0() -> AccountType {
    AccountType::Standard {
        index: 0,
        standard_account_type: StandardAccountType::BIP44Account,
    }
}

/// The coins `account` holds after a reload of the store at `path`.
fn reloaded_account_coins(
    path: &std::path::Path,
    wallet_id: WalletId,
    account: AccountType,
) -> Vec<OutPoint> {
    let reopened = SqlitePersister::open(SqlitePersisterConfig::new(path)).expect("reopen");
    let state = reopened.load().expect("load");
    let slice = state.wallets.get(&wallet_id).expect("the wallet loads");
    slice
        .wallet_info
        .accounts
        .funds_account(&account)
        .expect("account")
        .utxos
        .keys()
        .copied()
        .collect()
}

/// One block pays two funds accounts, CoinJoin past its gap and BIP44
/// within it. The re-scan passes re-send the BIP44 coins; the store holds
/// one unspent row per coin, a reload restores every coin of both, and the
/// BIP44 coins reload into the BIP44 account.
#[tokio::test]
async fn coins_of_two_accounts_at_one_height_survive_a_reload() {
    const BIP44_PAID: u32 = 5;
    let (persister, _tmp, path) = fresh_persister();
    let persister = Arc::new(persister);
    let wallet = Wallet::new_random(
        key_wallet::Network::Testnet,
        WalletAccountCreationOptions::Default,
    )
    .expect("wallet");
    let coinjoin = AccountType::CoinJoin { index: 0 };
    let count = external_gap(&wallet, coinjoin) + PAST_GAP;
    let coinjoin_addresses = external_addresses(&wallet, coinjoin, count);
    let bip44_addresses = external_addresses(&wallet, bip44_account_0(), BIP44_PAID);
    let past_gap = transaction_paying(&coinjoin_addresses, VALUE, 1);
    let within_gap = transaction_paying(&bip44_addresses, VALUE, 2);
    let manifest_wallet = wallet.clone();
    let (manager, wallet_id, mut queued) = wallet_manager_with_persistence(wallet);
    register(&persister, wallet_id, &manifest_wallet);

    let block = dashcore::Block::dummy(HEIGHT, vec![past_gap.clone(), within_gap.clone()]);
    process_block_with_rescans(&manager, wallet_id, &block, HEIGHT).await;
    persist_queued(&manager, &mut queued, &persister).await;
    persister.flush(wallet_id).expect("flush");
    drop(persister);

    assert_eq!(
        stored_unspent_rows(&path, &coinjoin_addresses),
        vec![1; coinjoin_addresses.len()]
    );
    assert_eq!(
        stored_unspent_rows(&path, &bip44_addresses),
        vec![1; bip44_addresses.len()]
    );
    let reloaded = reloaded_coins(&path, wallet_id);
    let vouts_of = |txid: dashcore::Txid| -> Vec<u32> {
        let mut vouts: Vec<u32> = reloaded
            .iter()
            .filter(|o| o.txid == txid)
            .map(|o| o.vout)
            .collect();
        vouts.sort_unstable();
        vouts
    };
    assert_eq!(vouts_of(past_gap.txid()), (0..count).collect::<Vec<_>>());
    assert_eq!(
        vouts_of(within_gap.txid()),
        (0..BIP44_PAID).collect::<Vec<_>>()
    );
    let bip44_held = reloaded_account_coins(&path, wallet_id, bip44_account_0());
    assert!((0..BIP44_PAID).all(|vout| bip44_held.contains(&OutPoint {
        txid: within_gap.txid(),
        vout
    })));
}

/// Where one transaction's coins sit across the wallet's funds accounts, and
/// the wallet's total.
#[derive(Debug, PartialEq)]
struct Placement {
    coinjoin: Vec<u32>,
    bip44: Vec<u32>,
    held: usize,
    distinct: usize,
    total: u64,
}

fn placement(info: &ManagedWalletInfo, txid: dashcore::Txid) -> Placement {
    let vouts = |account: AccountType| -> Vec<u32> {
        let mut vouts: Vec<u32> = info
            .accounts
            .funds_account(&account)
            .expect("account")
            .utxos
            .keys()
            .filter(|o| o.txid == txid)
            .map(|o| o.vout)
            .collect();
        vouts.sort_unstable();
        vouts
    };
    let held: Vec<OutPoint> = info
        .accounts
        .all_funding_accounts()
        .into_iter()
        .flat_map(|account| account.utxos.keys().copied())
        .filter(|o| o.txid == txid)
        .collect();
    Placement {
        coinjoin: vouts(AccountType::CoinJoin { index: 0 }),
        bip44: vouts(bip44_account_0()),
        held: held.len(),
        distinct: held.iter().collect::<std::collections::HashSet<_>>().len(),
        total: info.balance.total(),
    }
}

/// CoinJoin coins past the registered pool snapshot, credited only by the
/// block's re-scan passes, reload into the CoinJoin account, none into
/// BIP44. Replaying the block over the reloaded state credits nothing
/// twice: the coins, their accounts and the balance stay as reloaded.
#[tokio::test]
async fn coinjoin_coins_past_the_pool_snapshot_reload_into_coinjoin_and_replay_once() {
    let (persister, _tmp, path) = fresh_persister();
    let persister = Arc::new(persister);
    let wallet = Wallet::new_random(
        key_wallet::Network::Testnet,
        WalletAccountCreationOptions::Default,
    )
    .expect("wallet");
    let coinjoin = AccountType::CoinJoin { index: 0 };
    let count = external_gap(&wallet, coinjoin) + PAST_GAP;
    let addresses = external_addresses(&wallet, coinjoin, count);
    let manifest_wallet = wallet.clone();
    let (manager, wallet_id, mut queued) = wallet_manager_with_persistence(wallet);
    register(&persister, wallet_id, &manifest_wallet);

    let block = block_paying(&addresses, VALUE, HEIGHT);
    let passes = process_block_with_rescans(&manager, wallet_id, &block, HEIGHT).await;
    assert!(passes > 1, "the block is re-scanned");
    persist_queued(&manager, &mut queued, &persister).await;
    persister.flush(wallet_id).expect("flush");
    drop(persister);
    drop(manager);

    let txid = block.txdata[0].txid();
    let expected = || Placement {
        coinjoin: (0..count).collect(),
        bip44: Vec::new(),
        held: count as usize,
        distinct: count as usize,
        total: u64::from(count) * VALUE,
    };
    let reopened = SqlitePersister::open(SqlitePersisterConfig::new(&path)).expect("reopen");
    let reloaded = reopened
        .load()
        .expect("load")
        .wallets
        .remove(&wallet_id)
        .expect("the wallet loads")
        .wallet_info;
    drop(reopened);
    let after_reload = placement(&reloaded, txid);

    let (replaying, replay_id, _events) =
        wallet_manager_with_core_wallet(manifest_wallet, reloaded);
    assert_eq!(replay_id, wallet_id);
    process_block_with_rescans(&replaying, wallet_id, &block, HEIGHT).await;
    let guard = replaying.read().await;
    let after_replay = placement(
        &guard
            .get_wallet_info(&wallet_id)
            .expect("the wallet")
            .core_wallet,
        txid,
    );
    assert_eq!(
        (after_reload, after_replay),
        (expected(), expected()),
        "(after the reload, after the replay)"
    );
}
