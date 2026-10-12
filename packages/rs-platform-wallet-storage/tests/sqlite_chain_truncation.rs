#![cfg(feature = "sqlite")]

//! A chain reorganization must reach the store: after a restart the wallet
//! holds the balances, history and scan position it held without one.
//!
//! Every case drives a real `WalletManager` through the real wallet-event
//! adapter into this crate's SQLite persister, then reopens the database and
//! compares what `load()` hands back with the wallet that never restarted.

mod common;

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use dashcore::{
    address::Payload,
    block::{Header, Version},
    ephemerealdata::instant_lock::InstantLock,
    hashes::Hash,
    Address, Block, BlockHash, CompactTarget, Network, OutPoint, PubkeyHash, Transaction, TxIn,
    TxMerkleNode, TxOut, Txid,
};
use key_wallet::{
    wallet::{
        initialization::WalletAccountCreationOptions,
        managed_wallet_info::wallet_info_interface::WalletInfoInterface, ManagedWalletInfo,
    },
    WalletCoreBalance,
};
use platform_wallet::{
    changeset::{
        changeset::ChainTruncation, spawn_wallet_event_adapter, AccountRegistrationEntry,
        CoreChangeSet, PlatformWalletChangeSet, PlatformWalletPersistence, WalletMetadataEntry,
    },
    key_wallet_manager::{WalletEvent, WalletInterface, WalletManager},
    wallet::platform_wallet::{PlatformWalletInfo, WalletId},
};
use platform_wallet_storage::{SqlitePersister, SqlitePersisterConfig};
use proptest::prelude::*;
use tokio::sync::{mpsc, RwLock};
use tokio_util::sync::CancellationToken;

type Manager = Arc<RwLock<WalletManager<PlatformWalletInfo>>>;

const MNEMONIC: &str =
    "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
/// Height of the block that funds the wallet; no case forks below it.
const BASE: u32 = 100;
const FUNDING: u64 = 10_000_000;
const FEE: u64 = 1_000;

/// What a user can tell apart: balances, coins, history and scan position.
#[derive(Debug, Clone, PartialEq)]
struct WalletView {
    balance: WalletCoreBalance,
    synced_height: u32,
    /// Compared with `<=`, see [`WalletView::follows`].
    last_processed_height: u32,
    /// `(value, is_confirmed, is_instantlocked, height)` per unspent coin.
    coins: BTreeMap<OutPoint, (u64, bool, bool, u32)>,
    /// Block height of every recorded transaction, `None` when unconfirmed.
    history: BTreeMap<Txid, Option<u32>>,
}

impl WalletView {
    /// Whether this restored wallet is the `memory` one.
    ///
    /// Everything must be equal except `last_processed_height`, which may
    /// trail: the wallet reports a block only when it holds something of
    /// its own, so a store never hears of the blocks in between.
    fn follows(&self, memory: &WalletView) -> Result<(), String> {
        let mut restored = self.clone();
        if restored.last_processed_height <= memory.last_processed_height {
            restored.last_processed_height = memory.last_processed_height;
        }
        if restored == *memory {
            Ok(())
        } else {
            Err(format!(
                "restored wallet differs from the one that never restarted\n restored: {self:?}\n   memory: {memory:?}"
            ))
        }
    }
}

fn view(info: &ManagedWalletInfo) -> WalletView {
    WalletView {
        balance: info.balance,
        synced_height: info.metadata.synced_height,
        last_processed_height: info.metadata.last_processed_height,
        coins: info
            .accounts
            .all_funding_accounts()
            .into_iter()
            .flat_map(|account| account.utxos.values())
            .map(|coin| {
                (
                    coin.outpoint,
                    (
                        coin.value(),
                        coin.is_confirmed,
                        coin.is_instantlocked,
                        coin.height,
                    ),
                )
            })
            .collect(),
        history: info
            .accounts
            .all_accounts()
            .into_iter()
            .flat_map(|account| account.transactions().values())
            .map(|record| (record.txid, record.height()))
            .collect(),
    }
}

