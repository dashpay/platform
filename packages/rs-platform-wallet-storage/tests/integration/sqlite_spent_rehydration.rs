#![cfg(feature = "sqlite")]

use crate::common;

use std::collections::BTreeMap;

use common::{chain_lock, registration, Case};
use dashcore::{hashes::Hash, Address, Network, OutPoint, ScriptBuf, Transaction, TxOut, Txid};
use key_wallet::{
    managed_account::{
        managed_account_trait::ManagedAccountTrait, transaction_record::TransactionRecord,
    },
    transaction_checking::TransactionContext,
    wallet::{
        initialization::WalletAccountCreationOptions,
        managed_wallet_info::{
            coin_selection::{CoinSelector, SelectionStrategy},
            fee::FeeRate,
            wallet_info_interface::WalletInfoInterface,
        },
        ManagedWalletInfo, Wallet,
    },
    Utxo,
};
use platform_wallet::changeset::{
    changeset::SweepBatch, CoreChangeSet, PlatformWalletChangeSet, PlatformWalletPersistence,
};
use platform_wallet::test_support::core_fixtures::{
    block, check, foreign_address, outside, pay, records_of, spend_of, stored_utxo,
    wallet_with_receive_address,
};
use platform_wallet_storage::{
    LoadPolicy, SqlitePersister, SqlitePersisterConfig, WalletStorageError,
};

const MEMPOOL: TransactionContext = TransactionContext::Mempool;

/// One past half of `i64::MAX`: two owned outputs of it cannot be summed as `i64`.
const UNREPRESENTABLE_HALF: u64 = (1 << 62) + 1;

/// An output burning `value` to an empty script.
fn burn(value: u64) -> TxOut {
    TxOut {
        value,
        script_pubkey: ScriptBuf::new(),
    }
}

/// A later round carrying only `core`.
fn core_round(core: CoreChangeSet) -> PlatformWalletChangeSet {
    PlatformWalletChangeSet {
        core: Some(core),
        ..Default::default()
    }
}

/// The sweep a sync persists when `winner`, mined at 101, removed `txids`.
fn sweep(txids: Vec<Txid>, winner: &Transaction, released: Vec<OutPoint>) -> SweepBatch {
    SweepBatch {
        txids,
        superseded_by: winner.txid(),
        winner_mined_height: Some(101),
        released_outpoints: released,
    }
}

fn load_wallet(persister: &SqlitePersister, wallet_id: &[u8; 32]) -> (Wallet, ManagedWalletInfo) {
    let restored = persister.load().unwrap().wallets.remove(wallet_id).unwrap();
    (restored.wallet, restored.wallet_info)
}

/// `(wallet, outpoint, status)` of every `asset_locks` row, sorted.
fn asset_lock_rows(persister: &SqlitePersister) -> Vec<([u8; 32], OutPoint, String)> {
    let conn = persister.lock_conn_for_test();
    let mut stmt = conn
        .prepare("SELECT wallet_id, outpoint, status FROM asset_locks")
        .unwrap();
    let mut rows: Vec<_> = stmt
        .query_map([], |row| {
            let wallet: Vec<u8> = row.get(0)?;
            let outpoint: Vec<u8> = row.get(1)?;
            Ok((wallet, outpoint, row.get::<_, String>(2)?))
        })
        .unwrap()
        .map(|row| {
            let (wallet, outpoint, status) = row.unwrap();
            (
                wallet.try_into().unwrap(),
                platform_wallet_storage::sqlite::schema::blob::decode_outpoint(&outpoint).unwrap(),
                status,
            )
        })
        .collect();
    rows.sort();
    rows
}

fn count(persister: &SqlitePersister, sql: &str) -> i64 {
    let conn = persister.lock_conn_for_test();
    conn.query_row(sql, [], |row| row.get(0)).unwrap()
}

/// Pool rows as a live sync persists them, so load routes each coin to its owner.
fn insert_pool_rows(persister: &SqlitePersister, wallet_id: &[u8; 32], rows: &[(&str, &Address)]) {
    let conn = persister.lock_conn_for_test();
    for (account_type, address) in rows {
        conn.execute(
            "INSERT INTO core_address_pool \
             (wallet_id, account_type, account_index, pool_type, address_index, script) \
             VALUES (?1, ?2, 0, 0, 0, ?3)",
            rusqlite::params![
                &wallet_id[..],
                account_type,
                address.script_pubkey().as_bytes()
            ],
        )
        .unwrap();
    }
}

/// Store `records` and `coins` for `wallet`, checkpointed at 300, then drop
/// `unpooled`'s pool row and track `contact` only on a contact's watch-only chain.
fn store_with_contact_chain(
    wallet: &Wallet,
    records: Vec<TransactionRecord>,
    coins: Vec<Utxo>,
    (unpooled, contact): (&Address, &Address),
) -> (SqlitePersister, tempfile::TempDir) {
    let (persister, dir, _) = common::fresh_persister();
    let core = synced(CoreChangeSet {
        records,
        new_utxos: coins,
        ..Default::default()
    });
    persister
        .store(wallet.wallet_id, registration(wallet, Some(core)))
        .unwrap();
    let script = unpooled.script_pubkey();
    let sql = "DELETE FROM core_address_pool WHERE script = ?1";
    persister
        .lock_conn_for_test()
        .execute(sql, [script.as_bytes()])
        .unwrap();
    insert_pool_rows(
        &persister,
        &wallet.wallet_id,
        &[("dashpay_external", contact)],
    );
    (persister, dir)
}

/// `core`, closed by a sync checkpoint at 300.
fn synced(core: CoreChangeSet) -> CoreChangeSet {
    CoreChangeSet {
        last_processed_height: Some(300),
        synced_height: Some(300),
        ..core
    }
}

/// Persist one Core round for `wallet` as a sync would, then reopen the store strictly.
fn store_then_reopen(wallet: &Wallet, core: CoreChangeSet) -> (SqlitePersister, tempfile::TempDir) {
    let (persister, dir, path) = common::fresh_persister();
    persister
        .store(wallet.wallet_id, registration(wallet, Some(core)))
        .expect("the store accepts what the live checker recorded");
    drop(persister);
    let config = SqlitePersisterConfig::new(path).with_load_policy(LoadPolicy::Strict);
    (SqlitePersister::open(config).unwrap(), dir)
}

/// A fresh wallet with a receive address on its first BIP44 and first BIP32 account.
fn two_account_wallet() -> (Wallet, ManagedWalletInfo, Address, Address) {
    let (wallet, mut info, first) = wallet_with_receive_address();
    let xpub = wallet.accounts.standard_bip32_accounts[&0].account_xpub;
    let account = info.accounts.standard_bip32_accounts.get_mut(&0).unwrap();
    let second = account.next_receive_address(Some(&xpub), true).unwrap();
    (wallet, info, first, second)
}

fn coin_set(info: &ManagedWalletInfo) -> Vec<OutPoint> {
    info.accounts.standard_bip44_accounts[&0]
        .utxos
        .keys()
        .copied()
        .collect()
}

/// Every funding account still holding a coin or a record of `transactions`.
fn traces(info: &ManagedWalletInfo, transactions: &[&Transaction]) -> Vec<String> {
    let txids: Vec<Txid> = transactions.iter().map(|tx| tx.txid()).collect();
    info.accounts
        .all_funding_accounts()
        .into_iter()
        .filter_map(|account| {
            let coins = account
                .utxos
                .keys()
                .filter(|outpoint| txids.contains(&outpoint.txid))
                .count();
            let records = txids
                .iter()
                .filter(|txid| account.transactions().contains_key(*txid))
                .count();
            let account_type = account.managed_account_type().to_account_type();
            (coins + records > 0)
                .then(|| format!("{account_type:?}: {coins} coins, {records} records"))
        })
        .collect()
}

fn typed_error(error: platform_wallet::changeset::PersistenceError) -> WalletStorageError {
    let platform_wallet::changeset::PersistenceError::Backend { source, .. } = error else {
        panic!("expected backend error")
    };
    *source.downcast::<WalletStorageError>().unwrap()
}

struct Fixture {
    persister: SqlitePersister,
    _dir: tempfile::TempDir,
    wallet_id: [u8; 32],
    funding: Transaction,
    spent: OutPoint,
    available: OutPoint,
}

impl Fixture {
    async fn new(spend_context: TransactionContext) -> Self {
        Self::build(block(100), spend_context, true).await
    }

    /// Funding pays 100_000 (`spent`, which the stored spend takes) and
    /// 20_000 (`available`). Without `store_funding_record` the funding is
    /// persisted only through its UTXOs: a height-only row with no record to replay.
    async fn build(
        funding_context: TransactionContext,
        spend_context: TransactionContext,
        store_funding_record: bool,
    ) -> Self {
        let (mut wallet, mut info, address) = wallet_with_receive_address();
        let funding = spend_of(
            &[outside(13)],
            vec![pay(100_000, &address), pay(20_000, &address)],
        );
        let [spent, available] = [0, 1].map(|vout| OutPoint::new(funding.txid(), vout));
        let spending = spend_of(&[spent], vec![burn(99_000)]);
        let mut records = records_of(&mut info, &mut wallet, &funding, funding_context).await;
        let coins = &info.accounts.standard_bip44_accounts[&0].utxos;
        let (new_utxos, spent_coin) = (coins.values().cloned().collect(), coins[&spent].clone());
        records.extend(records_of(&mut info, &mut wallet, &spending, spend_context).await);
        assert_eq!(records.len(), 2);
        if !store_funding_record {
            records.retain(|record| record.txid != funding.txid());
        }
        let (persister, dir, path) = common::fresh_persister();
        let core = synced(CoreChangeSet {
            records,
            new_utxos,
            spent_utxos: vec![spent_coin],
            ..Default::default()
        });
        persister
            .store(wallet.wallet_id, registration(&wallet, Some(core)))
            .unwrap();
        drop(persister);
        let persister = SqlitePersister::open(SqlitePersisterConfig::new(path)).unwrap();
        if !store_funding_record {
            let (height, has_record): (Option<u32>, bool) = persister
                .lock_conn_for_test()
                .query_row(
                    "SELECT height, record_blob IS NOT NULL FROM core_transactions WHERE txid = ?1",
                    [funding.txid().as_byte_array().as_slice()],
                    |row| Ok((row.get(0)?, row.get(1)?)),
                )
                .unwrap();
            assert_eq!((height, has_record), (Some(100), false));
        }
        Self {
            persister,
            _dir: dir,
            wallet_id: wallet.wallet_id,
            funding,
            spent,
            available,
        }
    }

    fn load(&self) -> (Wallet, ManagedWalletInfo) {
        load_wallet(&self.persister, &self.wallet_id)
    }

    /// The txid of the one stored row that carries a record.
    fn recorded_txid(&self) -> Txid {
        let stored: Vec<u8> = self
            .persister
            .lock_conn_for_test()
            .query_row(
                "SELECT txid FROM core_transactions WHERE record_blob IS NOT NULL",
                [],
                |row| row.get(0),
            )
            .unwrap();
        Txid::from_slice(&stored).unwrap()
    }

