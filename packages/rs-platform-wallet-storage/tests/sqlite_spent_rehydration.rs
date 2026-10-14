#![cfg(feature = "sqlite")]

mod common;

use dashcore::{
    address::Payload, bls_sig_utils::BLSSignature, ephemerealdata::chain_lock::ChainLock,
    hashes::Hash, Address, BlockHash, Network, OutPoint, PubkeyHash, ScriptBuf, Transaction, TxIn,
    TxOut, Txid,
};
use key_wallet::{
    transaction_checking::{BlockInfo, TransactionContext, WalletTransactionChecker},
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
    AccountRegistrationEntry, CoreChangeSet, PlatformWalletChangeSet, PlatformWalletPersistence,
    WalletMetadataEntry,
};
use platform_wallet_storage::{SqlitePersister, SqlitePersisterConfig};

fn block(height: u32) -> TransactionContext {
    TransactionContext::InBlock(BlockInfo::new(
        height,
        BlockHash::from_byte_array([height as u8; 32]),
        height,
    ))
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
        Self::with_funding(block(100), spend_context).await
    }

    async fn with_funding(
        funding_context: TransactionContext,
        spend_context: TransactionContext,
    ) -> Self {
        Self::build(funding_context, spend_context, true).await
    }

    /// The funding transaction persisted only through its UTXOs: a
    /// height-only `core_transactions` row with no record to replay.
    async fn with_height_only_funding(spend_context: TransactionContext) -> Self {
        let fixture = Self::build(block(100), spend_context, false).await;
        let (height, has_record): (Option<u32>, bool) = fixture
            .persister
            .lock_conn_for_test()
            .query_row(
                "SELECT height, record_blob IS NOT NULL FROM core_transactions WHERE txid = ?1",
                [fixture.funding.txid().as_byte_array().as_slice()],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        assert_eq!((height, has_record), (Some(100), false));
        fixture
    }

    async fn build(
        funding_context: TransactionContext,
        spend_context: TransactionContext,
        store_funding_record: bool,
    ) -> Self {
        let mut wallet =
            Wallet::new_random(Network::Testnet, WalletAccountCreationOptions::Default).unwrap();
        let mut info = ManagedWalletInfo::from_wallet(&wallet, 0);
        let xpub = wallet.accounts.standard_bip44_accounts[&0].account_xpub;
        let address = info
            .accounts
            .standard_bip44_accounts
            .get_mut(&0)
            .unwrap()
            .next_receive_address(Some(&xpub), true)
            .unwrap();
        let funding = Transaction {
            version: 1,
            lock_time: 0,
            input: vec![TxIn {
                previous_output: OutPoint::new(Txid::from_byte_array([13; 32]), 0),
                ..Default::default()
            }],
            output: vec![
                TxOut {
                    value: 100_000,
                    script_pubkey: address.script_pubkey(),
                },
                TxOut {
                    value: 20_000,
                    script_pubkey: address.script_pubkey(),
                },
            ],
            special_transaction_payload: None,
        };
        let spent = OutPoint::new(funding.txid(), 0);
        let available = OutPoint::new(funding.txid(), 1);
        let spending = Transaction {
            version: 1,
            lock_time: 0,
            input: vec![TxIn {
                previous_output: spent,
                ..Default::default()
            }],
            output: vec![TxOut {
                value: 99_000,
                script_pubkey: ScriptBuf::new(),
            }],
            special_transaction_payload: None,
        };
        let funding_result = info
            .check_core_transaction(&funding, funding_context, &mut wallet, true, true)
            .await;
        let coins: Vec<_> = info.accounts.standard_bip44_accounts[&0]
            .utxos
            .values()
            .cloned()
            .collect();
        let spent_coin = info.accounts.standard_bip44_accounts[&0].utxos[&spent].clone();
        let spending_result = info
            .check_core_transaction(&spending, spend_context, &mut wallet, true, true)
            .await;
        let mut records = funding_result.new_records;
        records.extend(spending_result.new_records);
        assert_eq!(records.len(), 2);
        if !store_funding_record {
            records.retain(|record| record.txid != funding.txid());
        }
        let (persister, dir, path) = common::fresh_persister();
        persister
            .store(
                wallet.wallet_id,
                PlatformWalletChangeSet {
                    wallet_metadata: Some(WalletMetadataEntry {
                        network: Network::Testnet,
                        wallet_group_id: [0; 32],
                        birth_height: 0,
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
                    core: Some(CoreChangeSet {
                        records,
                        new_utxos: coins,
                        spent_utxos: vec![spent_coin],
                        last_processed_height: Some(300),
                        synced_height: Some(300),
                        ..Default::default()
                    }),
                    ..Default::default()
                },
            )
            .unwrap();
        drop(persister);
        let persister = SqlitePersister::open(SqlitePersisterConfig::new(path)).unwrap();
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
        let mut state = self.persister.load().unwrap();
        let restored = state.wallets.remove(&self.wallet_id).unwrap();
        (restored.wallet, restored.wallet_info)
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
        let result = info
            .check_core_transaction(&self.funding, block(100), wallet, true, true)
            .await;
        self.assert_spent_excluded(info);
        self.persister
            .store(
                self.wallet_id,
                PlatformWalletChangeSet {
                    core: Some(CoreChangeSet {
                        records: result.new_records,
                        new_utxos: info.accounts.standard_bip44_accounts[&0]
                            .utxos
                            .values()
                            .cloned()
                            .collect(),
                        ..Default::default()
                    }),
                    ..Default::default()
                },
            )
            .unwrap();
        let (_, reloaded) = self.load();
        self.assert_spent_excluded(&reloaded);
    }
}

#[tokio::test]
async fn should_reject_spent_output_after_reload_and_funding_redelivery() {
    let fixture = Fixture::new(block(200)).await;
    let (mut wallet, mut info) = fixture.load();
    fixture.assert_spent_excluded(&info);
    fixture.redeliver(&mut wallet, &mut info).await;
}

#[tokio::test]
async fn should_keep_spent_output_excluded_after_finality_pruning() {
    let fixture = Fixture::new(block(200)).await;
    let (mut wallet, mut info) = fixture.load();
    info.apply_chain_lock(ChainLock {
        block_height: 300,
        block_hash: BlockHash::from_byte_array([30; 32]),
        signature: BLSSignature::from([0; 96]),
    });
    info.update_synced_height(300);
    assert!(info.observed_spent_outpoints().is_empty());
    fixture.redeliver(&mut wallet, &mut info).await;
}

#[tokio::test]
async fn should_keep_spent_output_excluded_after_finality_pruning_with_height_only_funding() {
    let fixture = Fixture::with_height_only_funding(block(200)).await;
    let (mut wallet, mut info) = fixture.load();
    fixture.assert_spent_excluded(&info);
    info.apply_chain_lock(ChainLock {
        block_height: 300,
        block_hash: BlockHash::from_byte_array([30; 32]),
        signature: BLSSignature::from([0; 96]),
    });
    info.update_synced_height(300);
    assert!(info.observed_spent_outpoints().is_empty());
    fixture.redeliver(&mut wallet, &mut info).await;
}

#[tokio::test]
async fn should_keep_unconfirmed_spend_reservation_with_height_only_funding() {
    let fixture = Fixture::with_height_only_funding(TransactionContext::Mempool).await;
    let (mut wallet, mut info) = fixture.load();
    fixture.assert_spent_excluded(&info);
    fixture.redeliver(&mut wallet, &mut info).await;
    fixture.assert_spent_stored(true);
}

#[tokio::test]
async fn should_reconcile_stale_unspent_projection_against_confirmed_history() {
    let fixture = Fixture::new(block(200)).await;
    fixture
        .persister
        .lock_conn_for_test()
        .execute("UPDATE core_utxos SET spent = 0", [])
        .unwrap();
    let (mut wallet, mut info) = fixture.load();
    fixture.assert_spent_excluded(&info);
    fixture.redeliver(&mut wallet, &mut info).await;
}

#[tokio::test]
async fn should_not_release_inputs_reserved_by_unconfirmed_spend() {
    let fixture = Fixture::new(TransactionContext::Mempool).await;
    let (mut wallet, mut info) = fixture.load();
    fixture.assert_spent_excluded(&info);
    assert!(!info.observed_spent_outpoints().contains_key(&fixture.spent));
    fixture.redeliver(&mut wallet, &mut info).await;
    fixture.assert_spent_stored(true);
}

#[tokio::test]
async fn should_keep_unconfirmed_funding_reserved_when_it_confirms_after_reload() {
    let fixture =
        Fixture::with_funding(TransactionContext::Mempool, TransactionContext::Mempool).await;
    let (mut wallet, mut info) = fixture.load();
    assert!(!info.accounts.standard_bip44_accounts[&0]
        .utxos
        .contains_key(&fixture.spent));
    fixture.redeliver(&mut wallet, &mut info).await;
    fixture.assert_spent_stored(true);
}

#[tokio::test]
async fn should_handle_funding_and_spend_in_the_same_block() {
    let fixture = Fixture::new(block(100)).await;
    let (mut wallet, mut info) = fixture.load();
    fixture.assert_spent_excluded(&info);
    fixture.redeliver(&mut wallet, &mut info).await;
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
async fn should_restore_persisted_finality_before_advancing_sync_checkpoint() {
    use key_wallet::managed_account::managed_account_trait::ManagedAccountTrait;

    let fixture = Fixture::new(block(100)).await;
    fixture
        .persister
        .store(
            fixture.wallet_id,
            PlatformWalletChangeSet {
                core: Some(CoreChangeSet {
                    last_applied_chain_lock: Some(ChainLock {
                        block_height: 300,
                        block_hash: BlockHash::from_byte_array([30; 32]),
                        signature: BLSSignature::from([0; 96]),
                    }),
                    ..Default::default()
                }),
                ..Default::default()
            },
        )
        .unwrap();
    let (mut wallet, mut info) = fixture.load();
    assert!(info.accounts.standard_bip44_accounts[&0]
        .keys()
        .transaction_is_finalized(&fixture.funding.txid()));
    info.update_synced_height(300);
    fixture.redeliver(&mut wallet, &mut info).await;
}

/// A foreign P2PKH address no account of the test wallet derives.
fn foreign_address(marker: u8) -> Address {
    Address::new(
        Network::Testnet,
        Payload::PubkeyHash(PubkeyHash::from_byte_array([marker; 20])),
    )
}

/// The persisted UTXO for `funding`'s output `vout`, as a sync would store it.
fn stored_utxo(funding: &Transaction, vout: u32, address: Address) -> Utxo {
    Utxo {
        outpoint: OutPoint::new(funding.txid(), vout),
        txout: funding.output[vout as usize].clone(),
        address,
        height: 100,
        is_coinbase: false,
        is_confirmed: true,
        is_instantlocked: false,
        is_locked: false,
        is_trusted: false,
    }
}

#[tokio::test]
async fn should_keep_replayed_output_only_in_its_owning_account() {
    let mut wallet =
        Wallet::new_random(Network::Testnet, WalletAccountCreationOptions::Default).unwrap();
    let mut info = ManagedWalletInfo::from_wallet(&wallet, 0);
    let xpub = wallet.accounts.standard_bip32_accounts[&0].account_xpub;
    let address = info
        .accounts
        .standard_bip32_accounts
        .get_mut(&0)
        .unwrap()
        .next_receive_address(Some(&xpub), true)
        .unwrap();
    // vout 1: no pool row and no replay owner, so the first-account fallback
    // must hold. vout 2: tracked only by a contact's watch-only chain.
    let (unowned, contact) = (foreign_address(0x61), foreign_address(0x62));
    let funding = Transaction {
        version: 1,
        lock_time: 0,
        input: vec![TxIn {
            previous_output: OutPoint::new(Txid::from_byte_array([14; 32]), 0),
            ..Default::default()
        }],
        output: vec![
            TxOut {
                value: 70_000,
                script_pubkey: address.script_pubkey(),
            },
            TxOut {
                value: 5_000,
                script_pubkey: unowned.script_pubkey(),
            },
            TxOut {
                value: 9_000,
                script_pubkey: contact.script_pubkey(),
            },
        ],
        special_transaction_payload: None,
    };
    let coin = OutPoint::new(funding.txid(), 0);
    let fallback = OutPoint::new(funding.txid(), 1);
    let contact_coin = OutPoint::new(funding.txid(), 2);
    let result = info
        .check_core_transaction(&funding, block(100), &mut wallet, true, true)
        .await;
    let mut coins: Vec<_> = info.accounts.standard_bip32_accounts[&0]
        .utxos
        .values()
        .cloned()
        .collect();
    assert_eq!(coins.len(), 1);
    coins.push(stored_utxo(&funding, 1, unowned));
    coins.push(stored_utxo(&funding, 2, contact.clone()));
    let (persister, _dir, _) = common::fresh_persister();
    persister
        .store(
            wallet.wallet_id,
            PlatformWalletChangeSet {
                wallet_metadata: Some(WalletMetadataEntry {
                    network: Network::Testnet,
                    wallet_group_id: [0; 32],
                    birth_height: 0,
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
                core: Some(CoreChangeSet {
                    records: result.new_records,
                    new_utxos: coins,
                    last_processed_height: Some(300),
                    synced_height: Some(300),
                    ..Default::default()
                }),
                ..Default::default()
            },
        )
        .unwrap();
    {
        let conn = persister.lock_conn_for_test();
        // Without its pool row the loader cannot attribute the coin and parks
        // it in the first funds account; replay then finds the real owner.
        conn.execute(
            "DELETE FROM core_address_pool WHERE script = ?1",
            [address.script_pubkey().as_bytes()],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO core_address_pool (wallet_id, account_type, account_index, pool_type, address_index, script) \
             VALUES (?1, 'dashpay_external', 0, 0, 0, ?2)",
            rusqlite::params![&wallet.wallet_id[..], contact.script_pubkey().as_bytes()],
        )
        .unwrap();
    }

    let mut state = persister.load().unwrap();
    let info = state.wallets.remove(&wallet.wallet_id).unwrap().wallet_info;
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
    let stored: i64 = persister
        .lock_conn_for_test()
        .query_row(
            "SELECT count(*) FROM core_utxos WHERE spent = 0",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(stored, 3, "load must not delete stored rows");
}

#[tokio::test]
async fn should_drop_replay_credit_for_contact_only_script() {
    let mut wallet =
        Wallet::new_random(Network::Testnet, WalletAccountCreationOptions::Default).unwrap();
    let mut info = ManagedWalletInfo::from_wallet(&wallet, 0);
    let xpub = wallet.accounts.standard_bip44_accounts[&0].account_xpub;
    let [contact, ours]: [Address; 2] = info
        .accounts
        .standard_bip44_accounts
        .get_mut(&0)
        .unwrap()
        .next_receive_addresses(Some(&xpub), 2, true)
        .unwrap()
        .try_into()
        .unwrap();
    let funding = Transaction {
        version: 1,
        lock_time: 0,
        input: vec![TxIn {
            previous_output: OutPoint::new(Txid::from_byte_array([16; 32]), 0),
            ..Default::default()
        }],
        output: vec![
            TxOut {
                value: 40_000,
                script_pubkey: contact.script_pubkey(),
            },
            TxOut {
                value: 7_000,
                script_pubkey: ours.script_pubkey(),
            },
        ],
        special_transaction_payload: None,
    };
    let contact_coin = OutPoint::new(funding.txid(), 0);
    let our_coin = OutPoint::new(funding.txid(), 1);
    let result = info
        .check_core_transaction(&funding, block(100), &mut wallet, true, true)
        .await;
    let coins: Vec<_> = info.accounts.standard_bip44_accounts[&0]
        .utxos
        .values()
        .cloned()
        .collect();
    assert_eq!(coins.len(), 2);
    let (persister, _dir, _) = common::fresh_persister();
    persister
        .store(
            wallet.wallet_id,
            PlatformWalletChangeSet {
                wallet_metadata: Some(WalletMetadataEntry {
                    network: Network::Testnet,
                    wallet_group_id: [0; 32],
                    birth_height: 0,
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
                core: Some(CoreChangeSet {
                    records: result.new_records,
                    new_utxos: coins,
                    last_processed_height: Some(300),
                    synced_height: Some(300),
                    ..Default::default()
                }),
                ..Default::default()
            },
        )
        .unwrap();
    {
        // The store tracks the script only on a contact's watch-only chain,
        // yet the rebuilt funds account still derives it, so replay credits it.
        let conn = persister.lock_conn_for_test();
        conn.execute(
            "DELETE FROM core_address_pool WHERE script = ?1",
            [contact.script_pubkey().as_bytes()],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO core_address_pool \
             (wallet_id, account_type, account_index, pool_type, address_index, script) \
             VALUES (?1, 'dashpay_external', 0, 0, 0, ?2)",
            rusqlite::params![&wallet.wallet_id[..], contact.script_pubkey().as_bytes()],
        )
        .unwrap();
    }

    let mut state = persister.load().unwrap();
    let info = state.wallets.remove(&wallet.wallet_id).unwrap().wallet_info;
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

#[tokio::test]
async fn should_restore_extra_input_released_by_a_persisted_conflict_after_restart() {
    assert_conflict_restart(ConflictCase::Released).await;
}

#[tokio::test]
async fn should_restore_released_input_with_height_only_funding_after_restart() {
    assert_conflict_restart(ConflictCase::HeightOnlyFunding).await;
}

#[tokio::test]
async fn should_keep_extra_input_reserved_by_a_surviving_spend_after_restart() {
    assert_conflict_restart(ConflictCase::SurvivingClaim).await;
}

#[tokio::test]
async fn should_resolve_locked_competitor_using_persisted_chainlock_after_restart() {
    assert_conflict_restart(ConflictCase::PersistedChainLock).await;
}

#[tokio::test]
async fn should_not_rewrite_conflicting_history_during_recovery_load() {
    assert_conflict_restart(ConflictCase::Recovery).await;
}

#[tokio::test]
async fn should_preserve_a_recordless_winners_claim_during_replay() {
    assert_conflict_restart(ConflictCase::RecordlessClaim).await;
}

#[tokio::test]
async fn should_preserve_a_recordless_winners_placeholder_during_replay() {
    assert_conflict_restart(ConflictCase::RecordlessPlaceholder).await;
}

#[tokio::test]
async fn should_preserve_unknown_materialized_claim_during_startup_conflict_repair() {
    for height in [None, Some(102)] {
        assert_conflict_restart(ConflictCase::UnknownClaim {
            placeholder: false,
            height,
        })
        .await;
    }
}

#[tokio::test]
async fn should_preserve_unknown_placeholder_claim_during_startup_conflict_repair() {
    for height in [None, Some(102)] {
        assert_conflict_restart(ConflictCase::UnknownClaim {
            placeholder: true,
            height,
        })
        .await;
    }
}

#[tokio::test]
async fn should_reject_malformed_claimant_during_startup_conflict_repair() {
    assert_conflict_restart(ConflictCase::MalformedClaim).await;
}

#[tokio::test]
async fn should_roll_back_a_failed_replay_repair() {
    assert_conflict_restart(ConflictCase::FailedRepair).await;
}

#[tokio::test]
async fn should_not_let_a_defeated_lock_remove_a_surviving_spender() {
    assert_conflict_restart(ConflictCase::DefeatedLock).await;
}

#[derive(Clone, Copy)]
enum ConflictCase {
    Released,
    HeightOnlyFunding,
    SurvivingClaim,
    PersistedChainLock,
    Recovery,
    RecordlessClaim,
    RecordlessPlaceholder,
    UnknownClaim {
        placeholder: bool,
        height: Option<u32>,
    },
    MalformedClaim,
    FailedRepair,
    Assets,
    AssetsRecovery,
    AssetsFailedRepair,
    Unconverged,
    DefeatedLock,
}

async fn assert_conflict_restart(case: ConflictCase) {
    let record_funding = !matches!(case, ConflictCase::HeightOnlyFunding);
    let surviving_claim = matches!(
        case,
        ConflictCase::SurvivingClaim | ConflictCase::DefeatedLock
    );
    let chainlocked = matches!(
        case,
        ConflictCase::PersistedChainLock | ConflictCase::DefeatedLock
    );
    let recordless_claim = matches!(
        case,
        ConflictCase::RecordlessClaim | ConflictCase::RecordlessPlaceholder
    );
    let unknown_claim = matches!(case, ConflictCase::UnknownClaim { .. });
    let expect_released = !surviving_claim && !recordless_claim && !unknown_claim;
    let recovery = matches!(case, ConflictCase::Recovery | ConflictCase::AssetsRecovery);
    let assets = matches!(
        case,
        ConflictCase::Assets | ConflictCase::AssetsRecovery | ConflictCase::AssetsFailedRepair
    );

    use dashcore::ephemerealdata::instant_lock::InstantLock;
    use key_wallet::managed_account::managed_account_trait::ManagedAccountTrait;

    let mut wallet =
        Wallet::new_random(Network::Testnet, WalletAccountCreationOptions::Default).unwrap();
    let mut info = ManagedWalletInfo::from_wallet(&wallet, 0);
    let xpub = wallet.accounts.standard_bip44_accounts[&0].account_xpub;
    let address = info
        .accounts
        .standard_bip44_accounts
        .get_mut(&0)
        .unwrap()
        .next_receive_address(Some(&xpub), true)
        .unwrap();
    let funding = Transaction {
        version: 1,
        lock_time: 0,
        input: vec![TxIn {
            previous_output: OutPoint::new(Txid::from_byte_array([71; 32]), 0),
            ..Default::default()
        }],
        output: [100_000, 20_000]
            .into_iter()
            .map(|value| TxOut {
                value,
                script_pubkey: address.script_pubkey(),
            })
            .collect(),
        special_transaction_payload: None,
    };
    let coins: Vec<_> = funding
        .output
        .iter()
        .enumerate()
        .map(|(vout, output)| {
            Utxo::new(
                OutPoint::new(funding.txid(), vout as u32),
                output.clone(),
                address.clone(),
                100,
                false,
            )
        })
        .collect();
    let loser = Transaction {
        version: 1,
        lock_time: 0,
        input: coins
            .iter()
            .map(|coin| TxIn {
                previous_output: coin.outpoint,
                ..Default::default()
            })
            .collect(),
        output: vec![TxOut {
            value: 119_000,
            script_pubkey: address.script_pubkey(),
        }],
        special_transaction_payload: None,
    };
    let winner = Transaction {
        version: 1,
        lock_time: 0,
        input: vec![TxIn {
            previous_output: coins[0].outpoint,
            ..Default::default()
        }],
        output: vec![TxOut {
            value: 99_000,
            script_pubkey: ScriptBuf::new(),
        }],
        special_transaction_payload: None,
    };
    let mut records = info
        .check_core_transaction(&funding, block(100), &mut wallet, true, true)
        .await
        .new_records;
    if !record_funding {
        records.clear();
    }
    let mut competing = info.clone();
    let mut other_spender = info.clone();
    records.extend(
        info.check_core_transaction(&loser, TransactionContext::Mempool, &mut wallet, true, true)
            .await
            .new_records,
    );
    records.extend(
        competing
            .check_core_transaction(
                &winner,
                TransactionContext::Mempool,
                &mut wallet,
                true,
                true,
            )
            .await
            .new_records,
    );
    let mut surviving_txid = None;
    if surviving_claim {
        let mut extra_spender = winner.clone();
        extra_spender.input[0].previous_output = coins[1].outpoint;
        extra_spender.output[0].value = 19_000;
        surviving_txid = Some(extra_spender.txid());
        records.extend(
            other_spender
                .check_core_transaction(
                    &extra_spender,
                    TransactionContext::Mempool,
                    &mut wallet,
                    true,
                    true,
                )
                .await
                .new_records,
        );
    }
    let mut locks = std::collections::BTreeMap::new();
    if chainlocked {
        records
            .iter_mut()
            .find(|record| record.txid == winner.txid())
            .unwrap()
            .context = block(101);
        locks.insert(
            loser.txid(),
            InstantLock {
                inputs: coins.iter().map(|coin| coin.outpoint).collect(),
                txid: loser.txid(),
                ..Default::default()
            },
        );
    } else {
        locks.insert(
            winner.txid(),
            InstantLock {
                inputs: vec![coins[0].outpoint],
                txid: winner.txid(),
                ..Default::default()
            },
        );
    }
    let dir = common::secure_tempdir().unwrap();
    let path = dir.path().join("released-input.sqlite");
    let persister = SqlitePersister::open(SqlitePersisterConfig::new(&path)).unwrap();
    persister
        .store(
            wallet.wallet_id,
            PlatformWalletChangeSet {
                wallet_metadata: Some(WalletMetadataEntry {
                    network: Network::Testnet,
                    wallet_group_id: [0; 32],
                    birth_height: 0,
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
                core: Some(CoreChangeSet {
                    records,
                    new_utxos: coins
                        .iter()
                        .cloned()
                        .chain([Utxo::new(
                            OutPoint::new(loser.txid(), 0),
                            loser.output[0].clone(),
                            address,
                            0,
                            false,
                        )])
                        .collect(),
                    spent_utxos: coins.clone(),
                    instant_locks_for_non_final_records: locks,
                    last_applied_chain_lock: chainlocked.then_some(ChainLock {
                        block_height: 101,
                        block_hash: BlockHash::from_byte_array([101; 32]),
                        signature: [0; 96].into(),
                    }),
                    ..Default::default()
                }),
                ..Default::default()
            },
        )
        .unwrap();
    let extra_key: Vec<u8> = persister
        .lock_conn_for_test()
        .query_row(
            "SELECT outpoint FROM core_utxos WHERE wallet_id = ?1 AND value = 20000",
            [wallet.wallet_id.as_slice()],
            |row| row.get(0),
        )
        .unwrap();
    // The store itself names the loser as claimant; only an unknown durable claim stays held.
    if let ConflictCase::UnknownClaim { height, .. } = case {
        persister.lock_conn_for_test().execute(
            "UPDATE core_utxos SET spent_in_txid = NULL, winner_mined_height = ?1 WHERE wallet_id = ?2 AND outpoint = ?3",
            rusqlite::params![height, wallet.wallet_id.as_slice(), &extra_key],
        ).unwrap();
    }
    if matches!(case, ConflictCase::MalformedClaim) {
        persister.lock_conn_for_test().execute(
            "UPDATE core_utxos SET spent_in_txid = zeroblob(31) WHERE wallet_id = ?1 AND outpoint = ?2",
            rusqlite::params![wallet.wallet_id.as_slice(), &extra_key],
        ).unwrap();
        assert!(matches!(
            typed_error(persister.load().unwrap_err()),
            platform_wallet_storage::WalletStorageError::BlobDecode { .. }
        ));
        assert!(persister
            .get_core_tx_record(wallet.wallet_id, &loser.txid())
            .unwrap()
            .is_some());
        return;
    }
    let missing_winner = Txid::from_byte_array([72; 32]);
    if recordless_claim {
        let conn = persister.lock_conn_for_test();
        conn.execute("INSERT INTO core_transactions (wallet_id, txid, height, finalized) VALUES (?1, ?2, 101, 0)",
            rusqlite::params![wallet.wallet_id.as_slice(), missing_winner.as_byte_array().as_slice()]).unwrap();
        conn.execute(
            "UPDATE core_utxos SET spent_in_txid = ?1 WHERE wallet_id = ?2 AND value = 20000",
            rusqlite::params![
                missing_winner.as_byte_array().as_slice(),
                wallet.wallet_id.as_slice()
            ],
        )
        .unwrap();
    }
    if matches!(
        case,
        ConflictCase::RecordlessPlaceholder
            | ConflictCase::UnknownClaim {
                placeholder: true,
                ..
            }
    ) {
        persister.lock_conn_for_test().execute(
            "UPDATE core_utxos SET value = 0, script = X'', is_sweep_placeholder = 1 WHERE wallet_id = ?1 AND outpoint = ?2",
            rusqlite::params![wallet.wallet_id.as_slice(), &extra_key]).unwrap();
    }
    let mut other_wallet_id = None;
    if assets {
        use key_wallet::wallet::managed_wallet_info::asset_lock_builder::AssetLockFundingType;
        use platform_wallet::changeset::{AssetLockChangeSet, AssetLockEntry};
        use platform_wallet::wallet::asset_lock::tracked::AssetLockStatus;
        let entries: std::collections::BTreeMap<_, _> =
            [AssetLockStatus::Broadcast, AssetLockStatus::Consumed]
                .into_iter()
                .enumerate()
                .map(|(vout, status)| {
                    let out_point = OutPoint::new(loser.txid(), vout as u32);
                    (
                        out_point,
                        AssetLockEntry {
                            out_point,
                            transaction: loser.clone(),
                            account_index: 0,
                            funding_type: AssetLockFundingType::IdentityTopUp,
                            identity_index: 0,
                            amount_duffs: 1000,
                            status,
                            proof: None,
                        },
                    )
                })
                .collect();
        let other =
            Wallet::new_random(Network::Testnet, WalletAccountCreationOptions::Default).unwrap();
        other_wallet_id = Some(other.wallet_id);
        let outpoint = OutPoint::new(loser.txid(), 0);
        persister
            .store(
                other.wallet_id,
                PlatformWalletChangeSet {
                    wallet_metadata: Some(WalletMetadataEntry {
                        network: Network::Testnet,
                        wallet_group_id: [0; 32],
                        birth_height: 0,
                    }),
                    account_registrations: other
                        .accounts
                        .all_accounts()
                        .into_iter()
                        .map(|account| AccountRegistrationEntry {
                            account_type: account.account_type,
                            account_xpub: account.account_xpub,
                        })
                        .collect(),
                    asset_locks: Some(AssetLockChangeSet {
                        asset_locks: [(outpoint, entries[&outpoint].clone())]
                            .into_iter()
                            .collect(),
                        ..Default::default()
                    }),
                    ..Default::default()
                },
            )
            .unwrap();
        persister
            .store(
                wallet.wallet_id,
                PlatformWalletChangeSet {
                    asset_locks: Some(AssetLockChangeSet {
                        asset_locks: entries,
                        ..Default::default()
                    }),
                    ..Default::default()
                },
            )
            .unwrap();
    }
    if matches!(
        case,
        ConflictCase::FailedRepair | ConflictCase::AssetsFailedRepair
    ) {
        persister.lock_conn_for_test().execute_batch(
            "CREATE TRIGGER fail_replay_repair BEFORE UPDATE OF spent ON core_utxos WHEN NEW.spent = 0 BEGIN SELECT RAISE(ABORT, 'injected replay repair failure'); END;",
        ).unwrap();
        assert!(persister.load().is_err());
        if assets {
            assert_eq!(
                asset_count(&persister),
                3,
                "failed reconciliation must roll back asset lifecycle deletions"
            );
        }
        assert!(persister
            .get_core_tx_record(wallet.wallet_id, &loser.txid())
            .unwrap()
            .is_some());
        let spent: bool = persister
            .lock_conn_for_test()
            .query_row(
                "SELECT spent FROM core_utxos WHERE wallet_id = ?1 AND outpoint = ?2",
                rusqlite::params![wallet.wallet_id.as_slice(), &extra_key],
                |row| row.get(0),
            )
            .unwrap();
        assert!(spent);
        persister
            .lock_conn_for_test()
            .execute_batch("DROP TRIGGER fail_replay_repair")
            .unwrap();
    }
    if matches!(case, ConflictCase::Unconverged) {
        persister.lock_conn_for_test().execute_batch("CREATE TRIGGER preserve_history BEFORE DELETE ON core_transactions BEGIN SELECT RAISE(IGNORE); END;").unwrap();
        let error = persister.load().unwrap_err();
        assert!(
            matches!(typed_error(error), platform_wallet_storage::WalletStorageError::WalletRehydrationFailed { wallet_id, .. } if wallet_id == wallet.wallet_id)
        );
        assert!(persister
            .get_core_tx_record(wallet.wallet_id, &loser.txid())
            .unwrap()
            .is_some());
        return;
    }
    drop(persister);
    for _ in 0..2 {
        let policy = if recovery {
            platform_wallet_storage::LoadPolicy::Recovery
        } else {
            platform_wallet_storage::LoadPolicy::Strict
        };
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
                asset_count(&persister),
                if recovery { 3 } else { 2 },
                "consumed history is retained and Recovery rolls back removals"
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
            !account.utxos.contains_key(&coins[0].outpoint),
            "winner still spends the shared input"
        );
        assert_eq!(
            account.utxos.contains_key(&coins[1].outpoint),
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
            let claim: Vec<u8> = persister
                .lock_conn_for_test()
                .query_row(
                    "SELECT spent_in_txid FROM core_utxos WHERE wallet_id = ?1 AND outpoint = ?2",
                    rusqlite::params![wallet.wallet_id.as_slice(), &extra_key],
                    |row| row.get(0),
                )
                .unwrap();
            assert_eq!(claim, missing_winner.as_byte_array());
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
            assert_eq!(selection.unwrap().selected[0].outpoint, coins[1].outpoint);
        }
        if recordless_claim || unknown_claim {
            let loaded = state.wallets.get_mut(&wallet.wallet_id).unwrap();
            loaded
                .wallet_info
                .check_core_transaction(&funding, block(100), &mut loaded.wallet, true, true)
                .await;
            assert!(
                !loaded.wallet_info.accounts.standard_bip44_accounts[&0]
                    .utxos
                    .contains_key(&coins[1].outpoint),
                "recordless claim must survive funding redelivery"
            );
            assert_eq!(loaded.wallet_info.balance.total(), 0);
        }
        if let ConflictCase::UnknownClaim {
            placeholder,
            height,
        } = case
        {
            let persisted: (bool, Option<Vec<u8>>, Option<u32>, bool) = persister.lock_conn_for_test().query_row(
                "SELECT spent, spent_in_txid, winner_mined_height, is_sweep_placeholder FROM core_utxos WHERE wallet_id = ?1 AND outpoint = ?2",
                rusqlite::params![wallet.wallet_id.as_slice(), &extra_key],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            ).expect("unknown claim must survive repair and reopen");
            assert_eq!(
                persisted,
                (true, None, height, placeholder),
                "repair must retain nullable provenance through reload and funding redelivery"
            );
        }
    }
}

fn asset_count(persister: &SqlitePersister) -> i64 {
    persister
        .lock_conn_for_test()
        .query_row("SELECT COUNT(*) FROM asset_locks", [], |row| row.get(0))
        .unwrap()
}

#[tokio::test]
async fn should_remove_swept_asset_locks_on_restart() {
    assert_conflict_restart(ConflictCase::Assets).await;
}

#[tokio::test]
async fn should_rollback_swept_asset_locks_in_recovery() {
    assert_conflict_restart(ConflictCase::AssetsRecovery).await;
}

#[tokio::test]
async fn should_rollback_swept_asset_locks_on_sql_failure() {
    assert_conflict_restart(ConflictCase::AssetsFailedRepair).await;
}

#[tokio::test]
async fn should_guard_recordless_spends_with_missing_funding_and_finality() {
    for funding_state in ["absent", "height_only", "record"] {
        for spender_row in [false, true] {
            for placeholder in [false, true] {
                for known_claimant in [false, true] {
                    let fixture =
                        Fixture::build(block(100), block(200), funding_state == "record").await;
                    {
                        let conn = fixture.persister.lock_conn_for_test();
                        if spender_row {
                            conn.execute(
                                "UPDATE core_transactions SET record_blob = NULL WHERE txid != ?1",
                                [fixture.funding.txid().as_byte_array().as_slice()],
                            )
                            .unwrap();
                        } else {
                            conn.execute(
                                "DELETE FROM core_transactions WHERE txid != ?1",
                                [fixture.funding.txid().as_byte_array().as_slice()],
                            )
                            .unwrap();
                        }
                        if funding_state == "absent" {
                            conn.execute(
                                "DELETE FROM core_transactions WHERE txid = ?1",
                                [fixture.funding.txid().as_byte_array().as_slice()],
                            )
                            .unwrap();
                        }
                        if !known_claimant {
                            conn.execute(
                                "UPDATE core_utxos SET spent_in_txid = NULL WHERE spent = 1",
                                [],
                            )
                            .unwrap();
                        }
                        if placeholder {
                            conn.execute("UPDATE core_utxos SET value = 0, script = X'', is_sweep_placeholder = 1 WHERE spent = 1", []).unwrap();
                        }
                    }
                    let (mut wallet, mut info) = fixture.load();
                    info.apply_chain_lock(ChainLock {
                        block_height: 300,
                        block_hash: BlockHash::from_byte_array([30; 32]),
                        signature: [0; 96].into(),
                    });
                    info.check_core_transaction(
                        &fixture.funding,
                        block(100),
                        &mut wallet,
                        true,
                        true,
                    )
                    .await;
                    fixture.assert_spent_excluded(&info);
                    // A later loser must not release a different durable claimant's input.
                    let mut loser = fixture.funding.clone();
                    loser.input = [fixture.spent, fixture.available]
                        .into_iter()
                        .map(|previous_output| TxIn {
                            previous_output,
                            ..Default::default()
                        })
                        .collect();
                    loser.output.truncate(1);
                    loser.output[0].value = 119_000;
                    info.check_core_transaction(
                        &loser,
                        TransactionContext::Mempool,
                        &mut wallet,
                        true,
                        true,
                    )
                    .await;
                    let mut winner = loser.clone();
                    winner.input.remove(0);
                    winner.output[0] = TxOut {
                        value: 19_000,
                        script_pubkey: ScriptBuf::new(),
                    };
                    info.check_core_transaction(&winner, block(301), &mut wallet, true, true)
                        .await;
                    info.check_core_transaction(
                        &fixture.funding,
                        block(100),
                        &mut wallet,
                        true,
                        true,
                    )
                    .await;
                    assert!(
                        !info.accounts.standard_bip44_accounts[&0]
                            .utxos
                            .contains_key(&fixture.spent),
                        "a later conflict cannot release a recordless durable claim"
                    );
                    assert_eq!(info.balance.total(), 0);
                }
            }
        }
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
    let funding = Transaction {
        version: 1,
        lock_time: 0,
        input: vec![TxIn {
            previous_output: OutPoint::new(Txid::from_byte_array([17; 32]), 0),
            ..Default::default()
        }],
        output: vec![
            TxOut {
                value: 100_000,
                script_pubkey: address.script_pubkey(),
            },
            TxOut {
                value: 20_000,
                script_pubkey: address.script_pubkey(),
            },
        ],
        special_transaction_payload: None,
    };
    let claimed = OutPoint::new(funding.txid(), 0);
    let unclaimed = OutPoint::new(funding.txid(), 1);

    // A recordless claim: the spent coin, with neither its funding nor its spender.
    let (persister, _dir) = store_then_reopen(
        &wallet,
        CoreChangeSet {
            spent_utxos: vec![stored_utxo(&funding, 0, address)],
            last_processed_height: Some(300),
            synced_height: Some(300),
            ..Default::default()
        },
    );
    let stored_claims: i64 = persister
        .lock_conn_for_test()
        .query_row(
            "SELECT COUNT(*) FROM core_utxos WHERE spent = 1",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(stored_claims, 1, "the store holds the recordless claim");

    let mut state = persister.load().unwrap();
    let restored = state.wallets.remove(&wallet.wallet_id).unwrap();
    let (mut wallet, mut info) = (restored.wallet, restored.wallet_info);
    assert!(!info.accounts.standard_bip44_accounts.contains_key(&7));

    let (managed, _) = late_managed();
    wallet
        .add_account(late_type, Some(late_account.account_xpub))
        .unwrap();
    info.accounts.insert_funds_bearing_account(managed).unwrap();

    info.check_core_transaction(&funding, block(100), &mut wallet, true, true)
        .await;

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
    let fixture = Fixture::with_height_only_funding(context).await;
    let conn = fixture.persister.lock_conn_for_test();
    let bytes: Vec<u8> = conn
        .query_row(
            "SELECT record_blob FROM core_transactions WHERE record_blob IS NOT NULL",
            [],
            |row| row.get(0),
        )
        .unwrap();
    let mut record: key_wallet::managed_account::transaction_record::TransactionRecord =
        blob::decode(&bytes).unwrap();
    let address = record.input_details[0].address.clone();
    record.transaction.input = values
        .iter()
        .enumerate()
        .map(|(index, _)| TxIn {
            previous_output: OutPoint::new(Txid::from_byte_array([90; 32]), index as u32),
            ..Default::default()
        })
        .collect();
    record.input_details = values
        .iter()
        .enumerate()
        .map(|(index, value)| InputDetail {
            index: index as u32,
            value: *value,
            address: address.clone(),
        })
        .collect();
    record.transaction.output = vec![TxOut {
        value: received,
        script_pubkey: address.script_pubkey(),
    }];
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

async fn assert_replay_amount_rejected(values: &[u64], received: u64) {
    use platform_wallet_storage::WalletStorageError;
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
        "unsafe checker arithmetic must be rejected before replay"
    );
}

#[tokio::test]
async fn should_reject_individually_overflowing_replay_input_with_fitting_net() {
    assert_replay_amount_rejected(&[1_u64 << 63], 1).await;
}

#[tokio::test]
async fn should_reject_cumulatively_overflowing_replay_inputs_with_fitting_net() {
    assert_replay_amount_rejected(&[i64::MAX as u64, 1], 1).await;
}

#[tokio::test]
async fn should_reject_overflowing_replay_output_with_fitting_net() {
    assert_replay_amount_rejected(&[i64::MAX as u64], 1_u64 << 63).await;
}

#[tokio::test]
async fn should_skip_unconfirmed_record_with_overflowing_replay_inputs() {
    let fixture = stage_replay_amounts(TransactionContext::Mempool, &[i64::MAX as u64, 1], 1).await;

    let (_, info) = fixture.load();

    fixture.assert_spent_excluded(&info);
}

/// A fresh wallet with one derived receive address on its first BIP44 account.
fn wallet_with_receive_address() -> (Wallet, ManagedWalletInfo, Address) {
    let wallet =
        Wallet::new_random(Network::Testnet, WalletAccountCreationOptions::Default).unwrap();
    let mut info = ManagedWalletInfo::from_wallet(&wallet, 0);
    let xpub = wallet.accounts.standard_bip44_accounts[&0].account_xpub;
    let address = info
        .accounts
        .standard_bip44_accounts
        .get_mut(&0)
        .unwrap()
        .next_receive_address(Some(&xpub), true)
        .unwrap();
    (wallet, info, address)
}

/// Persist one Core round for `wallet` as a sync would, then reopen the store strictly.
fn store_then_reopen(wallet: &Wallet, core: CoreChangeSet) -> (SqlitePersister, tempfile::TempDir) {
    let (persister, dir, path) = common::fresh_persister();
    persister
        .store(
            wallet.wallet_id,
            PlatformWalletChangeSet {
                wallet_metadata: Some(WalletMetadataEntry {
                    network: Network::Testnet,
                    wallet_group_id: [0; 32],
                    birth_height: 0,
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
                core: Some(core),
                ..Default::default()
            },
        )
        .expect("the store accepts what the live checker recorded");
    drop(persister);
    let config = SqlitePersisterConfig::new(path)
        .with_load_policy(platform_wallet_storage::LoadPolicy::Strict);
    (SqlitePersister::open(config).unwrap(), dir)
}

fn spend_of(previous_output: OutPoint, output: Vec<TxOut>) -> Transaction {
    Transaction {
        version: 1,
        lock_time: 0,
        input: vec![TxIn {
            previous_output,
            ..Default::default()
        }],
        output,
        special_transaction_payload: None,
    }
}

#[tokio::test]
async fn should_load_wallet_holding_mempool_receipt_with_overflowing_foreign_output() {
    let (mut wallet, mut info, address) = wallet_with_receive_address();
    // Unminable, but nothing rejects it on the way in: only the owned output is accounted.
    let receipt = spend_of(
        OutPoint::new(Txid::from_byte_array([81; 32]), 0),
        vec![
            TxOut {
                value: 5_000,
                script_pubkey: address.script_pubkey(),
            },
            TxOut {
                value: i64::MAX as u64,
                script_pubkey: foreign_address(82).script_pubkey(),
            },
        ],
    );
    let records = info
        .check_core_transaction(
            &receipt,
            TransactionContext::Mempool,
            &mut wallet,
            true,
            true,
        )
        .await
        .new_records;
    assert_eq!(records.len(), 1, "the live checker records the receipt");
    let new_utxos = info.accounts.standard_bip44_accounts[&0]
        .utxos
        .values()
        .cloned()
        .collect();
    let (persister, _dir) = store_then_reopen(
        &wallet,
        CoreChangeSet {
            records,
            new_utxos,
            ..Default::default()
        },
    );

    let mut state = persister
        .load()
        .expect("an unminable mempool receipt must not fail a strict load");

    let info = state.wallets.remove(&wallet.wallet_id).unwrap().wallet_info;
    let coins = &info.accounts.standard_bip44_accounts[&0].utxos;
    assert_eq!(
        coins.keys().copied().collect::<Vec<_>>(),
        vec![OutPoint::new(receipt.txid(), 0)],
        "the owned receipt survives the restart"
    );
    assert_eq!(info.balance.total(), 5_000);
}

#[tokio::test]
async fn should_keep_reservation_of_forged_mempool_spend_with_overflowing_foreign_output() {
    let (mut wallet, mut info, address) = wallet_with_receive_address();
    let funding = spend_of(
        OutPoint::new(Txid::from_byte_array([83; 32]), 0),
        vec![TxOut {
            value: 100_000,
            script_pubkey: address.script_pubkey(),
        }],
    );
    let mut records = info
        .check_core_transaction(&funding, block(100), &mut wallet, true, true)
        .await
        .new_records;
    let coin =
        info.accounts.standard_bip44_accounts[&0].utxos[&OutPoint::new(funding.txid(), 0)].clone();
    // Mempool signatures are not verified, so a peer can name a wallet coin as an input.
    let forged = spend_of(
        coin.outpoint,
        vec![
            TxOut {
                value: i64::MAX as u64,
                script_pubkey: foreign_address(84).script_pubkey(),
            },
            TxOut {
                value: 1_000,
                script_pubkey: address.script_pubkey(),
            },
        ],
    );
    records.extend(
        info.check_core_transaction(
            &forged,
            TransactionContext::Mempool,
            &mut wallet,
            true,
            true,
        )
        .await
        .new_records,
    );
    assert_eq!(
        records.len(),
        2,
        "the live checker records the forged spend"
    );
    let change = OutPoint::new(forged.txid(), 1);
    let new_utxos = [
        coin.clone(),
        info.accounts.standard_bip44_accounts[&0].utxos[&change].clone(),
    ]
    .into();
    let (persister, _dir) = store_then_reopen(
        &wallet,
        CoreChangeSet {
            records,
            new_utxos,
            spent_utxos: vec![coin.clone()],
            ..Default::default()
        },
    );

    let mut state = persister
        .load()
        .expect("an unminable mempool spend must not fail a strict load");

    let loaded = state.wallets.get_mut(&wallet.wallet_id).unwrap();
    loaded
        .wallet_info
        .check_core_transaction(&funding, block(100), &mut loaded.wallet, true, true)
        .await;
    let coins = &loaded.wallet_info.accounts.standard_bip44_accounts[&0].utxos;
    assert_eq!(
        coins.keys().copied().collect::<Vec<_>>(),
        vec![change],
        "the reserved coin stays out of the spendable set through funding redelivery"
    );
    assert_eq!(loaded.wallet_info.balance.total(), 1_000);
}

/// Two confirmed wallet coins named as inputs by a forged, unminable mempool
/// spend: its foreign output overflows the replay arithmetic, its change is small.
struct ForgedSpend {
    wallet: Wallet,
    funding: Transaction,
    /// The coin a final competitor also spends.
    shared: Utxo,
    /// The coin only the forged spend claims.
    extra: Utxo,
    forged: Transaction,
    change: Utxo,
    /// Funding and forged-spend records, as the live checker produced them.
    records: Vec<key_wallet::managed_account::transaction_record::TransactionRecord>,
    /// A confirmed transaction spending only `shared`, and its record.
    winner: Transaction,
    winner_records: Vec<key_wallet::managed_account::transaction_record::TransactionRecord>,
}

impl ForgedSpend {
    async fn build() -> Self {
        let (mut wallet, mut info, address) = wallet_with_receive_address();
        let funding = spend_of(
            OutPoint::new(Txid::from_byte_array([85; 32]), 0),
            [100_000, 20_000]
                .into_iter()
                .map(|value| TxOut {
                    value,
                    script_pubkey: address.script_pubkey(),
                })
                .collect(),
        );
        let mut records = info
            .check_core_transaction(&funding, block(100), &mut wallet, true, true)
            .await
            .new_records;
        let coins = &info.accounts.standard_bip44_accounts[&0].utxos;
        let shared = coins[&OutPoint::new(funding.txid(), 0)].clone();
        let extra = coins[&OutPoint::new(funding.txid(), 1)].clone();
        let winner = spend_of(
            shared.outpoint,
            vec![TxOut {
                value: 99_000,
                script_pubkey: foreign_address(86).script_pubkey(),
            }],
        );
        let winner_records = info
            .clone()
            .check_core_transaction(&winner, block(101), &mut wallet.clone(), true, true)
            .await
            .new_records;
        let mut forged = spend_of(
            shared.outpoint,
            vec![
                TxOut {
                    value: i64::MAX as u64,
                    script_pubkey: foreign_address(87).script_pubkey(),
                },
                TxOut {
                    value: 1_000,
                    script_pubkey: address.script_pubkey(),
                },
            ],
        );
        forged.input.push(TxIn {
            previous_output: extra.outpoint,
            ..Default::default()
        });
        records.extend(
            info.check_core_transaction(
                &forged,
                TransactionContext::Mempool,
                &mut wallet,
                true,
                true,
            )
            .await
            .new_records,
        );
        assert_eq!(
            records.len(),
            2,
            "the live checker records the forged spend"
        );
        assert_eq!(winner_records.len(), 1);
        let change = info.accounts.standard_bip44_accounts[&0].utxos
            [&OutPoint::new(forged.txid(), 1)]
            .clone();
        Self {
            wallet,
            funding,
            shared,
            extra,
            forged,
            change,
            records,
            winner,
            winner_records,
        }
    }

    /// Persist the round a sync would write, optionally with the final competitor, and reopen.
    fn store(&self, with_winner: bool) -> (SqlitePersister, tempfile::TempDir) {
        let mut records = self.records.clone();
        if with_winner {
            records.extend(self.winner_records.clone());
        }
        store_then_reopen(
            &self.wallet,
            CoreChangeSet {
                records,
                new_utxos: vec![self.shared.clone(), self.extra.clone(), self.change.clone()],
                spent_utxos: vec![self.shared.clone(), self.extra.clone()],
                ..Default::default()
            },
        )
    }
}

fn coin_set(info: &ManagedWalletInfo) -> Vec<OutPoint> {
    info.accounts.standard_bip44_accounts[&0]
        .utxos
        .keys()
        .copied()
        .collect()
}

#[tokio::test]
async fn should_settle_unsafe_mempool_loser_against_persisted_final_winner_on_restart() {
    use key_wallet::managed_account::managed_account_trait::ManagedAccountTrait;
    let scenario = ForgedSpend::build().await;
    let (persister, _dir) = scenario.store(true);

    let mut state = persister.load().unwrap();

    let info = state
        .wallets
        .remove(&scenario.wallet.wallet_id)
        .unwrap()
        .wallet_info;
    assert!(
        !info.accounts.standard_bip44_accounts[&0]
            .transactions()
            .contains_key(&scenario.forged.txid()),
        "the forged spend lost to the confirmed competitor"
    );
    assert!(persister
        .get_core_tx_record(scenario.wallet.wallet_id, &scenario.forged.txid())
        .unwrap()
        .is_none());
    assert_eq!(
        coin_set(&info),
        vec![scenario.extra.outpoint],
        "the extra input is released and the loser's change is gone"
    );
    assert_eq!(info.balance.total(), 20_000);
}

#[tokio::test]
async fn should_settle_unsafe_mempool_loser_against_final_winner_after_restart() {
    let scenario = ForgedSpend::build().await;
    let (persister, _dir) = scenario.store(false);
    let mut state = persister.load().unwrap();
    let loaded = state.wallets.get_mut(&scenario.wallet.wallet_id).unwrap();

    let result = loaded
        .wallet_info
        .check_core_transaction(&scenario.winner, block(101), &mut loaded.wallet, true, true)
        .await;

    assert_eq!(
        result.swept_transactions,
        vec![scenario.forged.txid()],
        "a final competitor arriving after restart still removes the forged spend"
    );
    assert_eq!(result.released_outpoints, vec![scenario.extra.outpoint]);
    loaded
        .wallet_info
        .check_core_transaction(
            &scenario.funding,
            block(100),
            &mut loaded.wallet,
            true,
            true,
        )
        .await;
    assert_eq!(
        coin_set(&loaded.wallet_info),
        vec![scenario.extra.outpoint],
        "funding redelivery returns only the released coin"
    );
    assert_eq!(loaded.wallet_info.balance.total(), 20_000);
}

#[tokio::test]
async fn should_report_balances_while_holding_unsafe_mempool_record_after_restart() {
    use key_wallet::managed_account::managed_account_trait::ManagedAccountTrait;
    let scenario = ForgedSpend::build().await;
    let (persister, _dir) = scenario.store(false);

    let mut state = persister.load().unwrap();

    let loaded = state.wallets.get_mut(&scenario.wallet.wallet_id).unwrap();
    assert!(
        loaded.wallet_info.accounts.standard_bip44_accounts[&0]
            .transactions()
            .contains_key(&scenario.forged.txid()),
        "the unsettled forged spend stays in history"
    );
    // Recompute from scratch, then through the late-funding path that revisits stored records.
    loaded.wallet_info.update_balance();
    assert_eq!(
        coin_set(&loaded.wallet_info),
        vec![scenario.change.outpoint]
    );
    assert_eq!(loaded.wallet_info.balance.total(), 1_000);
    loaded
        .wallet_info
        .check_core_transaction(
            &scenario.funding,
            block(100),
            &mut loaded.wallet,
            true,
            true,
        )
        .await;
    loaded.wallet_info.update_balance();
    assert_eq!(
        coin_set(&loaded.wallet_info),
        vec![scenario.change.outpoint],
        "both claimed coins stay reserved while no competitor is final"
    );
    assert_eq!(loaded.wallet_info.balance.total(), 1_000);
    assert_eq!(loaded.wallet_info.balance.confirmed(), 0);
}

#[tokio::test]
async fn should_recompute_unsafe_mempool_record_when_its_funding_arrives_after_restart() {
    use key_wallet::managed_account::managed_account_trait::ManagedAccountTrait;
    let (mut wallet, mut info, address) = wallet_with_receive_address();
    let owned = |value| TxOut {
        value,
        script_pubkey: address.script_pubkey(),
    };
    let funding = spend_of(
        OutPoint::new(Txid::from_byte_array([88; 32]), 0),
        vec![owned(100_000)],
    );
    let coin = OutPoint::new(funding.txid(), 0);
    // Seen before its funding, so stored without the input's amount.
    let forged = spend_of(
        coin,
        vec![
            TxOut {
                value: i64::MAX as u64,
                script_pubkey: foreign_address(89).script_pubkey(),
            },
            owned(1_000),
        ],
    );
    let rival = spend_of(coin, vec![owned(500)]);
    let mut records = info
        .clone()
        .check_core_transaction(
            &forged,
            TransactionContext::Mempool,
            &mut wallet.clone(),
            true,
            true,
        )
        .await
        .new_records;
    records.extend(
        info.check_core_transaction(&rival, TransactionContext::Mempool, &mut wallet, true, true)
            .await
            .new_records,
    );
    assert_eq!(records.len(), 2);
    let (persister, _dir) = store_then_reopen(
        &wallet,
        CoreChangeSet {
            records,
            new_utxos: vec![
                stored_utxo(&forged, 1, address.clone()),
                stored_utxo(&rival, 0, address.clone()),
            ],
            ..Default::default()
        },
    );
    let mut state = persister.load().unwrap();
    let loaded = state.wallets.get_mut(&wallet.wallet_id).unwrap();

    loaded
        .wallet_info
        .check_core_transaction(&funding, block(100), &mut loaded.wallet, true, true)
        .await;
    loaded.wallet_info.update_balance();

    let record =
        &loaded.wallet_info.accounts.standard_bip44_accounts[&0].transactions()[&forged.txid()];
    assert_eq!(
        record.input_details.len(),
        1,
        "the late funding is attributed to the record restored as stored"
    );
    assert_eq!(record.net_amount, 1_000 - 100_000);
    assert_eq!(loaded.wallet_info.balance.total(), 1_500);
}

#[tokio::test]
async fn should_release_extra_input_when_its_surviving_spender_is_removed_after_a_live_sweep() {
    assert_extra_input_follows_its_surviving_spender(true).await;
}

#[tokio::test]
async fn should_release_extra_input_when_its_surviving_spender_is_removed_after_startup_repair() {
    assert_extra_input_follows_its_surviving_spender(false).await;
}

/// The loser spends a shared and an extra coin, a second unconfirmed spend
/// takes the extra coin and a third one, and the confirmed winner spends only
/// the shared coin. `live_sweep` persists the sweep the running wallet reported;
/// otherwise the conflict is stored unsettled and repaired at load.
async fn assert_extra_input_follows_its_surviving_spender(live_sweep: bool) {
    use platform_wallet::changeset::changeset::SweepBatch;

    let (mut wallet, mut info, address) = wallet_with_receive_address();
    let owned = |value| TxOut {
        value,
        script_pubkey: address.script_pubkey(),
    };
    let foreign = |marker, value| TxOut {
        value,
        script_pubkey: foreign_address(marker).script_pubkey(),
    };
    let funding = spend_of(
        OutPoint::new(Txid::from_byte_array([90; 32]), 0),
        vec![owned(100_000), owned(20_000), owned(5_000)],
    );
    let coin = |vout| OutPoint::new(funding.txid(), vout);
    let also_spending = |mut transaction: Transaction, previous_output| {
        transaction.input.push(TxIn {
            previous_output,
            ..Default::default()
        });
        transaction
    };
    let loser = also_spending(spend_of(coin(0), vec![foreign(91, 119_000)]), coin(1));
    let survivor = also_spending(spend_of(coin(1), vec![foreign(92, 24_000)]), coin(2));
    let winner = spend_of(coin(0), vec![owned(99_000)]);
    let final_spend = spend_of(coin(2), vec![foreign(93, 4_000)]);

    let mut records = Vec::new();
    for (transaction, context) in [
        (&funding, block(100)),
        (&loser, TransactionContext::Mempool),
        (&survivor, TransactionContext::Mempool),
    ] {
        records.extend(
            info.check_core_transaction(transaction, context, &mut wallet, true, true)
                .await
                .new_records,
        );
    }
    assert_eq!(
        records.len(),
        3,
        "one wallet records both unconfirmed spenders of the extra coin"
    );
    let settled = info
        .check_core_transaction(&winner, block(101), &mut wallet, true, true)
        .await;
    assert_eq!(settled.swept_transactions, vec![loser.txid()]);
    assert!(
        settled.released_outpoints.is_empty(),
        "the survivor still spends the extra coin"
    );
    let running = info
        .clone()
        .check_core_transaction(&final_spend, block(102), &mut wallet.clone(), true, true)
        .await;
    assert_eq!(
        running.released_outpoints,
        vec![coin(1)],
        "the running wallet frees the extra coin with its surviving spender"
    );
    let coins: Vec<_> = (0..3)
        .map(|vout| stored_utxo(&funding, vout, address.clone()))
        .collect();
    let seen = CoreChangeSet {
        records,
        new_utxos: coins.clone(),
        spent_utxos: coins,
        ..Default::default()
    };
    let settlement = CoreChangeSet {
        records: settled.new_records,
        new_utxos: vec![stored_utxo(&winner, 0, address.clone())],
        sweeps: vec![SweepBatch {
            txids: settled.swept_transactions,
            superseded_by: winner.txid(),
            winner_mined_height: Some(101),
            released_outpoints: settled.released_outpoints,
        }],
        ..Default::default()
    };
    assert_eq!(settlement.records.len(), 1);
    let (persister, _dir) = if live_sweep {
        let (persister, dir) = store_then_reopen(&wallet, seen);
        persister
            .store(
                wallet.wallet_id,
                PlatformWalletChangeSet {
                    core: Some(settlement),
                    ..Default::default()
                },
            )
            .unwrap();
        (persister, dir)
    } else {
        let mut unsettled = seen;
        unsettled.records.extend(settlement.records);
        unsettled.new_utxos.extend(settlement.new_utxos);
        store_then_reopen(&wallet, unsettled)
    };

    let mut state = persister.load().unwrap();
    let loaded = state.wallets.get_mut(&wallet.wallet_id).unwrap();
    let result = loaded
        .wallet_info
        .check_core_transaction(&final_spend, block(102), &mut loaded.wallet, true, true)
        .await;

    assert_eq!(result.swept_transactions, vec![survivor.txid()]);
    assert_eq!(
        result.released_outpoints,
        vec![coin(1)],
        "the extra coin is free once its surviving spender is gone"
    );
    loaded
        .wallet_info
        .check_core_transaction(&funding, block(100), &mut loaded.wallet, true, true)
        .await;
    assert!(
        loaded.wallet_info.accounts.standard_bip44_accounts[&0]
            .utxos
            .contains_key(&coin(1)),
        "funding redelivery returns the released coin"
    );
}

/// A forged, unminable mempool transaction paying the wallet twice for a total
/// past `i64`, spending a foreign coin and one of the wallet's own, as stored.
struct UnrepresentableReceipt {
    wallet: Wallet,
    funding: Transaction,
    /// The wallet's own coin only the forged transaction claims.
    extra: OutPoint,
    forged: Transaction,
    /// A confirmed transaction spending only the foreign coin.
    winner: Transaction,
    persister: SqlitePersister,
    _dir: tempfile::TempDir,
}

impl UnrepresentableReceipt {
    /// `with_winner` stores the confirmed competitor in the same round.
    async fn stored(with_winner: bool) -> Self {
        let (mut wallet, mut info, address) = wallet_with_receive_address();
        let owned = |value| TxOut {
            value,
            script_pubkey: address.script_pubkey(),
        };
        let value = (1_u64 << 62) + 1;
        let funding = spend_of(
            OutPoint::new(Txid::from_byte_array([96; 32]), 0),
            vec![owned(20_000)],
        );
        let extra = OutPoint::new(funding.txid(), 0);
        let shared = OutPoint::new(Txid::from_byte_array([95; 32]), 0);
        let mut forged = spend_of(shared, vec![owned(value), owned(value)]);
        forged.input.push(TxIn {
            previous_output: extra,
            ..Default::default()
        });
        let winner = spend_of(shared, vec![owned(1_000)]);
        let mut records = info
            .clone()
            .check_core_transaction(&funding, block(100), &mut wallet.clone(), true, true)
            .await
            .new_records;
        // Seen before its own input is funded: a debug checker cannot net this
        // record against a known input, a release one wraps and stores it too.
        records.extend(
            info.check_core_transaction(
                &forged,
                TransactionContext::Mempool,
                &mut wallet,
                true,
                true,
            )
            .await
            .new_records,
        );
        assert_eq!(
            records.len(),
            2,
            "the live checker records the forged receipt"
        );
        let running = info
            .clone()
            .check_core_transaction(&winner, block(101), &mut wallet.clone(), true, true)
            .await;
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
        let (persister, _dir) = store_then_reopen(
            &wallet,
            CoreChangeSet {
                records,
                new_utxos,
                spent_utxos: vec![coin],
                ..Default::default()
            },
        );
        Self {
            wallet,
            funding,
            extra,
            forged,
            winner,
            persister,
            _dir,
        }
    }
}

#[tokio::test]
async fn should_settle_unrepresentable_mempool_receipt_against_final_winner_after_restart() {
    let stored = UnrepresentableReceipt::stored(false).await;
    let mut state = stored.persister.load().unwrap();
    let loaded = state.wallets.get_mut(&stored.wallet.wallet_id).unwrap();

    let result = loaded
        .wallet_info
        .check_core_transaction(&stored.winner, block(101), &mut loaded.wallet, true, true)
        .await;

    assert_eq!(
        result.swept_transactions,
        vec![stored.forged.txid()],
        "a final competitor arriving after restart still removes the forged receipt"
    );
    assert_eq!(
        result.released_outpoints,
        vec![stored.extra],
        "the wallet coin only the forged receipt claimed is released"
    );
    loaded
        .wallet_info
        .check_core_transaction(&stored.funding, block(100), &mut loaded.wallet, true, true)
        .await;
    let mut expected = vec![stored.extra, OutPoint::new(stored.winner.txid(), 0)];
    expected.sort();
    assert_eq!(
        coin_set(&loaded.wallet_info),
        expected,
        "the forged outputs are gone and the released coin is back"
    );
    assert_eq!(loaded.wallet_info.balance.total(), 21_000);
}

#[tokio::test]
async fn should_settle_unrepresentable_mempool_receipt_against_persisted_final_winner_on_restart() {
    use key_wallet::managed_account::managed_account_trait::ManagedAccountTrait;
    let stored = UnrepresentableReceipt::stored(true).await;

    let mut state = stored.persister.load().unwrap();

    let info = state
        .wallets
        .remove(&stored.wallet.wallet_id)
        .unwrap()
        .wallet_info;
    assert!(
        !info.accounts.standard_bip44_accounts[&0]
            .transactions()
            .contains_key(&stored.forged.txid()),
        "the forged receipt lost to the stored competitor at load"
    );
    assert!(stored
        .persister
        .get_core_tx_record(stored.wallet.wallet_id, &stored.forged.txid())
        .unwrap()
        .is_none());
    let mut expected = vec![stored.extra, OutPoint::new(stored.winner.txid(), 0)];
    expected.sort();
    assert_eq!(
        coin_set(&info),
        expected,
        "the forged outputs are gone and the coin only it claimed is released"
    );
    assert_eq!(info.balance.total(), 21_000);
}

#[tokio::test]
async fn should_keep_stored_owned_outputs_when_a_record_restored_without_them_is_stored_again() {
    use key_wallet::managed_account::managed_account_trait::ManagedAccountTrait;
    let stored = UnrepresentableReceipt::stored(false).await;
    let txid = stored.forged.txid();
    let mut state = stored.persister.load().unwrap();
    let info = &state
        .wallets
        .get_mut(&stored.wallet.wallet_id)
        .unwrap()
        .wallet_info;
    let restored = info.accounts.standard_bip44_accounts[&0].transactions()[&txid].clone();
    assert!(
        restored.output_details.is_empty(),
        "history holds the record without the amounts that are unsafe to sum"
    );

    stored
        .persister
        .store(
            stored.wallet.wallet_id,
            PlatformWalletChangeSet {
                core: Some(CoreChangeSet {
                    records: vec![restored],
                    ..Default::default()
                }),
                ..Default::default()
            },
        )
        .unwrap();

    let persisted = stored
        .persister
        .get_core_tx_record(stored.wallet.wallet_id, &txid)
        .unwrap()
        .unwrap();
    let owned: Vec<_> = persisted.output_details.iter().map(|d| d.value).collect();
    assert_eq!(
        owned,
        vec![(1 << 62) + 1, (1 << 62) + 1],
        "the store keeps the owned outputs it already knew"
    );
}

/// A record whose input amounts are unsafe to sum goes back into history
/// without them; the store keeps them, also when the record is stored again.
#[tokio::test]
async fn should_restore_mempool_record_with_unsafe_input_amounts_without_its_input_details() {
    use key_wallet::managed_account::managed_account_trait::ManagedAccountTrait;
    let fixture = stage_replay_amounts(TransactionContext::Mempool, &[i64::MAX as u64, 1], 1).await;
    let stored: Vec<u8> = fixture
        .persister
        .lock_conn_for_test()
        .query_row(
            "SELECT txid FROM core_transactions WHERE record_blob IS NOT NULL",
            [],
            |row| row.get(0),
        )
        .unwrap();
    let txid = Txid::from_slice(&stored).unwrap();

    let (_, info) = fixture.load();

    let restored = info.accounts.standard_bip44_accounts[&0].transactions()[&txid].clone();
    assert!(
        restored.input_details.is_empty(),
        "history holds the record without the amounts that are unsafe to sum"
    );
    assert_eq!(info.balance.total(), 20_000);

    fixture
        .persister
        .store(
            fixture.wallet_id,
            PlatformWalletChangeSet {
                core: Some(CoreChangeSet {
                    records: vec![restored],
                    ..Default::default()
                }),
                ..Default::default()
            },
        )
        .unwrap();
    let persisted = fixture
        .persister
        .get_core_tx_record(fixture.wallet_id, &txid)
        .unwrap()
        .unwrap();
    let inputs: Vec<_> = persisted.input_details.iter().map(|d| d.value).collect();
    assert_eq!(
        inputs,
        vec![i64::MAX as u64, 1],
        "the store keeps the input amounts it already knew"
    );
}

#[tokio::test]
async fn should_report_wallet_scoped_unconverged_replay() {
    assert_conflict_restart(ConflictCase::Unconverged).await;
}

fn typed_error(
    error: platform_wallet::changeset::PersistenceError,
) -> platform_wallet_storage::WalletStorageError {
    let platform_wallet::changeset::PersistenceError::Backend { source, .. } = error else {
        panic!("expected backend error")
    };
    *source
        .downcast::<platform_wallet_storage::WalletStorageError>()
        .unwrap()
}

/// How a mempool receipt paying two funding accounts reaches the store.
#[derive(Clone, Copy, Default)]
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
    child: bool,
    /// The receipt also spends a coin of the first account funded only after
    /// the restart, and so does a plain mempool rival that marks it spent.
    unfunded_input: bool,
}

/// One past half of `i64::MAX`: two owned outputs of it cannot be summed as `i64`.
const UNREPRESENTABLE_HALF: u64 = (1 << 62) + 1;

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
        let mut wallet =
            Wallet::new_random(Network::Testnet, WalletAccountCreationOptions::Default).unwrap();
        let mut info = ManagedWalletInfo::from_wallet(&wallet, 0);
        let xpub = wallet.accounts.standard_bip44_accounts[&0].account_xpub;
        let first = info
            .accounts
            .standard_bip44_accounts
            .get_mut(&0)
            .unwrap()
            .next_receive_address(Some(&xpub), true)
            .unwrap();
        let xpub = wallet.accounts.standard_bip32_accounts[&0].account_xpub;
        let second = info
            .accounts
            .standard_bip32_accounts
            .get_mut(&0)
            .unwrap()
            .next_receive_address(Some(&xpub), true)
            .unwrap();
        let pay = |value, address: &Address| TxOut {
            value,
            script_pubkey: address.script_pubkey(),
        };
        let funding = spend_of(
            OutPoint::new(Txid::from_byte_array([96; 32]), 0),
            vec![pay(20_000, &first)],
        );
        let shared = OutPoint::new(Txid::from_byte_array([97; 32]), 0);
        let mut receipt = spend_of(
            shared,
            vec![pay(shape.value, &first), pay(shape.value, &second)],
        );
        if shape.foreign_output {
            receipt
                .output
                .push(pay(i64::MAX as u64, &foreign_address(98)));
        }
        if shape.unfunded_input {
            // Seen before this input is funded, as a debug checker requires.
            receipt.input.push(TxIn {
                previous_output: OutPoint::new(funding.txid(), 0),
                ..Default::default()
            });
        }
        let winner = spend_of(shared, vec![pay(1_000, &first)]);
        let mut records = info
            .check_core_transaction(
                &receipt,
                TransactionContext::Mempool,
                &mut wallet,
                true,
                true,
            )
            .await
            .new_records;
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
        let child = shape
            .child
            .then(|| spend_of(OutPoint::new(receipt.txid(), 0), vec![pay(1_500, &first)]));
        if let Some(child) = &child {
            records.extend(
                info.check_core_transaction(
                    child,
                    TransactionContext::Mempool,
                    &mut wallet,
                    true,
                    true,
                )
                .await
                .new_records,
            );
            spent_utxos.push(new_utxos[0].clone());
            new_utxos.push(stored_utxo(child, 0, first.clone()));
        }
        if shape.unfunded_input {
            let rival = spend_of(OutPoint::new(funding.txid(), 0), vec![pay(500, &first)]);
            records.extend(
                info.check_core_transaction(
                    &rival,
                    TransactionContext::Mempool,
                    &mut wallet,
                    true,
                    true,
                )
                .await
                .new_records,
            );
            new_utxos.push(stored_utxo(&rival, 0, first.clone()));
        }
        let mut running = info.clone();
        running
            .check_core_transaction(&winner, block(101), &mut wallet.clone(), true, true)
            .await;
        assert_eq!(
            traces(&running, &[&receipt, child.as_ref().unwrap_or(&receipt)]),
            Vec::<String>::new(),
            "the running wallet settles every account"
        );
        let (persister, _dir) = store_then_reopen(
            &wallet,
            CoreChangeSet {
                records,
                new_utxos,
                spent_utxos,
                ..Default::default()
            },
        );
        if !shape.unrouted {
            // The pool rows a live sync persists, so load routes each coin to its owner.
            let conn = persister.lock_conn_for_test();
            for (label, address) in [("standard_bip44", &first), ("standard_bip32", &second)] {
                conn.execute(
                    "INSERT INTO core_address_pool \
                     (wallet_id, account_type, account_index, pool_type, address_index, script) \
                     VALUES (?1, ?2, 0, 0, 0, ?3)",
                    rusqlite::params![
                        &wallet.wallet_id[..],
                        label,
                        address.script_pubkey().as_bytes()
                    ],
                )
                .unwrap();
            }
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

/// Every funding account still holding a coin or a record of `transactions`.
fn traces(info: &ManagedWalletInfo, transactions: &[&Transaction]) -> Vec<String> {
    use key_wallet::managed_account::managed_account_trait::ManagedAccountTrait;
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

/// A final competitor arriving after the restart must leave no coin of the
/// receipt in any account, in the running wallet and after the next load.
async fn assert_two_account_receipt_settles_after_restart(shape: TwoAccountShape) {
    use platform_wallet::changeset::changeset::SweepBatch;

    let stored = TwoAccountReceipt::stored(shape).await;
    let losers = [
        &stored.receipt,
        stored.child.as_ref().unwrap_or(&stored.receipt),
    ];
    let mut state = stored.persister.load().unwrap();
    let loaded = state.wallets.get_mut(&stored.wallet.wallet_id).unwrap();

    let result = loaded
        .wallet_info
        .check_core_transaction(&stored.winner, block(101), &mut loaded.wallet, true, true)
        .await;

    assert!(result.swept_transactions.contains(&stored.receipt.txid()));
    assert_eq!(
        traces(&loaded.wallet_info, &losers),
        Vec::<String>::new(),
        "no account of the running wallet keeps a coin of the removed receipt"
    );
    assert_eq!(loaded.wallet_info.balance.total(), 1_000);

    stored
        .persister
        .store(
            stored.wallet.wallet_id,
            PlatformWalletChangeSet {
                core: Some(CoreChangeSet {
                    records: result.new_records,
                    new_utxos: vec![stored_utxo(&stored.winner, 0, stored.first.clone())],
                    sweeps: vec![SweepBatch {
                        txids: result.swept_transactions,
                        superseded_by: stored.winner.txid(),
                        winner_mined_height: Some(101),
                        released_outpoints: result.released_outpoints,
                    }],
                    ..Default::default()
                }),
                ..Default::default()
            },
        )
        .unwrap();
    let mut state = stored.persister.load().unwrap();
    let reloaded = &state
        .wallets
        .get_mut(&stored.wallet.wallet_id)
        .unwrap()
        .wallet_info;
    assert_eq!(
        traces(reloaded, &losers),
        Vec::<String>::new(),
        "the next load holds no coin of the removed receipt either"
    );
    assert_eq!(reloaded.balance.total(), 1_000);
}

#[tokio::test]
async fn should_settle_two_account_mempool_receipt_against_final_winner_after_restart() {
    assert_two_account_receipt_settles_after_restart(TwoAccountShape {
        value: 5_000,
        ..Default::default()
    })
    .await;
}

#[tokio::test]
async fn should_settle_unrepresentable_two_account_receipt_against_final_winner_after_restart() {
    assert_two_account_receipt_settles_after_restart(TwoAccountShape {
        value: UNREPRESENTABLE_HALF,
        ..Default::default()
    })
    .await;
}

#[tokio::test]
async fn should_settle_unrepresentable_two_account_receipt_stored_under_its_other_account() {
    assert_two_account_receipt_settles_after_restart(TwoAccountShape {
        value: UNREPRESENTABLE_HALF,
        recorded_under_first: true,
        ..Default::default()
    })
    .await;
}

#[tokio::test]
async fn should_settle_two_account_receipt_with_unrepresentable_foreign_output_after_restart() {
    assert_two_account_receipt_settles_after_restart(TwoAccountShape {
        value: 7_000,
        foreign_output: true,
        ..Default::default()
    })
    .await;
}

/// The child is recorded only in the account whose coin it spends, which is
/// not the one the stored receipt names.
#[tokio::test]
async fn should_settle_child_of_unrepresentable_two_account_receipt_after_restart() {
    assert_two_account_receipt_settles_after_restart(TwoAccountShape {
        value: UNREPRESENTABLE_HALF,
        child: true,
        ..Default::default()
    })
    .await;
}

/// Without pool rows load parks both coins in the first account, while the
/// stored receipt names the second.
#[tokio::test]
async fn should_settle_unrepresentable_two_account_receipt_with_unrouted_coins_after_restart() {
    assert_two_account_receipt_settles_after_restart(TwoAccountShape {
        value: UNREPRESENTABLE_HALF,
        unrouted: true,
        ..Default::default()
    })
    .await;
}

/// Each account holds the receipt without the amounts that are unsafe to
/// sum, so reading balances and attributing a late funding stay in range.
#[tokio::test]
async fn should_recompute_unrepresentable_two_account_receipt_when_its_funding_arrives_after_restart(
) {
    use key_wallet::managed_account::managed_account_trait::ManagedAccountTrait;
    let stored = TwoAccountReceipt::stored(TwoAccountShape {
        value: UNREPRESENTABLE_HALF,
        unfunded_input: true,
        ..Default::default()
    })
    .await;
    let txid = stored.receipt.txid();
    let mut state = stored.persister.load().unwrap();
    let loaded = state.wallets.get_mut(&stored.wallet.wallet_id).unwrap();
    let copies = |info: &ManagedWalletInfo| {
        let accounts = &info.accounts;
        (
            accounts.standard_bip44_accounts[&0].transactions()[&txid].clone(),
            accounts.standard_bip32_accounts[&0].transactions()[&txid].clone(),
        )
    };
    let (first, second) = copies(&loaded.wallet_info);
    assert!(
        first.output_details.is_empty() && second.output_details.is_empty(),
        "each account holds the receipt without the amounts unsafe to sum"
    );
    assert_eq!(
        loaded.wallet_info.balance.total(),
        2 * UNREPRESENTABLE_HALF + 500,
        "both coins are credited"
    );

    loaded
        .wallet_info
        .check_core_transaction(&stored.funding, block(100), &mut loaded.wallet, true, true)
        .await;
    loaded.wallet_info.update_balance();

    let (first, second) = copies(&loaded.wallet_info);
    assert_eq!(
        (first.input_details.len(), first.net_amount),
        (1, -20_000),
        "the late funding is attributed in the account that owns it"
    );
    assert!(second.input_details.is_empty());
    assert_eq!(
        loaded.wallet_info.balance.total(),
        2 * UNREPRESENTABLE_HALF + 500
    );
}

/// A stored mempool record spending `values`, beaten after the restart by a
/// final transaction the wallet has no other interest in: no owned output,
/// and no input the wallet can attribute. Whatever the checker reports is
/// persisted as a sync would, and the store is loaded again.
async fn assert_recordless_final_competitor_settles_after_restart(values: &[u64]) {
    use platform_wallet::changeset::changeset::SweepBatch;

    let fixture = stage_replay_amounts(TransactionContext::Mempool, values, 1).await;
    let stored: Vec<u8> = fixture
        .persister
        .lock_conn_for_test()
        .query_row(
            "SELECT txid FROM core_transactions WHERE record_blob IS NOT NULL",
            [],
            |row| row.get(0),
        )
        .unwrap();
    let txid = Txid::from_slice(&stored).unwrap();
    let loser = fixture
        .persister
        .get_core_tx_record(fixture.wallet_id, &txid)
        .unwrap()
        .unwrap();
    let address = loser.output_details[0].address.clone().unwrap();
    // The loser's own output, as the round that stored the record wrote it.
    fixture
        .persister
        .store(
            fixture.wallet_id,
            PlatformWalletChangeSet {
                core: Some(CoreChangeSet {
                    new_utxos: vec![stored_utxo(&loser.transaction, 0, address)],
                    ..Default::default()
                }),
                ..Default::default()
            },
        )
        .unwrap();
    let (mut wallet, mut info) = fixture.load();
    assert_eq!(traces(&info, &[&loser.transaction]).len(), 1);

    let winner = spend_of(
        loser.transaction.input[0].previous_output,
        vec![TxOut {
            value: 900,
            script_pubkey: foreign_address(77).script_pubkey(),
        }],
    );
    let result = info
        .check_core_transaction(&winner, block(101), &mut wallet, true, true)
        .await;

    assert!(
        !result.is_relevant && result.new_records.is_empty(),
        "the competitor leaves no record of its own"
    );
    assert_eq!(result.swept_transactions, vec![txid]);
    assert_eq!(
        result.released_outpoints,
        vec![loser.transaction.input[1].previous_output],
        "the input only the loser claimed is released"
    );
    assert_eq!(traces(&info, &[&loser.transaction]), Vec::<String>::new());
    fixture
        .persister
        .store(
            fixture.wallet_id,
            PlatformWalletChangeSet {
                core: Some(CoreChangeSet {
                    sweeps: vec![SweepBatch {
                        txids: result.swept_transactions,
                        superseded_by: winner.txid(),
                        winner_mined_height: Some(101),
                        released_outpoints: result.released_outpoints,
                    }],
                    ..Default::default()
                }),
                ..Default::default()
            },
        )
        .unwrap();

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

#[tokio::test]
async fn should_settle_safe_mempool_record_against_recordless_final_competitor_after_restart() {
    assert_recordless_final_competitor_settles_after_restart(&[5_000, 1]).await;
}

#[tokio::test]
async fn should_settle_unsafe_input_mempool_record_against_recordless_final_competitor_after_restart(
) {
    assert_recordless_final_competitor_settles_after_restart(&[i64::MAX as u64, 1]).await;
}

/// How a mempool spend of two forged receipts and two real coins reaches the store.
#[derive(Clone, Copy, Default)]
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
        let mut wallet =
            Wallet::new_random(Network::Testnet, WalletAccountCreationOptions::Default).unwrap();
        let mut info = ManagedWalletInfo::from_wallet(&wallet, 0);
        let xpub = wallet.accounts.standard_bip44_accounts[&0].account_xpub;
        let first = info
            .accounts
            .standard_bip44_accounts
            .get_mut(&0)
            .unwrap()
            .next_receive_address(Some(&xpub), true)
            .unwrap();
        let xpub = wallet.accounts.standard_bip32_accounts[&0].account_xpub;
        let second = info
            .accounts
            .standard_bip32_accounts
            .get_mut(&0)
            .unwrap()
            .next_receive_address(Some(&xpub), true)
            .unwrap();
        let pay = |value, address: &Address| TxOut {
            value,
            script_pubkey: address.script_pubkey(),
        };
        let outside = |byte| OutPoint::new(Txid::from_byte_array([byte; 32]), 0);
        let funding = spend_of(outside(73), vec![pay(20_000, &first), pay(30_000, &first)]);
        let (shared, extra) = (
            OutPoint::new(funding.txid(), 0),
            OutPoint::new(funding.txid(), 1),
        );
        let receipts =
            [71, 72].map(|byte| spend_of(outside(byte), vec![pay(shape.forged, &first)]));
        let mut outputs = vec![pay(1_000, &first)];
        if shape.two_accounts {
            outputs.push(pay(1_500, &second));
        }
        let mut spend = spend_of(shared, outputs);
        let more = [
            extra,
            OutPoint::new(receipts[0].txid(), 0),
            OutPoint::new(receipts[1].txid(), 0),
        ];
        spend.input.extend(more.map(|previous_output| TxIn {
            previous_output,
            ..Default::default()
        }));
        let winner = spend_of(shared, vec![pay(49_000, &foreign_address(74))]);

        let mempool = TransactionContext::Mempool;
        let mut records = Vec::new();
        let mut new_utxos = Vec::new();
        let mut spent_utxos = Vec::new();
        if !shape.late_funding {
            records.extend(
                info.check_core_transaction(&funding, block(100), &mut wallet, true, true)
                    .await
                    .new_records,
            );
            new_utxos.extend([0, 1].map(|vout| stored_utxo(&funding, vout, first.clone())));
            spent_utxos.extend(new_utxos.clone());
        }
        // Each receipt is seen by a wallet that does not hold the spend: a
        // debug checker cannot attribute both to one record.
        for receipt in &receipts {
            records.extend(
                info.clone()
                    .check_core_transaction(
                        receipt,
                        mempool.clone(),
                        &mut wallet.clone(),
                        true,
                        true,
                    )
                    .await
                    .new_records,
            );
            let coin = stored_utxo(receipt, 0, first.clone());
            new_utxos.push(coin.clone());
            spent_utxos.push(coin);
        }
        let mut slices = info
            .check_core_transaction(&spend, mempool.clone(), &mut wallet, true, true)
            .await
            .new_records;
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
            let rival = spend_of(shared, vec![pay(500, &first)]);
            records.extend(
                info.check_core_transaction(&rival, mempool, &mut wallet, true, true)
                    .await
                    .new_records,
            );
            new_utxos.push(stored_utxo(&rival, 0, first.clone()));
        }
        let (persister, _dir) = store_then_reopen(
            &wallet,
            CoreChangeSet {
                records,
                new_utxos,
                spent_utxos,
                ..Default::default()
            },
        );
        if shape.two_accounts {
            // The pool rows a live sync persists, so load routes each coin to its owner.
            let conn = persister.lock_conn_for_test();
            for (label, address) in [("standard_bip44", &first), ("standard_bip32", &second)] {
                conn.execute(
                    "INSERT INTO core_address_pool \
                     (wallet_id, account_type, account_index, pool_type, address_index, script) \
                     VALUES (?1, ?2, 0, 0, 0, ?3)",
                    rusqlite::params![
                        &wallet.wallet_id[..],
                        label,
                        address.script_pubkey().as_bytes()
                    ],
                )
                .unwrap();
            }
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
async fn assert_forged_input_spend_settles_after_restart(shape: ForgedInputShape) {
    use platform_wallet::changeset::changeset::SweepBatch;

    let stored = ForgedInputSpend::stored(shape).await;
    let outputs = 1_000 + if shape.two_accounts { 1_500 } else { 0 };
    let mut state = stored.persister.load().unwrap();
    let loaded = state.wallets.get_mut(&stored.wallet.wallet_id).unwrap();
    assert_eq!(
        loaded.wallet_info.balance.total(),
        outputs,
        "only the spend's own outputs are spendable after the restart"
    );

    let result = loaded
        .wallet_info
        .check_core_transaction(&stored.winner, block(101), &mut loaded.wallet, true, true)
        .await;

    assert_eq!(result.swept_transactions, vec![stored.spend.txid()]);
    assert_eq!(
        traces(&loaded.wallet_info, &[&stored.spend]),
        Vec::<String>::new(),
        "no account of the running wallet keeps a coin of the removed spend"
    );
    stored
        .persister
        .store(
            stored.wallet.wallet_id,
            PlatformWalletChangeSet {
                core: Some(CoreChangeSet {
                    records: result.new_records,
                    sweeps: vec![SweepBatch {
                        txids: result.swept_transactions,
                        superseded_by: stored.winner.txid(),
                        winner_mined_height: Some(101),
                        released_outpoints: result.released_outpoints,
                    }],
                    ..Default::default()
                }),
                ..Default::default()
            },
        )
        .unwrap();

    let mut state = stored.persister.load().unwrap();
    let reloaded = &state
        .wallets
        .get_mut(&stored.wallet.wallet_id)
        .unwrap()
        .wallet_info;
    assert_eq!(traces(reloaded, &[&stored.spend]), Vec::<String>::new());
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

/// Control: with receipts that are safe to sum the spend is an ordinary record.
#[tokio::test]
async fn should_settle_mempool_spend_against_final_outside_spend_of_its_coin_after_restart() {
    assert_forged_input_spend_settles_after_restart(ForgedInputShape {
        forged: 7_000,
        ..Default::default()
    })
    .await;
}

#[tokio::test]
async fn should_settle_unsafe_input_spend_against_final_outside_spend_of_its_coin_after_restart() {
    assert_forged_input_spend_settles_after_restart(ForgedInputShape {
        forged: UNREPRESENTABLE_HALF,
        ..Default::default()
    })
    .await;
}

#[tokio::test]
async fn should_settle_two_account_unsafe_input_spend_against_final_outside_spend_after_restart() {
    assert_forged_input_spend_settles_after_restart(ForgedInputShape {
        forged: UNREPRESENTABLE_HALF,
        two_accounts: true,
        ..Default::default()
    })
    .await;
}

/// The real coins are funded only after the restart. Returns the spend's
/// attributed inputs, its net, and the wallet total once the funding is in.
async fn late_funding_of_forged_input_spend(forged: u64) -> (usize, i64, u64) {
    use key_wallet::managed_account::managed_account_trait::ManagedAccountTrait;
    let stored = ForgedInputSpend::stored(ForgedInputShape {
        forged,
        late_funding: true,
        ..Default::default()
    })
    .await;
    let mut state = stored.persister.load().unwrap();
    let loaded = state.wallets.get_mut(&stored.wallet.wallet_id).unwrap();
    assert_eq!(loaded.wallet_info.balance.total(), 1_500);

    loaded
        .wallet_info
        .check_core_transaction(&stored.funding, block(100), &mut loaded.wallet, true, true)
        .await;
    loaded.wallet_info.update_balance();

    let account = &loaded.wallet_info.accounts.standard_bip44_accounts[&0];
    let record = &account.transactions()[&stored.spend.txid()];
    (
        record.input_details.len(),
        record.net_amount,
        loaded.wallet_info.balance.total(),
    )
}

/// A restored record holds no input amounts and marks nothing spent by
/// itself. The coin the rival also spends is attributed to it, and that one
/// amount is all its net sums; the coin only it spends is credited when its
/// funding arrives, where a replayed record would keep it reserved.
#[tokio::test]
async fn should_attribute_late_funding_to_restored_unsafe_input_spend_without_overflow() {
    assert_eq!(
        late_funding_of_forged_input_spend(UNREPRESENTABLE_HALF).await,
        (1, 1_000 - 20_000, 1_500 + 30_000)
    );
}

/// Control: replayed as an ordinary record, the same spend also knows its
/// receipts and keeps both real coins reserved.
#[tokio::test]
async fn should_attribute_late_funding_to_replayed_mempool_spend() {
    assert_eq!(
        late_funding_of_forged_input_spend(7_000).await,
        (4, 1_000 - 50_000 - 14_000, 1_500)
    );
}

/// How the store reads once a load has settled a conflict that removes a
/// DashPay payment's transaction.
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
    policy: platform_wallet_storage::LoadPolicy,
) -> SettledPayment {
    use dpp::prelude::Identifier;
    use platform_wallet::changeset::{IdentityChangeSet, IdentityEntry};
    use platform_wallet::wallet::identity::{IdentityStatus, PaymentEntry};
    use platform_wallet_storage::LoadPolicy;

    let (mut wallet, mut info, address) = wallet_with_receive_address();
    let pay = |value, address: &Address| TxOut {
        value,
        script_pubkey: address.script_pubkey(),
    };
    let funding = spend_of(
        OutPoint::new(Txid::from_byte_array([91; 32]), 0),
        vec![pay(100_000, &address)],
    );
    let mut records = info
        .check_core_transaction(&funding, block(100), &mut wallet, true, true)
        .await
        .new_records;
    let coin = OutPoint::new(funding.txid(), 0);
    let loser = spend_of(
        coin,
        vec![pay(59_000, &foreign_address(92)), pay(40_000, &address)],
    );
    let mempool = TransactionContext::Mempool;
    records.extend(
        info.check_core_transaction(&loser, mempool, &mut wallet, true, true)
            .await
            .new_records,
    );
    let winner = spend_of(coin, vec![pay(99_000, &address)]);
    let won = info
        .check_core_transaction(&winner, block(101), &mut wallet, true, true)
        .await;
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
    persister
        .store(
            wallet.wallet_id,
            PlatformWalletChangeSet {
                identities: Some(identities),
                ..Default::default()
            },
        )
        .unwrap();
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
    let count = |persister: &SqlitePersister, sql: &str| -> i64 {
        let conn = persister.lock_conn_for_test();
        conn.query_row(sql, [], |row| row.get(0)).unwrap()
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

fn sent_payment() -> platform_wallet::wallet::identity::PaymentEntry {
    use platform_wallet::wallet::identity::PaymentEntry;
    PaymentEntry::new_sent(dpp::prelude::Identifier::from([0xB7; 32]), 59_000, None)
}

#[tokio::test]
async fn should_fail_pending_sent_payment_whose_transaction_load_settlement_removes() {
    use platform_wallet::wallet::identity::PaymentStatus::{Failed, Pending};
    use platform_wallet_storage::LoadPolicy;

    let settled = settle_payment_at_load(sent_payment(), Pending, LoadPolicy::Strict).await;

    assert_eq!(settled.loser_rows, 0, "the load removed the losing record");
    assert_eq!(
        (settled.returned, settled.reopened),
        (Failed, Failed),
        "the payment reads Failed on the settling load and on the next"
    );
    assert_eq!(settled.stored_verdicts, 1);
    assert_eq!(settled.unrelated, Pending, "no other payment moves");
}

/// An InstantSend-locked payment reads `Confirmed` until a final competitor removes it.
#[tokio::test]
async fn should_fail_confirmed_sent_payment_whose_transaction_load_settlement_removes() {
    use platform_wallet::wallet::identity::PaymentStatus::{Confirmed, Failed};
    use platform_wallet_storage::LoadPolicy;

    let settled = settle_payment_at_load(sent_payment(), Confirmed, LoadPolicy::Strict).await;

    assert_eq!((settled.returned, settled.reopened), (Failed, Failed));
}

/// A received payment's status is settled when it is recorded; a sweep of the
/// transaction it is filed under does not move it.
#[tokio::test]
async fn should_leave_received_payment_alone_when_load_settlement_removes_its_transaction() {
    use platform_wallet::wallet::identity::{PaymentEntry, PaymentStatus::Confirmed};
    use platform_wallet_storage::LoadPolicy;

    let contact = dpp::prelude::Identifier::from([0xB7; 32]);
    let received = PaymentEntry::new_received(contact, 59_000, None);
    let settled = settle_payment_at_load(received, Confirmed, LoadPolicy::Strict).await;

    assert_eq!(settled.loser_rows, 0);
    assert_eq!((settled.returned, settled.reopened), (Confirmed, Confirmed));
    assert_eq!(settled.stored_verdicts, 0);
}

/// A recovery load returns the settled state and writes none of it: the
/// verdict rolls back with the record removal it follows from.
#[tokio::test]
async fn should_roll_back_sent_payment_verdict_with_a_recovery_load() {
    use platform_wallet::wallet::identity::PaymentStatus::{Failed, Pending};
    use platform_wallet_storage::LoadPolicy;

    let settled = settle_payment_at_load(sent_payment(), Pending, LoadPolicy::Recovery).await;

    assert_eq!(
        settled.returned, Failed,
        "the returned state is the settled one"
    );
    assert_eq!(
        (settled.loser_rows, settled.stored_verdicts),
        (1, 0),
        "neither the removal nor the verdict reached the store"
    );
    assert_eq!(
        settled.reopened, Failed,
        "the next strict load settles again"
    );
}