fn foreign() -> Address {
    Address::new(
        Network::Testnet,
        Payload::PubkeyHash(PubkeyHash::from_byte_array([0xF0; 20])),
    )
}

fn transaction(inputs: &[OutPoint], outputs: &[(&Address, u64)]) -> Transaction {
    Transaction {
        version: 1,
        lock_time: 0,
        input: inputs
            .iter()
            .map(|previous_output| TxIn {
                previous_output: *previous_output,
                ..Default::default()
            })
            .collect(),
        output: outputs
            .iter()
            .map(|(address, value)| TxOut {
                value: *value,
                script_pubkey: address.script_pubkey(),
            })
            .collect(),
        special_transaction_payload: None,
    }
}

fn instant_lock(tx: &Transaction) -> InstantLock {
    InstantLock {
        txid: tx.txid(),
        inputs: tx.input.iter().map(|input| input.previous_output).collect(),
        ..Default::default()
    }
}

/// One database fed by the wallet-event adapter.
struct Store {
    persister: Option<Arc<SqlitePersister>>,
    path: PathBuf,
    _dir: tempfile::TempDir,
}

impl Store {
    fn seeded(wallet_id: WalletId, registrations: &[AccountRegistrationEntry]) -> Self {
        let (persister, dir, path) = common::fresh_persister();
        persister
            .store(
                wallet_id,
                PlatformWalletChangeSet {
                    wallet_metadata: Some(WalletMetadataEntry {
                        network: Network::Testnet,
                        wallet_group_id: [0; 32],
                        birth_height: 0,
                    }),
                    account_registrations: registrations.to_vec(),
                    ..Default::default()
                },
            )
            .unwrap();
        Self {
            persister: Some(Arc::new(persister)),
            path,
            _dir: dir,
        }
    }

    fn persister(&self) -> &Arc<SqlitePersister> {
        self.persister.as_ref().expect("store is open")
    }

    /// One adapter drain over `events`: a single `store()` round.
    async fn persist(&self, manager: &Manager, events: Vec<WalletEvent>) {
        if events.is_empty() {
            return;
        }
        let (sender, receiver) = mpsc::unbounded_channel();
        for event in events {
            sender.send(event).unwrap();
        }
        drop(sender);
        let fault = Arc::new(AtomicBool::new(false));
        spawn_wallet_event_adapter(
            Arc::clone(manager),
            Arc::downgrade(self.persister()),
            receiver,
            Arc::clone(&fault),
            CancellationToken::new(),
        )
        .await
        .unwrap();
        assert!(!fault.load(Ordering::Relaxed), "the store rejected a round");
    }

    /// Close the database and read it back the way a relaunching client does.
    fn restart(&mut self, wallet_id: WalletId) -> ManagedWalletInfo {
        // The open-path registry refuses a second live persister on one file.
        drop(self.persister.take());
        let reopened = SqlitePersister::open(SqlitePersisterConfig::new(&self.path)).unwrap();
        let info = reopened
            .load()
            .unwrap()
            .wallets
            .remove(&wallet_id)
            .expect("the wallet is restored")
            .wallet_info;
        self.persister = Some(Arc::new(reopened));
        info
    }

    fn transaction_rows(&self, txid: &Txid) -> Vec<Option<u32>> {
        let conn = self.persister().lock_conn_for_test();
        let mut statement = conn
            .prepare("SELECT height FROM core_transactions WHERE txid = ?1")
            .unwrap();
        let rows = statement
            .query_map([txid.as_byte_array().as_slice()], |row| row.get(0))
            .unwrap();
        rows.map(Result::unwrap).collect()
    }

