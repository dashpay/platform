//! Crash recovery and resume for asset locks.
//!
//! Contains methods for recovering asset locks from persisted state,
//! resolving status from wallet info, resuming interrupted locks,
//! and re-deriving private keys.

use crate::broadcaster::{BroadcastError, TransactionBroadcaster};
use std::time::Duration;

use dashcore::Address as DashAddress;
use dashcore::OutPoint;
use key_wallet::bip32::DerivationPath;
use key_wallet::managed_account::managed_account_trait::ManagedAccountTrait;
use key_wallet::wallet::managed_wallet_info::asset_lock_builder::AssetLockFundingType;

use crate::changeset::changeset::AssetLockChangeSet;
use crate::error::PlatformWalletError;

use super::super::manager::AssetLockManager;
use super::super::tracked::{AssetLockStatus, TrackedAssetLock};
use super::tracking::BuiltPromotion;

// ---------------------------------------------------------------------------
// Blocking accessor (for synchronous / evo-tool contexts)
// ---------------------------------------------------------------------------

impl<B: TransactionBroadcaster + ?Sized> AssetLockManager<B> {
    /// Blocking version of [`recover_asset_lock`](Self::recover_asset_lock).
    ///
    /// Uses `tokio::sync::RwLock::blocking_write` / `blocking_read` — must NOT
    /// be called from within a tokio async context.
    ///
    /// When `proof` is `None`, the method looks up the transaction's actual
    /// on-chain context from `ManagedWalletInfo` to determine the correct
    /// status (and constructs a `ChainAssetLockProof` if the TX is in a
    /// chain-locked block).
    ///
    /// # Lifecycle
    ///
    /// Silently no-ops (with a warning) when the owning wallet has been
    /// removed from the `PlatformWalletManager`. This path returns `()`
    /// — it is a best-effort catch-up whose every other failure mode is
    /// already logged-and-dropped — so a stale handle is refused the
    /// same way rather than by a signature change.
    ///
    /// Like the async mutators, the authoritative check happens AFTER
    /// `status_persist_serial` is taken and before the commit-phase
    /// wallet lookup: phase 2 resolves status without any lock held and
    /// may call into host persistence, which is more than enough time
    /// for a removal (and a re-import re-creating the same
    /// deterministic id) to land. The advisory pre-check up front only
    /// saves that work.
    #[allow(clippy::too_many_arguments)]
    pub fn recover_asset_lock_blocking(
        &self,
        tx: dashcore::Transaction,
        amount: u64,
        account_index: u32,
        funding_type: AssetLockFundingType,
        identity_index: u32,
        out_point: OutPoint,
        proof: Option<dpp::prelude::AssetLockProof>,
    ) {
        if let Err(e) = self.ensure_active() {
            tracing::warn!(
                outpoint = %out_point,
                error = %e,
                "recover_asset_lock_blocking: refusing to recover through a \
                 retired asset-lock manager"
            );
            return;
        }

        // Phase 1 (lock held): claim the tracked-asset-lock slot and
        // pull the in-memory record out so the lookup work is
        // bounded to a single hashmap fetch.
        let in_memory_record = {
            let mut wm = self.wallet_manager.blocking_write();
            let Some(info) = wm.get_wallet_info_mut(&self.wallet_id) else {
                return;
            };
            if info.tracked_asset_locks.contains_key(&out_point) {
                return;
            }
            // Only fetch the in-memory record when we actually need
            // it (no proof was provided). Otherwise the proof we
            // already have determines the status without a lookup.
            if proof.is_none() {
                super::proof::funding_tx_record(
                    &info.core_wallet.accounts,
                    account_index,
                    &out_point.txid,
                )
            } else {
                None
            }
            // wm dropped here — release before persister I/O.
        };

        // Phase 2 (no lock held): resolve status. The persister
        // fallback's I/O (synchronous lookup, possibly an FFI
        // callback into a SwiftData query) is no longer serialized
        // behind the wallet-manager write lock.
        let (status, proof) = match proof {
            Some(ref p) => {
                let status = match p {
                    dpp::prelude::AssetLockProof::Instant(_) => AssetLockStatus::InstantSendLocked,
                    dpp::prelude::AssetLockProof::Chain(_) => AssetLockStatus::ChainLocked,
                };
                (status, proof)
            }
            None => self.resolve_status_with_in_memory(in_memory_record, account_index, &out_point),
        };

        // Phase 3 (locks held): commit the tracked-asset-lock entry and
        // enqueue it as one serialized unit. Without the ordering mutex
        // a concurrent flow could finalize this row and enqueue its
        // proof-bearing snapshot in the window between the insert below
        // and the enqueue, leaving the older recovered snapshot durable
        // (see `status_persist_serial`). `blocking_lock` matches the
        // `blocking_write` already used here — this method is documented
        // as callable only OUTSIDE a tokio async context.
        let _serial = self.status_persist_serial.blocking_lock();

        // Authoritative stale-handle check, under the same mutex
        // `deactivate` must hold to retire this manager — so a removal
        // racing phase 2 either finished (and this insert is refused)
        // or is still waiting for this critical section to end.
        if let Err(e) = self.ensure_active_under_serial(&_serial) {
            tracing::warn!(
                outpoint = %out_point,
                error = %e,
                "recover_asset_lock_blocking: wallet was removed while resolving \
                 the lock's status — dropping the recovery instead of writing to \
                 replacement wallet state"
            );
            return;
        }

        // We re-check `tracked_asset_locks.contains_key` because
        // another caller could have raced in during phase 2 — first
        // writer wins.
        let cs = {
            let mut wm = self.wallet_manager.blocking_write();
            let Some(info) = wm.get_wallet_info_mut(&self.wallet_id) else {
                return;
            };
            if info.tracked_asset_locks.contains_key(&out_point) {
                return;
            }
            let lock = TrackedAssetLock {
                out_point,
                transaction: tx,
                account_index,
                funding_type,
                identity_index,
                amount,
                status,
                proof,
            };
            let mut cs = AssetLockChangeSet::default();
            cs.asset_locks.insert(out_point, (&lock).into());
            info.tracked_asset_locks.insert(out_point, lock);
            cs
            // wm dropped here — must release before queue_asset_lock_changeset
            // since the persister flush may need the wallet-manager lock
            // for other sub-changesets.
        };
        self.queue_asset_lock_changeset(cs);
    }

