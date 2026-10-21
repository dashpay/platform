//! Exact, restart-safe association of a host drain request with its asset lock.

use std::collections::BTreeMap;
#[cfg(any(feature = "shielded", test))]
use std::sync::atomic::AtomicBool;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use dashcore::OutPoint;

use crate::broadcaster::TransactionBroadcaster;
use crate::changeset::PersistenceCapabilities;
#[cfg(any(feature = "shielded", test))]
use crate::changeset::{AssetLockChangeSet, PlatformWalletChangeSet};
use crate::error::PlatformWalletError;

use super::manager::AssetLockManager;
use super::tracked::{AssetLockStatus, TrackedAssetLock};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssetLockOperationState {
    NotSent,
    Pending,
    Completed,
    /// Original funding conflicted; retry cannot create a replacement lock.
    Conflicted,
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
    // An unpublished Built row cannot enter the live manager: generic resume
    // could dispatch it before durability. Retain it here so explicit retries
    // can repeat the checkpoint after a rolled-back or uncertain write.
    checkpoint: Mutex<Option<TrackedAssetLock>>,
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
            checkpoint: Mutex::new(None),
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

    pub(super) fn retain_checkpoint(&self, lock: TrackedAssetLock) {
        *self.checkpoint.lock().unwrap_or_else(|e| e.into_inner()) = Some(lock);
    }

    pub(super) fn clear_checkpoint(&self) {
        self.checkpoint
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .take();
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
        let snapshot = self.operation.snapshot();
        if last
            && (snapshot.out_point.is_none()
                || matches!(
                    snapshot.state,
                    AssetLockOperationState::Completed | AssetLockOperationState::Conflicted
                ))
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
        state: if lock.status == AssetLockStatus::DrainCompleted {
            AssetLockOperationState::Completed
        } else if lock.status == AssetLockStatus::Conflicted {
            AssetLockOperationState::Conflicted
        } else {
            AssetLockOperationState::Pending
        },
        out_point: Some(lock.out_point),
    }
}

impl<B: TransactionBroadcaster + ?Sized> AssetLockManager<B> {
    /// Keep queued/active callers' leases, but do not retain terminal contexts
    /// for the manager's lifetime. The durable row remains authoritative.
    fn publish_settled_operation(
        &self,
        operation_id: [u8; 16],
        out_point: OutPoint,
        state: AssetLockOperationState,
    ) {
        let mut operations = self
            .drain_operations
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        if let Some(context) = operations.get(&operation_id) {
            context.set(state, Some(out_point));
            context.clear_checkpoint();
            if context.registrations.load(Ordering::SeqCst) == 0 {
                operations.remove(&operation_id);
            }
        }
    }

    /// Durable drain-completion receipt, then live publication. Called once
    /// Platform has executed this drain's Type18, or rejected a resumed Type18
    /// because this exact outpoint was already consumed by an earlier attempt.
    /// Durable BEFORE visible: a failed store or flush leaves the live row and
    /// the context Pending, and the next resume re-derives completion from
    /// Platform's verdict. Idempotent on an already completed row.
    #[cfg(any(feature = "shielded", test))]
    pub(crate) async fn complete_wallet_drain(
        &self,
        out_point: OutPoint,
        operation_id: [u8; 16],
    ) -> Result<(), PlatformWalletError> {
        self.require_wallet_drain_persistence()?;
        {
            let mut manager = self.wallet_manager.write().await;
            let lock = manager
                .get_wallet_info_mut(&self.wallet_id)
                .and_then(|info| info.tracked_asset_locks.get_mut(&out_point))
                .filter(|lock| lock.operation_id == Some(operation_id))
                .ok_or(PlatformWalletError::AssetLockNotTracked(out_point))?;
            self.ensure_operation_evidence(lock)?;
            if lock.status != AssetLockStatus::DrainCompleted {
                let mut candidate = lock.clone();
                candidate.status = AssetLockStatus::DrainCompleted;
                candidate.proof = None;
                let mut changes = AssetLockChangeSet::default();
                changes.asset_locks.insert(out_point, (&candidate).into());
                self.persister
                    .store(PlatformWalletChangeSet {
                        asset_locks: Some(changes),
                        ..Default::default()
                    })
                    .map_err(|e| PlatformWalletError::Persistence(e.to_string()))?;
                // `require_wallet_drain_persistence` admits inline-committing
                // backends only, so the row is already durable: a failed flush
                // notification must not suppress its live publication.
                if let Err(error) = self.persister.flush() {
                    tracing::warn!(%out_point, %error, "drain completion committed inline; flush notification failed");
                }
                lock.status = AssetLockStatus::DrainCompleted;
                lock.proof = None;
            }
        }
        self.publish_settled_operation(operation_id, out_point, AssetLockOperationState::Completed);
        self.resume_dispatch_claims
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .remove(&out_point);
        Ok(())
    }