    /// Stored `(synced_height, last_processed_height)`.
    fn heights(&self, wallet_id: WalletId) -> (Option<u32>, Option<u32>) {
        self.persister()
            .lock_conn_for_test()
            .query_row(
                "SELECT synced_height, last_processed_height FROM core_sync_state \
                 WHERE wallet_id = ?1",
                [wallet_id.as_slice()],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap()
    }
}

/// A wallet funded at [`BASE`], its event stream, and a simulated chain tip.
struct Chain {
    manager: Manager,
    wallet_id: WalletId,
    events: mpsc::UnboundedReceiver<WalletEvent>,
    registrations: Vec<AccountRegistrationEntry>,
    ours: Address,
    funding: OutPoint,
    tip: u32,
    /// Distinguishes the blocks of one branch from the ones they replace.
    branch: u8,
    /// Seeds the foreign inputs of received payments.
    nonce: u8,
}

impl Chain {
    async fn funded() -> Self {
        let mut manager = WalletManager::<PlatformWalletInfo>::new(Network::Testnet);
        let events = manager.take_persistence_receiver().unwrap();
        let wallet_id = manager
            .create_wallet_from_mnemonic(MNEMONIC, 0, WalletAccountCreationOptions::Default)
            .unwrap();
        let wallet = manager.get_wallet(&wallet_id).unwrap();
        let xpub = wallet.accounts.standard_bip44_accounts[&0].account_xpub;
        let registrations = wallet
            .accounts
            .all_accounts()
            .into_iter()
            .map(|account| AccountRegistrationEntry {
                account_type: account.account_type,
                account_xpub: account.account_xpub,
            })
            .collect();
        let ours = manager
            .get_wallet_info_mut(&wallet_id)
            .unwrap()
            .core_wallet
            .accounts
            .standard_bip44_accounts
            .get_mut(&0)
            .unwrap()
            .next_receive_address(Some(&xpub), true)
            .unwrap();
        let funding = transaction(
            &[OutPoint::new(Txid::from_byte_array([13; 32]), 0)],
            &[(&ours, FUNDING)],
        );
        let mut chain = Self {
            manager: Arc::new(RwLock::new(manager)),
            wallet_id,
            events,
            registrations,
            ours,
            funding: OutPoint::new(funding.txid(), 0),
            tip: BASE - 1,
            branch: 0,
            nonce: 0,
        };
        chain.mine(vec![funding]).await;
        chain.sync().await;
        chain
    }

    fn store(&self) -> Store {
        Store::seeded(self.wallet_id, &self.registrations)
    }

    /// Mine `transactions` in the next block of the current branch.
    async fn mine(&mut self, transactions: Vec<Transaction>) {
        self.tip += 1;
        let block = Block {
            header: Header {
                version: Version::default(),
                prev_blockhash: BlockHash::from_byte_array([self.branch; 32]),
                merkle_root: TxMerkleNode::all_zeros(),
                time: self.tip,
                bits: CompactTarget::from_consensus(0x1d00ffff),
                nonce: 0,
            },
            txdata: transactions,
        };
        self.manager
            .write()
            .await
            .process_block_for_wallets(
                &block,
                block.block_hash(),
                self.tip,
                &BTreeSet::from([self.wallet_id]),
            )
            .await;
    }

    /// Commit the scan checkpoint at the tip.
    async fn sync(&mut self) {
        self.manager
            .write()
            .await
            .update_wallet_synced_height(&self.wallet_id, self.tip);
    }

    /// Replace every block above `height` with a new, still empty branch.
    async fn fork_at(&mut self, height: u32) {
        self.manager
            .write()
            .await
            .truncate_above(height)
            .expect("no chain lock is applied in these cases");
        self.tip = height;
        self.branch += 1;
    }

    async fn relay(&mut self, tx: &Transaction, lock: Option<InstantLock>) {
        self.manager
            .write()
            .await
            .process_mempool_transaction(tx, lock)
            .await;
    }

    /// A payment from `coin` that returns the change to the wallet.
    fn spend(&self, coin: OutPoint, value: u64) -> Transaction {
        transaction(
            &[coin],
            &[(&self.ours, value / 2), (&foreign(), value / 2 - FEE)],
        )
    }

    /// A payment to the wallet from somebody else.
    fn receive(&mut self, value: u64) -> Transaction {
        self.nonce += 1;
        transaction(
            &[OutPoint::new(Txid::from_byte_array([self.nonce; 32]), 7)],
            &[(&self.ours, value)],
        )
    }