    fn assert_spent_excluded(&self, info: &ManagedWalletInfo) {
        let coins = &info.accounts.standard_bip44_accounts[&0].utxos;
        assert!(!coins.contains_key(&self.spent));
        assert!(coins.contains_key(&self.available));
        assert_eq!(info.balance.total(), 20_000);
        let selector = CoinSelector::new(SelectionStrategy::LargestFirst);
        assert!(selector
            .select_coins(coins.values(), 50_000, FeeRate::default(), 300)
            .is_err());
        let selection = selector
            .select_coins(coins.values(), 5_000, FeeRate::default(), 300)
            .unwrap();
        assert_eq!(selection.selected.len(), 1);
        assert_eq!(selection.selected[0].outpoint, self.available);
    }

    fn assert_spent_stored(&self, expected: bool) {
        let spent: bool = self
            .persister
            .lock_conn_for_test()
            .query_row(
                "SELECT spent FROM core_utxos WHERE substr(outpoint, 2, 32) = ?1 AND value = 100000",
                [self.spent.txid.as_byte_array().as_slice()],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(spent, expected, "stored spent flag of the reserved input");
    }

    async fn redeliver(&self, wallet: &mut Wallet, info: &mut ManagedWalletInfo) {
        let records = records_of(info, wallet, &self.funding, block(100)).await;
        self.assert_spent_excluded(info);
        let new_utxos = info.accounts.standard_bip44_accounts[&0]
            .utxos
            .values()
            .cloned()
            .collect();
        let core = CoreChangeSet {
            records,
            new_utxos,
            ..Default::default()
        };
        self.persister
            .store(self.wallet_id, core_round(core))
            .unwrap();
        let (_, reloaded) = self.load();
        self.assert_spent_excluded(&reloaded);
    }
}

/// One stored spend of a funded coin, restored and then confronted with its
/// funding transaction again.
struct SpentCase {
    label: &'static str,
    funding: TransactionContext,
    spend: TransactionContext,
    height_only_funding: bool,
    /// The store's projection still parks the spent output as unspent.
    stale_projection: bool,
    /// A chain lock at 300 is stored before the restart.
    persisted_chain_lock: bool,
    /// A chain lock at 300 and a sync checkpoint prune observed spends after it.
    pruned: bool,
}

/// The spent coin stays out of the spendable set after the restart and through
/// funding redelivery; an unconfirmed spend keeps it reserved, and stored spent.
#[tokio::test]
async fn should_keep_spent_output_excluded_after_restart_and_funding_redelivery() {
    let case = |label, spend| SpentCase {
        label,
        funding: block(100),
        spend,
        height_only_funding: false,
        stale_projection: false,
        persisted_chain_lock: false,
        pruned: false,
    };
    let cases = [
        case("spent output after reload and redelivery", block(200)),
        SpentCase {
            pruned: true,
            ..case("finality pruning", block(200))
        },
        SpentCase {
            pruned: true,
            height_only_funding: true,
            ..case("finality pruning with height-only funding", block(200))
        },
        SpentCase {
            height_only_funding: true,
            ..case("unconfirmed spend with height-only funding", MEMPOOL)
        },
        SpentCase {
            stale_projection: true,
            ..case(
                "stale unspent projection against confirmed history",
                block(200),
            )
        },
        case("inputs reserved by an unconfirmed spend", MEMPOOL),
        SpentCase {
            funding: MEMPOOL,
            ..case("unconfirmed funding confirming after reload", MEMPOOL)
        },
        case("funding and spend in the same block", block(100)),
        SpentCase {
            persisted_chain_lock: true,
            ..case(
                "persisted finality before the sync checkpoint advances",
                block(100),
            )
        },
    ];
    for case in cases {
        let _case = Case(case.label.into());
        let fixture = Fixture::build(
            case.funding.clone(),
            case.spend.clone(),
            !case.height_only_funding,
        )
        .await;
        if case.stale_projection {
            let conn = fixture.persister.lock_conn_for_test();
            conn.execute("UPDATE core_utxos SET spent = 0", []).unwrap();
        }
        if case.persisted_chain_lock {
            let core = CoreChangeSet {
                last_applied_chain_lock: Some(chain_lock(300, 30)),
                ..Default::default()
            };
            let round = core_round(core);
            fixture.persister.store(fixture.wallet_id, round).unwrap();
        }
        let (mut wallet, mut info) = fixture.load();
        let unconfirmed_spend = case.spend == MEMPOOL;
        if case.funding == MEMPOOL {
            let coins = &info.accounts.standard_bip44_accounts[&0].utxos;
            assert!(!coins.contains_key(&fixture.spent));
        } else {
            fixture.assert_spent_excluded(&info);
        }
        if unconfirmed_spend {
            assert!(!info.observed_spent_outpoints().contains_key(&fixture.spent));
        }
        if case.persisted_chain_lock {
            let account = &info.accounts.standard_bip44_accounts[&0];
            assert!(account
                .keys()
                .transaction_is_finalized(&fixture.funding.txid()));
            info.update_synced_height(300);
        }
        if case.pruned {
            info.apply_chain_lock(chain_lock(300, 30));
            info.update_synced_height(300);
            assert!(info.observed_spent_outpoints().is_empty());
        }
        fixture.redeliver(&mut wallet, &mut info).await;
        if unconfirmed_spend {
            fixture.assert_spent_stored(true);
        }
    }
}

#[test]
fn should_restore_without_an_async_runtime() {
    let fixture = tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap()
        .block_on(Fixture::new(block(200)));
    let (_, info) = fixture.load();
    fixture.assert_spent_excluded(&info);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn should_restore_on_managers_blocking_pool() {
    let fixture = Fixture::new(block(200)).await;
    tokio::task::spawn_blocking(move || {
        let (_, info) = fixture.load();
        fixture.assert_spent_excluded(&info);
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn should_keep_replayed_output_only_in_its_owning_account() {
    let mut wallet =
        Wallet::new_random(Network::Testnet, WalletAccountCreationOptions::Default).unwrap();
    let mut info = ManagedWalletInfo::from_wallet(&wallet, 0);
    let xpub = wallet.accounts.standard_bip32_accounts[&0].account_xpub;
    let account = info.accounts.standard_bip32_accounts.get_mut(&0).unwrap();
    let address = account.next_receive_address(Some(&xpub), true).unwrap();
    // vout 1: no pool row and no replay owner, so the first-account fallback
    // must hold. vout 2: tracked only by a contact's watch-only chain.
    let (unowned, contact) = (foreign_address(0x61), foreign_address(0x62));
    let outputs = vec![
        pay(70_000, &address),
        pay(5_000, &unowned),
        pay(9_000, &contact),
    ];
    let funding = spend_of(&[outside(14)], outputs);
    let [coin, fallback, contact_coin] = [0, 1, 2].map(|vout| OutPoint::new(funding.txid(), vout));
    let records = records_of(&mut info, &mut wallet, &funding, block(100)).await;
    let account = &info.accounts.standard_bip32_accounts[&0];
    let mut coins: Vec<_> = account.utxos.values().cloned().collect();
    assert_eq!(coins.len(), 1);
    coins.push(stored_utxo(&funding, 1, unowned));
    coins.push(stored_utxo(&funding, 2, contact.clone()));
    // Without its pool row the loader cannot attribute the coin and parks
    // it in the first funds account; replay then finds the real owner.
    let (persister, _dir) = store_with_contact_chain(&wallet, records, coins, (&address, &contact));

    let (_, info) = load_wallet(&persister, &wallet.wallet_id);
    let holders = |outpoint: &OutPoint| {
        info.accounts
            .all_funding_accounts()
            .into_iter()
            .filter(|account| account.utxos.contains_key(outpoint))
            .count()
    };
    assert_eq!(
        holders(&coin),
        1,
        "one outpoint must live in exactly one account"
    );
    assert!(info.accounts.standard_bip32_accounts[&0]
        .utxos
        .contains_key(&coin));
    assert!(
        info.accounts.standard_bip44_accounts[&0]
            .utxos
            .contains_key(&fallback),
        "an output replay cannot attribute keeps the first-account fallback"
    );
    assert_eq!(holders(&contact_coin), 0, "a contact's coin is not ours");
    assert_eq!(info.balance.total(), 75_000);
    assert_eq!(info.get_spendable_utxos().len(), 2);
    assert_eq!(
        count(
            &persister,
            "SELECT count(*) FROM core_utxos WHERE spent = 0"
        ),
        3,
        "load must not delete stored rows"
    );
}

#[tokio::test]
async fn should_drop_replay_credit_for_contact_only_script() {
    let mut wallet =
        Wallet::new_random(Network::Testnet, WalletAccountCreationOptions::Default).unwrap();
    let mut info = ManagedWalletInfo::from_wallet(&wallet, 0);
    let xpub = wallet.accounts.standard_bip44_accounts[&0].account_xpub;
    let account = info.accounts.standard_bip44_accounts.get_mut(&0).unwrap();
    let [contact, ours]: [Address; 2] = account
        .next_receive_addresses(Some(&xpub), 2, true)
        .unwrap()
        .try_into()
        .unwrap();
    let funding = spend_of(
        &[outside(16)],
        vec![pay(40_000, &contact), pay(7_000, &ours)],
    );
    let [contact_coin, our_coin] = [0, 1].map(|vout| OutPoint::new(funding.txid(), vout));
    let records = records_of(&mut info, &mut wallet, &funding, block(100)).await;
    let account = &info.accounts.standard_bip44_accounts[&0];
    let coins: Vec<_> = account.utxos.values().cloned().collect();
    assert_eq!(coins.len(), 2);
    // The store tracks the script only on a contact's watch-only chain,
    // yet the rebuilt funds account still derives it, so replay credits it.
    let (persister, _dir) = store_with_contact_chain(&wallet, records, coins, (&contact, &contact));

    let (_, info) = load_wallet(&persister, &wallet.wallet_id);
    assert!(
        info.accounts
            .all_funding_accounts()
            .into_iter()
            .all(|account| !account.utxos.contains_key(&contact_coin)),
        "a replay-credited contact coin must not survive load"
    );
    assert!(info.accounts.standard_bip44_accounts[&0]
        .utxos
        .contains_key(&our_coin));
    assert_eq!(info.balance.total(), 7_000);
}

/// A loser spending a shared and an extra coin, a competing winner spending
/// only the shared one, stored unsettled and repaired at load.
#[derive(Clone, Copy, Debug)]
enum ConflictCase {
    /// A persisted conflict releases the extra input.
    Released,
    /// The same, with the funding persisted as a height-only row.
    HeightOnlyFunding,
    /// A surviving spend keeps the extra input reserved.
    SurvivingClaim,
    /// A persisted chainlock resolves an InstantSend-locked competitor.
    PersistedChainLock,
    /// A recovery load does not rewrite the conflicting history.
    Recovery,
    /// A recordless winner's claim on the extra input survives replay.
    RecordlessClaim,
    /// So does a recordless winner's sweep placeholder.
    RecordlessPlaceholder,
    /// An unknown durable claim survives startup conflict repair.
    UnknownClaim {
        placeholder: bool,
        height: Option<u32>,
    },
    /// A malformed claimant fails the load.
    MalformedClaim,
    /// A failed replay repair rolls back.
    FailedRepair,
    /// Swept asset locks are removed on restart.
    Assets,
    /// Recovery rolls the swept asset locks back.
    AssetsRecovery,
    /// A SQL failure rolls the swept asset locks back.
    AssetsFailedRepair,
    /// A replay that cannot converge is reported for its wallet.
    Unconverged,
    /// A defeated lock cannot remove a surviving spender.
    DefeatedLock,
}

#[tokio::test]
async fn should_settle_a_persisted_conflict_on_restart() {
    use ConflictCase::*;
    let unknown = |placeholder, height| UnknownClaim {
        placeholder,
        height,
    };
    for case in [
        Released,
        HeightOnlyFunding,
        SurvivingClaim,
        PersistedChainLock,
        Recovery,
        RecordlessClaim,
        RecordlessPlaceholder,
        unknown(false, None),
        unknown(false, Some(102)),
        unknown(true, None),
        unknown(true, Some(102)),
        MalformedClaim,
        FailedRepair,
        DefeatedLock,
        Assets,
        AssetsRecovery,
        AssetsFailedRepair,
        Unconverged,
    ] {
        let _case = Case(format!("{case:?}"));
        assert_conflict_restart(case).await;
    }
}

async fn assert_conflict_restart(case: ConflictCase) {
    use dashcore::ephemerealdata::instant_lock::InstantLock;
    use ConflictCase::*;

    let record_funding = !matches!(case, HeightOnlyFunding);
    let surviving_claim = matches!(case, SurvivingClaim | DefeatedLock);
    let chainlocked = matches!(case, PersistedChainLock | DefeatedLock);
    let recordless_claim = matches!(case, RecordlessClaim | RecordlessPlaceholder);
    let unknown_claim = matches!(case, UnknownClaim { .. });
    let expect_released = !surviving_claim && !recordless_claim && !unknown_claim;
    let recovery = matches!(case, Recovery | AssetsRecovery);
    let assets = matches!(case, Assets | AssetsRecovery | AssetsFailedRepair);

    let (mut wallet, mut info, address) = wallet_with_receive_address();
    let funding = spend_of(
        &[outside(71)],
        vec![pay(100_000, &address), pay(20_000, &address)],
    );
    let coins: Vec<_> = (0..2)
        .map(|vout| {
            let outpoint = OutPoint::new(funding.txid(), vout);
            let output = funding.output[vout as usize].clone();
            Utxo::new(outpoint, output, address.clone(), 100, false)
        })
        .collect();
    let (shared, extra) = (coins[0].outpoint, coins[1].outpoint);
    let loser = spend_of(&[shared, extra], vec![pay(119_000, &address)]);
    let winner = spend_of(&[shared], vec![burn(99_000)]);
    let mut records = records_of(&mut info, &mut wallet, &funding, block(100)).await;
    if !record_funding {
        records.clear();
    }
    let (mut competing, mut other_spender) = (info.clone(), info.clone());
    records.extend(records_of(&mut info, &mut wallet, &loser, MEMPOOL).await);
    records.extend(records_of(&mut competing, &mut wallet, &winner, MEMPOOL).await);
    let mut surviving_txid = None;
    if surviving_claim {
        let extra_spender = spend_of(&[extra], vec![burn(19_000)]);
        surviving_txid = Some(extra_spender.txid());
        records.extend(records_of(&mut other_spender, &mut wallet, &extra_spender, MEMPOOL).await);
    }
    let lock = |txid, inputs| InstantLock {
        inputs,
        txid,
        ..Default::default()
    };
    let mut locks = BTreeMap::new();
    if chainlocked {
        let record = records.iter_mut().find(|r| r.txid == winner.txid());
        record.unwrap().context = block(101);
        locks.insert(loser.txid(), lock(loser.txid(), vec![shared, extra]));
    } else {
        locks.insert(winner.txid(), lock(winner.txid(), vec![shared]));
    }
    let (persister, _dir, path) = common::fresh_persister();
    let loser_coin = Utxo::new(
        OutPoint::new(loser.txid(), 0),
        loser.output[0].clone(),
        address,
        0,
        false,
    );
    let core = CoreChangeSet {
        records,
        new_utxos: coins.iter().cloned().chain([loser_coin]).collect(),
        spent_utxos: coins.clone(),
        instant_locks_for_non_final_records: locks,
        last_applied_chain_lock: chainlocked.then(|| chain_lock(101, 101)),
        ..Default::default()
    };
    persister
        .store(wallet.wallet_id, registration(&wallet, Some(core)))
        .unwrap();
    let wallet_key = wallet.wallet_id.as_slice();
    let extra_key: Vec<u8> = persister
        .lock_conn_for_test()
        .query_row(
            "SELECT outpoint FROM core_utxos WHERE wallet_id = ?1 AND value = 20000",
            [wallet_key],
            |row| row.get(0),
        )
        .unwrap();
    // `(spent, spent_in_txid, winner_mined_height, is_sweep_placeholder)` of the extra coin.
    let extra_row = |persister: &SqlitePersister| -> (bool, Option<Vec<u8>>, Option<u32>, bool) {
        persister
            .lock_conn_for_test()
            .query_row(
                "SELECT spent, spent_in_txid, winner_mined_height, is_sweep_placeholder \
             FROM core_utxos WHERE wallet_id = ?1 AND outpoint = ?2",
                rusqlite::params![wallet_key, &extra_key],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .expect("the extra coin's row survives repair and reopen")
    };
    let exec = |sql: &str, params: &[&dyn rusqlite::ToSql]| {
        persister.lock_conn_for_test().execute(sql, params).unwrap();
    };
    let loser_kept = || {
        let record = persister.get_core_tx_record(wallet.wallet_id, &loser.txid());
        record.unwrap().is_some()
    };
    // The store itself names the loser as claimant; only an unknown durable claim stays held.
    if let UnknownClaim { height, .. } = case {
        exec(
            "UPDATE core_utxos SET spent_in_txid = NULL, winner_mined_height = ?1 \
             WHERE wallet_id = ?2 AND outpoint = ?3",
            &[&height, &wallet_key, &extra_key],
        );
    }
    if matches!(case, MalformedClaim) {
        exec(
            "UPDATE core_utxos SET spent_in_txid = zeroblob(31) WHERE wallet_id = ?1 AND outpoint = ?2",
            &[&wallet_key, &extra_key],
        );
        assert!(matches!(
            typed_error(persister.load().unwrap_err()),
            WalletStorageError::BlobDecode { .. }
        ));
        assert!(loser_kept());
        return;
    }
    let missing_winner = Txid::from_byte_array([72; 32]);
    if recordless_claim {
        let missing = missing_winner.as_byte_array().as_slice();
        exec(
            "INSERT INTO core_transactions (wallet_id, txid, height, finalized) VALUES (?1, ?2, 101, 0)",
            &[&wallet_key, &missing],
        );
        exec(
            "UPDATE core_utxos SET spent_in_txid = ?1 WHERE wallet_id = ?2 AND value = 20000",
            &[&missing, &wallet_key],
        );
    }
    if matches!(
        case,
        RecordlessPlaceholder
            | UnknownClaim {
                placeholder: true,
                ..
            }
    ) {
        exec(
            "UPDATE core_utxos SET value = 0, script = X'', is_sweep_placeholder = 1 \
             WHERE wallet_id = ?1 AND outpoint = ?2",
            &[&wallet_key, &extra_key],
        );
    }
    let mut other_wallet_id = None;
    // The asset-lock rows a load leaves: the loser's non-consumed lifecycle row
    // goes with the Core repair, in the same transaction, so whatever that row
    // carries goes with it. Its consumed history and the other wallet's row of
    // the same outpoint stay.
    let expected_assets = |swept_removed: bool, other: [u8; 32]| {
        let row = |wallet, vout, status: &str| {
            (wallet, OutPoint::new(loser.txid(), vout), status.to_owned())
        };
        let mut rows = vec![
            row(wallet.wallet_id, 1, "consumed"),
            row(other, 0, "broadcast"),
        ];
        if !swept_removed {
            rows.push(row(wallet.wallet_id, 0, "broadcast"));
        }
        rows.sort();
        rows
    };
    if assets {
        use key_wallet::wallet::managed_wallet_info::asset_lock_builder::AssetLockFundingType;
        use platform_wallet::changeset::{AssetLockChangeSet, AssetLockEntry};
        use platform_wallet::wallet::asset_lock::tracked::AssetLockStatus;
        let entry = |vout, status| {
            let out_point = OutPoint::new(loser.txid(), vout);
            let entry = AssetLockEntry {
                out_point,
                transaction: loser.clone(),
                account_index: 0,
                funding_type: AssetLockFundingType::IdentityTopUp,
                identity_index: 0,
                amount_duffs: 1000,
                status,
                proof: None,
            };
            (out_point, entry)
        };
        let entries: BTreeMap<_, _> = [
            entry(0, AssetLockStatus::Broadcast),
            entry(1, AssetLockStatus::Consumed),
        ]
        .into();
        let locks = |asset_locks| {
            Some(AssetLockChangeSet {
                asset_locks,
                ..Default::default()
            })
        };
        let other =
            Wallet::new_random(Network::Testnet, WalletAccountCreationOptions::Default).unwrap();
        other_wallet_id = Some(other.wallet_id);
        let other_round = PlatformWalletChangeSet {
            asset_locks: locks([entry(0, AssetLockStatus::Broadcast)].into()),
            ..registration(&other, None)
        };
        persister.store(other.wallet_id, other_round).unwrap();
        let round = PlatformWalletChangeSet {
            asset_locks: locks(entries),
            ..Default::default()
        };
        persister.store(wallet.wallet_id, round).unwrap();
    }
    if matches!(case, FailedRepair | AssetsFailedRepair) {
        persister.lock_conn_for_test().execute_batch(
            "CREATE TRIGGER fail_replay_repair BEFORE UPDATE OF spent ON core_utxos WHEN NEW.spent = 0 BEGIN SELECT RAISE(ABORT, 'injected replay repair failure'); END;",
        ).unwrap();
        assert!(persister.load().is_err());
        if assets {
            assert_eq!(
                asset_lock_rows(&persister),
                expected_assets(false, other_wallet_id.unwrap()),
                "failed reconciliation must roll back asset lifecycle deletions"
            );
        }
        assert!(loser_kept());
        assert!(extra_row(&persister).0);
        exec("DROP TRIGGER fail_replay_repair", &[]);
    }
    if matches!(case, Unconverged) {
        persister.lock_conn_for_test().execute_batch("CREATE TRIGGER preserve_history BEFORE DELETE ON core_transactions BEGIN SELECT RAISE(IGNORE); END;").unwrap();
        let error = persister.load().unwrap_err();
        assert!(
            matches!(typed_error(error), WalletStorageError::WalletRehydrationFailed { wallet_id, .. } if wallet_id == wallet.wallet_id)
        );
        assert!(loser_kept());
        return;
    }
    drop(persister);
    let policy = if recovery {
        LoadPolicy::Recovery
    } else {
        LoadPolicy::Strict
    };
    for _ in 0..2 {
        let persister =
            SqlitePersister::open(SqlitePersisterConfig::new(&path).with_load_policy(policy))
                .unwrap();
        let mut state = persister.load().unwrap();
        if assets {
            assert!(
                state.wallets[&wallet.wallet_id]
                    .unused_asset_locks
                    .is_empty(),
                "swept asset locks must not be resumable"
            );
            assert_eq!(
                asset_lock_rows(&persister),
                expected_assets(!recovery, other_wallet_id.unwrap()),
                "only the loser's non-consumed row goes; Recovery rolls the removal back"
            );
        }
        if let Some(other_id) = other_wallet_id {
            assert!(
                state.wallets[&other_id]
                    .unused_asset_locks
                    .values()
                    .any(|locks| locks.contains_key(&OutPoint::new(loser.txid(), 0))),
                "the same asset lock in another wallet must survive"
            );
        }
        let info = &state.wallets[&wallet.wallet_id].wallet_info;
        let account = &info.accounts.standard_bip44_accounts[&0];
        assert!(!account.transactions().contains_key(&loser.txid()));
        if let Some(txid) = surviving_txid {
            assert!(account.transactions().contains_key(&txid));
            assert!(persister
                .get_core_tx_record(wallet.wallet_id, &txid)
                .unwrap()
                .is_some());
        }
        assert!(
            !account.utxos.contains_key(&shared),
            "winner still spends the shared input"
        );
        assert_eq!(
            account.utxos.contains_key(&extra),
            expect_released,
            "the extra input is free only when no surviving transaction claims it"
        );
        assert_eq!(
            info.balance.total(),
            if expect_released { 20_000 } else { 0 }
        );
        assert_eq!(
            persister
                .get_core_tx_record(wallet.wallet_id, &loser.txid())
                .unwrap()
                .is_some(),
            recovery
        );
        if recordless_claim {
            let claim = extra_row(&persister).1;
            assert_eq!(
                claim.as_deref(),
                Some(missing_winner.as_byte_array().as_slice())
            );
        }
        let selection = CoinSelector::new(SelectionStrategy::LargestFirst).select_coins(
            account.utxos.values(),
            5_000,
            FeeRate::default(),
            100,
        );
        if !expect_released {
            assert!(selection.is_err());
        } else {
            assert_eq!(selection.unwrap().selected[0].outpoint, extra);
        }
        if recordless_claim || unknown_claim {
            let loaded = state.wallets.get_mut(&wallet.wallet_id).unwrap();
            let (info, wallet) = (&mut loaded.wallet_info, &mut loaded.wallet);
            check(info, wallet, &funding, block(100)).await;
            assert!(
                !info.accounts.standard_bip44_accounts[&0]
                    .utxos
                    .contains_key(&extra),
                "recordless claim must survive funding redelivery"
            );
            assert_eq!(info.balance.total(), 0);
        }
        if let UnknownClaim {
            placeholder,
            height,
        } = case
        {
            assert_eq!(
                extra_row(&persister),
                (true, None, height, placeholder),
                "repair must retain nullable provenance through reload and funding redelivery"
            );
        }
    }
}

#[tokio::test]
async fn should_guard_recordless_spends_with_missing_funding_and_finality() {
    let flags = [false, true];
    let cases = ["absent", "height_only", "record"]
        .into_iter()
        .flat_map(|funding| {
            flags.into_iter().flat_map(move |spender_row| {
                flags.into_iter().flat_map(move |placeholder| {
                    flags.map(move |known_claimant| {
                        (funding, spender_row, placeholder, known_claimant)
                    })
                })
            })
        });
    for (funding_state, spender_row, placeholder, known_claimant) in cases {
        let _case = Case(format!(
            "funding={funding_state} spender_row={spender_row} \
             placeholder={placeholder} known_claimant={known_claimant}"
        ));
        let fixture = Fixture::build(block(100), block(200), funding_state == "record").await;
        {
            let conn = fixture.persister.lock_conn_for_test();
            let funding = fixture.funding.txid();
            let funding = [funding.as_byte_array().as_slice()];
            let spender = if spender_row {
                "UPDATE core_transactions SET record_blob = NULL WHERE txid != ?1"
            } else {
                "DELETE FROM core_transactions WHERE txid != ?1"
            };
            conn.execute(spender, funding).unwrap();
            if funding_state == "absent" {
                let sql = "DELETE FROM core_transactions WHERE txid = ?1";
                conn.execute(sql, funding).unwrap();
            }
            if !known_claimant {
                let sql = "UPDATE core_utxos SET spent_in_txid = NULL WHERE spent = 1";
                conn.execute(sql, []).unwrap();
            }
            if placeholder {
                conn.execute("UPDATE core_utxos SET value = 0, script = X'', is_sweep_placeholder = 1 WHERE spent = 1", []).unwrap();
            }
        }
        let (mut wallet, mut info) = fixture.load();
        info.apply_chain_lock(chain_lock(300, 30));
        check(&mut info, &mut wallet, &fixture.funding, block(100)).await;
        fixture.assert_spent_excluded(&info);
        // A later loser must not release a different durable claimant's input.
        let inputs = [fixture.spent, fixture.available];
        let mut loser = spend_of(&inputs, vec![fixture.funding.output[0].clone()]);
        loser.output[0].value = 119_000;
        check(&mut info, &mut wallet, &loser, MEMPOOL).await;
        let winner = spend_of(&[fixture.available], vec![burn(19_000)]);
        check(&mut info, &mut wallet, &winner, block(301)).await;
        check(&mut info, &mut wallet, &fixture.funding, block(100)).await;
        assert!(
            !info.accounts.standard_bip44_accounts[&0]
                .utxos
                .contains_key(&fixture.spent),
            "a later conflict cannot release a recordless durable claim"
        );
        assert_eq!(info.balance.total(), 0);
    }
}

/// A claim restored at load holds for a funding account registered afterwards:
/// the account that receives the spent coin is unknown to the store, is added
/// once the wallet is loaded, and then sees the funding transaction again.
#[tokio::test]
async fn should_guard_a_restored_claim_in_an_account_registered_after_load() {
    use key_wallet::account::{AccountType, StandardAccountType};
    use key_wallet::managed_account::ManagedCoreFundsAccount;

    let late_type = AccountType::Standard {
        index: 7,
        standard_account_type: StandardAccountType::BIP44Account,
    };
    let wallet =
        Wallet::new_random(Network::Testnet, WalletAccountCreationOptions::Default).unwrap();
    let late_account = {
        let mut with_late = wallet.clone();
        with_late.add_account(late_type, None).unwrap();
        with_late.accounts.standard_bip44_accounts[&7].clone()
    };
    let late_managed = || {
        let mut managed = ManagedCoreFundsAccount::from_account(&late_account);
        let address = managed
            .next_receive_address(Some(&late_account.account_xpub), true)
            .unwrap();
        (managed, address)
    };
    let (_, address) = late_managed();
    let funding = spend_of(
        &[outside(17)],
        vec![pay(100_000, &address), pay(20_000, &address)],
    );
    let [claimed, unclaimed] = [0, 1].map(|vout| OutPoint::new(funding.txid(), vout));

    // A recordless claim: the spent coin, with neither its funding nor its spender.
    let (persister, _dir) = store_then_reopen(
        &wallet,
        synced(CoreChangeSet {
            spent_utxos: vec![stored_utxo(&funding, 0, address)],
            ..Default::default()
        }),
    );
    assert_eq!(
        count(
            &persister,
            "SELECT COUNT(*) FROM core_utxos WHERE spent = 1"
        ),
        1,
        "the store holds the recordless claim"
    );

    let (mut wallet, mut info) = load_wallet(&persister, &wallet.wallet_id);
    assert!(!info.accounts.standard_bip44_accounts.contains_key(&7));

    let (managed, _) = late_managed();
    wallet
        .add_account(late_type, Some(late_account.account_xpub))
        .unwrap();
    info.accounts.insert_funds_bearing_account(managed).unwrap();

    check(&mut info, &mut wallet, &funding, block(100)).await;

    let coins = &info.accounts.standard_bip44_accounts[&7].utxos;
    assert!(
        coins.contains_key(&unclaimed),
        "the late account receives the funding transaction"
    );
    assert!(
        !coins.contains_key(&claimed),
        "a claim restored before the account existed still guards it"
    );
    assert_eq!(info.balance.total(), 20_000);
}

/// Replace the stored history with one `context` record spending `values` and receiving `received`.
async fn stage_replay_amounts(
    context: TransactionContext,
    values: &[u64],
    received: u64,
) -> Fixture {
    use key_wallet::managed_account::transaction_record::{InputDetail, OutputDetail, OutputRole};
    use platform_wallet_storage::sqlite::schema::blob;
    let fixture = Fixture::build(block(100), context, false).await;
    let conn = fixture.persister.lock_conn_for_test();
    let sql = "SELECT record_blob FROM core_transactions WHERE record_blob IS NOT NULL";
    let bytes: Vec<u8> = conn.query_row(sql, [], |row| row.get(0)).unwrap();
    let mut record: TransactionRecord = blob::decode(&bytes).unwrap();
    let address = record.input_details[0].address.clone();
    let parent = Txid::from_byte_array([90; 32]);
    let parents: Vec<_> = (0..values.len() as u32)
        .map(|vout| OutPoint::new(parent, vout))
        .collect();
    record.transaction = spend_of(&parents, vec![pay(received, &address)]);
    record.input_details = (0..)
        .zip(values)
        .map(|(index, value)| InputDetail {
            index,
            value: *value,
            address: address.clone(),
        })
        .collect();
    record.output_details = vec![OutputDetail {
        index: 0,
        value: received,
        address: Some(address),
        role: OutputRole::Received,
    }];
    record.net_amount =
        i64::try_from(i128::from(received) - values.iter().map(|v| i128::from(*v)).sum::<i128>())
            .unwrap();
    record.txid = record.transaction.txid();
    conn.execute("DELETE FROM core_transactions", []).unwrap();
    conn.execute(
        "INSERT INTO core_transactions (wallet_id, txid, height, finalized, record_blob) VALUES (?1, ?2, ?3, 0, ?4)",
        rusqlite::params![
            fixture.wallet_id.as_slice(),
            record.txid.as_byte_array().as_slice(),
            record.block_info().map(|block| block.height()),
            blob::encode(&record).unwrap()
        ],
    )
    .unwrap();
    drop(conn);
    fixture
}

#[tokio::test]
async fn should_reject_overflowing_replay_amounts_with_fitting_net() {
    let max = i64::MAX as u64;
    for (label, values, received) in [
        ("individually overflowing input", &[1_u64 << 63][..], 1),
        ("cumulatively overflowing inputs", &[max, 1], 1),
        ("overflowing output", &[max], 1_u64 << 63),
    ] {
        let _case = Case(label.into());
        let fixture = stage_replay_amounts(block(200), values, received).await;
        let error = fixture
            .persister
            .load()
            .expect_err("unsafe checker operands must return a typed error");
        assert!(
            matches!(
                typed_error(error),
                WalletStorageError::IntegerOverflow { .. }
            ),
            "{label}: unsafe checker arithmetic must be rejected before replay"
        );
    }
}

/// A mempool record whose input amounts are unsafe to sum is restored
/// without replay and without them, its inputs stay reserved, and the store
/// keeps the amounts, also when the record is stored again.
#[tokio::test]
async fn should_restore_mempool_record_with_unsafe_input_amounts_without_its_input_details() {
    let fixture = stage_replay_amounts(MEMPOOL, &[i64::MAX as u64, 1], 1).await;
    let txid = fixture.recorded_txid();

    let (_, info) = fixture.load();

    fixture.assert_spent_excluded(&info);
    let restored = info.accounts.standard_bip44_accounts[&0].transactions()[&txid].clone();
    assert!(
        restored.input_details.is_empty(),
        "history holds the record without the amounts that are unsafe to sum"
    );
    let round = core_round(CoreChangeSet {
        records: vec![restored],
        ..Default::default()
    });
    fixture.persister.store(fixture.wallet_id, round).unwrap();
    let persisted = fixture
        .persister
        .get_core_tx_record(fixture.wallet_id, &txid);
    let inputs: Vec<_> = persisted.unwrap().unwrap().input_details;
    assert_eq!(
        inputs.iter().map(|d| d.value).collect::<Vec<_>>(),
        vec![i64::MAX as u64, 1],
        "the store keeps the input amounts it already knew"
    );
}

#[tokio::test]
async fn should_load_wallet_holding_mempool_receipt_with_overflowing_foreign_output() {
    let (mut wallet, mut info, address) = wallet_with_receive_address();
    // Unminable, but nothing rejects it on the way in: only the owned output is accounted.
    let receipt = spend_of(
        &[outside(81)],
        vec![
            pay(5_000, &address),
            pay(i64::MAX as u64, &foreign_address(82)),
        ],
    );
    let records = records_of(&mut info, &mut wallet, &receipt, MEMPOOL).await;
    assert_eq!(records.len(), 1, "the live checker records the receipt");
    let new_utxos = info.accounts.standard_bip44_accounts[&0]
        .utxos
        .values()
        .cloned()
        .collect();
    let core = CoreChangeSet {
        records,
        new_utxos,
        ..Default::default()
    };
    let (persister, _dir) = store_then_reopen(&wallet, core);

    let mut state = persister
        .load()
        .expect("an unminable mempool receipt must not fail a strict load");

    let info = state.wallets.remove(&wallet.wallet_id).unwrap().wallet_info;
    assert_eq!(
        coin_set(&info),
        vec![OutPoint::new(receipt.txid(), 0)],
        "the owned receipt survives the restart"
    );
    assert_eq!(info.balance.total(), 5_000);
}

#[tokio::test]
async fn should_keep_reservation_of_forged_mempool_spend_with_overflowing_foreign_output() {
    let (mut wallet, mut info, address) = wallet_with_receive_address();
    let funding = spend_of(&[outside(83)], vec![pay(100_000, &address)]);
    let mut records = records_of(&mut info, &mut wallet, &funding, block(100)).await;
    let coin =
        info.accounts.standard_bip44_accounts[&0].utxos[&OutPoint::new(funding.txid(), 0)].clone();
    // Mempool signatures are not verified, so a peer can name a wallet coin as an input.
    let forged = spend_of(
        &[coin.outpoint],
        vec![
            pay(i64::MAX as u64, &foreign_address(84)),
            pay(1_000, &address),
        ],
    );
    records.extend(records_of(&mut info, &mut wallet, &forged, MEMPOOL).await);
    assert_eq!(
        records.len(),
        2,
        "the live checker records the forged spend"
    );
    let change = OutPoint::new(forged.txid(), 1);
    let change_coin = info.accounts.standard_bip44_accounts[&0].utxos[&change].clone();
    let core = CoreChangeSet {
        records,
        new_utxos: vec![coin.clone(), change_coin],
        spent_utxos: vec![coin],
        ..Default::default()
    };
    let (persister, _dir) = store_then_reopen(&wallet, core);

    let mut state = persister
        .load()
        .expect("an unminable mempool spend must not fail a strict load");

    let loaded = state.wallets.get_mut(&wallet.wallet_id).unwrap();
    let (info, wallet) = (&mut loaded.wallet_info, &mut loaded.wallet);
    check(info, wallet, &funding, block(100)).await;
    assert_eq!(
        coin_set(info),
        vec![change],
        "the reserved coin stays out of the spendable set through funding redelivery"
    );
    assert_eq!(info.balance.total(), 1_000);
}

/// How a forged, unminable mempool transaction makes its amounts unsafe.
#[derive(Clone, Copy, Debug)]
enum UnsafeLoser {
    /// It spends two confirmed wallet coins, a final competitor only one of
    /// them; its foreign output overflows the replay arithmetic, its change is small.
    OverflowingForeignOutput,
    /// It spends a foreign coin and a wallet coin, a final competitor only the
    /// foreign one; it pays the wallet twice, for a total past `i64`.
    UnrepresentableReceipt,
}

/// An [`UnsafeLoser`] as stored, optionally beside its final competitor.
struct StoredLoser {
    wallet: Wallet,
    funding: Transaction,
    /// The wallet coin only the forged transaction claims.
    extra: OutPoint,
    forged: Transaction,
    /// The confirmed competitor, mined at 101.
    winner: Transaction,
    /// The bip44 coins, and the balance, once `winner` has settled the conflict.
    settled_coins: Vec<OutPoint>,
    settled_balance: u64,
    persister: SqlitePersister,
    _dir: tempfile::TempDir,
}

impl StoredLoser {
    async fn new(shape: UnsafeLoser, with_winner: bool) -> Self {
        let (mut wallet, mut info, address) = wallet_with_receive_address();
        let (funding, extra, forged, winner, core, settled_coins, settled_balance) = match shape {
            UnsafeLoser::OverflowingForeignOutput => {
                let funding = spend_of(
                    &[outside(85)],
                    vec![pay(100_000, &address), pay(20_000, &address)],
                );
                let mut records = records_of(&mut info, &mut wallet, &funding, block(100)).await;
                let coins = &info.accounts.standard_bip44_accounts[&0].utxos;
                let shared = coins[&OutPoint::new(funding.txid(), 0)].clone();
                let extra = coins[&OutPoint::new(funding.txid(), 1)].clone();
                let winner = spend_of(&[shared.outpoint], vec![pay(99_000, &foreign_address(86))]);
                let (mut scratch, mut scratch_wallet) = (info.clone(), wallet.clone());
                let winner_records =
                    records_of(&mut scratch, &mut scratch_wallet, &winner, block(101)).await;
                let outputs = vec![
                    pay(i64::MAX as u64, &foreign_address(87)),
                    pay(1_000, &address),
                ];
                let forged = spend_of(&[shared.outpoint, extra.outpoint], outputs);
                records.extend(records_of(&mut info, &mut wallet, &forged, MEMPOOL).await);
                assert_eq!(
                    records.len(),
                    2,
                    "the live checker records the forged spend"
                );
                assert_eq!(winner_records.len(), 1);
                let change = info.accounts.standard_bip44_accounts[&0].utxos
                    [&OutPoint::new(forged.txid(), 1)]
                    .clone();
                if with_winner {
                    records.extend(winner_records);
                }
                let core = CoreChangeSet {
                    records,
                    new_utxos: vec![shared.clone(), extra.clone(), change],
                    spent_utxos: vec![shared, extra.clone()],
                    ..Default::default()
                };
                let settled = vec![extra.outpoint];
                (
                    funding,
                    extra.outpoint,
                    forged,
                    winner,
                    core,
                    settled,
                    20_000,
                )
            }
            UnsafeLoser::UnrepresentableReceipt => {
                let funding = spend_of(&[outside(96)], vec![pay(20_000, &address)]);
                let extra = OutPoint::new(funding.txid(), 0);
                let shared = outside(95);
                let value = UNREPRESENTABLE_HALF;
                let outputs = vec![pay(value, &address), pay(value, &address)];
                let forged = spend_of(&[shared, extra], outputs);
                let winner = spend_of(&[shared], vec![pay(1_000, &address)]);
                let (mut scratch, mut scratch_wallet) = (info.clone(), wallet.clone());
                let mut records =
                    records_of(&mut scratch, &mut scratch_wallet, &funding, block(100)).await;
                // Seen before its own input is funded: a debug checker cannot net this
                // record against a known input, a release one wraps and stores it too.
                records.extend(records_of(&mut info, &mut wallet, &forged, MEMPOOL).await);
                assert_eq!(
                    records.len(),
                    2,
                    "the live checker records the forged receipt"
                );
                let (mut scratch, mut scratch_wallet) = (info.clone(), wallet.clone());
                let running = check(&mut scratch, &mut scratch_wallet, &winner, block(101)).await;
                assert_eq!(
                    running.swept_transactions,
                    vec![forged.txid()],
                    "the running wallet removes the forged receipt"
                );
                let coin = stored_utxo(&funding, 0, address.clone());
                let mut new_utxos = vec![
                    coin.clone(),
                    stored_utxo(&forged, 0, address.clone()),
                    stored_utxo(&forged, 1, address.clone()),
                ];
                if with_winner {
                    records.extend(running.new_records);
                    new_utxos.push(stored_utxo(&winner, 0, address.clone()));
                }
                let core = CoreChangeSet {
                    records,
                    new_utxos,
                    spent_utxos: vec![coin],
                    ..Default::default()
                };
                let mut settled = vec![extra, OutPoint::new(winner.txid(), 0)];
                settled.sort();
                (funding, extra, forged, winner, core, settled, 21_000)
            }
        };
        let (persister, _dir) = store_then_reopen(&wallet, core);
        Self {
            wallet,
            funding,
            extra,
            forged,
            winner,
            settled_coins,
            settled_balance,
            persister,
            _dir,
        }
    }
}

const UNSAFE_LOSERS: [UnsafeLoser; 2] = [
    UnsafeLoser::OverflowingForeignOutput,
    UnsafeLoser::UnrepresentableReceipt,
];

#[tokio::test]
async fn should_settle_unsafe_mempool_loser_against_persisted_final_winner_on_restart() {
    for shape in UNSAFE_LOSERS {
        let _case = Case(format!("{shape:?}"));
        let stored = StoredLoser::new(shape, true).await;

        let (_, info) = load_wallet(&stored.persister, &stored.wallet.wallet_id);

        let forged = stored.forged.txid();
        assert!(
            !info.accounts.standard_bip44_accounts[&0]
                .transactions()
                .contains_key(&forged),
            "the forged transaction lost to the stored competitor at load"
        );
        let record = stored
            .persister
            .get_core_tx_record(stored.wallet.wallet_id, &forged);
        assert!(record.unwrap().is_none());
        assert_eq!(
            coin_set(&info),
            stored.settled_coins,
            "the forged outputs are gone and the coin only it claimed is released"
        );
        assert_eq!(info.balance.total(), stored.settled_balance);
    }
}

#[tokio::test]
async fn should_settle_unsafe_mempool_loser_against_final_winner_after_restart() {
    for shape in UNSAFE_LOSERS {
        let _case = Case(format!("{shape:?}"));
        let stored = StoredLoser::new(shape, false).await;
        let (mut wallet, mut info) = load_wallet(&stored.persister, &stored.wallet.wallet_id);

        let result = check(&mut info, &mut wallet, &stored.winner, block(101)).await;

        assert_eq!(
            result.swept_transactions,
            vec![stored.forged.txid()],
            "a final competitor arriving after restart still removes the forged transaction"
        );
        assert_eq!(
            result.released_outpoints,
            vec![stored.extra],
            "the wallet coin only the forged transaction claimed is released"
        );
        check(&mut info, &mut wallet, &stored.funding, block(100)).await;
        assert_eq!(
            coin_set(&info),
            stored.settled_coins,
            "the forged outputs are gone and funding redelivery returns the released coin"
        );
        assert_eq!(info.balance.total(), stored.settled_balance);
    }
}

#[tokio::test]
async fn should_report_balances_while_holding_unsafe_mempool_record_after_restart() {
    let stored = StoredLoser::new(UnsafeLoser::OverflowingForeignOutput, false).await;
    let change = OutPoint::new(stored.forged.txid(), 1);

    let (mut wallet, mut info) = load_wallet(&stored.persister, &stored.wallet.wallet_id);

    assert!(
        info.accounts.standard_bip44_accounts[&0]
            .transactions()
            .contains_key(&stored.forged.txid()),
        "the unsettled forged spend stays in history"
    );
    // Recompute from scratch, then through the late-funding path that revisits stored records.
    info.update_balance();
    assert_eq!(coin_set(&info), vec![change]);
    assert_eq!(info.balance.total(), 1_000);
    check(&mut info, &mut wallet, &stored.funding, block(100)).await;
    info.update_balance();
    assert_eq!(
        coin_set(&info),
        vec![change],
        "both claimed coins stay reserved while no competitor is final"
    );
    assert_eq!(info.balance.total(), 1_000);
    assert_eq!(info.balance.confirmed(), 0);
}

#[tokio::test]
async fn should_keep_stored_owned_outputs_when_a_record_restored_without_them_is_stored_again() {
    let stored = StoredLoser::new(UnsafeLoser::UnrepresentableReceipt, false).await;
    let txid = stored.forged.txid();
    let (_, info) = load_wallet(&stored.persister, &stored.wallet.wallet_id);
    let restored = info.accounts.standard_bip44_accounts[&0].transactions()[&txid].clone();
    assert!(
        restored.output_details.is_empty(),
        "history holds the record without the amounts that are unsafe to sum"
    );

    let round = core_round(CoreChangeSet {
        records: vec![restored],
        ..Default::default()
    });
    stored
        .persister
        .store(stored.wallet.wallet_id, round)
        .unwrap();

    let persisted = stored
        .persister
        .get_core_tx_record(stored.wallet.wallet_id, &txid);
    let owned: Vec<_> = persisted.unwrap().unwrap().output_details;
    assert_eq!(
        owned.iter().map(|d| d.value).collect::<Vec<_>>(),
        vec![UNREPRESENTABLE_HALF, UNREPRESENTABLE_HALF],
        "the store keeps the owned outputs it already knew"
    );
}

#[tokio::test]
async fn should_recompute_unsafe_mempool_record_when_its_funding_arrives_after_restart() {
    let (mut wallet, mut info, address) = wallet_with_receive_address();
    let funding = spend_of(&[outside(88)], vec![pay(100_000, &address)]);
    let coin = OutPoint::new(funding.txid(), 0);
    // Seen before its funding, so stored without the input's amount.
    let forged = spend_of(
        &[coin],
        vec![
            pay(i64::MAX as u64, &foreign_address(89)),
            pay(1_000, &address),
        ],
    );
    let rival = spend_of(&[coin], vec![pay(500, &address)]);
    let (mut scratch, mut scratch_wallet) = (info.clone(), wallet.clone());
    let mut records = records_of(&mut scratch, &mut scratch_wallet, &forged, MEMPOOL).await;
    records.extend(records_of(&mut info, &mut wallet, &rival, MEMPOOL).await);
    assert_eq!(records.len(), 2);
    let new_utxos = vec![
        stored_utxo(&forged, 1, address.clone()),
        stored_utxo(&rival, 0, address.clone()),
    ];
    let core = CoreChangeSet {
        records,
        new_utxos,
        ..Default::default()
    };
    let (persister, _dir) = store_then_reopen(&wallet, core);
    let (mut wallet, mut info) = load_wallet(&persister, &wallet.wallet_id);

    check(&mut info, &mut wallet, &funding, block(100)).await;
    info.update_balance();

    let record = &info.accounts.standard_bip44_accounts[&0].transactions()[&forged.txid()];
    assert_eq!(
        record.input_details.len(),
        1,
        "the late funding is attributed to the record restored as stored"
    );
    assert_eq!(record.net_amount, 1_000 - 100_000);
    assert_eq!(info.balance.total(), 1_500);
}

/// The loser spends a shared and an extra coin, a second unconfirmed spend
/// takes the extra coin and a third one, and the confirmed winner spends only
/// the shared coin. A live sweep persists the sweep the running wallet
/// reported; otherwise the conflict is stored unsettled and repaired at load.
/// Either way the extra coin is released once its surviving spender is removed.
#[tokio::test]
async fn should_release_extra_input_when_its_surviving_spender_is_removed_after_restart() {
    for live_sweep in [true, false] {
        let _case = Case(format!("live_sweep={live_sweep}"));
        let (mut wallet, mut info, address) = wallet_with_receive_address();
        let foreign = |marker, value| pay(value, &foreign_address(marker));
        let funding = spend_of(
            &[outside(90)],
            [100_000, 20_000, 5_000].map(|v| pay(v, &address)).into(),
        );
        let coin = |vout| OutPoint::new(funding.txid(), vout);
        let loser = spend_of(&[coin(0), coin(1)], vec![foreign(91, 119_000)]);
        let survivor = spend_of(&[coin(1), coin(2)], vec![foreign(92, 24_000)]);
        let winner = spend_of(&[coin(0)], vec![pay(99_000, &address)]);
        let final_spend = spend_of(&[coin(2)], vec![foreign(93, 4_000)]);

        let mut records = Vec::new();
        for (transaction, context) in [
            (&funding, block(100)),
            (&loser, MEMPOOL),
            (&survivor, MEMPOOL),
        ] {
            records.extend(records_of(&mut info, &mut wallet, transaction, context).await);
        }
        assert_eq!(
            records.len(),
            3,
            "one wallet records both unconfirmed spenders of the extra coin"
        );
        let settled = check(&mut info, &mut wallet, &winner, block(101)).await;
        assert_eq!(settled.swept_transactions, vec![loser.txid()]);
        assert!(
            settled.released_outpoints.is_empty(),
            "the survivor still spends the extra coin"
        );
        let (mut scratch, mut scratch_wallet) = (info.clone(), wallet.clone());
        let running = check(&mut scratch, &mut scratch_wallet, &final_spend, block(102)).await;
        assert_eq!(
            running.released_outpoints,
            vec![coin(1)],
            "the running wallet frees the extra coin with its surviving spender"
        );
        let coins: Vec<_> = (0..3)
            .map(|vout| stored_utxo(&funding, vout, address.clone()))
            .collect();
        let mut seen = CoreChangeSet {
            records,
            new_utxos: coins.clone(),
            spent_utxos: coins,
            ..Default::default()
        };
        let settlement = CoreChangeSet {
            records: settled.new_records,
            new_utxos: vec![stored_utxo(&winner, 0, address.clone())],
            sweeps: vec![sweep(
                settled.swept_transactions,
                &winner,
                settled.released_outpoints,
            )],
            ..Default::default()
        };
        assert_eq!(settlement.records.len(), 1);
        let (persister, _dir) = if live_sweep {
            let (persister, dir) = store_then_reopen(&wallet, seen);
            let round = core_round(settlement);
            persister.store(wallet.wallet_id, round).unwrap();
            (persister, dir)
        } else {
            seen.records.extend(settlement.records);
            seen.new_utxos.extend(settlement.new_utxos);
            store_then_reopen(&wallet, seen)
        };

        let (mut wallet, mut info) = load_wallet(&persister, &wallet.wallet_id);
        let result = check(&mut info, &mut wallet, &final_spend, block(102)).await;

        assert_eq!(result.swept_transactions, vec![survivor.txid()]);
        assert_eq!(
            result.released_outpoints,
            vec![coin(1)],
            "the extra coin is free once its surviving spender is gone"
        );
        check(&mut info, &mut wallet, &funding, block(100)).await;
        assert!(
            info.accounts.standard_bip44_accounts[&0]
                .utxos
                .contains_key(&coin(1)),
            "funding redelivery returns the released coin"
        );
    }
}

/// How a mempool receipt paying two funding accounts reaches the store.
#[derive(Clone, Copy, Debug, Default)]
struct TwoAccountShape {
    /// Value of each of the two owned outputs.
    value: u64,
    /// Adds a foreign output no chain could hold.
    foreign_output: bool,
    /// The wallet-level record names the first account, not the second.
    recorded_under_first: bool,
    /// Persists no address pool rows, so load parks both coins in the first account.
    unrouted: bool,
    /// Adds a mempool child spending the first account's coin, with its change there.
    /// It is recorded only in the account whose coin it spends, which is not
    /// the one the stored receipt names.
    child: bool,
    /// The receipt also spends a coin of the first account funded only after
    /// the restart, and so does a plain mempool rival that marks it spent.
    unfunded_input: bool,
}

struct TwoAccountReceipt {
    wallet: Wallet,
    /// An address of the first (BIP44) account.
    first: Address,
    funding: Transaction,
    receipt: Transaction,
    child: Option<Transaction>,
    /// A confirmed transaction spending the receipt's foreign input.
    winner: Transaction,
    persister: SqlitePersister,
    _dir: tempfile::TempDir,
}

impl TwoAccountReceipt {
    async fn stored(shape: TwoAccountShape) -> Self {
        let (mut wallet, mut info, first, second) = two_account_wallet();
        let funding = spend_of(&[outside(96)], vec![pay(20_000, &first)]);
        let shared = outside(97);
        let mut inputs = vec![shared];
        if shape.unfunded_input {
            // Seen before this input is funded, as a debug checker requires.
            inputs.push(OutPoint::new(funding.txid(), 0));
        }
        let mut outputs = vec![pay(shape.value, &first), pay(shape.value, &second)];
        if shape.foreign_output {
            outputs.push(pay(i64::MAX as u64, &foreign_address(98)));
        }
        let receipt = spend_of(&inputs, outputs);
        let winner = spend_of(&[shared], vec![pay(1_000, &first)]);
        let mut records = records_of(&mut info, &mut wallet, &receipt, MEMPOOL).await;
        assert_eq!(records.len(), 2, "the checker records it in both accounts");
        // One wallet-level record per transaction, as a sync folds the
        // per-account slices: it names one account and details both.
        if !shape.recorded_under_first {
            records.reverse();
        }
        let other = records.pop().unwrap();
        records[0].output_details.extend(other.output_details);
        records[0].output_details.sort_by_key(|detail| detail.index);
        let mut new_utxos = vec![
            stored_utxo(&receipt, 0, first.clone()),
            stored_utxo(&receipt, 1, second.clone()),
        ];
        let mut spent_utxos = Vec::new();
        let child = shape.child.then(|| {
            spend_of(
                &[OutPoint::new(receipt.txid(), 0)],
                vec![pay(1_500, &first)],
            )
        });
        if let Some(child) = &child {
            records.extend(records_of(&mut info, &mut wallet, child, MEMPOOL).await);
            spent_utxos.push(new_utxos[0].clone());
            new_utxos.push(stored_utxo(child, 0, first.clone()));
        }
        if shape.unfunded_input {
            let rival = spend_of(&[OutPoint::new(funding.txid(), 0)], vec![pay(500, &first)]);
            records.extend(records_of(&mut info, &mut wallet, &rival, MEMPOOL).await);
            new_utxos.push(stored_utxo(&rival, 0, first.clone()));
        }
        let (mut running, mut running_wallet) = (info.clone(), wallet.clone());
        check(&mut running, &mut running_wallet, &winner, block(101)).await;
        assert_eq!(
            traces(&running, &[&receipt, child.as_ref().unwrap_or(&receipt)]),
            Vec::<String>::new(),
            "the running wallet settles every account"
        );
        let core = CoreChangeSet {
            records,
            new_utxos,
            spent_utxos,
            ..Default::default()
        };
        let (persister, _dir) = store_then_reopen(&wallet, core);
        if !shape.unrouted {
            let rows = [("standard_bip44", &first), ("standard_bip32", &second)];
            insert_pool_rows(&persister, &wallet.wallet_id, &rows);
        }
        Self {
            wallet,
            first,
            funding,
            receipt,
            child,
            winner,
            persister,
            _dir,
        }
    }
}

/// A final competitor arriving after the restart must leave no coin of the
/// receipt in any account, in the running wallet and after the next load.
#[tokio::test]
async fn should_settle_two_account_mempool_receipt_against_final_winner_after_restart() {
    let shape = |value| TwoAccountShape {
        value,
        ..Default::default()
    };
    let unrepresentable = shape(UNREPRESENTABLE_HALF);
    for shape in [
        shape(5_000),
        unrepresentable,
        TwoAccountShape {
            recorded_under_first: true,
            ..unrepresentable
        },
        TwoAccountShape {
            foreign_output: true,
            ..shape(7_000)
        },
        TwoAccountShape {
            child: true,
            ..unrepresentable
        },
        TwoAccountShape {
            unrouted: true,
            ..unrepresentable
        },
    ] {
        let _case = Case(format!("{shape:?}"));
        let stored = TwoAccountReceipt::stored(shape).await;
        let losers = [
            &stored.receipt,
            stored.child.as_ref().unwrap_or(&stored.receipt),
        ];
        let (mut wallet, mut info) = load_wallet(&stored.persister, &stored.wallet.wallet_id);

        let result = check(&mut info, &mut wallet, &stored.winner, block(101)).await;

        assert!(result.swept_transactions.contains(&stored.receipt.txid()));
        assert_eq!(
            traces(&info, &losers),
            Vec::<String>::new(),
            "no account of the running wallet keeps a coin of the removed receipt"
        );
        assert_eq!(info.balance.total(), 1_000);

        let round = core_round(CoreChangeSet {
            records: result.new_records,
            new_utxos: vec![stored_utxo(&stored.winner, 0, stored.first.clone())],
            sweeps: vec![sweep(
                result.swept_transactions,
                &stored.winner,
                result.released_outpoints,
            )],
            ..Default::default()
        });
        stored
            .persister
            .store(stored.wallet.wallet_id, round)
            .unwrap();
        let (_, reloaded) = load_wallet(&stored.persister, &stored.wallet.wallet_id);
        assert_eq!(
            traces(&reloaded, &losers),
            Vec::<String>::new(),
            "the next load holds no coin of the removed receipt either"
        );
        assert_eq!(reloaded.balance.total(), 1_000);
    }
}

/// Each account holds the receipt without the amounts that are unsafe to
/// sum, so reading balances stays in range. A late funding rebuilds the owning
/// account's complete slice (rust-dashcore#1106): its net is its own coin less
/// the funded input, which stays in range, and the balance is unchanged.
#[tokio::test]
async fn should_recompute_unrepresentable_two_account_receipt_when_its_funding_arrives_after_restart(
) {
    let stored = TwoAccountReceipt::stored(TwoAccountShape {
        value: UNREPRESENTABLE_HALF,
        unfunded_input: true,
        ..Default::default()
    })
    .await;
    let txid = stored.receipt.txid();
    let (mut wallet, mut info) = load_wallet(&stored.persister, &stored.wallet.wallet_id);
    let copies = |info: &ManagedWalletInfo| {
        let accounts = &info.accounts;
        (
            accounts.standard_bip44_accounts[&0].transactions()[&txid].clone(),
            accounts.standard_bip32_accounts[&0].transactions()[&txid].clone(),
        )
    };
    let (first, second) = copies(&info);
    assert!(
        first.output_details.is_empty() && second.output_details.is_empty(),
        "each account holds the receipt without the amounts unsafe to sum"
    );
    assert_eq!(
        info.balance.total(),
        2 * UNREPRESENTABLE_HALF + 500,
        "both coins are credited"
    );

    check(&mut info, &mut wallet, &stored.funding, block(100)).await;
    info.update_balance();

    let (first, second) = copies(&info);
    assert_eq!(
        (first.input_details.len(), first.net_amount),
        (1, UNREPRESENTABLE_HALF as i64 - 20_000),
        "the late funding is attributed in the account that owns it"
    );
    assert!(second.input_details.is_empty());
    assert_eq!(info.balance.total(), 2 * UNREPRESENTABLE_HALF + 500);
}

/// A stored mempool record spending `values` (safe to sum, then not), beaten
/// after the restart by a final transaction the wallet has no other interest
/// in: no owned output, and no input the wallet can attribute. Whatever the
/// checker reports is persisted as a sync would, and the store is loaded again.
#[tokio::test]
async fn should_settle_mempool_record_against_recordless_final_competitor_after_restart() {
    for values in [[5_000, 1], [i64::MAX as u64, 1]] {
        let _case = Case(format!("inputs={values:?}"));
        let fixture = stage_replay_amounts(MEMPOOL, &values, 1).await;
        let txid = fixture.recorded_txid();
        let loser = fixture
            .persister
            .get_core_tx_record(fixture.wallet_id, &txid)
            .unwrap()
            .unwrap();
        let address = loser.output_details[0].address.clone().unwrap();
        // The loser's own output, as the round that stored the record wrote it.
        let round = core_round(CoreChangeSet {
            new_utxos: vec![stored_utxo(&loser.transaction, 0, address)],
            ..Default::default()
        });
        fixture.persister.store(fixture.wallet_id, round).unwrap();
        let (mut wallet, mut info) = fixture.load();
        assert_eq!(traces(&info, &[&loser.transaction]).len(), 1);

        let inputs = &loser.transaction.input;
        let winner = spend_of(
            &[inputs[0].previous_output],
            vec![pay(900, &foreign_address(77))],
        );
        let result = check(&mut info, &mut wallet, &winner, block(101)).await;

        assert!(
            !result.is_relevant && result.new_records.is_empty(),
            "the competitor leaves no record of its own"
        );
        assert_eq!(result.swept_transactions, vec![txid]);
        assert_eq!(
            result.released_outpoints,
            vec![inputs[1].previous_output],
            "the input only the loser claimed is released"
        );
        assert_eq!(traces(&info, &[&loser.transaction]), Vec::<String>::new());
        let round = core_round(CoreChangeSet {
            sweeps: vec![sweep(
                result.swept_transactions,
                &winner,
                result.released_outpoints,
            )],
            ..Default::default()
        });
        fixture.persister.store(fixture.wallet_id, round).unwrap();

        let (_, reloaded) = fixture.load();
        assert_eq!(
            traces(&reloaded, &[&loser.transaction]),
            Vec::<String>::new(),
            "the next load holds neither the record nor its coin"
        );
        let rows: i64 = fixture
            .persister
            .lock_conn_for_test()
            .query_row(
                "SELECT count(*) FROM core_transactions WHERE txid = ?1",
                [txid.as_byte_array().as_slice()],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(rows, 0, "and the store holds its record no longer");
    }
}

/// How a mempool spend of two forged receipts and two real coins reaches the store.
#[derive(Clone, Copy, Debug, Default)]
struct ForgedInputShape {
    /// Value of each forged receipt: two of `UNREPRESENTABLE_HALF` cannot be
    /// summed as `i64`, so the spend's input amounts are unsafe.
    forged: u64,
    /// The spend also pays the second funding account.
    two_accounts: bool,
    /// The real coins are funded only after the restart, and a plain mempool
    /// rival spends one of them too, which marks it spent.
    late_funding: bool,
}

struct ForgedInputSpend {
    wallet: Wallet,
    /// Pays the wallet 20_000 (`shared`) and 30_000 (`extra`), confirmed.
    funding: Transaction,
    receipts: [Transaction; 2],
    /// Spends `shared`, `extra` and both receipts.
    spend: Transaction,
    /// A confirmed transaction spending `shared` and paying outside the wallet.
    winner: Transaction,
    persister: SqlitePersister,
    _dir: tempfile::TempDir,
}

impl ForgedInputSpend {
    async fn stored(shape: ForgedInputShape) -> Self {
        let (mut wallet, mut info, first, second) = two_account_wallet();
        let funding = spend_of(
            &[outside(73)],
            vec![pay(20_000, &first), pay(30_000, &first)],
        );
        let [shared, extra] = [0, 1].map(|vout| OutPoint::new(funding.txid(), vout));
        let receipts =
            [71, 72].map(|byte| spend_of(&[outside(byte)], vec![pay(shape.forged, &first)]));
        let mut outputs = vec![pay(1_000, &first)];
        if shape.two_accounts {
            outputs.push(pay(1_500, &second));
        }
        let receipt_coins = receipts
            .each_ref()
            .map(|receipt| OutPoint::new(receipt.txid(), 0));
        let spend = spend_of(
            &[shared, extra, receipt_coins[0], receipt_coins[1]],
            outputs,
        );
        let winner = spend_of(&[shared], vec![pay(49_000, &foreign_address(74))]);

        let mut records = Vec::new();
        let mut new_utxos = Vec::new();
        let mut spent_utxos = Vec::new();
        if !shape.late_funding {
            records.extend(records_of(&mut info, &mut wallet, &funding, block(100)).await);
            new_utxos.extend([0, 1].map(|vout| stored_utxo(&funding, vout, first.clone())));
            spent_utxos.extend(new_utxos.clone());
        }
        // Each receipt is seen by a wallet that does not hold the spend: a
        // debug checker cannot attribute both to one record.
        for receipt in &receipts {
            let (mut scratch, mut scratch_wallet) = (info.clone(), wallet.clone());
            records.extend(records_of(&mut scratch, &mut scratch_wallet, receipt, MEMPOOL).await);
            let coin = stored_utxo(receipt, 0, first.clone());
            new_utxos.push(coin.clone());
            spent_utxos.push(coin);
        }
        let mut slices = records_of(&mut info, &mut wallet, &spend, MEMPOOL).await;
        assert_eq!(slices.len(), 1 + usize::from(shape.two_accounts));
        // One wallet-level record, as a sync folds the per-account slices.
        let mut folded = slices.remove(0);
        for slice in slices {
            folded.output_details.extend(slice.output_details);
        }
        records.push(folded);
        new_utxos.push(stored_utxo(&spend, 0, first.clone()));
        if shape.two_accounts {
            new_utxos.push(stored_utxo(&spend, 1, second.clone()));
        }
        if shape.late_funding {
            let rival = spend_of(&[shared], vec![pay(500, &first)]);
            records.extend(records_of(&mut info, &mut wallet, &rival, MEMPOOL).await);
            new_utxos.push(stored_utxo(&rival, 0, first.clone()));
        }
        let core = CoreChangeSet {
            records,
            new_utxos,
            spent_utxos,
            ..Default::default()
        };
        let (persister, _dir) = store_then_reopen(&wallet, core);
        if shape.two_accounts {
            let rows = [("standard_bip44", &first), ("standard_bip32", &second)];
            insert_pool_rows(&persister, &wallet.wallet_id, &rows);
        }
        Self {
            wallet,
            funding,
            receipts,
            spend,
            winner,
            persister,
            _dir,
        }
    }
}

/// A final competitor that spends one of the wallet's own coins and pays
/// outside it. Beside an ordinary spend it leaves no record, since that spend
/// already took the coin; beside a restored one the wallet still attributes
/// the coin and records it. Either way it must remove the stored spend, in the
/// running wallet and for the next load, and release what only that spend claimed.
/// The 7_000 receipts are safe to sum: the control, an ordinary record.
#[tokio::test]
async fn should_settle_mempool_spend_against_final_outside_spend_of_its_coin_after_restart() {
    let shape = |forged, two_accounts| ForgedInputShape {
        forged,
        two_accounts,
        late_funding: false,
    };
    for shape in [
        shape(7_000, false),
        shape(UNREPRESENTABLE_HALF, false),
        shape(UNREPRESENTABLE_HALF, true),
    ] {
        let _case = Case(format!("{shape:?}"));
        let stored = ForgedInputSpend::stored(shape).await;
        let outputs = 1_000 + if shape.two_accounts { 1_500 } else { 0 };
        let (mut wallet, mut info) = load_wallet(&stored.persister, &stored.wallet.wallet_id);
        assert_eq!(
            info.balance.total(),
            outputs,
            "only the spend's own outputs are spendable after the restart"
        );

        let result = check(&mut info, &mut wallet, &stored.winner, block(101)).await;

        assert_eq!(result.swept_transactions, vec![stored.spend.txid()]);
        assert_eq!(
            traces(&info, &[&stored.spend]),
            Vec::<String>::new(),
            "no account of the running wallet keeps a coin of the removed spend"
        );
        let round = core_round(CoreChangeSet {
            records: result.new_records,
            sweeps: vec![sweep(
                result.swept_transactions,
                &stored.winner,
                result.released_outpoints,
            )],
            ..Default::default()
        });
        stored
            .persister
            .store(stored.wallet.wallet_id, round)
            .unwrap();

        let (_, reloaded) = load_wallet(&stored.persister, &stored.wallet.wallet_id);
        assert_eq!(traces(&reloaded, &[&stored.spend]), Vec::<String>::new());
        let coins: Vec<OutPoint> = reloaded
            .accounts
            .all_funding_accounts()
            .into_iter()
            .flat_map(|account| account.utxos.keys().copied())
            .collect();
        let mut released = vec![
            OutPoint::new(stored.funding.txid(), 1),
            OutPoint::new(stored.receipts[0].txid(), 0),
            OutPoint::new(stored.receipts[1].txid(), 0),
        ];
        released.sort();
        assert_eq!(
            coins, released,
            "the next load returns what only the spend claimed, not the coin the competitor spent"
        );
        assert_eq!(reloaded.balance.total(), 30_000 + 2 * shape.forged);
    }
}

/// The real coins are funded only after the restart. A restored record holds
/// no input amounts and marks nothing spent by itself: the coin the rival
/// also spends is attributed to it, and that one amount is all its net sums;
/// the coin only it spends is credited when its funding arrives. Replayed as
/// an ordinary record (the 7_000 control), the same spend also knows its
/// receipts and keeps both real coins reserved.
#[tokio::test]
async fn should_attribute_late_funding_to_a_mempool_spend_restored_or_replayed() {
    // (forged receipt value, attributed inputs, net, wallet total once funded)
    for (forged, expected) in [
        (UNREPRESENTABLE_HALF, (1, 1_000 - 20_000, 1_500 + 30_000)),
        (7_000, (4, 1_000 - 50_000 - 14_000, 1_500)),
    ] {
        let _case = Case(format!("forged={forged}"));
        let stored = ForgedInputSpend::stored(ForgedInputShape {
            forged,
            late_funding: true,
            ..Default::default()
        })
        .await;
        let (mut wallet, mut info) = load_wallet(&stored.persister, &stored.wallet.wallet_id);
        assert_eq!(info.balance.total(), 1_500);

        check(&mut info, &mut wallet, &stored.funding, block(100)).await;
        info.update_balance();

        let account = &info.accounts.standard_bip44_accounts[&0];
        let record = &account.transactions()[&stored.spend.txid()];
        assert_eq!(
            (
                record.input_details.len(),
                record.net_amount,
                info.balance.total()
            ),
            expected
        );
    }
}

/// How the store reads once a load has settled a conflict that removes a
/// DashPay payment's transaction.
#[derive(Debug, PartialEq)]
struct SettledPayment {
    /// Stored records of the payment's transaction.
    loser_rows: i64,
    /// The payment's status in the state the settling load returned.
    returned: platform_wallet::wallet::identity::PaymentStatus,
    /// Overlay rows written for the payment: one per verdict that reached the store.
    stored_verdicts: i64,
    /// The payment's status as a later strict load reads it.
    reopened: platform_wallet::wallet::identity::PaymentStatus,
    /// An unrelated sent payment of the same identity, on that later load.
    unrelated: platform_wallet::wallet::identity::PaymentStatus,
}

/// A DashPay payment whose transaction lost a double-spend. The winner's
/// record reached the store; the round carrying the sweep, and with it the
/// payment's verdict, did not. A load under `policy` settles the conflict.
async fn settle_payment_at_load(
    mut payment: platform_wallet::wallet::identity::PaymentEntry,
    status: platform_wallet::wallet::identity::PaymentStatus,
    policy: LoadPolicy,
) -> SettledPayment {
    use dpp::prelude::Identifier;
    use platform_wallet::changeset::{IdentityChangeSet, IdentityEntry};
    use platform_wallet::wallet::identity::{IdentityStatus, PaymentEntry};

    let (mut wallet, mut info, address) = wallet_with_receive_address();
    let funding = spend_of(&[outside(91)], vec![pay(100_000, &address)]);
    let mut records = records_of(&mut info, &mut wallet, &funding, block(100)).await;
    let coin = OutPoint::new(funding.txid(), 0);
    let loser = spend_of(
        &[coin],
        vec![pay(59_000, &foreign_address(92)), pay(40_000, &address)],
    );
    records.extend(records_of(&mut info, &mut wallet, &loser, MEMPOOL).await);
    let winner = spend_of(&[coin], vec![pay(99_000, &address)]);
    let won = check(&mut info, &mut wallet, &winner, block(101)).await;
    assert_eq!(won.swept_transactions, vec![loser.txid()]);
    records.extend(won.new_records);

    let funded = stored_utxo(&funding, 0, address.clone());
    let (persister, dir) = store_then_reopen(
        &wallet,
        CoreChangeSet {
            records,
            new_utxos: vec![
                funded.clone(),
                stored_utxo(&loser, 1, address.clone()),
                stored_utxo(&winner, 0, address.clone()),
            ],
            spent_utxos: vec![funded],
            ..Default::default()
        },
    );
    let owner = Identifier::from([0xA7; 32]);
    let contact = Identifier::from([0xB7; 32]);
    let (txid, unrelated) = (loser.txid().to_string(), "a".repeat(64));
    payment.status = status;
    let payments = [
        (txid.clone(), payment),
        (
            unrelated.clone(),
            PaymentEntry::new_sent(contact, 1_000, None),
        ),
    ];
    let entry = IdentityEntry {
        id: owner,
        balance: 0,
        revision: 0,
        identity_index: Some(0),
        last_updated_balance_block_time: None,
        last_synced_keys_block_time: None,
        dpns_names: Vec::new(),
        contested_dpns_names: Vec::new(),
        status: IdentityStatus::Unknown,
        wallet_id: Some(wallet.wallet_id),
        dashpay_profile: None,
        dashpay_payments: payments.into(),
        contact_profiles: Default::default(),
        ignored_senders: Default::default(),
    };
    let mut identities = IdentityChangeSet::default();
    identities.identities.insert(owner, entry);
    let round = PlatformWalletChangeSet {
        identities: Some(identities),
        ..Default::default()
    };
    persister.store(wallet.wallet_id, round).unwrap();
    drop(persister);

    let open = |policy| {
        let path = dir.path().join("wallet.db");
        SqlitePersister::open(SqlitePersisterConfig::new(path).with_load_policy(policy)).unwrap()
    };
    let status_of = |persister: &SqlitePersister, txid: &str| {
        let state = persister.load().unwrap();
        let identities = &state.wallets[&wallet.wallet_id].identity_manager;
        let managed = &identities.wallet_identities[&wallet.wallet_id][&0];
        managed.dashpay().payments[txid].status
    };

    let settling = open(policy);
    let returned = status_of(&settling, &txid);
    let loser_rows = count(
        &settling,
        "SELECT count(*) FROM core_transactions WHERE height IS NULL",
    );
    let stored_verdicts = count(&settling, "SELECT count(*) FROM dashpay_payments_overlay");
    drop(settling);
    let reopened = open(LoadPolicy::Strict);
    SettledPayment {
        loser_rows,
        returned,
        stored_verdicts,
        reopened: status_of(&reopened, &txid),
        unrelated: status_of(&reopened, &unrelated),
    }
}

/// A sent payment whose transaction a load settles away reads `Failed` on that
/// load and the next, an InstantSend-locked one (`Confirmed` until a final
/// competitor removes it) too. A received payment's status is settled when it
/// is recorded, so a sweep of its transaction does not move it. A recovery load
/// returns the settled state and writes none of it: the verdict rolls back
/// with the record removal it follows from, and the next strict load settles again.
#[tokio::test]
async fn should_settle_the_dashpay_payment_of_a_transaction_a_load_settlement_removes() {
    use platform_wallet::wallet::identity::PaymentEntry;
    use platform_wallet::wallet::identity::PaymentStatus::{Confirmed, Failed, Pending};

    let contact = dpp::prelude::Identifier::from([0xB7; 32]);
    let sent = || PaymentEntry::new_sent(contact, 59_000, None);
    let received = PaymentEntry::new_received(contact, 59_000, None);
    let settled = |loser_rows, returned, stored_verdicts, reopened| SettledPayment {
        loser_rows,
        returned,
        stored_verdicts,
        reopened,
        unrelated: Pending,
    };
    for (label, payment, status, policy, expected) in [
        (
            "pending sent",
            sent(),
            Pending,
            LoadPolicy::Strict,
            settled(0, Failed, 1, Failed),
        ),
        (
            "confirmed sent",
            sent(),
            Confirmed,
            LoadPolicy::Strict,
            settled(0, Failed, 1, Failed),
        ),
        (
            "received",
            received,
            Confirmed,
            LoadPolicy::Strict,
            settled(0, Confirmed, 0, Confirmed),
        ),
        (
            "pending sent, recovery load",
            sent(),
            Pending,
            LoadPolicy::Recovery,
            settled(1, Failed, 0, Failed),
        ),
    ] {
        let _case = Case(label.into());
        let outcome = settle_payment_at_load(payment, status, policy).await;
        assert_eq!(outcome, expected, "{label}");
    }
}
