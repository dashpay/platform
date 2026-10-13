//! Exact, restart-safe association of a host drain request with its asset lock.

use std::collections::BTreeMap;
#[cfg(any(feature = "shielded", test))]
use std::collections::BTreeSet;
#[cfg(any(feature = "shielded", test))]
use std::sync::atomic::AtomicBool;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use dashcore::OutPoint;

use crate::broadcaster::TransactionBroadcaster;
use crate::changeset::PersistenceCapabilities;
use crate::error::PlatformWalletError;

use super::manager::AssetLockManager;
use super::tracked::{AssetLockStatus, TrackedAssetLock};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssetLockOperationState {
    NotSent,
    Pending,
    Completed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AssetLockOperationSnapshot {
    pub state: AssetLockOperationState,
    pub out_point: Option<OutPoint>,
}

/// Held by the caller across a caught panic as well as by the wallet while an
/// operation is unresolved. Pending is deliberately the initial state: absence
/// of an observed return is never evidence that no transaction was dispatched.
#[derive(Debug)]
pub struct AssetLockOperationContext {
    pub operation_id: [u8; 16],
    #[cfg(any(feature = "shielded", test))]
    started: AtomicBool,
    registrations: AtomicUsize,
    snapshot: Mutex<AssetLockOperationSnapshot>,
}

impl AssetLockOperationContext {
    pub fn new(operation_id: [u8; 16]) -> Self {
        Self {
            operation_id,
            #[cfg(any(feature = "shielded", test))]
            started: AtomicBool::new(false),
            registrations: AtomicUsize::new(0),
            snapshot: Mutex::new(AssetLockOperationSnapshot {
                state: AssetLockOperationState::Pending,
                out_point: None,
            }),
        }
    }

    pub fn snapshot(&self) -> AssetLockOperationSnapshot {
        *self
            .snapshot
            .lock()
            .unwrap_or_else(|error| error.into_inner())
    }

    pub(crate) fn set(&self, state: AssetLockOperationState, out_point: Option<OutPoint>) {
        *self
            .snapshot
            .lock()
            .unwrap_or_else(|error| error.into_inner()) =
            AssetLockOperationSnapshot { state, out_point };
    }
}

/// Keeps a queued or running drain visible until its invocation finishes.
/// Dropping the last registration releases a pre-build operation, including
/// on cancellation or an early error. Once an outpoint may be durable, its
/// context remains registered until durable recovery establishes its outcome.
#[must_use = "hold the registration until the drain worker and outcome snapshot finish"]
#[derive(Debug)]
pub struct AssetLockOperationRegistration {
    operations: Arc<Mutex<BTreeMap<[u8; 16], Arc<AssetLockOperationContext>>>>,
    operation: Arc<AssetLockOperationContext>,
}

impl Drop for AssetLockOperationRegistration {
    fn drop(&mut self) {
        let mut operations = self
            .operations
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let last = self.operation.registrations.fetch_sub(1, Ordering::SeqCst) == 1;
        if last
            && self.operation.snapshot().out_point.is_none()
            && operations
                .get(&self.operation.operation_id)
                .is_some_and(|current| Arc::ptr_eq(current, &self.operation))
        {
            operations.remove(&self.operation.operation_id);
        }
        // Do not change the caller's snapshot: only a later successful durable
        // read can resolve an uncertain pre-build return to NotSent.
    }
}

fn snapshot_for_lock(lock: &TrackedAssetLock) -> AssetLockOperationSnapshot {
    AssetLockOperationSnapshot {
        state: if lock.status == AssetLockStatus::Consumed {
            AssetLockOperationState::Completed
        } else {
            AssetLockOperationState::Pending
        },
        out_point: Some(lock.out_point),
    }
}

impl<B: TransactionBroadcaster + ?Sized> AssetLockManager<B> {
    /// Retain every exact signed attempt before dispatch. Failed writes retain
    /// the candidate in memory too: a backend may have committed before its
    /// flush failed, so the next attempt must never reuse its append index.
    #[cfg(feature = "shielded")]
    pub(crate) async fn persist_wallet_drain_attempt(
        &self,
        out_point: OutPoint,
        signed_attempt: Vec<u8>,
    ) -> Result<(), PlatformWalletError> {
        self.require_wallet_drain_persistence()?;
        let candidate = {
            let mut manager = self.wallet_manager.write().await;
            let lock = manager
                .get_wallet_info_mut(&self.wallet_id)
                .and_then(|info| info.tracked_asset_locks.get_mut(&out_point))
                .ok_or(PlatformWalletError::AssetLockNotTracked(out_point))?;
            if lock.operation_id.is_none() {
                return Err(PlatformWalletError::Persistence(
                    "drain attempt has no operation UUID".into(),
                ));
            }
            if lock.status == AssetLockStatus::Consumed {
                return Err(PlatformWalletError::AssetLockAlreadyConsumed(out_point));
            }
            if !lock.shielded_funding_attempts.contains(&signed_attempt) {
                lock.shielded_funding_attempts.push(signed_attempt);
            }
            lock.clone()
        };
        self.persist_drain_lock_snapshot(&candidate)
    }

    /// Commit the tombstone before publishing completion. The caller owns the
    /// activity-store guard and must publish history synchronously afterwards.
    /// Refuse a busy manager without waiting: no cancellation or lock-order
    /// inversion may separate durable completion from its memory publication.
    #[cfg(feature = "shielded")]
    pub(crate) fn complete_wallet_drain(
        &self,
        out_point: OutPoint,
        operation_id: [u8; 16],
        history: crate::changeset::ShieldedChangeSet,
    ) -> Result<bool, PlatformWalletError> {
        self.require_wallet_drain_persistence()?;
        let Ok(mut manager) = self.wallet_manager.try_write() else {
            return Ok(false);
        };
        let lock = manager
            .get_wallet_info_mut(&self.wallet_id)
            .and_then(|info| info.tracked_asset_locks.get_mut(&out_point))
            .filter(|lock| lock.operation_id == Some(operation_id))
            .ok_or(PlatformWalletError::AssetLockNotTracked(out_point))?;
        let mut candidate = lock.clone();
        candidate.status = AssetLockStatus::Consumed;
        candidate.proof = None;
        let mut changes = crate::changeset::AssetLockChangeSet::default();
        changes
            .asset_locks
            .insert(candidate.out_point, (&candidate).into());
        self.persister
            .store(crate::changeset::PlatformWalletChangeSet {
                asset_locks: Some(changes),
                shielded: Some(history),
                ..Default::default()
            })
            .map_err(|e| PlatformWalletError::Persistence(e.to_string()))?;
        self.persister
            .flush()
            .map_err(|e| PlatformWalletError::Persistence(e.to_string()))?;
        lock.status = AssetLockStatus::Consumed;
        lock.proof = None;
        self.resume_dispatch_claims
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .remove(&out_point);
        Ok(true)
    }

    #[cfg(feature = "shielded")]
    fn persist_drain_lock_snapshot(
        &self,
        lock: &TrackedAssetLock,
    ) -> Result<(), PlatformWalletError> {
        let mut changes = crate::changeset::AssetLockChangeSet::default();
        changes.asset_locks.insert(lock.out_point, lock.into());
        self.persister
            .store(crate::changeset::PlatformWalletChangeSet {
                asset_locks: Some(changes),
                ..Default::default()
            })
            .map_err(|e| PlatformWalletError::Persistence(e.to_string()))?;
        self.persister
            .flush()
            .map_err(|e| PlatformWalletError::Persistence(e.to_string()))
    }

    pub(crate) fn require_wallet_drain_persistence(&self) -> Result<(), PlatformWalletError> {
        let capabilities = self.persister.persistence_capabilities();
        let required = PersistenceCapabilities::WALLET_DRAIN;
        if !capabilities.contains(required) {
            return Err(PlatformWalletError::Persistence(format!(
                "wallet drain requires persistence capabilities {:?}",
                capabilities.missing(required).names()
            )));
        }
        Ok(())
    }

    /// Only a successful complete durable read can establish NotSent. A live
    /// context wins over an absent row, including during build and failed flush.
    pub async fn wallet_drain_operation_status(
        &self,
        operation_id: [u8; 16],
    ) -> Result<AssetLockOperationSnapshot, PlatformWalletError> {
        self.wallet_drain_operation_status_inner(operation_id, None)
            .await
    }

    async fn wallet_drain_operation_status_inner(
        &self,
        operation_id: [u8; 16],
        ignore_context: Option<&Arc<AssetLockOperationContext>>,
    ) -> Result<AssetLockOperationSnapshot, PlatformWalletError> {
        self.require_wallet_drain_persistence()?;
        let persisted = self.persister.load().map_err(|error| {
            PlatformWalletError::Persistence(format!("cannot read drain operation: {error}"))
        })?;
        // A successful global load may omit a wallet whose metadata/accounts
        // could not be restored. That is not evidence that its operation never
        // reached the network; require the requested wallet's snapshot too.
        let persisted_wallet = persisted.wallets.get(&self.wallet_id).ok_or_else(|| {
            PlatformWalletError::Persistence(
                "cannot read drain operation: wallet missing from durable snapshot".into(),
            )
        })?;
        let durable = persisted_wallet
            .unused_asset_locks
            .values()
            .flat_map(|locks| locks.values())
            .find(|lock| lock.operation_id == Some(operation_id))
            .map(snapshot_for_lock);
        let live = {
            let manager = self.wallet_manager.read().await;
            let info = manager
                .get_wallet_info(&self.wallet_id)
                .ok_or_else(|| PlatformWalletError::WalletNotFound(hex::encode(self.wallet_id)))?;
            info.tracked_asset_locks
                .values()
                .find(|lock| lock.operation_id == Some(operation_id))
                .map(snapshot_for_lock)
        };
        let context = self
            .drain_operations
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .get(&operation_id)
            .filter(|context| !ignore_context.is_some_and(|ignored| Arc::ptr_eq(context, ignored)))
            .map(|context| context.snapshot());
        // Terminal evidence always wins, including a generic exact-outpoint
        // resume completing while the original context still says Pending.
        for snapshot in [live, durable, context].into_iter().flatten() {
            if snapshot.state == AssetLockOperationState::Completed {
                return Ok(snapshot);
            }
        }
        Ok(live
            .or(durable)
            .or(context)
            .unwrap_or(AssetLockOperationSnapshot {
                state: AssetLockOperationState::NotSent,
                out_point: None,
            }))
    }

    /// Register before queueing a worker, making status fail closed while
    /// the operation has not reached its first asynchronous poll yet.
    pub fn register_wallet_drain_operation(
        &self,
        operation: Arc<AssetLockOperationContext>,
    ) -> AssetLockOperationRegistration {
        let mut operations = self
            .drain_operations
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let current = operations.entry(operation.operation_id).or_insert_with(|| {
            #[cfg(any(feature = "shielded", test))]
            operation.started.store(false, Ordering::SeqCst);
            Arc::clone(&operation)
        });
        // A same-UUID contender leases the existing owner's visibility too.
        // Otherwise the owner could exit while the contender is still queued,
        // leaving status falsely NotSent until that contender starts polling.
        current.registrations.fetch_add(1, Ordering::SeqCst);
        AssetLockOperationRegistration {
            operations: Arc::clone(&self.drain_operations),
            operation: Arc::clone(current),
        }
    }

    /// Called under the wallet's shield single-flight guard. Reusing a UUID
    /// can resume its own lock, but can never create a second one.
    #[cfg(any(feature = "shielded", test))]
    pub(crate) async fn begin_wallet_drain(
        &self,
        operation: &Arc<AssetLockOperationContext>,
    ) -> Result<Option<OutPoint>, PlatformWalletError> {
        let first_attempt = !operation.started.swap(true, Ordering::SeqCst);
        let previous = self
            .wallet_drain_operation_status_inner(
                operation.operation_id,
                first_attempt.then_some(operation),
            )
            .await?;
        match previous.state {
            AssetLockOperationState::Completed => {
                operation.set(previous.state, previous.out_point);
                return Err(PlatformWalletError::AssetLockAlreadyConsumed(
                    previous.out_point.ok_or_else(|| {
                        PlatformWalletError::Persistence(
                            "completed drain has no asset-lock outpoint".into(),
                        )
                    })?,
                ));
            }
            AssetLockOperationState::Pending => {
                operation.set(previous.state, previous.out_point);
                return previous.out_point.map(Some).ok_or_else(|| {
                    PlatformWalletError::TransactionBroadcastUnconfirmed(
                        "drain operation remains unresolved; no new asset lock was built".into(),
                    )
                });
            }
            AssetLockOperationState::NotSent => {}
        }
        // A concurrent invocation may have registered the same UUID first and
        // just resolved to NotSent. Its guard still owns the slot until drop;
        // an unregistered contender must retry instead of building invisibly.
        if !self
            .drain_operations
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .get(&operation.operation_id)
            .is_some_and(|current| Arc::ptr_eq(current, operation))
        {
            return Err(PlatformWalletError::TransactionBroadcastUnconfirmed(
                "previous drain invocation is still settling; retry after its outcome resolves"
                    .into(),
            ));
        }
        // A second UUID is not a bypass around an unresolved drain. This
        // check also covers another SDK caller or a host that lost its marker.
        let persisted = self.persister.load().map_err(|error| {
            PlatformWalletError::Persistence(format!("cannot read pending drains: {error}"))
        })?;
        let persisted_wallet = persisted.wallets.get(&self.wallet_id).ok_or_else(|| {
            PlatformWalletError::Persistence(
                "cannot read pending drains: wallet missing from durable snapshot".into(),
            )
        })?;
        let durable_pending = persisted_wallet
            .unused_asset_locks
            .values()
            .flat_map(|locks| locks.values())
            .any(|lock| lock.operation_id.is_some() && lock.status != AssetLockStatus::Consumed);
        let live_locks = self.list_tracked_locks().await;
        let live_pending = live_locks
            .iter()
            .any(|lock| lock.operation_id.is_some() && lock.status != AssetLockStatus::Consumed);
        let completed = live_locks
            .iter()
            .chain(
                persisted
                    .wallets
                    .get(&self.wallet_id)
                    .into_iter()
                    .flat_map(|wallet| wallet.unused_asset_locks.values())
                    .flat_map(|locks| locks.values()),
            )
            .filter(|lock| lock.status == AssetLockStatus::Consumed)
            .filter_map(|lock| lock.operation_id)
            .collect::<BTreeSet<_>>();
        let context_pending = self
            .drain_operations
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .values()
            .any(|context| {
                !Arc::ptr_eq(context, operation)
                    && !completed.contains(&context.operation_id)
                    && context.snapshot().state == AssetLockOperationState::Pending
            });
        if durable_pending || live_pending || context_pending {
            // This UUID did not dispatch anything. Preserve the existing
            // operation, whose own status/outpoint must be resolved first.
            operation.set(AssetLockOperationState::NotSent, None);
            return Err(PlatformWalletError::AssetLockTransaction(
                "another wallet drain is pending; resume that transfer before starting a new one"
                    .into(),
            ));
        }
        operation.set(AssetLockOperationState::Pending, None);
        Ok(None)
    }
}
