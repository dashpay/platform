//! Crash recovery and resume for asset locks.
//!
//! Contains methods for recovering asset locks from persisted state,
//! resolving status from wallet info, resuming interrupted locks,
//! and re-deriving private keys.

use crate::broadcaster::{BroadcastError, TransactionBroadcaster};
use std::collections::BTreeSet;
use std::time::Duration;

use dashcore::Address as DashAddress;
use dashcore::{OutPoint, Txid};
use dpp::prelude::CoreBlockHeight;
use key_wallet::bip32::DerivationPath;
use key_wallet::managed_account::managed_account_trait::ManagedAccountTrait;
use key_wallet::wallet::managed_wallet_info::asset_lock_builder::AssetLockFundingType;
use key_wallet::wallet::managed_wallet_info::wallet_info_interface::WalletInfoInterface;

use crate::changeset::changeset::AssetLockChangeSet;
use crate::error::PlatformWalletError;
use crate::wallet::platform_wallet::PlatformWalletInfo;

use super::super::manager::AssetLockManager;
use super::super::tracked::{AssetLockStatus, TrackedAssetLock};

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

        // Phase 3 (lock held): commit the tracked-asset-lock entry.
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

/// Find the first outpoint of `lock`'s transaction that some **other,
/// confirmed** transaction of this wallet already spent, returning
/// `(conflicting_input, spending_txid, spender_height,
/// spender_chain_locked)`.
///
/// A hit means the asset lock is a double spend of a settled outpoint.
/// Peers reject such a transaction at the mempool boundary and relay
/// nothing back — Core has not sent BIP61 `reject` messages by default
/// since 0.17 — so the lock can neither be mined nor IS-locked, and a
/// proof wait on it never terminates. Callers turn a hit into
/// [`PlatformWalletError::AssetLockInputConflict`] instead of
/// (re-)broadcasting into that void.
///
/// **The gate is `is_confirmed()`, deliberately not `is_chain_locked()`.**
/// Under the default `keep-finalized-transactions = OFF` build,
/// `apply_chain_lock` evicts a record the moment a chainlock buries it and
/// retains only the txid, so a chainlocked spender essentially never
/// appears in `transaction_history()` at all: demanding ChainLock finality
/// here would make the whole screen dead code in production while leaving
/// the very failure it exists for — an old, long-settled spender — reported
/// as an unbounded proof wait.
///
/// Condemning the lock on a merely-`InBlock` sibling is fund-safe. That
/// sibling is necessarily one of this wallet's own transactions (nobody
/// else can sign this wallet's outpoints), so the value it carries is
/// already the wallet's; discarding the conflicted lock strands nothing.
/// Even in the freak case where a reorg unmines the sibling, the inputs
/// return to this wallet's spendable set and fund a fresh lock — whereas
/// the conflicted lock itself would still be unrelayable for as long as
/// the sibling stood. `spender_chain_locked` is reported alongside the hit
/// purely so a host can express confidence in what it shows the user; it
/// is not a gate on raising the error.
///
/// **Best-effort in one direction only.** A hit is conclusive: the
/// spender is a confirmed transaction sitting in this wallet's own
/// history, and confirmed spends of an outpoint are mutually exclusive.
/// A miss proves nothing — for the same eviction reason above, precisely
/// the oldest and therefore most likely conflicts are invisible here. A
/// lock that clears this scan may still be a double spend, and the
/// existing timeout path remains its only backstop. Do not restructure
/// callers to treat "no conflict" as proof of liveness.
///
/// Confirmation is required rather than mere presence: an unconfirmed
/// sibling that spends the same outpoint is a competing candidate, not a
/// verdict. Either transaction can still win, and the tracked lock is
/// often the one the user actually wants to push through, so a mempool
/// record must not condemn it.
fn first_confirmed_input_conflict(
    info: &PlatformWalletInfo,
    lock: &TrackedAssetLock,
) -> Option<(OutPoint, Txid, Option<CoreBlockHeight>, bool)> {
    let lock_txid = lock.transaction.txid();
    let lock_inputs: BTreeSet<OutPoint> = lock
        .transaction
        .input
        .iter()
        .map(|input| input.previous_output)
        .collect();
    // A record surviving in history is usually still `InBlock` even when
    // the wallet's chainlock boundary has moved past its height — the
    // promotion is what evicts it. Consulting the boundary as well as the
    // record's own context is what keeps the reported finality honest for
    // the window between the two.
    let chain_locked_height = info
        .core_wallet
        .last_applied_chain_lock()
        .map(|chain_lock| chain_lock.block_height);

    // The persistence mirror's answer, restored at load. This is the only
    // source that works at app-launch catch-up: `transaction_history()` is
    // empty then except for the unresolved locks themselves, so the scan
    // below has nothing to find however dead the lock is. The mirror knows
    // because it recorded which transaction took the outpoint.
    if let Some((input, spend)) = lock_inputs.iter().find_map(|input| {
        info.restored_asset_lock_input_spends
            .get_key_value(input)
            .filter(|(_, spend)| spend.spender != lock_txid && spend.in_block)
    }) {
        return Some((*input, spend.spender, spend.height, spend.chain_locked));
    }

    info.core_wallet
        .transaction_history()
        .into_iter()
        .filter(|record| record.txid != lock_txid && record.is_confirmed())
        .find_map(|record| {
            let conflicting_input = record
                .transaction
                .input
                .iter()
                .map(|input| input.previous_output)
                .find(|outpoint| lock_inputs.contains(outpoint))?;
            let height = record.height();
            let spender_chain_locked = record.context.is_chain_locked()
                || chain_locked_height
                    .zip(height)
                    .is_some_and(|(boundary, spender_height)| spender_height <= boundary);
            Some((conflicting_input, record.txid, height, spender_chain_locked))
        })
}

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
    ///
    /// A `Built` / `Broadcast` lock is first screened by
    /// [`first_confirmed_input_conflict`]; a hit short-circuits to
    /// [`PlatformWalletError::AssetLockInputConflict`] without broadcasting
    /// or waiting, because such a lock is a double spend that no peer will
    /// relay. That screen is one-sided — read its docs before treating a
    /// clean pass as evidence the lock is alive.
    pub async fn resume_asset_lock(
        &self,
        out_point: &OutPoint,
        timeout: Option<Duration>,
    ) -> Result<(dpp::prelude::AssetLockProof, DerivationPath), PlatformWalletError> {
        tracing::info!(outpoint = %out_point, ?timeout, "resume_asset_lock: entered");

        // 1. Look up the tracked lock — snapshot the fields we need.
        let (tx, status, existing_proof, account_index, input_conflict) = {
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
            // Only the two proof-less statuses are candidates. A lock
            // carrying an IS/Chain proof, a `RecoveredFromChain` entry
            // (reconstructed from a record the chain itself accepted), and
            // a `Consumed` tombstone are all settled by evidence stronger
            // than this scan; re-classifying one of them as a double spend
            // on the strength of an unrelated history record would
            // invalidate a lock the network already honoured.
            let input_conflict = match lock.status {
                AssetLockStatus::Built | AssetLockStatus::Broadcast => {
                    first_confirmed_input_conflict(info, lock)
                }
                AssetLockStatus::InstantSendLocked
                | AssetLockStatus::ChainLocked
                | AssetLockStatus::RecoveredFromChain
                | AssetLockStatus::Consumed => None,
            };
            (
                lock.transaction.clone(),
                lock.status.clone(),
                lock.proof.clone(),
                lock.account_index,
                input_conflict,
            )
        };

        // Fail before the `Built` / `Broadcast` arms reach their
        // (re-)broadcast and their proof wait: the transaction is a double
        // spend of a settled outpoint, so the broadcast is discarded
        // without a reply and the wait — unbounded for the user-facing
        // funding flows — would never return. The typed error is what lets
        // a host offer to discard the lock instead of showing a spinner
        // forever.
        if let Some((input, spent_by, height, spender_chain_locked)) = input_conflict {
            tracing::warn!(
                outpoint = %out_point,
                %input,
                %spent_by,
                ?height,
                spender_chain_locked,
                "resume_asset_lock: asset lock double-spends an outpoint \
                 already consumed by a confirmed transaction; it can never \
                 confirm"
            );
            return Err(PlatformWalletError::AssetLockInputConflict {
                out_point: *out_point,
                input,
                spent_by,
                height,
                spender_chain_locked,
            });
        }

        // 2. Resume from the current status.
        let proof = match status {
            AssetLockStatus::Built => {
                // Re-broadcast and wait for proof.
                //
                // Only a DEFINITE rejection stops the resume. `MaybeSent`
                // means the outcome is unknown — and for a lock stuck at
                // `Built` that is the expected answer when the app died
                // between a successful broadcast and this status advance:
                // the tx is in a mempool (or mined) and every re-broadcast
                // reports the same ambiguity. Failing on it left the lock at
                // `Built` forever, so each recovery pass repeated the same
                // broadcast and the same abort, and the top-up never
                // completed. Advancing to `Broadcast` and waiting matches
                // what the `Broadcast` arm below already does with the
                // identical signal.
                match self.broadcaster.broadcast(&tx).await {
                    Ok(_) => {}
                    Err(BroadcastError::MaybeSent { reason }) => {
                        tracing::warn!(
                            outpoint = %out_point,
                            reason = %reason,
                            "resume_asset_lock: re-broadcast of a Built lock returned an \
                             unknown outcome (the network may already hold this tx); \
                             advancing to Broadcast and waiting for proof"
                        );
                    }
                    Err(rejected) => return Err(rejected.into()),
                }
                let cs = self
                    .advance_asset_lock_status(out_point, AssetLockStatus::Broadcast, None)
                    .await?;
                self.queue_asset_lock_changeset(cs);
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
        let cs = self
            .advance_asset_lock_status(out_point, new_status, Some(proof.clone()))
            .await?;
        self.queue_asset_lock_changeset(cs);

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
    use dashcore::{BlockHash, Network, OutPoint, Transaction, TxIn, Txid};
    use key_wallet::account::account_collection::AccountCollection;
    use key_wallet::account::account_type::StandardAccountType;
    use key_wallet::account::{Account, AccountType};
    use key_wallet::managed_account::managed_account_trait::ManagedAccountTrait;
    use key_wallet::managed_account::transaction_record::{
        TransactionDirection, TransactionRecord,
    };
    use key_wallet::transaction_checking::{BlockInfo, TransactionContext, TransactionType};
    use key_wallet::wallet::managed_wallet_info::ManagedWalletInfo;
    use key_wallet::wallet::Wallet;
    use key_wallet_manager::WalletManager;
    use tokio::sync::{Notify, RwLock};

    use crate::broadcaster::{BroadcastError, TransactionBroadcaster};
    use crate::changeset::{
        ClientStartState, PersistenceError, PlatformWalletChangeSet, PlatformWalletPersistence,
    };
    use crate::error::PlatformWalletError;
    use crate::test_support::{
        funded_wallet_manager, AlwaysMaybeSentBroadcaster, AlwaysRejectedBroadcaster,
    };
    use crate::wallet::asset_lock::manager::AssetLockManager;
    use crate::wallet::asset_lock::tracked::{AssetLockStatus, TrackedAssetLock};
    use crate::wallet::core::WalletGeneration;
    use crate::wallet::identity::IdentityManager;
    use crate::wallet::persister::WalletPersister;
    use crate::wallet::platform_wallet::{PlatformWalletInfo, WalletId};
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
        let (wallet_manager, wallet_id, _balance, signer) =
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

    /// Builds a tracked `Built`-status lock on a funded wallet and resumes it
    /// through `broadcaster`, returning the resume error and the lock's status
    /// afterwards. Shared by the two ambiguity/rejection cases below.
    async fn resume_built_lock_with(
        broadcaster: Arc<dyn TransactionBroadcaster>,
    ) -> (PlatformWalletError, AssetLockStatus) {
        let (wallet_manager, wallet_id, _balance, signer) =
            funded_wallet_manager(StandardAccountType::BIP44Account).await;
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
            Arc::new(Notify::new()),
            broadcaster,
            WalletPersister::new(wallet_id, Arc::new(RecordingPersistence::default())),
        );
        let (transaction, _path) = manager
            .build_asset_lock_transaction(
                1_000_000,
                0,
                AssetLockFundingType::IdentityTopUp,
                4,
                &signer,
            )
            .await
            .expect("build asset lock");
        let out_point = OutPoint::new(transaction.txid(), 0);
        {
            let mut wm = wallet_manager.write().await;
            wm.get_wallet_info_mut(&wallet_id)
                .expect("wallet must remain registered")
                .tracked_asset_locks
                .insert(
                    out_point,
                    TrackedAssetLock {
                        out_point,
                        transaction,
                        account_index: 0,
                        funding_type: AssetLockFundingType::IdentityTopUp,
                        identity_index: 4,
                        amount: 1_000_000,
                        status: AssetLockStatus::Built,
                        proof: None,
                    },
                );
        }

        let error = manager
            .resume_asset_lock(&out_point, Some(Duration::from_millis(10)))
            .await
            .expect_err("no proof event should arrive in either case");
        let status = wallet_manager
            .read()
            .await
            .get_wallet_info(&wallet_id)
            .expect("wallet")
            .tracked_asset_locks
            .get(&out_point)
            .expect("lock stays tracked")
            .status
            .clone();
        (error, status)
    }

    /// An AMBIGUOUS re-broadcast must not end the resume. A lock sitting at
    /// `Built` whose transaction was in fact already broadcast (app killed
    /// between the send and the status advance) draws `MaybeSent` on every
    /// retry, so failing on it pinned the lock at `Built` forever and the
    /// top-up could never complete. It must advance to `Broadcast` and go on
    /// to wait for the proof — here, until the 10ms test timeout.
    #[tokio::test]
    async fn built_resume_survives_an_ambiguous_rebroadcast_and_advances() {
        let (error, status) = resume_built_lock_with(Arc::new(AlwaysMaybeSentBroadcaster)).await;

        assert!(
            matches!(error, PlatformWalletError::FinalityTimeout(_)),
            "resume must reach the proof wait, not fail on the broadcast: {error:?}"
        );
        assert_eq!(
            status,
            AssetLockStatus::Broadcast,
            "an ambiguous re-broadcast must still advance the lock, or every \
             later pass repeats the same broadcast and the same failure"
        );
    }

    /// A DEFINITE rejection is the opposite case and must keep failing the
    /// resume: nothing is on the network, so no proof can ever arrive, and
    /// the lock stays at `Built` for a later retry to re-send.
    #[tokio::test]
    async fn built_resume_still_fails_on_a_definite_rejection() {
        let (error, status) = resume_built_lock_with(Arc::new(AlwaysRejectedBroadcaster)).await;

        assert!(
            matches!(error, PlatformWalletError::TransactionBroadcast(_)),
            "a definite rejection must surface as a broadcast failure: {error:?}"
        );
        assert_eq!(
            status,
            AssetLockStatus::Built,
            "a tx that never entered the network must stay resumable at Built"
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
        let (wallet_manager, wallet_id, _balance, signer) =
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
        let mut restored_info = PlatformWalletInfo {
            core_wallet: ManagedWalletInfo::from_wallet(&restored_wallet, 0),
            generation: Arc::new(WalletGeneration::new()),
            identity_manager: IdentityManager::new(),
            tracked_asset_locks: BTreeMap::new(),
            restored_asset_lock_input_spends: Default::default(),
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

    // -----------------------------------------------------------------
    // Input-conflict screen (double-spent asset locks)
    // -----------------------------------------------------------------

    /// Everything the input-conflict tests need: a funded wallet, a built
    /// asset-lock transaction over its spendable UTXO, its outpoint, and a
    /// manager whose broadcaster records every send so a test can prove
    /// the screen fired *before* the (re-)broadcast rather than after it.
    struct ConflictFixture {
        wallet_manager: Arc<RwLock<WalletManager<PlatformWalletInfo>>>,
        wallet_id: WalletId,
        manager: AssetLockManager<RecordingBroadcaster>,
        broadcaster: Arc<RecordingBroadcaster>,
        transaction: Transaction,
        out_point: OutPoint,
    }

    impl ConflictFixture {
        async fn new() -> Self {
            let (wallet_manager, wallet_id, _generation, signer) =
                funded_wallet_manager(StandardAccountType::BIP44Account).await;
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
                Arc::new(Notify::new()),
                Arc::clone(&broadcaster),
                WalletPersister::new(wallet_id, Arc::new(RecordingPersistence::default())),
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
            Self {
                wallet_manager,
                wallet_id,
                manager,
                broadcaster,
                transaction,
                out_point,
            }
        }

        /// The single outpoint the asset-lock transaction spends — the one
        /// a rescan-resurrected UTXO would have handed it a second time.
        fn funded_input(&self) -> OutPoint {
            self.transaction
                .input
                .first()
                .expect("asset lock spends at least one input")
                .previous_output
        }

        async fn track(
            &self,
            status: AssetLockStatus,
            proof: Option<dpp::prelude::AssetLockProof>,
        ) {
            let mut wm = self.wallet_manager.write().await;
            let info = wm
                .get_wallet_info_mut(&self.wallet_id)
                .expect("wallet must remain registered");
            info.tracked_asset_locks.insert(
                self.out_point,
                TrackedAssetLock {
                    out_point: self.out_point,
                    transaction: self.transaction.clone(),
                    account_index: 0,
                    funding_type: AssetLockFundingType::IdentityRegistration,
                    identity_index: 4,
                    amount: 1_000_000,
                    status,
                    proof,
                },
            );
        }

        /// File `record` in the wallet's BIP44 account by direct map
        /// insertion. Going through the detection pipeline instead would
        /// route the record by relevance and, for a chainlocked context,
        /// evict it again under the default `keep-finalized-transactions`
        /// build — the scan under test reads `transaction_history()`, so
        /// the record has to actually be there.
        async fn file_record(&self, record: TransactionRecord) {
            let mut wm = self.wallet_manager.write().await;
            let info = wm
                .get_wallet_info_mut(&self.wallet_id)
                .expect("wallet must remain registered");
            info.core_wallet
                .accounts
                .standard_bip44_accounts
                .get_mut(&0)
                .expect("funded fixture has BIP44 account 0")
                .transactions_mut()
                .insert(record.txid, record);
        }

        fn broadcast_count(&self) -> usize {
            self.broadcaster
                .transactions
                .lock()
                .expect("recording broadcaster mutex")
                .len()
        }
    }

    /// Wrap `transaction` as a history record filed against BIP44 account 0.
    fn record_for(transaction: Transaction, context: TransactionContext) -> TransactionRecord {
        TransactionRecord::new(
            transaction,
            AccountType::Standard {
                index: 0,
                standard_account_type: StandardAccountType::BIP44Account,
            },
            context,
            TransactionType::Standard,
            TransactionDirection::Outgoing,
            Vec::new(),
            Vec::new(),
            0,
        )
    }

    /// A distinct transaction that spends `spends`. Its txid falls out of
    /// the inputs, so it never collides with the asset lock's own.
    fn transaction_spending(spends: OutPoint) -> Transaction {
        Transaction {
            version: 1,
            lock_time: 0,
            input: vec![TxIn {
                previous_output: spends,
                ..Default::default()
            }],
            output: Vec::new(),
            special_transaction_payload: None,
        }
    }

    fn confirmed_at(height: u32) -> TransactionContext {
        TransactionContext::InBlock(BlockInfo::new(
            height,
            BlockHash::all_zeros(),
            1_700_000_000,
        ))
    }

    fn chain_locked_at(height: u32) -> TransactionContext {
        TransactionContext::InChainLockedBlock(BlockInfo::new(
            height,
            BlockHash::all_zeros(),
            1_700_000_000,
        ))
    }

    /// The incident this screen exists for: a restored wallet re-spends an
    /// outpoint one of its own earlier, already-confirmed transactions
    /// consumed long ago. Peers drop the double spend without a reply, so
    /// the pre-existing behaviour — re-broadcast, then wait, unbounded for
    /// the user-facing funding flows — could never terminate. The resume
    /// must fail with the typed terminal error and must not touch the
    /// network on the way out.
    ///
    /// The spender here is merely `InBlock`, which is the shape the screen
    /// actually meets in production: under the default
    /// `keep-finalized-transactions = OFF` build a chainlocked record is
    /// evicted from history, so a chainlock gate would never fire. The
    /// error is raised all the same, reporting the weaker finality rather
    /// than withholding the verdict.
    #[tokio::test]
    async fn broadcast_resume_reports_input_conflict_when_a_confirmed_tx_spent_the_input() {
        let fixture = ConflictFixture::new().await;
        fixture.track(AssetLockStatus::Broadcast, None).await;

        let spender = transaction_spending(fixture.funded_input());
        let spender_txid = spender.txid();
        fixture
            .file_record(record_for(spender, confirmed_at(1_234)))
            .await;

        let error = fixture
            .manager
            .resume_asset_lock(&fixture.out_point, Some(Duration::from_millis(10)))
            .await
            .expect_err("a double-spent asset lock must fail, not wait");
        match error {
            PlatformWalletError::AssetLockInputConflict {
                out_point,
                input,
                spent_by,
                height,
                spender_chain_locked,
            } => {
                assert_eq!(out_point, fixture.out_point);
                assert_eq!(input, fixture.funded_input());
                assert_eq!(spent_by, spender_txid);
                assert_eq!(height, Some(1_234));
                assert!(
                    !spender_chain_locked,
                    "an InBlock spender under no applied chainlock must \
                     report the weaker finality, not claim ChainLock"
                );
            }
            other => panic!("expected AssetLockInputConflict, got {other:?}"),
        }
        assert_eq!(
            fixture.broadcast_count(),
            0,
            "the screen must short-circuit ahead of the defensive re-broadcast"
        );
    }

    /// The same verdict with the strongest available evidence behind it: a
    /// spender sitting in a chain-locked block. Hosts render the difference
    /// as confidence, so the flag has to travel out with the error rather
    /// than being re-derived from the message.
    #[tokio::test]
    async fn input_conflict_reports_a_chain_locked_spender_as_chain_locked() {
        let fixture = ConflictFixture::new().await;
        fixture.track(AssetLockStatus::Broadcast, None).await;

        let spender = transaction_spending(fixture.funded_input());
        let spender_txid = spender.txid();
        fixture
            .file_record(record_for(spender, chain_locked_at(1_234)))
            .await;

        let error = fixture
            .manager
            .resume_asset_lock(&fixture.out_point, Some(Duration::from_millis(10)))
            .await
            .expect_err("a double-spent asset lock must fail, not wait");
        let rendered = error.to_string();
        match error {
            PlatformWalletError::AssetLockInputConflict {
                spent_by,
                height,
                spender_chain_locked,
                ..
            } => {
                assert_eq!(spent_by, spender_txid);
                assert_eq!(height, Some(1_234));
                assert!(
                    spender_chain_locked,
                    "an InChainLockedBlock spender must report ChainLock finality"
                );
            }
            other => panic!("expected AssetLockInputConflict, got {other:?}"),
        }
        assert!(
            rendered.contains("chainlocked: true"),
            "the rendered Display must carry the spender's finality: {rendered}"
        );
        assert_eq!(
            fixture.broadcast_count(),
            0,
            "the screen must short-circuit ahead of the defensive re-broadcast"
        );
    }

    /// An unconfirmed sibling spending the same outpoint is a competing
    /// candidate, not a verdict — either transaction can still win, and
    /// condemning the tracked lock on a mempool record would discard a
    /// perfectly live funding attempt. The resume must take its normal
    /// course (re-broadcast, then wait) instead.
    #[tokio::test]
    async fn broadcast_resume_ignores_an_unconfirmed_spend_of_the_same_input() {
        let fixture = ConflictFixture::new().await;
        fixture.track(AssetLockStatus::Broadcast, None).await;

        fixture
            .file_record(record_for(
                transaction_spending(fixture.funded_input()),
                TransactionContext::Mempool,
            ))
            .await;

        let error = fixture
            .manager
            .resume_asset_lock(&fixture.out_point, Some(Duration::from_millis(10)))
            .await
            .expect_err("no proof event should arrive within the deadline");
        assert!(
            !matches!(error, PlatformWalletError::AssetLockInputConflict { .. }),
            "an unconfirmed conflict must not condemn the lock, got {error:?}"
        );
        assert_eq!(
            fixture.broadcast_count(),
            1,
            "the resume must still reach its defensive re-broadcast"
        );
    }

    /// The asset-lock transaction is itself filed in wallet history once
    /// it is seen on chain, and it necessarily spends every outpoint it
    /// spends. Matching on the outpoints alone would therefore make every
    /// confirmed lock report itself as its own double spend; the txid
    /// guard is what prevents that.
    #[tokio::test]
    async fn resume_does_not_treat_the_locks_own_confirmed_record_as_a_conflict() {
        let fixture = ConflictFixture::new().await;
        fixture.track(AssetLockStatus::Broadcast, None).await;

        fixture
            .file_record(record_for(fixture.transaction.clone(), confirmed_at(1_234)))
            .await;

        let outcome = fixture
            .manager
            .resume_asset_lock(&fixture.out_point, Some(Duration::from_millis(10)))
            .await;
        assert!(
            !matches!(
                outcome,
                Err(PlatformWalletError::AssetLockInputConflict { .. })
            ),
            "a lock's own record must never condemn it, got {outcome:?}"
        );
    }

    /// Settled locks are decided by evidence the screen has no standing to
    /// overturn: a `Consumed` tombstone records a completed Platform spend,
    /// and a proof-carrying lock holds finality the network already granted.
    /// Both must return exactly what they returned before the screen
    /// existed, even with a confirmed conflicting record sitting in history
    /// — and neither may broadcast.
    #[tokio::test]
    async fn settled_locks_keep_their_outcome_despite_a_confirmed_conflicting_record() {
        let fixture = ConflictFixture::new().await;
        fixture
            .file_record(record_for(
                transaction_spending(fixture.funded_input()),
                confirmed_at(1_234),
            ))
            .await;

        let chain_proof = dpp::prelude::AssetLockProof::Chain(
            dpp::identity::state_transition::asset_lock_proof::chain::ChainAssetLockProof {
                core_chain_locked_height: 1_234,
                out_point: fixture.out_point,
            },
        );
        fixture
            .track(AssetLockStatus::ChainLocked, Some(chain_proof.clone()))
            .await;
        let (resumed_proof, _path) = fixture
            .manager
            .resume_asset_lock(&fixture.out_point, Some(Duration::from_millis(10)))
            .await
            .expect("a chain-locked lock resumes from its own proof");
        assert_eq!(resumed_proof, chain_proof);

        fixture.track(AssetLockStatus::Consumed, None).await;
        let consumed = fixture
            .manager
            .resume_asset_lock(&fixture.out_point, Some(Duration::from_millis(10)))
            .await
            .expect_err("a consumed lock must stay terminal");
        assert!(
            matches!(
                consumed,
                PlatformWalletError::AssetLockAlreadyConsumed(actual) if actual == fixture.out_point
            ),
            "expected AssetLockAlreadyConsumed, got {consumed:?}"
        );
        assert_eq!(
            fixture.broadcast_count(),
            0,
            "settled locks never re-enter the broadcast path"
        );
    }
}