    /// Determine asset lock status from a pre-snapshotted in-memory
    /// record, falling back to the persister if the snapshot was
    /// `None`. The caller is responsible for dropping the
    /// wallet-manager lock between the snapshot and this call so the
    /// persister fallback's I/O isn't serialized behind it.
    ///
    /// If the TX is in a chain-locked block, returns `ChainLocked` with a
    /// constructed `ChainAssetLockProof`. If the TX has an InstantSend
    /// context, returns `InstantSendLocked` (without a proof, since we lack
    /// the IS-lock data). Otherwise defaults to `Broadcast`.
    ///
    /// Persister errors are logged at `error` and treated the same as
    /// "not found" — we'd rather classify as `Broadcast` (and let
    /// `resume_asset_lock` re-derive the proof via SPV) than abort
    /// recovery entirely. The `error` log surfaces the failure to
    /// operators since this is a one-shot path: a silently-degraded
    /// `Broadcast` classification for a genuinely chain-locked tx
    /// makes `resume_asset_lock` take the wasteful `wait_for_proof`
    /// path instead of constructing a `ChainAssetLockProof`
    /// directly, with no other signal that anything went wrong.
    fn resolve_status_with_in_memory(
        &self,
        in_memory: Option<key_wallet::managed_account::transaction_record::TransactionRecord>,
        account_index: u32,
        out_point: &OutPoint,
    ) -> (AssetLockStatus, Option<dpp::prelude::AssetLockProof>) {
        use super::proof::record_or_persister;
        use key_wallet::transaction_checking::TransactionContext;

        let record = match record_or_persister(in_memory, &self.persister, &out_point.txid) {
            Ok(opt) => opt,
            Err(e) => {
                tracing::error!(
                    txid = %out_point.txid,
                    account_index,
                    error = %e,
                    "Persister fallback failed during asset-lock status \
                     recovery; classifying as Broadcast (resume will \
                     re-derive the proof via SPV)"
                );
                None
            }
        };

        match record {
            Some(record) => match &record.context {
                TransactionContext::InChainLockedBlock(_) => {
                    if let Some(height) = record.height() {
                        use dpp::identity::state_transition::asset_lock_proof::chain::ChainAssetLockProof;
                        let proof = dpp::prelude::AssetLockProof::Chain(ChainAssetLockProof {
                            core_chain_locked_height: height,
                            out_point: *out_point,
                        });
                        (AssetLockStatus::ChainLocked, Some(proof))
                    } else {
                        (AssetLockStatus::ChainLocked, None)
                    }
                }
                TransactionContext::InstantSend(_) => (AssetLockStatus::InstantSendLocked, None),
                _ => (AssetLockStatus::Broadcast, None),
            },
            None => (AssetLockStatus::Broadcast, None),
        }
    }
}

// ---------------------------------------------------------------------------
// Resumable asset lock
// ---------------------------------------------------------------------------

