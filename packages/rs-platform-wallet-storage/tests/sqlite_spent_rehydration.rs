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
                        spent_claim_batches: vec![platform_wallet::changeset::SpentClaimBatch {
                            claimed: info.spent_outpoint_claims().into_iter().collect(),
                            released: vec![],
                        }],
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
                    spent_claim_batches: vec![platform_wallet::changeset::SpentClaimBatch {
                        claimed: info.spent_outpoint_claims().into_iter().collect(),
                        released: vec![],
                    }],
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
    // Release controls carry a known loser claim; an unknown durable claim must stay held.
    if expect_released {
        persister
            .lock_conn_for_test()
            .execute(
                "UPDATE core_utxos SET spent_in_txid = ?1 WHERE wallet_id = ?2 AND outpoint = ?3",
                rusqlite::params![
                    loser.txid().as_byte_array().as_slice(),
                    wallet.wallet_id.as_slice(),
                    &extra_key
                ],
            )
            .unwrap();
    }
    if let ConflictCase::UnknownClaim { height, .. } = case {
        persister.lock_conn_for_test().execute(
            "UPDATE core_spent_claims SET claimant = NULL WHERE wallet_id = ?1 AND outpoint = ?2",
            rusqlite::params![wallet.wallet_id.as_slice(), &extra_key],
        ).unwrap();
        persister.lock_conn_for_test().execute(
            "UPDATE core_utxos SET spent_in_txid = NULL, winner_mined_height = ?1 WHERE wallet_id = ?2 AND outpoint = ?3",
            rusqlite::params![height, wallet.wallet_id.as_slice(), &extra_key],
        ).unwrap();
    }
    if matches!(case, ConflictCase::MalformedClaim) {
        persister
            .lock_conn_for_test()
            .execute_batch("PRAGMA ignore_check_constraints = ON")
            .unwrap();
        persister.lock_conn_for_test().execute(
            "UPDATE core_spent_claims SET claimant = zeroblob(31) WHERE wallet_id = ?1 AND outpoint = ?2",
            rusqlite::params![wallet.wallet_id.as_slice(), &extra_key],
        ).unwrap();
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
        conn.execute(
            "UPDATE core_spent_claims SET claimant = ?1 WHERE wallet_id = ?2 AND outpoint = ?3",
            rusqlite::params![
                missing_winner.as_byte_array().as_slice(),
                wallet.wallet_id.as_slice(),
                &extra_key
            ],
        )
        .unwrap();
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
                            conn.execute("UPDATE core_spent_claims SET claimant = NULL", [])
                                .unwrap();
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

async fn assert_replay_amount_rejected(values: &[u64], received: u64) {
    use key_wallet::managed_account::transaction_record::{InputDetail, OutputDetail, OutputRole};
    use platform_wallet_storage::{sqlite::schema::blob, WalletStorageError};
    let fixture = Fixture::with_height_only_funding(block(200)).await;
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
    conn.execute("INSERT INTO core_transactions (wallet_id, txid, height, finalized, record_blob) VALUES (?1, ?2, 200, 0, ?3)", rusqlite::params![fixture.wallet_id.as_slice(), record.txid.as_byte_array().as_slice(), blob::encode(&record).unwrap()]).unwrap();
    drop(conn);
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
