//! Orchestrated batch seeding of the shielded pool's anonymity set.
//!
//! Dash Platform's shielded pool enforces a 250-note anonymity-set
//! minimum on every *outgoing* shielded transition (transfer / unshield
//! / withdrawal / identity-create-from-pool): `validate_minimum_pool_notes`
//! in rs-drive-abci rejects them with "pool has N notes but minimum 250
//! required". After a devnet reset *without* the `DRIVE_SHIELDED_SNAPSHOT`
//! genesis ingest, the pool starts empty and the whole shielded feature
//! set is unusable until enough notes exist.
//!
//! This module seeds the pool from the example app in one user action:
//! it submits a series of `ShieldFromAssetLock` (Type 18) transitions,
//! each carrying one real note to the wallet's own default shielded
//! address plus up to 5 **zero-value anonymity-set filler** outputs
//! (`dummy_outputs`). Each batch publishes up to 6 Orchard actions — NOT
//! the 16-action consensus cap (`max_shielded_transition_actions`), but
//! the most that fits the 20 KiB transaction-size limit; see
//! [`MAX_ACTIONS_PER_BATCH`]. Inbound transitions (Type 15 Shield, Type
//! 18 ShieldFromAssetLock) are NOT subject to the 250-note minimum, so
//! seeding runs against an empty pool.
//!
//! ## Why this is a devnet/testnet-only utility
//!
//! Seeding burns real asset-lock value (one L1 lock + the per-action
//! shielded fee per batch) purely to inflate the note count. It is a
//! development/testing convenience; the mainnet pool is seeded at genesis
//! via `DRIVE_SHIELDED_SNAPSHOT`. [`shielded_seed_pool_notes`] therefore
//! hard-errors on `Network::Mainnet`.

use dash_sdk::platform::transition::put_settings::PutSettings;
use dash_sdk::platform::types::shielded::fetch_shielded_notes_count;
use dpp::address_funds::OrchardAddress;
use dpp::balances::credits::CREDITS_PER_DUFF;

use crate::wallet::asset_lock::orchestration::{AssetLockFunding, CL_FALLBACK_TIMEOUT};
use crate::wallet::shielded::fund_from_asset_lock::shield_from_asset_lock_num_actions;
use crate::wallet::shielded::CachedOrchardProver;
use crate::wallet::PlatformWallet;
use crate::PlatformWalletError;

/// Maximum Orchard actions per seeding batch.
///
/// The binding constraint is NOT the 16-action consensus cap
/// (`max_shielded_transition_actions`) but the 20 KiB transaction-size
/// limit (`system_limits.max_state_transition_size`, mirrored by
/// tenderdash's `mempool.max-tx-bytes = 20480`): the Halo 2 proof grows
/// with the action count, ~2,681 bytes per action on the wire. Measured
/// serialized `ShieldFromAssetLock` sizes (signing_tests size probe):
/// 2 actions → 8,294 B, 6 → 19,018 B, 7 → 21,699 B (rejected by
/// tenderdash as "Tx too large"). 6 actions is the largest batch that
/// fits, with ~1.4 KiB headroom for asset-lock-proof variants. Pinned by
/// `seed_pool_batch_fits_max_state_transition_size` in dpp's
/// shield_from_asset_lock signing tests.
const MAX_ACTIONS_PER_BATCH: usize = 6;

/// Default total pool-note target. Matches the consensus 250-note
/// anonymity-set minimum the seeding exists to satisfy.
pub const DEFAULT_SEED_POOL_TARGET_NOTES: u64 = 250;

/// Value (in credits) of the single *real* note each batch shields to the
/// wallet's own default address. Kept small — the point of seeding is the
/// note count, not the balance — but non-zero so the real output is a
/// spendable, recoverable note for the seeding wallet (the fillers are
/// zero-value and unrecoverable).
const REAL_NOTE_VALUE_CREDITS: u64 = 1_000_000;

/// Attempts per batch before a `FinalityTimeout` is treated as fatal.
/// Each retry pauses [`SEED_BATCH_RETRY_PAUSE`] first, long enough for a
/// core block to confirm the chained asset-lock change outputs that
/// caused the IS/CL proofs to stall (observed around ~25-30 rapid
/// back-to-back batches, core's unconfirmed-ancestor depth limit).
const SEED_BATCH_FINALITY_RETRIES: u32 = 3;

/// Pause before retrying a batch whose finality proof timed out.
const SEED_BATCH_RETRY_PAUSE: std::time::Duration = std::time::Duration::from_secs(60);