impl<B: TransactionBroadcaster + ?Sized> AssetLockManager<B> {
    /// Resume a tracked asset lock from whatever stage it's at.
    ///
    /// Looks up the tracked lock by `txid`, then:
    ///
    /// - **`Built`**: re-broadcasts the transaction and waits for a proof.
    /// - **`Broadcast`**: waits for a proof.
    /// - **`InstantSendLocked` / `ChainLocked`**: uses the existing proof
    ///   (upgrading a stale IS-lock to a ChainLock proof if necessary).
    ///
    /// After obtaining the proof, advances the tracked lock status and
    /// re-derives the one-time credit-output derivation path from the
    /// wallet's funding-account address pool.
    ///
    /// Returns `(proof, derivation_path)` ready for use in identity
    /// registration or top-up via the `_with_signer` SDK methods. The
    /// caller passes `derivation_path` to the same signer used for the
    /// build phase when the credit output is later consumed on Platform.
    ///
    /// `timeout` is `Option<Duration>` and is only consulted when the lock
    /// still needs a proof (`Built` / `Broadcast`): `None` waits
    /// **indefinitely** for finality. For `InstantSendLocked` / `ChainLocked`
    /// the proof already exists and no wait happens, so the value is moot.
    pub async fn resume_asset_lock(
        &self,
        out_point: &OutPoint,
        timeout: Option<Duration>,
    ) -> Result<(dpp::prelude::AssetLockProof, DerivationPath), PlatformWalletError> {
        tracing::info!(outpoint = %out_point, ?timeout, "resume_asset_lock: entered");

        // Fail a stale handle before doing any work. Advisory only —
        // this call goes on to await a broadcast and a proof, so the
        // removal it is meant to catch can equally land afterwards. The
        // guarantee comes from the same check inside
        // `promote_built_to_broadcast` / `advance_asset_lock_status`,
        // taken under `status_persist_serial`.
        self.ensure_active()?;

        // 1. Look up the tracked lock — snapshot the fields we need.
        let (tx, mut status, mut existing_proof, account_index) = {
            let wm = self.wallet_manager.read().await;
            let info = wm
                .get_wallet_info(&self.wallet_id)
                .ok_or_else(|| PlatformWalletError::WalletNotFound(hex::encode(self.wallet_id)))?;
            let tracked_count = info.tracked_asset_locks.len();
            let lock = info.tracked_asset_locks.get(out_point).ok_or_else(|| {
                tracing::warn!(
                    outpoint = %out_point,
                    tracked_count,
                    "resume_asset_lock: asset lock not in tracked_asset_locks map"
                );
                PlatformWalletError::AssetLockNotTracked(*out_point)
            })?;
            tracing::info!(
                outpoint = %out_point,
                status = ?lock.status,
                has_proof = lock.proof.is_some(),
                account_index = lock.account_index,
                "resume_asset_lock: lock looked up"
            );
            (
                lock.transaction.clone(),
                lock.status.clone(),
                lock.proof.clone(),
                lock.account_index,
            )
        };

        // Test-only pause between the read-locked snapshot above and the
        // write-locked compare-and-set below, so a test can deterministically
        // hold this resume on a stale `Built` snapshot while another flow
        // finalizes the same row. No-op unless a test installed the gate.
        #[cfg(test)]
        {
            let gate = self
                .resume_pre_promote_gate
                .lock()
                .expect("resume pre-promote gate mutex")
                .clone();
            if let Some(gate) = gate {
                // Signal first: the snapshot above is taken, so whatever the
                // test does next is guaranteed to race a stale `Built`.
                gate.arrived.notify_one();
                gate.release.notified().await;
            }
        }

        // 1b. Promote `Built` → `Broadcast` BEFORE calling `broadcast(&tx)`.
        // The snapshot above dropped the read lock, so a concurrent
        // create-path Rejected cleanup can race the re-broadcast: if the row
        // is still `Built` when `untrack_asset_lock` runs, the guard doesn't
        // fire, the row is deleted, and the funding reservation is released
        // while this call is still handing the same transaction to the
        // network. Advancing first pushes the status past `Built` under the
        // write lock, so either (a) we win and the untrack guard preserves
        // the row + reservation, or (b) untrack ran first, the row is
        // already gone, and this promotion fails before we ever call
        // `broadcast(&tx)`.
        //
        // The promotion is a compare-and-set rather than an unconditional
        // write because that same dropped read lock lets TWO resumes both
        // snapshot `Built`. If the first one broadcasts, obtains a proof and
        // finalizes the row to `InstantSendLocked` / `ChainLocked` (step 3),
        // an unconditional write from this delayed second caller would
        // downgrade the finalized row to `Broadcast` while leaving the proof
        // attached, and persist that inconsistent state. Instead we re-read
        // the row's current status and proof under the write lock and
        // re-dispatch from there — the arms below then take the already-have-
        // a-proof path instead of waiting again for a proof we already hold.
        if status == AssetLockStatus::Built {
            match self.promote_built_to_broadcast(out_point).await? {
                // The promotion queued its own changeset, atomically with
                // the compare-and-set — see `status_persist_serial`.
                BuiltPromotion::Promoted(_cs) => {}
                BuiltPromotion::AlreadyAdvanced {
                    current_status,
                    current_proof,
                } => {
                    tracing::info!(
                        outpoint = %out_point,
                        status = ?current_status,
                        has_proof = current_proof.is_some(),
                        "resume_asset_lock: row advanced past Built concurrently — \
                         re-dispatching from its current state"
                    );
                    status = current_status;
                    existing_proof = current_proof;
                }
            }
        }

        // 2. Resume from the current status.
        let proof = match status {
            AssetLockStatus::Built => {
                // Promoted to `Broadcast` in step 1b — this arm owns that
                // promotion, so it is the one that re-broadcasts.
                //
                // Hold the generation lifecycle gate across the liveness
                // check and the network send so wallet removal cannot
                // complete while this re-broadcast is still in flight
                // (host teardown would otherwise delete recovery material
                // for a transaction that can still reach the network).
                let _lifecycle = self.admit_broadcast().await?;
                match self.broadcaster.broadcast(&tx).await {
                    Ok(_) => {
                        drop(_lifecycle);
                    }
                    Err(e @ BroadcastError::Rejected { .. }) => {
                        // Keep `Broadcast`: a concurrent successful resume
                        // may own that status, and the `Broadcast` arm
                        // defensively re-broadcasts on later resumes.
                        return Err(e.into());
                    }
                    Err(e @ BroadcastError::MaybeSent { .. }) => {
                        // Outcome unknown — the tx may already be
                        // propagating. Keep `Broadcast` so a later resume
                        // can defensively re-broadcast and wait for proof.
                        return Err(e.into());
                    }
                }
                let proof = self.wait_for_proof(out_point, timeout).await?;
                self.validate_or_upgrade_proof(proof, account_index, out_point)
                    .await?
            }
            AssetLockStatus::Broadcast => {
                // Defensive re-broadcast, then wait for proof. A lock can
                // sit at `Broadcast` across app restarts long enough for
                // its funding tx to be evicted from every mempool (Core's
                // default `-mempoolexpiry` is two weeks), or the original
                // broadcast may have reached no peers at all (SPV
                // connectivity gap). Once no node holds the tx, no IS/CL
                // proof can ever arrive, and the wait — now unbounded for
                // the user-facing funding flows — would hang forever. A
                // re-broadcast revives an evicted/undelivered tx.
                //
                // Best-effort: unlike the `Built` arm, this tx was already
                // broadcast once (that's what `Broadcast` means), so it may
                // still be in a mempool or already mined — in which case the
                // network reports "already known" / "already in block
                // chain". The broadcaster can't distinguish that from a real
                // rejection (DAPI classifies every failure as `MaybeSent`),
                // so we log and proceed to `wait_for_proof` regardless
                // rather than failing the resume on a tx that is actually
                // fine. If the tx really was mined, `wait_for_proof`
                // resolves immediately from the SPV/persisted record.
                //
                // Same generation barrier as the `Built` arm: a defensive
                // re-broadcast is still a network send for this generation
                // and must not race wallet teardown.
                {
                    let _lifecycle = self.admit_broadcast().await?;
                    if let Err(e) = self.broadcaster.broadcast(&tx).await {
                        tracing::debug!(
                            outpoint = %out_point,
                            error = %e,
                            "resume_asset_lock: defensive re-broadcast of a \
                             Broadcast-status lock returned an error (likely \
                             already in a mempool or mined); proceeding to wait \
                             for proof"
                        );
                    }
                }
                let proof = self.wait_for_proof(out_point, timeout).await?;
                self.validate_or_upgrade_proof(proof, account_index, out_point)
                    .await?
            }
            AssetLockStatus::InstantSendLocked | AssetLockStatus::ChainLocked => {
                // Already have a proof — validate / upgrade if stale.
                let proof = existing_proof.ok_or_else(|| {
                    PlatformWalletError::AssetLockProofWait(format!(
                        "Asset lock {} is marked as {:?} but has no proof attached",
                        out_point, status
                    ))
                })?;
                self.validate_or_upgrade_proof(proof, account_index, out_point)
                    .await?
            }
            AssetLockStatus::RecoveredFromChain => {
                // Reconstructed from a chain-locked record after a
                // restore — Platform-side consumption is unknown. An
                // explicit resume is allowed to try consuming it:
                // Platform rejects an already-spent outpoint with a
                // typed error, and a genuinely unspent lock is real
                // recoverable value. The reconstruction path only
                // assigns this status alongside a chain proof — at
                // creation for records already finalized, or via
                // `enrich_from_record` when finality arrives later
                // (non-final detections enter as
                // `Broadcast`/`InstantSendLocked` and take those arms,
                // re-broadcast included, until then) — so the proof is
                // present by construction; the `None` arm is a
                // defensive fallback for a row whose persisted proof
                // was lost, and its wait resolves from the already
                // chain-locked record rather than blocking on new
                // network events.
                match existing_proof {
                    Some(proof) => {
                        self.validate_or_upgrade_proof(proof, account_index, out_point)
                            .await?
                    }
                    None => {
                        let proof = self.wait_for_proof(out_point, timeout).await?;
                        self.validate_or_upgrade_proof(proof, account_index, out_point)
                            .await?
                    }
                }
            }
            AssetLockStatus::Consumed => {
                // Terminal tombstone — the asset lock was already
                // burned by a successful identity registration / top-up.
                // Retaining this state makes the typed distinction from
                // an unknown outpoint available before and after restart.
                return Err(PlatformWalletError::AssetLockAlreadyConsumed(*out_point));
            }
        };

        // 3. Advance status and attach proof. A `RecoveredFromChain`
        // entry keeps its status: the resume proved (or refreshed)
        // Core-side finality, which that status already asserts — it
        // proved nothing new about Platform-side consumption, so
        // advancing into `InstantSendLocked`/`ChainLocked` (which
        // consumers read as "in flight") would silently re-enter the
        // pending window and resurrect the false-"Pending" rendering
        // on every restored lock whose resume didn't end in a spend.
        // Consumption is recorded separately (`consume_asset_lock`)
        // when the credit output actually lands on Platform.
        let new_status = if status == AssetLockStatus::RecoveredFromChain {
            AssetLockStatus::RecoveredFromChain
        } else {
            match &proof {
                dpp::prelude::AssetLockProof::Instant(_) => AssetLockStatus::InstantSendLocked,
                dpp::prelude::AssetLockProof::Chain(_) => AssetLockStatus::ChainLocked,
            }
        };
        // Queued by `advance_asset_lock_status` itself, atomically with
        // the in-memory write.
        let _cs = self
            .advance_asset_lock_status(out_point, new_status, Some(proof.clone()))
            .await?;

        // 4. Re-derive the one-time credit-output derivation path.
        let path = {
            let wm = self.wallet_manager.read().await;
            let info = wm
                .get_wallet_info(&self.wallet_id)
                .ok_or_else(|| PlatformWalletError::WalletNotFound(hex::encode(self.wallet_id)))?;
            let lock = info
                .tracked_asset_locks
                .get(out_point)
                .ok_or_else(|| PlatformWalletError::AssetLockNotTracked(*out_point))?;
            self.rederive_credit_output_path(lock).await?
        };

        Ok((proof, path))
    }

