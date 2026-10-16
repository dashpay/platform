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
use crate::test_support::funded_wallet_manager;
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
    let secret = dashcore::secp256k1::SecretKey::from_secret_bytes([0x11; 32]).unwrap();
    let public = dashcore::PublicKey::new(dashcore::secp256k1::PublicKey::from_secret_key(&secret));
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
    commits_inline: AtomicBool,
    fail_load: AtomicBool,
}

impl PlatformWalletPersistence for RecoveryPersistence {
    fn store_commits_inline(&self) -> bool {
        self.commits_inline.load(Ordering::SeqCst)
    }
    fn persistence_capabilities(&self) -> PersistenceCapabilities {
        PersistenceCapabilities::WALLET_DRAIN
    }
    fn store(
        &self,
        _wallet_id: [u8; 32],
        cs: PlatformWalletChangeSet,
    ) -> Result<(), PersistenceError> {
        if self.fail_store.load(Ordering::SeqCst) {
            return Err(PersistenceError::backend("store failed"));
        }
        if let Some(cs) = cs.asset_locks {
            self.staged.lock().unwrap().extend(cs.asset_locks);
        }
        if self.store_commits_inline() {
            self.commit_staged();
        }
        Ok(())
    }
    fn flush(&self, _wallet_id: [u8; 32]) -> Result<(), PersistenceError> {
        if self.fail_flush.load(Ordering::SeqCst) {
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
                    },
                )
            })
            .collect();
        Ok(ClientStartState {
            wallets: BTreeMap::from([(
                self.wallet.compute_wallet_id(),
                ClientWalletStartState {
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
    let (manager, id, generation, _) =
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
        // Drains admit inline-committing backends only; the buffered mode
        // exists for `should_refuse_drain_on_a_buffered_persistence_backend`.
        commits_inline: AtomicBool::new(true),
        fail_load: AtomicBool::new(false),
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
    (wallet, persistence, coordinator, dir)
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