    #[cfg(any(feature = "shielded", test))]
    fn persist_drain_lock_snapshot(
        &self,
        lock: &TrackedAssetLock,
    ) -> Result<(), PlatformWalletError> {
        let mut changes = AssetLockChangeSet::default();
        changes.asset_locks.insert(lock.out_point, lock.into());
        self.persister
            .store(PlatformWalletChangeSet {
                asset_locks: Some(changes),
                ..Default::default()
            })
            .map_err(|e| PlatformWalletError::Persistence(e.to_string()))?;
        self.persister
            .flush()
            .map_err(|e| PlatformWalletError::Persistence(e.to_string()))
    }

    /// A drain needs the `WALLET_DRAIN` capabilities **and** a backend whose
    /// successful `store` is already durable. Every drain write (checkpoint,
    /// rejection removal, completion) treats "store returned Ok" as "committed"
    /// and publishes live state on that basis; a buffered backend could instead
    /// retain a changeset whose flush failed and commit it through a later,
    /// unrelated flush, leaving live and durable state disagreeing with no
    /// caller left to reconcile them. The shipped FFI and SQLite persisters
    /// commit inline; buffered backends are refused before any Core funds move.
    pub(crate) fn require_wallet_drain_persistence(&self) -> Result<(), PlatformWalletError> {
        let capabilities = self.persister.persistence_capabilities();
        let required = PersistenceCapabilities::WALLET_DRAIN;
        if !capabilities.contains(required) {
            return Err(PlatformWalletError::Persistence(format!(
                "wallet drain requires persistence capabilities {:?}",
                capabilities.missing(required).names()
            )));
        }
        if !self.persister.store_commits_inline() {
            return Err(PlatformWalletError::Persistence(
                "wallet drain requires a persistence backend that commits each store inline; \
                 buffered backends are not supported"
                    .into(),
            ));
        }
        Ok(())
    }