/// Progress update emitted once per batch (before and after submission).
#[derive(Debug, Clone, Copy)]
pub struct SeedPoolProgress {
    /// 0-based index of the batch about to run / just completed.
    pub batch_index: u64,
    /// Estimated total number of batches needed to reach `target` from
    /// the count observed when the operation started. An estimate only —
    /// concurrent activity on the pool can shift the real count.
    pub batches_total_estimate: u64,
    /// Pool note count observed at this checkpoint.
    pub pool_notes_now: u64,
    /// The target total note count the operation is driving toward.
    pub target: u64,
}

/// Terminal outcome of a seeding run.
#[derive(Debug, Clone, Copy)]
pub struct SeedPoolOutcome {
    /// Pool note count observed after the final batch (and the
    /// already-satisfied early-return case).
    pub final_pool_notes: u64,
    /// Number of `ShieldFromAssetLock` batches actually submitted.
    pub batches_submitted: u64,
    /// The target the run was driving toward.
    pub target: u64,
}

impl PlatformWallet {
    /// Seed the shielded pool up to `target_total_notes` by submitting a
    /// series of `ShieldFromAssetLock` (Type 18) batches, each adding up
    /// to [`MAX_ACTIONS_PER_BATCH`] notes (1 real + up to 5 zero-value
    /// anonymity-set fillers).
    ///
    /// Devnet/testnet-only: hard-errors on `Network::Mainnet` (the mainnet
    /// pool is seeded at genesis via `DRIVE_SHIELDED_SNAPSHOT`).
    ///
    /// # Arguments
    ///
    /// * `wallet_id` — the 32-byte id of the wallet that funds the seeding
    ///   and owns the real notes (and whose default shielded address at
    ///   `account` receives them).
    /// * `account` — BIP44 account index whose default Orchard address
    ///   receives each batch's real note. Must be bound (`bind_shielded`).
    /// * `target_total_notes` — drive the on-chain pool note count up to
    ///   (at least) this value. If the pool already has at least this
    ///   many notes, the run is a no-op that returns the current count.
    /// * `funding_account_index` — BIP44 Core account whose UTXOs fund
    ///   each per-batch asset lock.
    /// * `asset_lock_signer` — external signer for each batch's asset-lock
    ///   proof signature (the raw key never crosses the FFI boundary).
    /// * `progress` — invoked before each batch (with the count observed
    ///   so far) and after each batch's proven execution. Lets the host
    ///   render a live "batch i/~n, M/target notes" counter during the
    ///   ~20–40 min run.
    /// * `settings` — optional `PutSettings` forwarded to each batch's
    ///   broadcast.
    ///
    /// Batches run **serially**: each waits for proven execution (the
    /// same `broadcast_and_wait` the single-note fund flow uses) before
    /// the next starts. A batch failure aborts the run and returns the
    /// error; notes from already-completed batches stay in the pool.
    #[cfg(feature = "shielded")]
    #[allow(clippy::too_many_arguments)]
    pub async fn shielded_seed_pool_notes<AS, F>(
        &self,
        coordinator: &std::sync::Arc<crate::wallet::shielded::NetworkShieldedCoordinator>,
        wallet_id: &[u8; 32],
        account: u32,
        target_total_notes: u64,
        funding_account_index: u32,
        asset_lock_signer: &AS,
        progress: F,
        settings: Option<PutSettings>,
    ) -> Result<SeedPoolOutcome, PlatformWalletError>
    where
        AS: ::key_wallet::signer::ExtendedPubKeySigner + Send + Sync,
        F: Fn(SeedPoolProgress) + Send + Sync,
    {
        // HARD GATE: this is a devnet/testnet seeding utility. The mainnet
        // pool is seeded at genesis (DRIVE_SHIELDED_SNAPSHOT); seeding it
        // from a client would burn real value to no purpose.
        if self.network() == key_wallet::Network::Mainnet {
            return Err(PlatformWalletError::ShieldedBuildError(
                "shielded_seed_pool_notes is a devnet/testnet utility and is disabled on mainnet \
                 (Network::Mainnet) — the mainnet shielded pool is seeded at genesis via \
                 DRIVE_SHIELDED_SNAPSHOT"
                    .to_string(),
            ));
        }

        // The wallet must own this id and have its shielded sub-wallet
        // bound, or there's no default address to send the real notes to.
        if self.wallet_id() != *wallet_id {
            return Err(PlatformWalletError::ShieldedBuildError(format!(
                "shielded_seed_pool_notes called with wallet_id {} but this wallet is {}",
                hex::encode(wallet_id),
                hex::encode(self.wallet_id())
            )));
        }
        // Snapshot the starting count so the batch-total estimate is
        // stable for the whole run.
        let sdk = self.sdk_arc();
        let start_notes = fetch_shielded_notes_count(&sdk).await.map_err(|e| {
            PlatformWalletError::ShieldedBuildError(format!(
                "failed to fetch shielded notes count before seeding: {e}"
            ))
        })?;

        let batches_total_estimate = estimate_batches(
            start_notes,
            target_total_notes,
            MAX_ACTIONS_PER_BATCH as u64,
        );

        // Already satisfied — nothing to do.
        if start_notes >= target_total_notes {
            let outcome = SeedPoolOutcome {
                final_pool_notes: start_notes,
                batches_submitted: 0,
                target: target_total_notes,
            };
            progress(SeedPoolProgress {
                batch_index: 0,
                batches_total_estimate,
                pool_notes_now: start_notes,
                target: target_total_notes,
            });
            return Ok(outcome);
        }

        // Resolved only once seeding is actually needed, so the
        // already-satisfied no-op path above works even when the
        // shielded sub-wallet isn't bound.
        let recipient = self.seed_pool_recipient(account).await?;

        // One prover handle for the whole run (zero-sized; shares the
        // process-global cached proving key).
        let prover = CachedOrchardProver::new();

        let mut pool_notes_now = start_notes;
        let mut batches_submitted = 0u64;

        while pool_notes_now < target_total_notes {
            let remaining = target_total_notes - pool_notes_now;

            // Notes this batch adds: 1 real + `dummy_outputs` fillers,
            // capped at the per-transition action limit. On the final
            // stretch, only add as many as still needed.
            let notes_this_batch =
                std::cmp::min(remaining, MAX_ACTIONS_PER_BATCH as u64).max(1) as usize;
            // `notes_this_batch` includes the real note; the rest are
            // fillers. (`max(1, ..)` above guarantees `>= 1`.)
            let dummy_outputs = notes_this_batch - 1;

            progress(SeedPoolProgress {
                batch_index: batches_submitted,
                batches_total_estimate,
                pool_notes_now,
                target: target_total_notes,
            });

            // Size the per-batch asset lock: pool_fee (priced from the
            // SAME on-wire action count consensus charges) + the real
            // note's value. `shielded_fund_from_asset_lock` re-derives
            // `shield_amount = lock_value − pool_fee` internally, so the
            // real note lands at ~REAL_NOTE_VALUE_CREDITS and the surplus
            // is structurally zero.
            let num_actions = shield_from_asset_lock_num_actions(dummy_outputs);
            let pool_fee = self.shield_from_asset_lock_pool_fee(num_actions)?;
            let lock_credits = pool_fee
                .checked_add(REAL_NOTE_VALUE_CREDITS)
                .ok_or_else(|| {
                    PlatformWalletError::ShieldedBuildError(
                        "seed batch lock amount overflowed credits".to_string(),
                    )
                })?;
            let amount_duffs = lock_credits.div_ceil(CREDITS_PER_DUFF);

            // Rapid back-to-back batches chain unconfirmed L1 change
            // outputs; around core's unconfirmed-ancestor depth limit
            // (~25 chained txs) InstantSend/ChainLock proofs stop
            // arriving until a core block confirms the chain, and the
            // funding resolution surfaces `FinalityTimeout`. That's a
            // transient pacing condition, not a failure — retry the
            // batch a few times with a pause to let a block land
            // instead of aborting the whole run.
            //
            // The timed-out lock is already broadcast and tracked, so the
            // retry MUST resume it (`FromExistingAssetLock` with the
            // outpoint from the error) rather than build a fresh lock:
            // re-funding from wallet balance would strand the original
            // lock, burn another UTXO per attempt, and chain the new lock
            // on top of the very unconfirmed-ancestor depth that caused
            // the stall.
            let mut attempt = 0u32;
            let mut funding = AssetLockFunding::FromWalletBalance {
                amount_duffs,
                account_index: funding_account_index,
            };
            loop {
                attempt += 1;
                match self
                    .shielded_fund_from_asset_lock(
                        coordinator,
                        funding,
                        vec![(recipient, None)],
                        asset_lock_signer,
                        &prover,
                        None,
                        dummy_outputs,
                        settings,
                        // Bounded: a ChainLock timeout here is the deliberate
                        // unconfirmed-ancestor pacing signal this loop retries on.
                        Some(CL_FALLBACK_TIMEOUT),
                        // Pool seeding stays within the single privacy domain by
                        // default (dashpay/platform#4184); no cross-domain consent.
                        crate::CrossDomainConsent::Denied,
                    )
                    .await
                {
                    Ok(()) => break,
                    Err(PlatformWalletError::FinalityTimeout(out_point))
                        if attempt < SEED_BATCH_FINALITY_RETRIES =>
                    {
                        tracing::warn!(
                            batch = batches_submitted,
                            attempt,
                            %out_point,
                            "seed batch finality timed out; pausing for a core block then \
                             resuming the tracked lock"
                        );
                        funding = AssetLockFunding::FromExistingAssetLock {
                            out_point,
                            consume_invitation_voucher: false,
                        };
                        tokio::time::sleep(SEED_BATCH_RETRY_PAUSE).await;
                    }
                    Err(e) => return Err(e),
                }
            }

            batches_submitted += 1;

            // Re-poll the on-chain count: this is the authoritative loop
            // condition (the proven batch's notes are now committed) and
            // the number the host shows. A poll failure here is fatal —
            // we can't tell whether to keep going.
            pool_notes_now = fetch_shielded_notes_count(&sdk).await.map_err(|e| {
                PlatformWalletError::ShieldedBuildError(format!(
                    "failed to fetch shielded notes count after seed batch {batches_submitted}: {e}"
                ))
            })?;

            progress(SeedPoolProgress {
                batch_index: batches_submitted,
                batches_total_estimate,
                pool_notes_now,
                target: target_total_notes,
            });

            tracing::info!(
                batch = batches_submitted,
                notes_this_batch,
                dummy_outputs,
                pool_notes_now,
                target = target_total_notes,
                "shielded seed-pool batch committed"
            );
        }

        Ok(SeedPoolOutcome {
            final_pool_notes: pool_notes_now,
            batches_submitted,
            target: target_total_notes,
        })
    }

