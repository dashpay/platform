//! Wallet-level drain lifecycle tests: registration, resume, completion and
//! status over a recording persistence backend. The Type18 network path is
//! covered at the manager level in `fund_from_asset_lock.rs`.

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use dashcore::blockdata::transaction::special_transaction::asset_lock::AssetLockPayload;
use dashcore::blockdata::transaction::special_transaction::TransactionPayload;
use dashcore::{Network, OutPoint, ScriptBuf, Transaction, TxOut};
use dpp::identity::state_transition::asset_lock_proof::chain::ChainAssetLockProof;
use dpp::prelude::AssetLockProof;
use dpp::version::PlatformVersion;
use key_wallet::account::StandardAccountType;
use key_wallet::wallet::managed_wallet_info::asset_lock_builder::AssetLockFundingType;
use key_wallet::wallet::ManagedWalletInfo;
use key_wallet::Wallet;
use tokio::sync::Notify;

use crate::broadcaster::SpvBroadcaster;
use crate::changeset::{
    AssetLockEntry, ClientStartState, ClientWalletStartState, PersistenceCapabilities,
    PersistenceError, PlatformWalletChangeSet, PlatformWalletPersistence,
};
use crate::error::PlatformWalletError;
use crate::events::PlatformEventManager;
use crate::spv::SpvRuntime;
use crate::test_support::{funded_wallet_manager, WalletSigner};
use crate::wallet::asset_lock::tracked::{AssetLockStatus, TrackedAssetLock};
use crate::wallet::asset_lock::{
    AssetLockFunding, AssetLockOperationContext, AssetLockOperationState,
};
use crate::wallet::PlatformWallet;

use super::keys::AccountViewingKeys;
use super::{FileBackedShieldedStore, NetworkShieldedCoordinator, OrchardKeySet};

const OPERATION_ID: [u8; 16] = [0x33; 16];

/// A pending drain lock (Core finality proven, Platform side unresolved)
/// tagged with [`OPERATION_ID`], plus the viewing keys of its recipient.
fn drain_lock_fixture() -> (TrackedAssetLock, BTreeMap<u32, AccountViewingKeys>) {
    let keys = OrchardKeySet::from_seed(&[0x42; 32], Network::Testnet, 0)
        .unwrap()
        .viewing_keys();
    let secret = dpp::secp256k1::SecretKey::from_secret_bytes([0x11; 32]).unwrap();
    let public = dashcore::PublicKey::new(dpp::secp256k1::PublicKey::from_secret_key(&secret));
    let transaction = Transaction {
        version: 3,
        lock_time: 0,
        input: vec![],
        output: vec![],
        special_transaction_payload: Some(TransactionPayload::AssetLockPayloadType(
            AssetLockPayload {
                version: 1,
                credit_outputs: vec![TxOut {
                    value: 10_000,
                    script_pubkey: ScriptBuf::new_p2pkh(&public.pubkey_hash()),
                }],
            },
        )),
    };
    let out_point = OutPoint::new(transaction.txid(), 0);
    let proof = AssetLockProof::Chain(ChainAssetLockProof {
        core_chain_locked_height: 42,
        out_point,
    });
    (
        TrackedAssetLock {
            operation_id: Some(OPERATION_ID),
            out_point,
            transaction,
            account_index: 0,
            funding_type: AssetLockFundingType::AssetLockShieldedAddressTopUp,
            identity_index: 0,
            amount: 10_000,
            status: AssetLockStatus::RecoveredFromChain,
            proof: Some(proof),
            shielded_recipient: None,
        },
        BTreeMap::from([(0, keys)]),
    )
}

/// Buffered backend with inline-commit and failure switches. `committed`
/// is what a restart restores; `staged` is what a flush commits.
struct RecoveryPersistence {
    wallet: Wallet,
    info: ManagedWalletInfo,
    committed: Mutex<BTreeMap<OutPoint, AssetLockEntry>>,
    staged: Mutex<BTreeMap<OutPoint, AssetLockEntry>>,
    fail_store: AtomicBool,
    fail_flush: AtomicBool,
    /// Fails the next store that carries an asset-lock row, and only it.
    fail_next_lock_store: AtomicBool,
    /// Fails the next flush only.
    fail_next_flush: AtomicBool,
    commits_inline: AtomicBool,
    fail_load: AtomicBool,
    records_recipients: AtomicBool,
}