    /// Generic exact-outpoint retries obey the same durable ordering as UUID
    /// retries. A stale live snapshot cannot bypass a committed conflict.
    pub(super) fn ensure_operation_evidence(
        &self,
        lock: &TrackedAssetLock,
    ) -> Result<(), PlatformWalletError> {
        if lock.status == AssetLockStatus::Conflicted {
            return Err(PlatformWalletError::AssetLockConflicted(lock.out_point));
        }
        let Some(operation_id) = lock.operation_id else {
            return Ok(());
        };
        self.require_wallet_drain_persistence()?;
        let persisted = self.persister.load().map_err(|error| {
            PlatformWalletError::Persistence(format!("cannot read operation evidence: {error}"))
        })?;
        let durable = persisted
            .wallets
            .get(&self.wallet_id)
            .and_then(|wallet| {
                wallet
                    .unused_asset_locks
                    .values()
                    .flat_map(|locks| locks.values())
                    .find(|row| row.operation_id == Some(operation_id))
            })
            .ok_or_else(|| {
                PlatformWalletError::Persistence(
                    "operation evidence missing from durable snapshot".into(),
                )
            })?;
        if durable.status == AssetLockStatus::Conflicted {
            return Err(PlatformWalletError::AssetLockConflicted(durable.out_point));
        }
        if durable.out_point != lock.out_point
            || durable.operation_revision != lock.operation_revision
        {
            return Err(PlatformWalletError::Persistence(
                "live operation evidence is stale".into(),
            ));
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
        let durable_lock = persisted_wallet
            .unused_asset_locks
            .values()
            .flat_map(|locks| locks.values())
            .find(|lock| lock.operation_id == Some(operation_id));
        let durable = durable_lock.map(snapshot_for_lock);
        let live_lock = {
            let manager = self.wallet_manager.read().await;
            let info = manager
                .get_wallet_info(&self.wallet_id)
                .ok_or_else(|| PlatformWalletError::WalletNotFound(hex::encode(self.wallet_id)))?;
            info.tracked_asset_locks
                .values()
                .find(|lock| lock.operation_id == Some(operation_id))
                .cloned()
        };
        let live = live_lock.as_ref().map(snapshot_for_lock);
        let context = self
            .drain_operations
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .get(&operation_id)
            .filter(|context| !ignore_context.is_some_and(|ignored| Arc::ptr_eq(context, ignored)))
            .map(|context| context.snapshot());
        if let Some(snapshot) = durable {
            if matches!(
                snapshot.state,
                AssetLockOperationState::Completed | AssetLockOperationState::Conflicted
            ) {
                if let Some(out_point) = snapshot.out_point {
                    self.publish_settled_operation(operation_id, out_point, snapshot.state);
                }
            }
            // A live conflict that has not committed blocks dispatch, but is
            // not yet a durable outcome. Never publish it as such.
            if snapshot.state == AssetLockOperationState::Pending
                && live_lock.as_ref().is_some_and(|live| {
                    live.status == AssetLockStatus::Conflicted
                        && durable_lock.is_some_and(|durable| {
                            live.operation_revision >= durable.operation_revision
                        })
                })
            {
                return Err(PlatformWalletError::TransactionBroadcastUnconfirmed(
                    "asset lock conflict evidence is awaiting persistence".into(),
                ));
            }
            return Ok(snapshot);
        }
        if live_lock.as_ref().is_some_and(|lock| {
            lock.operation_revision > 0 || lock.status == AssetLockStatus::Conflicted
        }) {
            return Err(PlatformWalletError::Persistence(
                "operation conflict evidence is missing from durable snapshot".into(),
            ));
        }
        Ok(live.or(context).unwrap_or(AssetLockOperationSnapshot {
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

    /// Retry an unpublished checkpoint or repair the gap between durability
    /// and live tracking. Status remains read-only; only an explicit retry
    /// can make the exact operation's missing row durable and resumable.
    #[cfg(any(feature = "shielded", test))]
    async fn hydrate_wallet_drain(
        &self,
        operation_id: [u8; 16],
        out_point: OutPoint,
    ) -> Result<(), PlatformWalletError> {
        let _checkpoint_guard = self.build_persist_serial.lock().await;
        let context = self
            .drain_operations
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .get(&operation_id)
            .cloned();
        let persisted = self.persister.load().map_err(|error| {
            PlatformWalletError::Persistence(format!("cannot restore checkpointed drain: {error}"))
        })?;
        let wallet = persisted.wallets.get(&self.wallet_id).ok_or_else(|| {
            PlatformWalletError::Persistence(
                "cannot restore checkpointed drain: wallet missing from durable snapshot".into(),
            )
        })?;
        let durable = wallet
            .unused_asset_locks
            .values()
            .find_map(|locks| locks.get(&out_point));
        let candidate = if let Some(durable) = durable {
            if durable.operation_id != Some(operation_id) {
                return Err(PlatformWalletError::Persistence(
                    "checkpointed drain conflicts with the durable operation UUID".into(),
                ));
            }
            durable.clone()
        } else {
            let candidate = context
                .as_ref()
                .and_then(|context| {
                    context
                        .checkpoint
                        .lock()
                        .unwrap_or_else(|e| e.into_inner())
                        .clone()
                })
                .filter(|lock| {
                    lock.out_point == out_point && lock.operation_id == Some(operation_id)
                })
                .ok_or(PlatformWalletError::AssetLockNotTracked(out_point))?;
            // No replacement transaction or funding key is created. Keep the
            // candidate and spend fence if this retry also fails/cancels.
            self.persist_drain_lock_snapshot(&candidate)?;
            candidate
        };
        if candidate.status == AssetLockStatus::Conflicted {
            return Err(PlatformWalletError::AssetLockConflicted(out_point));
        }
        let mut manager = self.wallet_manager.write().await;
        let info = manager
            .get_wallet_info_mut(&self.wallet_id)
            .ok_or_else(|| PlatformWalletError::WalletNotFound(hex::encode(self.wallet_id)))?;
        // A durable newer generation replaces stale live evidence. Within the
        // same generation, preserve a live proof/consumption advance.
        if let Some(live) = info.tracked_asset_locks.get(&out_point) {
            if live.operation_revision > candidate.operation_revision
                || (live.operation_revision == candidate.operation_revision
                    && live.status == AssetLockStatus::Conflicted)
            {
                return Err(PlatformWalletError::TransactionBroadcastUnconfirmed(
                    "asset lock evidence is awaiting persistence".into(),
                ));
            }
            if live.operation_revision < candidate.operation_revision {
                info.tracked_asset_locks
                    .insert(out_point, candidate.clone());
            }
        }
        let current = info
            .tracked_asset_locks
            .entry(out_point)
            .or_insert(candidate);
        if current.operation_id != Some(operation_id) {
            return Err(PlatformWalletError::Persistence(
                "checkpointed drain conflicts with the live operation UUID".into(),
            ));
        }
        if let Some(context) = &context {
            context.clear_checkpoint();
        }
        Ok(())
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
                let out_point = previous.out_point.ok_or_else(|| {
                    PlatformWalletError::TransactionBroadcastUnconfirmed(
                        "drain operation remains unresolved; no new asset lock was built".into(),
                    )
                })?;
                self.hydrate_wallet_drain(operation.operation_id, out_point)
                    .await?;
                return Ok(Some(out_point));
            }
            AssetLockOperationState::Conflicted => {
                operation.set(previous.state, previous.out_point);
                return Err(PlatformWalletError::AssetLockConflicted(
                    previous.out_point.ok_or_else(|| {
                        PlatformWalletError::Persistence("conflicted drain has no outpoint".into())
                    })?,
                ));
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
            .any(|lock| {
                lock.operation_id.is_some()
                    && !matches!(
                        lock.status,
                        AssetLockStatus::DrainCompleted | AssetLockStatus::Conflicted
                    )
            });
        let live_locks = self.list_tracked_locks().await;
        let settled = persisted_wallet
            .unused_asset_locks
            .values()
            .flat_map(|locks| locks.values())
            .filter(|lock| {
                matches!(
                    lock.status,
                    AssetLockStatus::DrainCompleted | AssetLockStatus::Conflicted
                )
            })
            .filter_map(|lock| lock.operation_id.map(|id| (id, lock.operation_revision)))
            .collect::<BTreeMap<_, _>>();
        let live_pending = live_locks.iter().any(|lock| {
            lock.operation_id.is_some_and(|id| {
                settled
                    .get(&id)
                    .is_none_or(|revision| *revision < lock.operation_revision)
            }) && !matches!(
                lock.status,
                AssetLockStatus::DrainCompleted | AssetLockStatus::Conflicted
            )
        });
        let context_pending = self
            .drain_operations
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .values()
            .any(|context| {
                !Arc::ptr_eq(context, operation)
                    && !settled.contains_key(&context.operation_id)
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