    /// The wallet's own default Orchard address for `account`, as an
    /// `OrchardAddress` ready for the bundle builder. Errors if the
    /// shielded sub-wallet isn't bound for `account`.
    #[cfg(feature = "shielded")]
    async fn seed_pool_recipient(
        &self,
        account: u32,
    ) -> Result<OrchardAddress, PlatformWalletError> {
        let raw = self
            .shielded_default_address(account)
            .await
            .ok_or_else(|| {
                PlatformWalletError::ShieldedBuildError(format!(
                "shielded sub-wallet is not bound for account {account}; call bind_shielded first"
            ))
            })?;
        OrchardAddress::from_raw_bytes(&raw).map_err(|e| {
            PlatformWalletError::ShieldedBuildError(format!(
                "failed to convert default shielded address to OrchardAddress: {e:?}"
            ))
        })
    }
}

/// Estimate the number of [`MAX_ACTIONS_PER_BATCH`]-note batches needed to
/// move the pool count from `current` to `target`, given `per_batch` notes
/// per batch. Returns `0` when already at/above target. Pure arithmetic so
/// it can be unit-tested without a wallet/SDK.
fn estimate_batches(current: u64, target: u64, per_batch: u64) -> u64 {
    let per_batch = per_batch.max(1);
    target.saturating_sub(current).div_ceil(per_batch)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn estimate_batches_rounds_up_and_floors_at_zero() {
        // 16 notes/batch: 250 from empty -> ceil(250/16) = 16 batches.
        assert_eq!(estimate_batches(0, 250, 16), 16);
        // Exactly divisible.
        assert_eq!(estimate_batches(0, 32, 16), 2);
        // One past a boundary needs an extra batch.
        assert_eq!(estimate_batches(0, 33, 16), 3);
        // Partway there.
        assert_eq!(estimate_batches(100, 250, 16), 10); // ceil(150/16)
                                                        // Already satisfied.
        assert_eq!(estimate_batches(250, 250, 16), 0);
        assert_eq!(estimate_batches(300, 250, 16), 0);
        // Degenerate per_batch is clamped to 1 (no div-by-zero).
        assert_eq!(estimate_batches(0, 5, 0), 5);
    }

    #[test]
    fn batch_size_math_matches_action_count() {
        // The loop computes notes_this_batch =
        // min(remaining, MAX_ACTIONS_PER_BATCH).max(1), dummy_outputs =
        // notes_this_batch - 1, and the on-wire action count must equal
        // max(1 + dummy_outputs, 2). Verify the coupling across the
        // boundary cases the loop hits (MAX_ACTIONS_PER_BATCH == 6).
        for (remaining, expected_notes, expected_actions) in [
            (250u64, 6usize, 6usize), // full batch
            (6, 6, 6),                // exactly a full batch
            (4, 4, 4),                // partial batch, > 2 actions
            (2, 2, 2),                // 1 real + 1 filler -> 2 actions
            (1, 1, 2),                // 1 real, 0 fillers -> Orchard MIN_ACTIONS=2
        ] {
            let notes_this_batch =
                std::cmp::min(remaining, MAX_ACTIONS_PER_BATCH as u64).max(1) as usize;
            assert_eq!(notes_this_batch, expected_notes, "remaining={remaining}");
            let dummy_outputs = notes_this_batch - 1;
            assert_eq!(
                shield_from_asset_lock_num_actions(dummy_outputs),
                expected_actions,
                "remaining={remaining}"
            );
        }
    }
}