impl PlatformWalletPersistence for RecoveryPersistence {
    fn store_commits_inline(&self) -> bool {
        self.commits_inline.load(Ordering::SeqCst)
    }
    fn persistence_capabilities(&self) -> PersistenceCapabilities {
        if self.records_recipients.load(Ordering::SeqCst) {
            PersistenceCapabilities::WALLET_DRAIN
                .union(PersistenceCapabilities::ASSET_LOCK_SHIELDED_RECIPIENTS)
        } else {
            PersistenceCapabilities::WALLET_DRAIN
        }
    }
    fn store(
        &self,
        _wallet_id: [u8; 32],
        cs: PlatformWalletChangeSet,
    ) -> Result<(), PersistenceError> {
        if self.fail_store.load(Ordering::SeqCst)
            || (cs
                .asset_locks
                .as_ref()
                .is_some_and(|locks| !locks.asset_locks.is_empty())
                && self.fail_next_lock_store.swap(false, Ordering::SeqCst))
        {
            return Err(PersistenceError::backend("store failed"));
        }
        if let Some(cs) = cs.asset_locks {
            let mut staged = self.staged.lock().unwrap();
            staged.extend(cs.asset_locks);
            // A removal cancels a row that is still only staged; committed
            // rows are kept so tests can read what a broadcast saw.
            for out_point in &cs.removed {
                staged.remove(out_point);
            }
        }
        if self.store_commits_inline() {
            self.commit_staged();
        }
        Ok(())
    }
    fn flush(&self, _wallet_id: [u8; 32]) -> Result<(), PersistenceError> {
        if self.fail_flush.load(Ordering::SeqCst)
            || self.fail_next_flush.swap(false, Ordering::SeqCst)
        {
            return Err(PersistenceError::backend("flush failed"));
        }
        self.commit_staged();
        Ok(())
    }
    fn load(&self) -> Result<ClientStartState, PersistenceError> {
        if self.fail_load.load(Ordering::SeqCst) {
            return Err(PersistenceError::backend("load failed"));
        }
        let locks = self
            .committed
            .lock()
            .unwrap()
            .iter()
            .map(|(out_point, row)| {
                (
                    *out_point,
                    TrackedAssetLock {
                        operation_id: row.operation_id,
                        out_point: *out_point,
                        transaction: row.transaction.clone(),
                        account_index: row.account_index,
                        funding_type: row.funding_type,
                        identity_index: row.identity_index,
                        amount: row.amount_duffs,
                        status: row.status.clone(),
                        proof: row.proof.clone(),
                        shielded_recipient: row.shielded_recipient,
                    },
                )
            })
            .collect();
        Ok(ClientStartState {
            wallets: BTreeMap::from([(
                self.wallet.compute_wallet_id(),
                ClientWalletStartState {
                    dashpay_backfill: Default::default(),
                    wallet: self.wallet.clone(),
                    wallet_info: self.info.clone(),
                    identity_manager: Default::default(),
                    unused_asset_locks: BTreeMap::from([(0, locks)]),
                    unconfirmed_outgoing_txs: vec![],
                },
            )]),
            ..Default::default()
        })
    }
}

impl RecoveryPersistence {
    fn commit_staged(&self) {
        self.committed
            .lock()
            .unwrap()
            .extend(self.staged.lock().unwrap().clone());
    }

    fn committed_status(&self, out_point: &OutPoint) -> AssetLockStatus {
        self.committed.lock().unwrap()[out_point].status.clone()
    }
}