    fn drain(&mut self) -> Vec<WalletEvent> {
        std::iter::from_fn(|| self.events.try_recv().ok()).collect()
    }

    async fn memory(&self) -> WalletView {
        let manager = self.manager.read().await;
        view(
            &manager
                .get_wallet_info(&self.wallet_id)
                .unwrap()
                .core_wallet,
        )
    }

    async fn largest_coin(&self) -> Option<(OutPoint, u64)> {
        self.memory()
            .await
            .coins
            .into_iter()
            .map(|(outpoint, (value, ..))| (outpoint, value))
            .max_by_key(|(outpoint, value)| (*value, *outpoint))
    }
}

/// How the rounds of one scenario reach the store.
#[derive(Debug, Clone, Copy)]
enum Rounds {
    /// One `store()` per step, as when the adapter keeps up with the wallet.
    PerStep,
    /// Every event of the scenario folded into one `store()`.
    Merged,
}

/// A scenario's store plus the events it has not persisted yet.
struct Session {
    chain: Chain,
    store: Store,
    rounds: Rounds,
    pending: Vec<WalletEvent>,
}

impl Session {
    async fn open(rounds: Rounds) -> Self {
        let mut chain = Chain::funded().await;
        let store = chain.store();
        // The funding block is history every case starts from.
        let funded = chain.drain();
        store.persist(&chain.manager, funded).await;
        Self {
            chain,
            store,
            rounds,
            pending: Vec::new(),
        }
    }

    /// End of one step: persist it now, or keep it for the single merged round.
    async fn step(&mut self) {
        self.pending.extend(self.chain.drain());
        if matches!(self.rounds, Rounds::PerStep) {
            self.flush().await;
        }
    }

    async fn flush(&mut self) {
        let events = std::mem::take(&mut self.pending);
        self.store.persist(&self.chain.manager, events).await;
    }