    /// Re-derive the one-time credit-output **derivation path** for a
    /// tracked asset lock.
    ///
    /// The credit output address was generated from a funding account
    /// (identity registration, top-up, etc.). This method finds that
    /// address in the funding account's address pool and retrieves
    /// its derivation path — the path is what the caller hands to a
    /// `key_wallet::signer::Signer` when later consuming the credit
    /// output on Platform.
    ///
    /// Previously this method derived the actual private key from the
    /// wallet's root xpriv; that path is no longer reachable for
    /// `ExternalSignable` wallets (the root key isn't in-process) and
    /// the signer-based architecture doesn't need it — the signer
    /// owns derivation end-to-end.
    async fn rederive_credit_output_path(
        &self,
        lock: &TrackedAssetLock,
    ) -> Result<DerivationPath, PlatformWalletError> {
        use dashcore::blockdata::transaction::special_transaction::TransactionPayload;
        use key_wallet::wallet::managed_wallet_info::asset_lock_builder::AssetLockFundingType;

        // 1. Extract the credit output from the AssetLockPayload.
        let payload = lock
            .transaction
            .special_transaction_payload
            .as_ref()
            .ok_or_else(|| {
                PlatformWalletError::AssetLockTransaction(
                    "Transaction has no special transaction payload".to_string(),
                )
            })?;
        let asset_lock_payload = match payload {
            TransactionPayload::AssetLockPayloadType(p) => p,
            _ => {
                return Err(PlatformWalletError::AssetLockTransaction(
                    "Transaction payload is not an AssetLockPayload".to_string(),
                ));
            }
        };
        let credit_output = asset_lock_payload.credit_outputs.first().ok_or_else(|| {
            PlatformWalletError::AssetLockTransaction(
                "AssetLockPayload has no credit outputs".to_string(),
            )
        })?;

        // 2. Get the address from the credit output's script_pubkey.
        let address = DashAddress::from_script(&credit_output.script_pubkey, self.sdk.network)
            .map_err(|e| {
                PlatformWalletError::AssetLockTransaction(format!(
                    "Failed to derive address from credit output script: {}",
                    e
                ))
            })?;

        // 3. Find the derivation path in the funding account and derive key under a single lock.
        let wm = self.wallet_manager.read().await;
        let info = wm
            .get_wallet_info(&self.wallet_id)
            .ok_or_else(|| PlatformWalletError::WalletNotFound(hex::encode(self.wallet_id)))?;
        let wi = &info.core_wallet;
        let funding_account = match lock.funding_type {
            AssetLockFundingType::IdentityRegistration => {
                wi.accounts.identity_registration.as_ref()
            }
            AssetLockFundingType::IdentityTopUp => {
                wi.accounts.identity_topup.get(&lock.identity_index)
            }
            AssetLockFundingType::IdentityTopUpNotBound => {
                wi.accounts.identity_topup_not_bound.as_ref()
            }
            AssetLockFundingType::IdentityInvitation => wi.accounts.identity_invitation.as_ref(),
            AssetLockFundingType::AssetLockAddressTopUp => {
                wi.accounts.asset_lock_address_topup.as_ref()
            }
            AssetLockFundingType::AssetLockShieldedAddressTopUp => {
                wi.accounts.asset_lock_shielded_address_topup.as_ref()
            }
        };

        let funding_account = funding_account.ok_or_else(|| {
            PlatformWalletError::AssetLockTransaction(format!(
                "Funding account {:?} not found for re-derivation",
                lock.funding_type
            ))
        })?;

        let derivation_path = funding_account
            .address_derivation_path(&address)
            .ok_or_else(|| {
                PlatformWalletError::AssetLockTransaction(format!(
                    "Address {} not found in funding account {:?}",
                    address, lock.funding_type
                ))
            })?;

        Ok(derivation_path)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::sync::{Arc, Mutex};
    use std::time::Duration;

    use async_trait::async_trait;
    use dashcore::hashes::Hash;
    use dashcore::{Network, OutPoint, Transaction, Txid};
    use key_wallet::account::account_collection::AccountCollection;
    use key_wallet::account::account_type::StandardAccountType;
    use key_wallet::account::{Account, AccountType};
    use key_wallet::wallet::managed_wallet_info::ManagedWalletInfo;
    use key_wallet::wallet::Wallet;
    use key_wallet_manager::WalletManager;
    use tokio::sync::{Notify, RwLock};

    use crate::broadcaster::{BroadcastError, TransactionBroadcaster};
    use crate::changeset::{
        ClientStartState, PersistenceError, PlatformWalletChangeSet, PlatformWalletPersistence,
    };
    use crate::error::PlatformWalletError;
    use crate::test_support::{funded_wallet_manager, AlwaysRejectedBroadcaster};
    use crate::wallet::asset_lock::manager::AssetLockManager;
    use crate::wallet::asset_lock::tracked::{AssetLockStatus, TrackedAssetLock};
    use crate::wallet::core::WalletGeneration;
    use crate::wallet::identity::IdentityManager;
    use crate::wallet::persister::WalletPersister;
    use crate::wallet::platform_wallet::PlatformWalletInfo;
    use crate::AssetLockFundingType;

    /// Persistence stub that records every stored changeset so the test
    /// can replay the registration rounds the way the FFI load path does.
    #[derive(Default)]
    struct RecordingPersistence {
        stored: Mutex<Vec<PlatformWalletChangeSet>>,
    }

    /// Captures the exact transaction passed to the resumed `Built` branch.
    /// Recovery must never rebuild a replacement transaction/outpoint.
    #[derive(Default)]
    struct RecordingBroadcaster {
        transactions: Mutex<Vec<Transaction>>,
    }

    #[async_trait]
    impl TransactionBroadcaster for RecordingBroadcaster {
        async fn broadcast(&self, transaction: &Transaction) -> Result<Txid, BroadcastError> {
            self.transactions
                .lock()
                .expect("recording broadcaster mutex")
                .push(transaction.clone());
            Ok(transaction.txid())
        }
    }

    impl PlatformWalletPersistence for RecordingPersistence {
        fn store(
            &self,
            _wallet_id: crate::wallet::platform_wallet::WalletId,
            changeset: PlatformWalletChangeSet,
        ) -> Result<(), PersistenceError> {
            self.stored
                .lock()
                .expect("recording persistence mutex")
                .push(changeset);
            Ok(())
        }

        fn flush(
            &self,
            _wallet_id: crate::wallet::platform_wallet::WalletId,
        ) -> Result<(), PersistenceError> {
            Ok(())
        }

        fn load(&self) -> Result<ClientStartState, PersistenceError> {
            Ok(ClientStartState::default())
        }
    }

    #[tokio::test]
    async fn built_resume_rebroadcasts_original_and_typed_failures_do_not_broadcast() {
        let (wallet_manager, wallet_id, generation, signer) =
            funded_wallet_manager(StandardAccountType::BIP44Account).await;
        let persistence = Arc::new(RecordingPersistence::default());
        let broadcaster = Arc::new(RecordingBroadcaster::default());
        let sdk = Arc::new(
            dash_sdk::SdkBuilder::new_mock()
                .with_network(Network::Testnet)
                .build()
                .expect("mock sdk"),
        );
        let manager = AssetLockManager::new(
            sdk,
            Arc::clone(&wallet_manager),
            wallet_id,
            Arc::clone(&generation),
            Arc::new(Notify::new()),
            Arc::clone(&broadcaster),
            WalletPersister::new(wallet_id, persistence),
        );
        let (transaction, _path) = manager
            .build_asset_lock_transaction(
                1_000_000,
                0,
                AssetLockFundingType::IdentityRegistration,
                4,
                &signer,
            )
            .await
            .expect("build asset lock");
        let out_point = OutPoint::new(transaction.txid(), 0);
        {
            let mut wm = wallet_manager.write().await;
            let info = wm
                .get_wallet_info_mut(&wallet_id)
                .expect("wallet must remain registered");
            info.tracked_asset_locks.insert(
                out_point,
                TrackedAssetLock {
                    out_point,
                    transaction: transaction.clone(),
                    account_index: 0,
                    funding_type: AssetLockFundingType::IdentityRegistration,
                    identity_index: 4,
                    amount: 1_000_000,
                    status: AssetLockStatus::Built,
                    proof: None,
                },
            );
        }

        let timed_out = manager
            .resume_asset_lock(&out_point, Some(Duration::from_millis(10)))
            .await
            .expect_err("no proof event should arrive");
        assert!(matches!(
            timed_out,
            PlatformWalletError::FinalityTimeout(actual) if actual == out_point
        ));
        {
            let broadcast = broadcaster
                .transactions
                .lock()
                .expect("recording broadcaster mutex");
            assert_eq!(broadcast.as_slice(), std::slice::from_ref(&transaction));
            assert_eq!(broadcast[0].txid(), out_point.txid);
        }

        // The real consume path leaves a terminal tombstone. That gives a
        // same-process retry a truthful typed error, while a foreign outpoint
        // remains distinguishable as never tracked.
        let consumed_changeset = manager
            .consume_asset_lock(&out_point)
            .await
            .expect("consume tracked lock");
        assert_eq!(
            consumed_changeset
                .asset_locks
                .get(&out_point)
                .expect("consumed snapshot")
                .status,
            AssetLockStatus::Consumed
        );
        {
            let wm = wallet_manager.read().await;
            assert_eq!(
                wm.get_wallet_info(&wallet_id)
                    .expect("wallet")
                    .tracked_asset_locks
                    .get(&out_point)
                    .expect("consumed tombstone")
                    .status,
                AssetLockStatus::Consumed
            );
        }
        let consumed = manager
            .resume_asset_lock(&out_point, Some(Duration::from_millis(10)))
            .await
            .expect_err("consumed lock must fail");
        assert!(matches!(
            consumed,
            PlatformWalletError::AssetLockAlreadyConsumed(actual) if actual == out_point
        ));

        let foreign = OutPoint::new(Txid::all_zeros(), 7);
        let unknown = manager
            .resume_asset_lock(&foreign, Some(Duration::from_millis(10)))
            .await
            .expect_err("foreign outpoint must fail");
        assert!(matches!(
            unknown,
            PlatformWalletError::AssetLockNotTracked(actual) if actual == foreign
        ));
        assert_eq!(
            broadcaster
                .transactions
                .lock()
                .expect("recording broadcaster mutex")
                .len(),
            1,
            "terminal/foreign failures must not trigger another broadcast"
        );
    }

    /// A lazily-created `IdentityTopUp` funding account must survive a
    /// restart. Its persisted registration round (account xpub + pool
    /// snapshot) is the ONLY record the load path can rebuild the account
    /// from — the `account_registrations` / `account_address_pools`
    /// changeset fields are not replayed by `apply_changeset` — so
    /// without it, `resume_asset_lock` on a relaunched wallet fails at
    /// `rederive_credit_output_path` ("Funding account IdentityTopUp not
    /// found for re-derivation") and the already-broadcast top-up is
    /// stranded.
    #[tokio::test]
    async fn topup_credit_output_path_rederives_after_restart() {
        const TOPUP_INDEX: u32 = 7;

        // --- Session 1: build a top-up asset lock. This lazily creates
        // the IdentityTopUp{7} account and must persist its registration.
        let (wallet_manager, wallet_id, generation, signer) =
            funded_wallet_manager(StandardAccountType::BIP44Account).await;
        let persistence = Arc::new(RecordingPersistence::default());
        // The mock SDK network must match the testnet wallet fixture:
        // `rederive_credit_output_path` decodes the credit-output address
        // against `sdk.network`.
        let sdk = Arc::new(
            dash_sdk::SdkBuilder::new_mock()
                .with_network(Network::Testnet)
                .build()
                .expect("mock sdk"),
        );
        let manager = AssetLockManager::new(
            Arc::clone(&sdk),
            wallet_manager,
            wallet_id,
            Arc::clone(&generation),
            Arc::new(Notify::new()),
            Arc::new(AlwaysRejectedBroadcaster),
            WalletPersister::new(
                wallet_id,
                Arc::clone(&persistence) as Arc<dyn PlatformWalletPersistence>,
            ),
        );
        let (tx, path) = manager
            .build_asset_lock_transaction(
                1_000_000,
                0,
                AssetLockFundingType::IdentityTopUp,
                TOPUP_INDEX,
                &signer,
            )
            .await
            .expect("build top-up asset lock");

        let topup_account_type = AccountType::IdentityTopUp {
            registration_index: TOPUP_INDEX,
        };
        let registrations: Vec<crate::changeset::AccountRegistrationEntry> = {
            let stored = persistence.stored.lock().expect("recording mutex");
            assert!(
                stored.iter().any(|cs| cs
                    .account_address_pools
                    .iter()
                    .any(|p| p.account_type == topup_account_type && !p.addresses.is_empty())),
                "the lazily-created top-up account must persist a non-empty \
                 initial address-pool snapshot"
            );
            stored
                .iter()
                .flat_map(|cs| cs.account_registrations.iter().cloned())
                .collect()
        };
        assert!(
            registrations
                .iter()
                .any(|r| r.account_type == topup_account_type),
            "the lazily-created top-up account must persist an \
             AccountRegistrationEntry, got {registrations:?}"
        );

        // --- "Restart": rebuild the wallet the way the FFI load path
        // does (`build_wallet_start_state`) — an external-signable
        // Wallet holding ONLY the persisted account registrations (pool
        // windows regenerate deterministically from each account xpub) —
        // and re-track the lock from its persisted row.
        let mut accounts = AccountCollection::new();
        for reg in &registrations {
            let account = Account::from_xpub(
                Some(wallet_id),
                reg.account_type,
                reg.account_xpub,
                Network::Testnet,
            )
            .expect("Account::from_xpub");
            accounts.insert(account).expect("insert restored account");
        }
        let restored_wallet = Wallet::new_external_signable(Network::Testnet, wallet_id, accounts);
        let generation = Arc::new(WalletGeneration::new());
        let mut restored_info = PlatformWalletInfo {
            core_wallet: ManagedWalletInfo::from_wallet(&restored_wallet, 0),
            generation: Arc::clone(&generation),
            identity_manager: IdentityManager::new(),
            tracked_asset_locks: BTreeMap::new(),
            dpns_name_states: BTreeMap::new(),
        };
        let out_point = OutPoint::new(tx.txid(), 0);
        let lock = TrackedAssetLock {
            out_point,
            transaction: tx,
            account_index: 0,
            funding_type: AssetLockFundingType::IdentityTopUp,
            identity_index: TOPUP_INDEX,
            amount: 1_000_000,
            status: AssetLockStatus::Built,
            proof: None,
        };
        restored_info
            .tracked_asset_locks
            .insert(out_point, lock.clone());

        let mut wm = WalletManager::<PlatformWalletInfo>::new(Network::Testnet);
        let restored_id = wm
            .insert_wallet(restored_wallet, restored_info)
            .expect("insert restored wallet");
        assert_eq!(
            restored_id, wallet_id,
            "external-signable restore must keep the persisted wallet id"
        );

        let restored_manager = AssetLockManager::new(
            sdk,
            Arc::new(RwLock::new(wm)),
            wallet_id,
            generation,
            Arc::new(Notify::new()),
            Arc::new(AlwaysRejectedBroadcaster),
            WalletPersister::new(wallet_id, persistence as Arc<dyn PlatformWalletPersistence>),
        );
        let rederived = restored_manager
            .rederive_credit_output_path(&lock)
            .await
            .expect(
                "credit-output path must re-derive from the persisted \
                 IdentityTopUp account after a restart",
            );
        assert_eq!(
            rederived, path,
            "re-derived credit-output path must match the build-time path"
        );
    }
}