struct TestDirectory(std::path::PathBuf);
impl Drop for TestDirectory {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

async fn wallet_fixture(
    lock: &TrackedAssetLock,
    keys: BTreeMap<u32, AccountViewingKeys>,
) -> (
    PlatformWallet,
    Arc<RecoveryPersistence>,
    NetworkShieldedCoordinator,
    TestDirectory,
) {
    let (wallet, persistence, coordinator, dir, _) = signed_wallet_fixture(lock, keys).await;
    (wallet, persistence, coordinator, dir)
}

/// [`wallet_fixture`] plus a signer over the fixture wallet's own seed, for
/// tests whose funding has to build and sign a fresh Core lock.
async fn signed_wallet_fixture(
    lock: &TrackedAssetLock,
    keys: BTreeMap<u32, AccountViewingKeys>,
) -> (
    PlatformWallet,
    Arc<RecoveryPersistence>,
    NetworkShieldedCoordinator,
    TestDirectory,
    WalletSigner,
) {
    let (manager, id, generation, signer) =
        funded_wallet_manager(StandardAccountType::BIP44Account).await;
    let (stored_wallet, info) = {
        let manager = manager.read().await;
        let (wallet, info) = manager.get_wallet_and_info(&id).unwrap();
        (wallet.clone(), info.core_wallet.clone())
    };
    let persistence = Arc::new(RecoveryPersistence {
        wallet: stored_wallet,
        info,
        committed: Mutex::new(BTreeMap::from([(lock.out_point, lock.into())])),
        staged: Mutex::new(BTreeMap::new()),
        fail_store: AtomicBool::new(false),
        fail_flush: AtomicBool::new(false),
        fail_next_lock_store: AtomicBool::new(false),
        fail_next_flush: AtomicBool::new(false),
        // Drains admit inline-committing backends only; the buffered mode
        // exists for `should_refuse_drain_on_a_buffered_persistence_backend`.
        commits_inline: AtomicBool::new(true),
        fail_load: AtomicBool::new(false),
        records_recipients: AtomicBool::new(true),
    });
    let sdk = Arc::new(
        dash_sdk::SdkBuilder::new_mock()
            .with_version(PlatformVersion::latest())
            .build()
            .unwrap(),
    );
    let spv = Arc::new(SpvRuntime::new(
        manager.clone(),
        Arc::new(PlatformEventManager::new(vec![])),
    ));
    let wallet = PlatformWallet::new(
        sdk.clone(),
        id,
        manager.clone(),
        generation,
        Arc::new(Notify::new()),
        persistence.clone(),
        Arc::new(SpvBroadcaster::new(spv)),
        Arc::new(std::sync::atomic::AtomicBool::new(false)),
        crate::changeset::DurableCursors::default(),
    );
    manager
        .write()
        .await
        .get_wallet_info_mut(&id)
        .unwrap()
        .tracked_asset_locks
        .insert(lock.out_point, lock.clone());
    *wallet.shielded_keys.write().await = Some(keys);
    let dir =
        TestDirectory(std::env::temp_dir().join(format!("wallet_drain_{}", rand::random::<u64>())));
    std::fs::create_dir_all(&dir.0).unwrap();
    let path = dir.0.join("shielded.sqlite");
    let store = FileBackedShieldedStore::open_path(&path, 100).unwrap();
    let coordinator = NetworkShieldedCoordinator::new(sdk, Network::Testnet, path, store);
    (wallet, persistence, coordinator, dir, signer)
}

/// A fresh wallet handle whose live tracked locks come from `persistence`'s
/// committed rows, as after a process restart.
async fn restart_wallet(
    old: PlatformWallet,
    persistence: Arc<RecoveryPersistence>,
) -> PlatformWallet {
    let restored = persistence
        .load()
        .unwrap()
        .wallets
        .remove(&old.wallet_id())
        .unwrap();
    old.wallet_manager
        .write()
        .await
        .get_wallet_info_mut(&old.wallet_id())
        .unwrap()
        .tracked_asset_locks = restored
        .unused_asset_locks
        .into_values()
        .flatten()
        .collect();
    let spv = Arc::new(SpvRuntime::new(
        old.wallet_manager.clone(),
        Arc::new(PlatformEventManager::new(vec![])),
    ));
    let wallet = PlatformWallet::new(
        old.sdk.clone(),
        old.wallet_id(),
        old.wallet_manager.clone(),
        old.generation.clone(),
        Arc::new(Notify::new()),
        persistence,
        Arc::new(SpvBroadcaster::new(spv)),
        Arc::new(std::sync::atomic::AtomicBool::new(false)),
        crate::changeset::DurableCursors::default(),
    );
    *wallet.shielded_keys.write().await = old.shielded_keys.read().await.clone();
    wallet
}

async fn live_status(wallet: &PlatformWallet, out_point: &OutPoint) -> AssetLockStatus {
    wallet
        .asset_locks
        .list_tracked_locks()
        .await
        .into_iter()
        .find(|lock| lock.out_point == *out_point)
        .map(|lock| lock.status)
        .unwrap()
}

#[tokio::test]
async fn should_release_registration_after_returned_prebuild_load_failure() {
    let (mut lock, keys) = drain_lock_fixture();
    lock.operation_id = None;
    let (wallet, persistence, coordinator, _dir) = wallet_fixture(&lock, keys).await;
    let coordinator = Arc::new(coordinator);
    let (_, _, _, signer) = funded_wallet_manager(StandardAccountType::BIP44Account).await;
    let prover = super::prover::CachedOrchardProver::new();
    let operation = Arc::new(AssetLockOperationContext::new([0x78; 16]));
    for failed_load in [true, false] {
        persistence.fail_load.store(failed_load, Ordering::SeqCst);
        let result = wallet
            .shielded_fund_from_asset_lock(
                &coordinator,
                AssetLockFunding::DrainWalletBalance {
                    account_index: 0,
                    minimum_lock_duffs: None,
                    operation: operation.clone(),
                },
                vec![],
                &signer,
                &prover,
                None,
                0,
                None,
                None,
                None,
            )
            .await;
        if failed_load {
            assert!(matches!(result, Err(PlatformWalletError::Persistence(_))));
            assert_eq!(operation.snapshot().state, AssetLockOperationState::Pending);
        } else {
            // Reaching ordinary recipient validation proves the abandoned
            // registration no longer blocks this same-context UUID retry.
            assert!(matches!(
                result,
                Err(PlatformWalletError::AddressOperation(_))
            ));
        }
    }
    assert_eq!(
        wallet
            .shielded_wallet_drain_operation_status(operation.operation_id)
            .await
            .unwrap()
            .state,
        AssetLockOperationState::NotSent
    );
    assert!(operation.snapshot().out_point.is_none());
}

#[tokio::test]
async fn should_release_cancelled_drain_waiting_for_the_shield_guard() {
    let (mut lock, keys) = drain_lock_fixture();
    lock.operation_id = None;
    let (wallet, _, coordinator, _dir) = wallet_fixture(&lock, keys).await;
    let coordinator = Arc::new(coordinator);
    let (_, _, _, signer) = funded_wallet_manager(StandardAccountType::BIP44Account).await;
    let prover = super::prover::CachedOrchardProver::new();
    let operation = Arc::new(AssetLockOperationContext::new([0x77; 16]));
    let blocker = wallet.shield_guard.lock().await;
    let mut future = Box::pin(wallet.shielded_fund_from_asset_lock(
        &coordinator,
        AssetLockFunding::DrainWalletBalance {
            account_index: 0,
            minimum_lock_duffs: None,
            operation: operation.clone(),
        },
        vec![],
        &signer,
        &prover,
        None,
        0,
        None,
        None,
        None,
    ));
    assert!(futures::poll!(&mut future).is_pending());
    assert_eq!(
        wallet
            .shielded_wallet_drain_operation_status(operation.operation_id)
            .await
            .unwrap()
            .state,
        AssetLockOperationState::Pending
    );
    drop(future);
    drop(blocker);
    assert_eq!(operation.snapshot().state, AssetLockOperationState::Pending);
    assert_eq!(
        wallet
            .shielded_wallet_drain_operation_status(operation.operation_id)
            .await
            .unwrap()
            .state,
        AssetLockOperationState::NotSent
    );
    let _registration = wallet.register_wallet_drain_operation(operation.clone());
    assert_eq!(
        wallet
            .asset_locks
            .begin_wallet_drain(&operation)
            .await
            .unwrap(),
        None
    );
    assert!(operation.snapshot().out_point.is_none());
}

#[tokio::test]
async fn should_refuse_drain_without_viewing_keys_before_core_funding() {
    for empty_binding in [false, true] {
        let (mut lock, keys) = drain_lock_fixture();
        lock.operation_id = None;
        let recipient = dpp::address_funds::OrchardAddress::from(keys[&0].default_address);
        let (wallet, persistence, coordinator, _dir) = wallet_fixture(&lock, keys.clone()).await;
        *wallet.shielded_keys.write().await = empty_binding.then(BTreeMap::new);
        let coordinator = Arc::new(coordinator);
        let (_, _, _, signer) = funded_wallet_manager(StandardAccountType::BIP44Account).await;
        let prover = super::prover::CachedOrchardProver::new();
        let operation = Arc::new(AssetLockOperationContext::new([0x97; 16]));
        let error = wallet
            .shielded_fund_from_asset_lock(
                &coordinator,
                AssetLockFunding::DrainWalletBalance {
                    account_index: 0,
                    minimum_lock_duffs: None,
                    operation: operation.clone(),
                },
                vec![(recipient, None)],
                &signer,
                &prover,
                None,
                0,
                None,
                None,
                None,
            )
            .await
            .unwrap_err();
        assert!(
            error
                .to_string()
                .contains("requires bound shielded viewing keys"),
            "{error}"
        );
        assert_eq!(operation.snapshot().state, AssetLockOperationState::NotSent);
        assert!(operation.snapshot().out_point.is_none());
        assert_eq!(persistence.committed.lock().unwrap().len(), 1);
        assert!(persistence.staged.lock().unwrap().is_empty());
        assert_eq!(wallet.asset_locks.list_tracked_locks().await.len(), 1);
        // Binding keys leaves the same UUID available for a fresh attempt.
        *wallet.shielded_keys.write().await = Some(keys);
        let _registration = wallet.register_wallet_drain_operation(operation.clone());
        assert_eq!(
            wallet
                .asset_locks
                .begin_wallet_drain(&operation)
                .await
                .unwrap(),
            None
        );
    }
}

#[tokio::test]
async fn should_keep_a_consumed_legacy_drain_row_pending_and_refuse_generic_consumption() {
    let (mut lock, keys) = drain_lock_fixture();
    lock.status = AssetLockStatus::Consumed;
    let (wallet, persistence, _, _dir) = wallet_fixture(&lock, keys).await;
    // Generic consumption never manufactures a drain completion, and nothing
    // short of `complete_wallet_drain` can write the receipt.
    assert!(wallet
        .asset_locks
        .consume_asset_lock(&lock.out_point)
        .await
        .is_err());
    assert!(wallet
        .asset_locks
        .advance_asset_lock_status(&lock.out_point, AssetLockStatus::DrainCompleted, None)
        .await
        .is_err());
    assert_eq!(
        persistence.committed_status(&lock.out_point),
        AssetLockStatus::Consumed
    );
    let wallet = restart_wallet(wallet, persistence).await;
    let status = wallet
        .shielded_wallet_drain_operation_status(OPERATION_ID)
        .await
        .unwrap();
    assert_eq!(status.state, AssetLockOperationState::Pending);
    assert_eq!(status.out_point, Some(lock.out_point));
}

#[tokio::test]
async fn should_evict_completed_context_after_last_lease_and_keep_pending_context() {
    for complete_while_registered in [false, true] {
        let (lock, keys) = drain_lock_fixture();
        let (wallet, _, _, _dir) = wallet_fixture(&lock, keys).await;
        let operation = Arc::new(AssetLockOperationContext::new(OPERATION_ID));
        operation.set(AssetLockOperationState::Pending, Some(lock.out_point));
        let weak = Arc::downgrade(&operation);
        let owner = wallet.register_wallet_drain_operation(operation.clone());
        let contender = wallet.register_wallet_drain_operation(Arc::new(
            AssetLockOperationContext::new(OPERATION_ID),
        ));
        drop(operation);
        let leases = if complete_while_registered {
            Some((owner, contender))
        } else {
            drop((owner, contender));
            assert!(
                weak.upgrade().is_some(),
                "pending checkpoint must remain retained"
            );
            None
        };
        wallet
            .asset_locks
            .complete_wallet_drain(lock.out_point, OPERATION_ID)
            .await
            .unwrap();
        if let Some((owner, contender)) = leases {
            assert_eq!(
                weak.upgrade().unwrap().snapshot().state,
                AssetLockOperationState::Completed
            );
            drop(owner);
            assert!(weak.upgrade().is_some(), "queued caller still owns a lease");
            drop(contender);
        }
        assert!(
            weak.upgrade().is_none(),
            "completed context must be evicted"
        );
        // A later status read of the durable receipt also releases a newly
        // registered context once that caller's final lease leaves.
        let restored = Arc::new(AssetLockOperationContext::new(OPERATION_ID));
        let weak = Arc::downgrade(&restored);
        let lease = wallet.register_wallet_drain_operation(restored.clone());
        drop(restored);
        assert_eq!(
            wallet
                .shielded_wallet_drain_operation_status(OPERATION_ID)
                .await
                .unwrap()
                .state,
            AssetLockOperationState::Completed
        );
        drop(lease);
        assert!(weak.upgrade().is_none());
    }
}

#[tokio::test]
async fn should_complete_drain_when_an_inline_commit_flush_notification_fails() {
    let (lock, keys) = drain_lock_fixture();
    let (wallet, persistence, _, _dir) = wallet_fixture(&lock, keys).await;
    persistence.commits_inline.store(true, Ordering::SeqCst);
    persistence.fail_flush.store(true, Ordering::SeqCst);
    wallet
        .asset_locks
        .complete_wallet_drain(lock.out_point, OPERATION_ID)
        .await
        .unwrap();
    assert_eq!(
        persistence.committed_status(&lock.out_point),
        AssetLockStatus::DrainCompleted
    );
    assert_eq!(
        live_status(&wallet, &lock.out_point).await,
        AssetLockStatus::DrainCompleted
    );
    let status = wallet
        .shielded_wallet_drain_operation_status(OPERATION_ID)
        .await
        .unwrap();
    assert_eq!(status.state, AssetLockOperationState::Completed);
    assert_eq!(status.out_point, Some(lock.out_point));
}

#[tokio::test]
async fn should_keep_drain_pending_when_completion_persistence_fails() {
    let (lock, keys) = drain_lock_fixture();
    let (wallet, persistence, _, _dir) = wallet_fixture(&lock, keys).await;
    {
        persistence.fail_store.store(true, Ordering::SeqCst);
        assert!(matches!(
            wallet
                .asset_locks
                .complete_wallet_drain(lock.out_point, OPERATION_ID)
                .await,
            Err(PlatformWalletError::Persistence(_))
        ));
        assert_ne!(
            persistence.committed_status(&lock.out_point),
            AssetLockStatus::DrainCompleted
        );
        assert_ne!(
            live_status(&wallet, &lock.out_point).await,
            AssetLockStatus::DrainCompleted
        );
        let status = wallet
            .shielded_wallet_drain_operation_status(OPERATION_ID)
            .await
            .unwrap();
        assert_eq!(status.state, AssetLockOperationState::Pending);
        assert_eq!(status.out_point, Some(lock.out_point));
    }
    persistence.fail_store.store(false, Ordering::SeqCst);
    wallet
        .asset_locks
        .complete_wallet_drain(lock.out_point, OPERATION_ID)
        .await
        .unwrap();
    let wallet = restart_wallet(wallet, persistence).await;
    assert_eq!(
        wallet
            .shielded_wallet_drain_operation_status(OPERATION_ID)
            .await
            .unwrap()
            .state,
        AssetLockOperationState::Completed
    );
}

/// A buffered backend could commit a retained changeset through a later,
/// unrelated flush, so every drain entry point refuses it before any Core
/// funds move or any row is written; the stored state is left untouched.
#[tokio::test]
async fn should_refuse_drain_on_a_buffered_persistence_backend() {
    let (lock, keys) = drain_lock_fixture();
    let (wallet, persistence, _, _dir) = wallet_fixture(&lock, keys).await;
    persistence.commits_inline.store(false, Ordering::SeqCst);

    fn refused<T: std::fmt::Debug>(result: Result<T, PlatformWalletError>) {
        match result {
            Err(PlatformWalletError::Persistence(message)) => {
                assert!(message.contains("commits each store inline"), "{message}");
            }
            other => panic!("buffered backend must be refused, got {other:?}"),
        }
    }
    refused(
        wallet
            .shielded_wallet_drain_operation_status(OPERATION_ID)
            .await
            .map(|snapshot| snapshot.state),
    );
    refused(
        wallet
            .asset_locks
            .complete_wallet_drain(lock.out_point, OPERATION_ID)
            .await,
    );
    let fresh = Arc::new(AssetLockOperationContext::new([0x77; 16]));
    let _registration = wallet
        .asset_locks
        .register_wallet_drain_operation(fresh.clone());
    refused(wallet.asset_locks.begin_wallet_drain(&fresh).await);
    assert!(fresh.snapshot().out_point.is_none());
    // The generic exact-outpoint resume of a drain's lock applies the same
    // admission policy before anything is built or submitted.
    let (_, _, _, signer) = funded_wallet_manager(StandardAccountType::BIP44Account).await;
    refused(
        wallet
            .asset_locks
            .resolve_funding_with_is_timeout_fallback(
                AssetLockFunding::FromExistingAssetLock {
                    out_point: lock.out_point,
                    consume_invitation_voucher: false,
                },
                lock.funding_type,
                0,
                None,
                &signer,
                None,
            )
            .await
            .map(|_| ()),
    );
    refused(
        wallet
            .asset_locks
            .resume_asset_lock(&lock.out_point, None)
            .await
            .map(|_| ()),
    );

    assert_ne!(
        persistence.committed_status(&lock.out_point),
        AssetLockStatus::DrainCompleted
    );
    assert!(persistence.staged.lock().unwrap().is_empty());
    assert_eq!(wallet.asset_locks.list_tracked_locks().await.len(), 1);

    // The same backend in inline-commit mode is admitted unchanged.
    persistence.commits_inline.store(true, Ordering::SeqCst);
    assert_eq!(
        wallet
            .shielded_wallet_drain_operation_status(OPERATION_ID)
            .await
            .unwrap()
            .state,
        AssetLockOperationState::Pending
    );
}

#[tokio::test]
async fn should_resume_the_same_lock_for_a_reused_uuid_without_a_second_core_lock() {
    let (lock, keys) = drain_lock_fixture();
    let (wallet, persistence, _, _dir) = wallet_fixture(&lock, keys).await;
    let operation = Arc::new(AssetLockOperationContext::new(OPERATION_ID));
    let registration = wallet.register_wallet_drain_operation(operation.clone());
    assert_eq!(
        wallet
            .asset_locks
            .begin_wallet_drain(&operation)
            .await
            .unwrap(),
        Some(lock.out_point)
    );
    assert_eq!(
        persistence.committed.lock().unwrap().len(),
        1,
        "a reused UUID must never create a second Core lock"
    );
    assert_eq!(operation.snapshot().state, AssetLockOperationState::Pending);
    assert_eq!(operation.snapshot().out_point, Some(lock.out_point));

    wallet
        .asset_locks
        .complete_wallet_drain(lock.out_point, OPERATION_ID)
        .await
        .unwrap();
    assert_eq!(
        operation.snapshot().state,
        AssetLockOperationState::Completed
    );

    // A retry under the completed UUID is refused with the typed error and
    // reports completion; a new UUID is free to start.
    let retry = Arc::new(AssetLockOperationContext::new(OPERATION_ID));
    let _retry_registration = wallet.register_wallet_drain_operation(retry.clone());
    assert!(matches!(
        wallet.asset_locks.begin_wallet_drain(&retry).await,
        Err(PlatformWalletError::AssetLockAlreadyConsumed(out_point)) if out_point == lock.out_point
    ));
    assert_eq!(retry.snapshot().state, AssetLockOperationState::Completed);
    drop(registration);
    let fresh = Arc::new(AssetLockOperationContext::new([0x44; 16]));
    let _fresh_registration = wallet.register_wallet_drain_operation(fresh.clone());
    assert_eq!(
        wallet.asset_locks.begin_wallet_drain(&fresh).await.unwrap(),
        None
    );
}

#[tokio::test]
async fn status_reads_durable_state_without_network_or_guards() {
    let (lock, keys) = drain_lock_fixture();
    let (wallet, _, coordinator, _dir) = wallet_fixture(&lock, keys).await;
    let guard = wallet.shield_guard.lock().await;
    let scan = coordinator.store().write().await;
    let mut future = Box::pin(wallet.shielded_wallet_drain_operation_status(OPERATION_ID));
    let result = futures::poll!(&mut future);
    drop(future);
    assert!(matches!(
        result,
        std::task::Poll::Ready(Ok(snapshot))
            if snapshot.state == AssetLockOperationState::Pending
                && snapshot.out_point == Some(lock.out_point)
    ));
    drop(scan);
    drop(guard);
    wallet
        .asset_locks
        .complete_wallet_drain(lock.out_point, OPERATION_ID)
        .await
        .unwrap();
    assert_eq!(
        wallet
            .shielded_wallet_drain_operation_status(OPERATION_ID)
            .await
            .unwrap()
            .state,
        AssetLockOperationState::Completed
    );
}

/// `shielded_fund_from_asset_lock` hands its `on_asset_lock_built` to the
/// lock builder: the caller hears the outpoint of a fresh lock from the
/// wallet balance, here one the never-started SPV client refuses to send, so
/// the lock is untracked again when the call fails.
#[tokio::test]
async fn should_report_a_fresh_shield_lock_to_the_caller() {
    let (mut lock, keys) = drain_lock_fixture();
    lock.operation_id = None;
    let recipient = dpp::address_funds::OrchardAddress::from(keys[&0].default_address);
    let (wallet, _persistence, coordinator, _dir, signer) =
        signed_wallet_fixture(&lock, keys).await;
    let coordinator = Arc::new(coordinator);
    let prover = super::prover::CachedOrchardProver::new();
    let seen = Mutex::new(Vec::new());
    let observer = |out_point: OutPoint| seen.lock().unwrap().push(out_point);

    wallet
        .shielded_fund_from_asset_lock(
            &coordinator,
            AssetLockFunding::FromWalletBalance {
                amount_duffs: 5_000_000,
                account_index: 0,
            },
            vec![(recipient, None)],
            &signer,
            &prover,
            None,
            0,
            None,
            None,
            Some(&observer),
        )
        .await
        .expect_err("a lock the SPV client refuses to send must fail the call");

    let seen = seen.into_inner().unwrap();
    assert_eq!(seen.len(), 1);
    assert!(!wallet
        .asset_locks
        .list_tracked_locks()
        .await
        .iter()
        .any(|tracked| tracked.out_point == seen[0]));
}

/// The pooled wallet drain reaches the observer through
/// `create_wallet_drain_asset_lock_proof`, before its row is stored: the
/// caller hears the drained lock's outpoint even when the broadcast is then
/// refused.
#[tokio::test]
async fn should_report_a_wallet_drain_lock_to_the_caller() {
    let (mut lock, keys) = drain_lock_fixture();
    lock.operation_id = None;
    let recipient = dpp::address_funds::OrchardAddress::from(keys[&0].default_address);
    let (wallet, _persistence, coordinator, _dir, signer) =
        signed_wallet_fixture(&lock, keys).await;
    let coordinator = Arc::new(coordinator);
    let prover = super::prover::CachedOrchardProver::new();
    let operation = Arc::new(AssetLockOperationContext::new([0x5a; 16]));
    let seen = Mutex::new(Vec::new());
    let observer = |out_point: OutPoint| seen.lock().unwrap().push(out_point);

    wallet
        .shielded_fund_from_asset_lock(
            &coordinator,
            AssetLockFunding::DrainWalletBalance {
                account_index: 0,
                minimum_lock_duffs: None,
                operation: operation.clone(),
            },
            vec![(recipient, None)],
            &signer,
            &prover,
            None,
            0,
            None,
            None,
            Some(&observer),
        )
        .await
        .expect_err("a lock the SPV client refuses to send must fail the call");

    // A definite rejection removed the lock again, so the operation records
    // that nothing was sent.
    assert_eq!(seen.into_inner().unwrap().len(), 1);
    let snapshot = operation.snapshot();
    assert_eq!(snapshot.state, AssetLockOperationState::NotSent);
    assert!(snapshot.out_point.is_none());
}

/// A generic outpoint resume that names a different recipient than the row
/// recorded is refused before any funding resolution: no send, no staged
/// write, the row unchanged.
#[tokio::test]
async fn should_refuse_a_generic_resume_to_a_recipient_other_than_the_recorded_one() {
    let (mut lock, keys) = drain_lock_fixture();
    lock.operation_id = None;
    let recorded = crate::test_support::orchard_address_fixture(0x51);
    let other = crate::test_support::orchard_address_fixture(0x52);
    lock.shielded_recipient = Some(recorded.to_raw_bytes());
    let (wallet, persistence, coordinator, _dir) = wallet_fixture(&lock, keys).await;
    let coordinator = Arc::new(coordinator);
    let (_, _, _, signer) = funded_wallet_manager(StandardAccountType::BIP44Account).await;
    let prover = super::prover::CachedOrchardProver::new();

    let error = wallet
        .shielded_fund_from_asset_lock(
            &coordinator,
            AssetLockFunding::FromExistingAssetLock {
                out_point: lock.out_point,
                consume_invitation_voucher: false,
            },
            vec![(other, None)],
            &signer,
            &prover,
            None,
            0,
            None,
            None,
            None,
        )
        .await
        .unwrap_err();
    assert!(
        matches!(
            &error,
            PlatformWalletError::AssetLockRecipientMismatch {
                out_point,
                recorded: r,
                requested: q,
            } if *out_point == lock.out_point && *r == recorded.to_raw_bytes() && *q == other.to_raw_bytes()
        ),
        "{error}"
    );
    assert!(persistence.staged.lock().unwrap().is_empty());
    assert_eq!(
        persistence.committed_status(&lock.out_point),
        AssetLockStatus::RecoveredFromChain
    );
    assert_eq!(
        live_status(&wallet, &lock.out_point).await,
        AssetLockStatus::RecoveredFromChain
    );
}

/// A repeated-UUID drain that names a different recipient is refused the
/// same way, and the operation stays Pending with its outpoint: the lock
/// exists and nothing was sent, so a later resume with the recorded
/// recipient can still complete it.
#[tokio::test]
async fn should_refuse_a_repeated_uuid_drain_to_a_recipient_other_than_the_recorded_one() {
    let (mut lock, keys) = drain_lock_fixture();
    let recorded = crate::test_support::orchard_address_fixture(0x53);
    let other = crate::test_support::orchard_address_fixture(0x54);
    lock.shielded_recipient = Some(recorded.to_raw_bytes());
    let (wallet, persistence, coordinator, _dir) = wallet_fixture(&lock, keys).await;
    let coordinator = Arc::new(coordinator);
    let (_, _, _, signer) = funded_wallet_manager(StandardAccountType::BIP44Account).await;
    let prover = super::prover::CachedOrchardProver::new();
    let operation = Arc::new(AssetLockOperationContext::new(OPERATION_ID));

    let error = wallet
        .shielded_fund_from_asset_lock(
            &coordinator,
            AssetLockFunding::DrainWalletBalance {
                account_index: 0,
                minimum_lock_duffs: None,
                operation: operation.clone(),
            },
            vec![(other, None)],
            &signer,
            &prover,
            None,
            0,
            None,
            None,
            None,
        )
        .await
        .unwrap_err();
    assert!(
        matches!(
            error,
            PlatformWalletError::AssetLockRecipientMismatch { .. }
        ),
        "{error}"
    );
    let snapshot = operation.snapshot();
    assert_eq!(snapshot.state, AssetLockOperationState::Pending);
    assert_eq!(snapshot.out_point, Some(lock.out_point));
    assert!(persistence.staged.lock().unwrap().is_empty());
    assert_ne!(
        persistence.committed_status(&lock.out_point),
        AssetLockStatus::DrainCompleted
    );
    assert_eq!(
        wallet
            .shielded_wallet_drain_operation_status(OPERATION_ID)
            .await
            .unwrap()
            .state,
        AssetLockOperationState::Pending
    );
}

/// A row tracked before recipients were recorded carries none and accepts
/// the resuming caller's recipient: the resume gets past the recipient check
/// (and fails later, at the network, in this fixture).
#[tokio::test]
async fn should_accept_the_callers_recipient_for_a_legacy_row_without_one() {
    let (mut lock, keys) = drain_lock_fixture();
    lock.operation_id = None;
    assert_eq!(lock.shielded_recipient, None);
    let recipient = crate::test_support::orchard_address_fixture(0x55);
    let (wallet, _persistence, coordinator, _dir) = wallet_fixture(&lock, keys).await;
    let coordinator = Arc::new(coordinator);
    let (_, _, _, signer) = funded_wallet_manager(StandardAccountType::BIP44Account).await;
    let prover = super::prover::CachedOrchardProver::new();

    let error = wallet
        .shielded_fund_from_asset_lock(
            &coordinator,
            AssetLockFunding::FromExistingAssetLock {
                out_point: lock.out_point,
                consume_invitation_voucher: false,
            },
            vec![(recipient, None)],
            &signer,
            &prover,
            None,
            0,
            None,
            Some(std::time::Duration::from_millis(10)),
            None,
        )
        .await
        .unwrap_err();
    // The recipient check passed and funding resolution ran: this fixture's
    // lock address is not in the funding account, which is where it fails.
    assert!(
        matches!(error, PlatformWalletError::AssetLockTransaction(_)),
        "a legacy row must not refuse the caller's recipient: {error}"
    );
}

/// A fresh shielded funding records its recipient on the row before the
/// lock is dispatched: the drain's durable checkpoint and the typed-amount
/// path's flushed row both carry it, even when the broadcast is then refused.
/// The typed-amount run is on a buffered backend, where only a flush commits.
#[tokio::test]
async fn should_record_the_recipient_with_the_lock_before_dispatch() {
    for drain in [false, true] {
        let (mut lock, keys) = drain_lock_fixture();
        lock.operation_id = None;
        let recipient = crate::test_support::orchard_address_fixture(0x56);
        let (wallet, persistence, coordinator, _dir, signer) =
            signed_wallet_fixture(&lock, keys).await;
        persistence.commits_inline.store(drain, Ordering::SeqCst);
        let coordinator = Arc::new(coordinator);
        let prover = super::prover::CachedOrchardProver::new();
        let operation = Arc::new(AssetLockOperationContext::new([0x57; 16]));
        let seen = Mutex::new(Vec::new());
        let observer = |out_point: OutPoint| seen.lock().unwrap().push(out_point);
        let funding = if drain {
            AssetLockFunding::DrainWalletBalance {
                account_index: 0,
                minimum_lock_duffs: None,
                operation: operation.clone(),
            }
        } else {
            AssetLockFunding::FromWalletBalance {
                amount_duffs: 5_000_000,
                account_index: 0,
            }
        };
        wallet
            .shielded_fund_from_asset_lock(
                &coordinator,
                funding,
                vec![(recipient, None)],
                &signer,
                &prover,
                None,
                0,
                None,
                None,
                Some(&observer),
            )
            .await
            .expect_err("a lock the SPV client refuses to send must fail the call");
        let seen = seen.into_inner().unwrap();
        assert_eq!(seen.len(), 1);
        let stored = persistence
            .committed
            .lock()
            .unwrap()
            .get(&seen[0])
            .cloned()
            .expect("the built lock's row was committed before the broadcast");
        assert_eq!(
            stored.shielded_recipient,
            Some(recipient.to_raw_bytes()),
            "drain={drain}"
        );
        assert_eq!(stored.operation_id, drain.then_some([0x57; 16]));
    }
}

/// One typed-amount shielded funding against the fixture, returning the call's
/// error and the outpoints the caller was told about.
async fn fund_typed_amount(
    wallet: &PlatformWallet,
    coordinator: &Arc<NetworkShieldedCoordinator>,
    signer: &WalletSigner,
    recipient: dpp::address_funds::OrchardAddress,
) -> (PlatformWalletError, Vec<OutPoint>) {
    let prover = super::prover::CachedOrchardProver::new();
    let seen = Mutex::new(Vec::new());
    let observer = |out_point: OutPoint| seen.lock().unwrap().push(out_point);
    let error = wallet
        .shielded_fund_from_asset_lock(
            coordinator,
            AssetLockFunding::FromWalletBalance {
                amount_duffs: 5_000_000,
                account_index: 0,
            },
            vec![(recipient, None)],
            signer,
            &prover,
            None,
            0,
            None,
            None,
            Some(&observer),
        )
        .await
        .expect_err("the fixture's SPV client never sends");
    (error, seen.into_inner().unwrap())
}

/// A typed-amount funding whose recipient cannot be made durable stops before
/// the broadcaster: a rejected store and a rejected flush of a buffered store
/// both fail the call with a persistence error, leave the lock untracked and
/// uncommitted, and, once the abandoned row's removal is on disk, hand the
/// inputs back so the next attempt builds again. The abandoned row of a
/// failed flush is withdrawn from the buffer, so the flush that writes its
/// removal does not commit it.
#[tokio::test]
async fn should_abort_typed_amount_funding_when_the_recipient_cannot_be_made_durable() {
    for fail_flush in [false, true] {
        let (mut lock, keys) = drain_lock_fixture();
        lock.operation_id = None;
        let recipient = crate::test_support::orchard_address_fixture(0x58);
        let (wallet, persistence, coordinator, _dir, signer) =
            signed_wallet_fixture(&lock, keys).await;
        let coordinator = Arc::new(coordinator);
        persistence.commits_inline.store(false, Ordering::SeqCst);
        // The row's own store or flush fails; its removal's goes through.
        let switch = if fail_flush {
            &persistence.fail_next_flush
        } else {
            &persistence.fail_next_lock_store
        };

        switch.store(true, Ordering::SeqCst);
        let (error, seen) = fund_typed_amount(&wallet, &coordinator, &signer, recipient).await;
        assert!(
            matches!(&error, PlatformWalletError::Persistence(message)
                if message.contains("shielded recipient was not made durable")),
            "fail_flush={fail_flush}: {error}"
        );
        assert_eq!(seen.len(), 1, "fail_flush={fail_flush}");
        assert!(!wallet
            .asset_locks
            .list_tracked_locks()
            .await
            .iter()
            .any(|tracked| tracked.out_point == seen[0]));
        assert!(!persistence.committed.lock().unwrap().contains_key(&seen[0]));

        // The inputs were handed back: the retry builds a lock again and gets
        // as far as the broadcaster.
        let (error, retried) = fund_typed_amount(&wallet, &coordinator, &signer, recipient).await;
        assert!(
            !matches!(error, PlatformWalletError::Persistence(_)),
            "fail_flush={fail_flush}: {error}"
        );
        assert_eq!(retried.len(), 1, "fail_flush={fail_flush}");
        assert_ne!(retried[0], seen[0]);
        assert!(
            !persistence.committed.lock().unwrap().contains_key(&seen[0]),
            "fail_flush={fail_flush}: the abandoned row was committed by a later flush"
        );
        assert_eq!(
            persistence.committed.lock().unwrap()[&retried[0]].shielded_recipient,
            Some(recipient.to_raw_bytes())
        );
    }
}

/// Where the backend keeps failing, so the abandoned row's removal cannot be
/// confirmed either, the funding is not reported as failed: a store or flush
/// that failed is no proof the row is on no disk, and a restart that finds
/// it resumes and sends it. The outcome is unknown, for a rejected store and
/// for a rejected flush alike, and nothing is sent or tracked.
#[tokio::test]
async fn should_report_a_typed_amount_funding_as_unconfirmed_when_its_row_removal_fails_too() {
    for fail_flush in [false, true] {
        let (mut lock, keys) = drain_lock_fixture();
        lock.operation_id = None;
        let recipient = crate::test_support::orchard_address_fixture(0x5a);
        let (wallet, persistence, coordinator, _dir, signer) =
            signed_wallet_fixture(&lock, keys).await;
        let coordinator = Arc::new(coordinator);
        persistence.commits_inline.store(false, Ordering::SeqCst);
        let switch = if fail_flush {
            &persistence.fail_flush
        } else {
            &persistence.fail_store
        };

        switch.store(true, Ordering::SeqCst);
        let (error, seen) = fund_typed_amount(&wallet, &coordinator, &signer, recipient).await;
        assert!(
            matches!(&error, PlatformWalletError::TransactionBroadcastUnconfirmed(message)
                if message.contains("a restart may still resume it")),
            "fail_flush={fail_flush}: {error}"
        );
        assert_eq!(seen.len(), 1, "fail_flush={fail_flush}");
        assert!(!wallet
            .asset_locks
            .list_tracked_locks()
            .await
            .iter()
            .any(|tracked| tracked.out_point == seen[0]));
        assert!(!persistence.committed.lock().unwrap().contains_key(&seen[0]));
    }
}

/// A backend that cannot record a recipient keeps the best-effort row every
/// other funding has: a rejected store does not stop the dispatch.
#[tokio::test]
async fn should_keep_best_effort_dispatch_on_a_backend_without_recipient_support() {
    let (mut lock, keys) = drain_lock_fixture();
    lock.operation_id = None;
    let recipient = crate::test_support::orchard_address_fixture(0x59);
    let (wallet, persistence, coordinator, _dir, signer) = signed_wallet_fixture(&lock, keys).await;
    let coordinator = Arc::new(coordinator);
    persistence
        .records_recipients
        .store(false, Ordering::SeqCst);
    persistence.fail_store.store(true, Ordering::SeqCst);

    let (error, seen) = fund_typed_amount(&wallet, &coordinator, &signer, recipient).await;

    assert!(
        !matches!(error, PlatformWalletError::Persistence(_)),
        "{error}"
    );
    assert_eq!(seen.len(), 1);
}