    /// Persist what is left, restart, and require the restored wallet to be
    /// the one that never restarted.
    async fn assert_restart_matches_memory(&mut self) -> WalletView {
        self.step().await;
        self.flush().await;
        let restored = view(&self.store.restart(self.chain.wallet_id));
        if let Err(difference) = restored.follows(&self.chain.memory().await) {
            panic!("{:?}: {difference}", self.rounds);
        }
        restored
    }
}

#[tokio::test]
async fn should_restore_a_spent_coin_when_its_spender_is_not_mined_again() {
    for rounds in [Rounds::PerStep, Rounds::Merged] {
        let mut session = Session::open(rounds).await;
        let funding = session.chain.funding;
        let spend = session.chain.spend(funding, FUNDING);
        session.chain.mine(vec![spend.clone()]).await;
        session.chain.sync().await;
        session.step().await;

        session.chain.fork_at(BASE).await;

        let restored = session.assert_restart_matches_memory().await;
        assert!(restored.coins.contains_key(&funding), "{rounds:?}");
        assert!(!restored.history.contains_key(&spend.txid()), "{rounds:?}");
        assert!(
            session.store.transaction_rows(&spend.txid()).is_empty(),
            "{rounds:?}: the dropped transaction must leave the store"
        );
    }
}

#[tokio::test]
async fn should_store_a_transaction_once_in_its_new_block_when_mined_again() {
    for rounds in [Rounds::PerStep, Rounds::Merged] {
        let mut session = Session::open(rounds).await;
        let funding = session.chain.funding;
        let spend = session.chain.spend(funding, FUNDING);
        session.chain.mine(vec![]).await;
        session.chain.mine(vec![spend.clone()]).await;
        session.chain.mine(vec![]).await;
        session.chain.sync().await;
        session.step().await;

        // The new branch mines it one block earlier, and the restart comes
        // before the scan is back at the old height.
        session.chain.fork_at(BASE).await;
        session.step().await;
        session.chain.mine(vec![spend.clone()]).await;
        session.chain.sync().await;

        let restored = session.assert_restart_matches_memory().await;
        assert_eq!(
            restored.history.get(&spend.txid()),
            Some(&Some(BASE + 1)),
            "{rounds:?}"
        );
        assert_eq!(restored.synced_height, BASE + 1, "{rounds:?}");
        assert_eq!(
            session.store.transaction_rows(&spend.txid()),
            vec![Some(BASE + 1)],
            "{rounds:?}: stored once, in the new block"
        );
    }
}

#[tokio::test]
async fn should_lower_stored_heights_to_the_fork_before_the_scan_resumes() {
    for rounds in [Rounds::PerStep, Rounds::Merged] {
        let mut session = Session::open(rounds).await;
        for _ in 0..3 {
            session.chain.mine(vec![]).await;
        }
        session.chain.sync().await;
        session.step().await;

        session.chain.fork_at(BASE).await;

        let restored = session.assert_restart_matches_memory().await;
        assert_eq!(restored.synced_height, BASE, "{rounds:?}");
        assert!(restored.last_processed_height <= BASE, "{rounds:?}");
    }
}

#[tokio::test]
async fn should_keep_an_instant_locked_transaction_unconfirmed_when_its_block_is_dropped() {
    for rounds in [Rounds::PerStep, Rounds::Merged] {
        let mut session = Session::open(rounds).await;
        let funding = session.chain.funding;
        let spend = session.chain.spend(funding, FUNDING);
        session
            .chain
            .relay(&spend, Some(instant_lock(&spend)))
            .await;
        session.step().await;
        session.chain.mine(vec![spend.clone()]).await;
        session.chain.sync().await;
        session.step().await;

        session.chain.fork_at(BASE).await;

        let restored = session.assert_restart_matches_memory().await;
        let change = OutPoint::new(spend.txid(), 0);
        assert_eq!(
            restored.coins.get(&change),
            Some(&(FUNDING / 2, false, true, 0)),
            "{rounds:?}: its change stays, unconfirmed and InstantSend-locked"
        );
        assert!(!restored.coins.contains_key(&funding), "{rounds:?}");
        assert_eq!(
            restored.history.get(&spend.txid()),
            Some(&None),
            "{rounds:?}"
        );
    }
}

/// A double spend sweeps the wallet's own unconfirmed payment, then a fork
/// drops the block that held the winner: the store holds what the wallet
/// holds, and neither transaction comes back.
///
/// The coin both spent is not asserted on its own: whether the wallet takes
/// it back is key-wallet's decision, and the store follows it.
// TODO(sweep-of-unstored-loser): run this merged too, and generate the
// double spend in the property test below, once a payment detected and swept
// within one `store()` round leaves its coin spent in the store. Today that
// round stores no spend of the coin, fork or no fork.
#[tokio::test]
async fn should_keep_a_swept_payment_out_when_the_block_of_its_double_spend_is_dropped() {
    let mut session = Session::open(Rounds::PerStep).await;
    let funding = session.chain.funding;
    let payment = session.chain.spend(funding, FUNDING);
    session.chain.relay(&payment, None).await;
    session.step().await;
    let winner = transaction(&[funding], &[(&foreign(), FUNDING - FEE)]);
    session.chain.mine(vec![winner.clone()]).await;
    session.chain.sync().await;
    session.step().await;

    session.chain.fork_at(BASE).await;

    let restored = session.assert_restart_matches_memory().await;
    for gone in [payment.txid(), winner.txid()] {
        assert!(!restored.history.contains_key(&gone));
    }
    assert!(
        !restored
            .coins
            .contains_key(&OutPoint::new(payment.txid(), 0)),
        "the swept payment's change is not a coin"
    );
}

/// The lock is all that tells a fork to keep a transaction, and the block
/// that mines the transaction replaces the context it arrived in. Stored on
/// its own it outlives both the block and a restart, so the relaunched
/// wallet keeps what the one that never restarted keeps.
#[tokio::test]
async fn should_keep_an_instant_locked_transaction_when_the_fork_follows_a_restart() {
    for rounds in [Rounds::PerStep, Rounds::Merged] {
        let mut session = Session::open(rounds).await;
        let funding = session.chain.funding;
        let spend = session.chain.spend(funding, FUNDING);
        session
            .chain
            .relay(&spend, Some(instant_lock(&spend)))
            .await;
        session.step().await;
        session.chain.mine(vec![spend.clone()]).await;
        session.chain.sync().await;
        session.step().await;
        session.flush().await;

        let mut relaunched = session.store.restart(session.chain.wallet_id);
        relaunched
            .truncate_above(BASE)
            .expect("no chain lock is applied in these cases");
        session.chain.fork_at(BASE).await;

        let relaunched = view(&relaunched);
        if let Err(difference) = relaunched.follows(&session.chain.memory().await) {
            panic!("{rounds:?}: {difference}");
        }
        assert_eq!(
            relaunched.history.get(&spend.txid()),
            Some(&None),
            "{rounds:?}: kept, as unconfirmed"
        );
    }
}

/// A truncation is several writes: rows deleted, records rewritten, coins
/// released, heights lowered. One that fails part-way must leave none behind.
#[tokio::test]
async fn should_leave_the_store_untouched_when_a_truncation_cannot_be_applied() {
    let mut session = Session::open(Rounds::PerStep).await;
    let wallet_id = session.chain.wallet_id;
    let funding = session.chain.funding;
    let kept = session.chain.spend(funding, FUNDING);
    let removed = session.chain.receive(250_000);
    session.chain.relay(&kept, Some(instant_lock(&kept))).await;
    session
        .chain
        .mine(vec![kept.clone(), removed.clone()])
        .await;
    session.chain.sync().await;
    session.step().await;
    // The kept transaction's record can no longer be rewritten as unconfirmed.
    session
        .store
        .persister()
        .lock_conn_for_test()
        .execute(
            "UPDATE core_transactions SET record_blob = X'00' WHERE txid = ?1",
            [kept.txid().as_byte_array().as_slice()],
        )
        .unwrap();

    session.chain.fork_at(BASE).await;
    let truncation = session
        .chain
        .drain()
        .into_iter()
        .find_map(|event| match event {
            WalletEvent::ChainTruncated {
                height,
                txids,
                unconfirmed_txids,
                restored_outpoints,
                ..
            } => Some(ChainTruncation {
                height,
                txids,
                unconfirmed_txids,
                restored_outpoints,
            }),
            _ => None,
        })
        .expect("the fork is reported");
    assert_eq!(truncation.txids, [removed.txid()]);
    assert_eq!(truncation.unconfirmed_txids, [kept.txid()]);

    let rejected = session.store.persister().store(
        wallet_id,
        PlatformWalletChangeSet {
            core: Some(CoreChangeSet {
                truncations: vec![truncation],
                ..Default::default()
            }),
            ..Default::default()
        },
    );

    assert!(
        rejected.is_err(),
        "an unreadable record must fail the round"
    );
    assert_eq!(
        session.store.transaction_rows(&removed.txid()),
        vec![Some(BASE + 1)],
        "the removal made before the failure is rolled back"
    );
    assert_eq!(
        session.store.transaction_rows(&kept.txid()),
        vec![Some(BASE + 1)]
    );
    assert_eq!(
        session.store.heights(wallet_id),
        (Some(BASE + 1), Some(BASE + 1))
    );
}

/// One step of a generated history.
#[derive(Debug, Clone)]
enum Step {
    /// Mine a block holding whatever is waiting in the mempool.
    Block,
    /// Mine a payment to the wallet.
    Receive,
    /// Spend the wallet's largest coin, returning change to it.
    Spend { relayed_first: bool, locked: bool },
    /// Commit the scan checkpoint at the tip.
    Sync,
    /// Replace the last `depth` blocks. What they held goes back to the
    /// mempool; `remine` mines it in the first block of the new branch.
    Reorg { depth: u32, remine: bool },
}

fn step() -> impl Strategy<Value = Step> {
    prop_oneof![
        2 => Just(Step::Block),
        2 => Just(Step::Receive),
        4 => (any::<bool>(), any::<bool>())
            .prop_map(|(relayed_first, locked)| Step::Spend { relayed_first, locked }),
        2 => Just(Step::Sync),
        3 => (1u32..=3, any::<bool>()).prop_map(|(depth, remine)| Step::Reorg { depth, remine }),
    ]
}

/// The chain as a miner sees it: what each block above [`BASE`] holds and
/// what waits in the mempool, parents ahead of their children.
#[derive(Default)]
struct Miner {
    blocks: Vec<Vec<Transaction>>,
    mempool: Vec<Transaction>,
}

impl Miner {
    async fn mine(&mut self, chain: &mut Chain, mut extra: Vec<Transaction>) {
        let mut block = std::mem::take(&mut self.mempool);
        block.append(&mut extra);
        chain.mine(block.clone()).await;
        self.blocks.push(block);
    }

    async fn apply(&mut self, chain: &mut Chain, step: &Step) {
        match step {
            Step::Block => self.mine(chain, vec![]).await,
            Step::Receive => {
                let payment = chain.receive(250_000);
                self.mine(chain, vec![payment]).await;
            }
            Step::Spend {
                relayed_first,
                locked,
            } => {
                let Some((coin, value)) = chain.largest_coin().await else {
                    return;
                };
                if value < 4 * FEE {
                    return;
                }
                let spend = chain.spend(coin, value);
                let lock = locked.then(|| instant_lock(&spend));
                if *relayed_first || lock.is_some() {
                    chain.relay(&spend, lock).await;
                    self.mempool.push(spend);
                } else {
                    self.mine(chain, vec![spend]).await;
                }
            }
            Step::Sync => chain.sync().await,
            Step::Reorg { depth, remine } => {
                let depth = (*depth as usize).min(self.blocks.len());
                if depth == 0 {
                    return;
                }
                let mut returned: Vec<Transaction> = self
                    .blocks
                    .split_off(self.blocks.len() - depth)
                    .into_iter()
                    .flatten()
                    .collect();
                chain.fork_at(chain.tip - depth as u32).await;
                returned.append(&mut self.mempool);
                self.mempool = returned;
                if *remine {
                    self.mine(chain, vec![]).await;
                }
            }
        }
    }
}

/// The differential: one wallet, two stores. One store takes a `store()`
/// round per step, the other a single round holding every event, so the
/// second exercises the changeset merge the first never needs.
async fn merged_and_stepwise_stores_agree(steps: Vec<Step>) -> Result<(), TestCaseError> {
    let mut chain = Chain::funded().await;
    let mut stepwise = chain.store();
    let mut merged = chain.store();
    let funded = chain.drain();
    stepwise.persist(&chain.manager, funded.clone()).await;
    merged.persist(&chain.manager, funded).await;

    let mut miner = Miner::default();
    let mut log = Vec::new();
    for step in &steps {
        miner.apply(&mut chain, step).await;
        let events = chain.drain();
        log.extend(events.iter().cloned());
        stepwise.persist(&chain.manager, events).await;
    }
    merged.persist(&chain.manager, log).await;

    let memory = chain.memory().await;
    let stepwise = view(&stepwise.restart(chain.wallet_id));
    let merged = view(&merged.restart(chain.wallet_id));
    for (rounds, restored) in [
        ("one round per step", stepwise),
        ("one merged round", merged),
    ] {
        if let Err(difference) = restored.follows(&memory) {
            return Err(TestCaseError::fail(format!("{rounds}: {difference}")));
        }
    }
    Ok(())
}

#[test]
fn should_store_the_same_wallet_from_one_merged_round_as_from_a_round_per_step() {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap();
    let config = ProptestConfig {
        cases: 48,
        failure_persistence: None,
        ..ProptestConfig::default()
    };
    proptest!(config, |(steps in proptest::collection::vec(step(), 1..12))| {
        runtime.block_on(merged_and_stepwise_stores_agree(steps))?;
    });
}
