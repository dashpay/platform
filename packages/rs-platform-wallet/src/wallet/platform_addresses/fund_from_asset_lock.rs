//! Orchestrated platform-address funding from a Core asset lock.
//!
//! Mirrors `IdentityWallet::register_identity_with_funding` from the
//! identity-side flow but credits Platform addresses with the asset
//! lock's value via an `AddressFundingFromAssetLockTransition` instead
//! of creating an identity.
//!
//! ## Pipeline
//!
//! 1. **Pre-flight** — exactly-one-`None`-recipient invariant; each
//!    address must belong to the supplied platform-payment account.
//! 2. **Resolve funding** — delegate to the shared
//!    [`AssetLockManager::resolve_funding_with_is_timeout_fallback`].
//!    `FromWalletBalance` builds an asset-lock tx out of the
//!    `AssetLockAddressTopUp` BIP44 family and waits for IS/CL;
//!    `FromExistingAssetLock` resumes from a tracked outpoint.
//! 3. **Submit** — `addresses.top_up_with_signers(...)` inside the
//!    shared `submit_with_cl_height_retry` wrapper. IS→CL fallback
//!    fires both on Core-side timeout (resolver returns `IsTimeout`)
//!    and on Platform-side IS rejection
//!    (`is_instant_lock_proof_invalid`).
//! 4. **Bookkeeping + cleanup** — write each recipient's new credit
//!    balance into `ManagedPlatformAccount` and emit a
//!    `PlatformAddressChangeSet`; then `consume_asset_lock` the
//!    tracked outpoint so the row is marked `Consumed` (terminal)
//!    and dropped from the in-memory tracked-lock map.

use crate::wallet::asset_lock::orchestration::{
    out_point_from_proof, submit_with_cl_height_retry, AssetLockFunding, FundingResolution,
    ResolvedFunding,
};
use crate::wallet::PlatformAddressWallet;
use crate::{error::is_instant_lock_proof_invalid, PlatformAddressChangeSet, PlatformWalletError};
use dash_sdk::platform::transition::put_settings::PutSettings;
use dash_sdk::platform::transition::top_up_address::TopUpAddress;
use dash_sdk::query_types::AddressInfos;
use dpp::address_funds::fee_strategy::AddressFundsFeeStrategyStep;
use dpp::address_funds::{AddressFundsFeeStrategy, PlatformAddress};
use dpp::balances::credits::CREDITS_PER_DUFF;
use dpp::fee::Credits;
use dpp::identity::signer::Signer;
use dpp::state_transition::address_funding_from_asset_lock_transition::calculate_address_funding_from_asset_lock_min_required_fee;
use dpp::version::PlatformVersion;
use key_wallet::wallet::managed_wallet_info::asset_lock_builder::AssetLockFundingType;
use key_wallet::PlatformP2PKHAddress;
use std::collections::BTreeMap;

const ADDRESS_FUNDING_FROM_ASSET_LOCK_INPUT_COUNT: usize = 0;

impl PlatformAddressWallet {
    /// Fund platform addresses from a Core L1 asset lock, with the
    /// asset-lock proof signed by an external `key_wallet::signer::Signer`.
    ///
    /// This is the orchestrated entry point: it covers the full
    /// build → broadcast → wait-for-IS-or-CL → submit-with-CL-retry →
    /// IS→CL-fallback → consume pipeline. The host never sees the
    /// asset-lock private key — both Core-side derivation (inside the
    /// asset-lock manager) and ST-side signing
    /// (`StateTransition::sign_with_core_signer`) go through
    /// `asset_lock_signer`, which atomically derives + signs +
    /// zeroises inside its trust boundary.
    ///
    /// # Arguments
    ///
    /// * `funding` — How to source the funding asset lock. `FromWalletBalance`
    ///   builds a fresh asset lock from Core UTXOs; `FromExistingAssetLock`
    ///   resumes from a tracked outpoint (after app relaunch or a stuck
    ///   broadcast).
    /// * `platform_account_index` — Platform payment account whose
    ///   addresses receive credits. Used for both the membership
    ///   pre-flight and the post-success balance write.
    /// * `addresses` — Map from recipient `PlatformAddress` to optional
    ///   amount in credits. Exactly one entry must be `None` — the
    ///   remainder-after-fees-and-explicit-outputs recipient (the lock
    ///   is consumed in full, so a remainder bucket is mandatory).
    /// * `_fee_strategy` — **IGNORED.** Retained so the pre-derivation
    ///   signature still compiles for out-of-tree callers; pass
    ///   whatever you passed before (`vec![]` is fine). The strategy
    ///   actually used is derived from `addresses` by
    ///   [`remainder_fee_strategy`], because the only address that can
    ///   legitimately absorb the fee is the remainder output and its
    ///   consensus index is a property of that map's ordering, not of
    ///   any caller's list order. A binding that computes the index
    ///   from its own list order silently mis-targets the fee whenever
    ///   the remainder is not also first lexicographically, so the
    ///   index is not the caller's to supply. This mirrors the C
    ///   ABI, where `fee_strategy` / `fee_strategy_count` are likewise
    ///   still accepted and ignored.
    ///
    ///   [`PlatformAddressWallet::fund_from_asset_lock_external`] has no
    ///   such compatibility obligation (it is new in this release) and
    ///   therefore omits the argument rather than carrying the vestige
    ///   forward.
    /// * `address_signer` — Signs per-input `AddressWitness` for any
    ///   additional inputs from existing platform addresses (today
    ///   none — combining external inputs with an asset-lock proof is
    ///   not exercised here, but `AddressFundingFromAssetLockTransitionV0`
    ///   does allow it).
    /// * `asset_lock_signer` — Signs the outer state-transition ECDSA
    ///   signature against the asset lock's credit-output key. The
    ///   wallet struct itself carries no key material; signing is
    ///   atomic + zeroising inside this signer.
    /// * `settings` — `PutSettings::user_fee_increase` is threaded
    ///   through to the ST builder. The CL-height retry wrapper bumps
    ///   this value on consensus-10506 to bypass Tenderdash's
    ///   invalid-tx hash cache; the caller's initial value is the
    ///   starting point.
    ///
    /// # Latency budget
    ///
    /// There is intentionally **no ceiling** on this call: a broadcast
    /// asset lock is committed on-chain, and its ChainLock is
    /// deterministic finality that will eventually arrive, so the flow
    /// waits for finality however long it takes rather than reporting a
    /// spurious failure. The components are:
    /// - 300s IS-wait inside the resolver's
    ///   `create_funded_asset_lock_proof` (`AssetLockManager`'s
    ///   InstantSend-preference window before falling back to ChainLock).
    /// - **Unbounded** ChainLock fallback
    ///   (`upgrade_to_chain_lock_proof(None)`) — waits for the ChainLock
    ///   indefinitely (testnet ChainLocks can take ~15min).
    /// - 210s CL-height retry budget (`CL_HEIGHT_RETRY_BUDGET`) per
    ///   `submit_with_cl_height_retry` wrapper.
    /// - Up to two passes through the submit wrapper on the
    ///   IS-rejection path: one for the IS proof, one for the
    ///   upgraded CL proof.
    ///
    /// Happy-path wall time on a healthy testnet is single-digit
    /// seconds (IS-lock typically arrives within 3s of broadcast,
    /// CL-height retry never fires). The unbounded wait only bites when
    /// InstantSend never propagates and the caller must wait out the
    /// slower ChainLock. Callers run this off the main thread and never
    /// cancel it (see the Cancellation note below).
    ///
    /// # Cancellation
    ///
    /// This function is NOT cancellation-safe. The two underlying
    /// retry loops (`submit_with_cl_height_retry` and the
    /// resolver's internal `wait_for_proof`) use
    /// `tokio::time::sleep` / `tokio::sync::Notify` without
    /// structured cancellation hooks. If the caller drops the
    /// returned future:
    /// - Any bumped `user_fee_increase` is lost; the next attempt
    ///   starts from the caller-supplied value, which may hit
    ///   Tenderdash's invalid-tx cache for the bumped variants.
    /// - In-flight submitted state transitions remain in
    ///   Tenderdash's mempool until they commit or expire.
    /// - The tracked asset lock stays at its last-observed status
    ///   (`Broadcast` / `InstantSendLocked` / `ChainLocked`) until
    ///   either `consume_asset_lock` completes or the next resume
    ///   advances it.
    ///
    /// The Swift `AddressFundFromAssetLockController.task` field deliberately
    /// does not call `.cancel()` to avoid these partial-state
    /// outcomes — the FFI call always runs to completion. UI
    /// dismissal hides the progress view without aborting the
    /// work; resume picks the lock back up via
    /// `FromExistingAssetLock`.
    #[allow(clippy::too_many_arguments)]
    pub async fn fund_from_asset_lock<S, AS>(
        &self,
        funding: AssetLockFunding,
        platform_account_index: u32,
        addresses: BTreeMap<PlatformAddress, Option<Credits>>,
        _fee_strategy: AddressFundsFeeStrategy,
        address_signer: &S,
        asset_lock_signer: &AS,
        settings: Option<PutSettings>,
    ) -> Result<PlatformAddressChangeSet, PlatformWalletError>
    where
        S: Signer<PlatformAddress> + Send + Sync,
        AS: ::key_wallet::signer::ExtendedPubKeySigner + Send + Sync,
    {
        self.fund_from_asset_lock_inner(
            funding,
            platform_account_index,
            addresses,
            address_signer,
            asset_lock_signer,
            settings,
            RecipientOwnership::AllOwned,
        )
        .await
    }

    /// Fund a THIRD PARTY's platform address from a Core L1 asset
    /// lock, with the sender's own address absorbing the remainder
    /// (change) and the fee.
    ///
    /// Sibling to [`PlatformAddressWallet::fund_from_asset_lock`]:
    /// identical pipeline, identical bookkeeping — the ONLY behavioural
    /// delta is the recipient pre-flight. The one signature difference
    /// is that this entry point omits the vestigial `_fee_strategy`
    /// argument: it is new here, so it has no source-compatibility debt
    /// to carry (see that method's `_fee_strategy` note).
    ///
    /// | | `fund_from_asset_lock` | `fund_from_asset_lock_external` |
    /// |---|---|---|
    /// | explicit-amount outputs (`Some(credits)`) | must belong to `platform_account_index` | **any valid P2PKH address** |
    /// | remainder output (`None`) | must belong to `platform_account_index` | must belong to `platform_account_index` |
    /// | address type | P2PKH only | P2PKH only |
    ///
    /// Why this is a separate entry point rather than a relaxation of
    /// the existing one: for every current caller, a typo'd recipient
    /// address is caught today by the membership check. Relaxing that
    /// check in place would silently convert "typo'd address → typed
    /// error before anything is broadcast" into "typo'd address →
    /// asset-lock credits irrecoverably delivered to a stranger". Opting
    /// in by function name keeps that failure mode confined to callers
    /// that actually mean to pay someone else.
    ///
    /// Why the remainder must still be owned: the asset lock is
    /// consumed in full, so the `None` bucket receives everything left
    /// after the explicit outputs and fees — i.e. the change. Sending
    /// change to a stranger is never the intent, and a caller bug that
    /// mixed up which entry was the remainder would leak the entire
    /// lock value rather than the intended payment. Consensus does not
    /// care (it never validates output ownership — see
    /// `test_external_recipient_asset_lock_funding_is_consensus_valid`
    /// in rs-drive-abci), so this is purely a wallet-level safety rail.
    ///
    /// Reconciliation needs no special casing: the third party's output
    /// simply does not resolve to a wallet-owned slot, which
    /// `reconcile_address_infos_with_persistence` already treats as a
    /// normal outcome (it warns and still reports `persisted = true`),
    /// while the sender's remainder output resolves and is credited as
    /// usual.
    ///
    /// See [`PlatformAddressWallet::fund_from_asset_lock`] for the
    /// argument-by-argument reference, the latency budget and the
    /// cancellation contract — all of which apply verbatim.
    #[allow(clippy::too_many_arguments)]
    pub async fn fund_from_asset_lock_external<S, AS>(
        &self,
        funding: AssetLockFunding,
        platform_account_index: u32,
        addresses: BTreeMap<PlatformAddress, Option<Credits>>,
        address_signer: &S,
        asset_lock_signer: &AS,
        settings: Option<PutSettings>,
    ) -> Result<PlatformAddressChangeSet, PlatformWalletError>
    where
        S: Signer<PlatformAddress> + Send + Sync,
        AS: ::key_wallet::signer::ExtendedPubKeySigner + Send + Sync,
    {
        self.fund_from_asset_lock_inner(
            funding,
            platform_account_index,
            addresses,
            address_signer,
            asset_lock_signer,
            settings,
            RecipientOwnership::ExternalExplicitOutputs,
        )
        .await
    }

    /// Shared body behind [`PlatformAddressWallet::fund_from_asset_lock`]
    /// and [`PlatformAddressWallet::fund_from_asset_lock_external`].
    ///
    /// `ownership` is the single point of divergence between the two
    /// public entry points; everything from Step 2 onward is identical,
    /// because nothing downstream of the pre-flight is
    /// ownership-sensitive (the proof carries every output regardless of
    /// who owns it, and reconciliation resolves whichever subset happens
    /// to be ours).
    ///
    /// There is deliberately no separate `resume_*` variant at this
    /// layer: resuming is expressed as
    /// `AssetLockFunding::FromExistingAssetLock`, a value of the
    /// `funding` parameter, so both public entry points already cover
    /// fresh-build and resume. (The FFI layer does expose distinct
    /// resume symbols, because a C ABI cannot take a Rust enum.)
    #[allow(clippy::too_many_arguments)]
    async fn fund_from_asset_lock_inner<S, AS>(
        &self,
        funding: AssetLockFunding,
        platform_account_index: u32,
        addresses: BTreeMap<PlatformAddress, Option<Credits>>,
        address_signer: &S,
        asset_lock_signer: &AS,
        settings: Option<PutSettings>,
        ownership: RecipientOwnership,
    ) -> Result<PlatformAddressChangeSet, PlatformWalletError>
    where
        S: Signer<PlatformAddress> + Send + Sync,
        AS: ::key_wallet::signer::ExtendedPubKeySigner + Send + Sync,
    {
        // Step 1: pre-flight. Failing fast here avoids broadcasting
        // an unfundable asset-lock tx.
        validate_recipient_addresses(self, platform_account_index, &addresses, ownership).await?;

        // Step 1b: derive the fee strategy from the recipient map. This
        // is deliberately NOT a caller-supplied value — see
        // `remainder_fee_strategy` for why a positional index cannot be
        // computed correctly outside this layer.
        let fee_strategy = remainder_fee_strategy(&addresses)?;

        // Step 1c: a fresh L1 asset lock is single-use and
        // non-refundable. Reject or floor it before build/broadcast if
        // it cannot satisfy DPP's static admission fee for this
        // address-funding transition shape.
        let funding = enforce_asset_lock_amount_covers_address_funding_floor(
            funding,
            addresses.len(),
            self.sdk.version(),
        )?;

        // Step 2: resolve funding. `AssetLockAddressTopUp` selects the
        // BIP44 funding family for the Core asset-lock tx. The
        // `destination_index = 0` argument is unused by this funding
        // type (the resolver only consults it for `IdentityTopUp`),
        // so any value is fine.
        let ResolvedFunding {
            proof,
            path,
            tracked_out_point,
        } = match self
            .asset_locks
            .resolve_funding_with_is_timeout_fallback(
                funding,
                AssetLockFundingType::AssetLockAddressTopUp,
                /* destination_index */ 0,
                asset_lock_signer,
            )
            .await?
        {
            FundingResolution::Resolved(rf) => rf,
            FundingResolution::IsTimeout { out_point } => {
                tracing::warn!(
                    "IS-lock did not propagate within 300s for funded platform-address top-up \
                     (tx {}), falling back to ChainLock proof",
                    out_point.txid
                );
                let chain_proof = self
                    .asset_locks
                    .upgrade_to_chain_lock_proof(&out_point, None)
                    .await?;
                // Re-derive the credit-output path. The lock is now
                // CL-attached; `resume_asset_lock` short-circuits to
                // the existing-proof branch and just hands the path
                // back.
                let (_, path) = self.asset_locks.resume_asset_lock(&out_point, None).await?;
                ResolvedFunding {
                    proof: chain_proof,
                    path,
                    tracked_out_point: Some(out_point),
                }
            }
        };

        // Step 3: submit. Two Platform-side fallback layers (matches
        // `register_identity_with_funding`): CL-height-too-low retries
        // bump `user_fee_increase` to bypass Tenderdash's invalid-tx
        // cache, and IS-lock rejection triggers an IS→CL upgrade on
        // the same outpoint.
        let proof_out_point = out_point_from_proof(&proof);
        // `proof_height` is the broadcast proof's committed block — the
        // height pin for the reconciled absolutes below.
        let (submit_result, effective_proof) = match submit_with_cl_height_retry(settings, |s| {
            addresses.top_up_with_signers(
                &self.sdk,
                proof.clone(),
                &path,
                fee_strategy.clone(),
                address_signer,
                asset_lock_signer,
                s,
            )
        })
        .await
        {
            Ok(infos) => (Ok(infos), proof.clone()),
            Err(e) if is_instant_lock_proof_invalid(&e) => {
                let out_point = proof_out_point;
                tracing::warn!(
                    "IS-lock proof rejected by Platform for platform-address top-up (tx {}), \
                     retrying with ChainLock proof",
                    out_point.txid
                );
                let chain_proof = self
                    .asset_locks
                    .upgrade_to_chain_lock_proof(&out_point, None)
                    .await?;
                // Advance the tracked status from `InstantSendLocked`
                // to `ChainLocked` with the upgraded proof attached
                // BEFORE the second submit. If the next call fails
                // (transport blip, fresh CL-height race that
                // exhausts the retry budget), the row accurately
                // reflects the lock's CL-attached state instead of
                // the stale IS proof Platform just rejected. The
                // catch-up scanner / Resume path then has a
                // truthful status to work from.
                let cs = self
                    .asset_locks
                    .advance_asset_lock_status(
                        &out_point,
                        crate::wallet::asset_lock::tracked::AssetLockStatus::ChainLocked,
                        Some(chain_proof.clone()),
                    )
                    .await?;
                self.asset_locks.queue_asset_lock_changeset(cs);
                let submit_result = submit_with_cl_height_retry(settings, |s| {
                    addresses.top_up_with_signers(
                        &self.sdk,
                        chain_proof.clone(),
                        &path,
                        fee_strategy.clone(),
                        address_signer,
                        asset_lock_signer,
                        s,
                    )
                })
                .await;
                (submit_result, chain_proof)
            }
            Err(e) => (Err(e), proof.clone()),
        };
        let (address_infos, proof_height) = self
            .asset_locks
            .reconcile_asset_lock_submit_result(
                submit_result,
                &proof_out_point,
                &effective_proof,
                None,
            )
            .await?;

        // Step 4: bookkeeping + cleanup. Write the proof-attested
        // balances back into ManagedPlatformAccount, then consume the
        // tracked asset lock (terminal — marks the row `Consumed` and
        // drops it from the in-memory map).

        // Post-condition: every requested recipient must appear in
        // the proof-attested `address_infos` with a `Some(_)` info.
        // A wholly-empty map, an absent recipient, or a
        // `Some(addr) -> None` entry would each be a DAPI /
        // proof-verifier contract violation, NOT a successful
        // zero-credit funding. We fail loud rather than silently
        // consume the asset lock with no recorded credits for some
        // or all recipients.
        validate_address_infos_complete(&addresses, &address_infos)?;

        // The shared seam applies the proof-attested balances to the
        // managed accounts, updates the provider's sync seed, and
        // persists — without the persist, rows for these recipients
        // stay frozen at pre-top-up values until the next BLAST sync;
        // on a process restart before that sync,
        // `initialize_from_persisted` would seed
        // `account.address_credit_balance` from the stale rows while
        // the asset-lock record is already `Consumed`, leaving
        // `auto_select_inputs` to under-budget and produce
        // protocol-level rejections until a sync repairs them.
        //
        // The seam persists before returning, and the persist MUST
        // happen before `consume_asset_lock` so we never have a
        // Consumed lock paired with a stale balance row on disk.
        // Persistence errors are logged inside the seam rather than
        // propagated: Platform already accepted the transition, and a
        // persistence hiccup shouldn't mask that.
        //
        // ADDR-09: every recipient of an asset-lock top-up is credited via
        // an on-chain `AddBalanceToAddress` DELTA at exactly this proof's
        // block height. The committed absolutes carry `proof_height` as
        // their height pin (`AddressFunds::as_of_height`), so the sync's
        // apply loops drop that delta (and any older one) instead of
        // re-applying it on top → no `X + X = 2X` double-count, on
        // incremental AND full-scan passes alike.
        //
        // Use the persistence-reporting variant: marking the lock
        // `Consumed` below is irreversible, so it MUST be gated on the
        // reconciled balances actually reaching disk. `persisted` is
        // false ONLY when the in-memory balances were updated but the
        // durable write failed — exactly the case where a Consumed lock
        // would pair with stale rows and under-budget the next spend
        // after a restart.
        let (cs, persisted) = self
            .reconcile_address_infos_with_persistence(
                &address_infos,
                proof_height,
                "fund from asset lock",
            )
            .await;

        if let Some(out_point) = tracked_out_point {
            if !persisted {
                // The proof-attested balances were applied in memory but
                // did not reach disk. Leave the lock non-Consumed: it
                // stays in the Resumable Funding list, and a user Resume
                // gets Platform's deterministic "lock already consumed"
                // rejection — the same benign recovery path as a failed
                // consume below — while the next platform-address sync
                // repairs the stale rows. Consuming here would strand the
                // lock as Consumed over durable balances that under-report
                // the credit.
                tracing::error!(
                    outpoint = %out_point,
                    "skipping consume_asset_lock: the reconciled balance changeset \
                     was not durably stored; the lock stays non-Consumed (Resumable) \
                     rather than pairing a Consumed lock with stale balance rows on disk"
                );
                return Ok(cs);
            }
            // Platform DID accept the top-up — propagating an Err
            // here would misreport the protocol outcome, since the
            // caller's recipient(s) already have credits attested
            // by the proof we just decoded. But: the lock row stays
            // in non-Consumed status, which means it will surface
            // in the Resumable Funding list and the user could try
            // to fund it again — Platform would deterministically
            // reject the duplicate ST with "lock already consumed".
            //
            // The expected failure mode is `WalletNotFound` (the
            // wallet handle vanished between submit-success and
            // this cleanup). Log that as a warn — the user-visible
            // recovery path (Resume + Platform's deterministic
            // rejection) is benign. Anything else is an unexpected
            // invariant violation — log as `error` so it shows up
            // in operational dashboards.
            if let Err(e) = self.asset_locks.consume_asset_lock(&out_point).await {
                match &e {
                    PlatformWalletError::WalletNotFound(_) => {
                        tracing::warn!(
                            outpoint = %out_point,
                            error = %e,
                            "consume_asset_lock: wallet handle vanished after successful Platform submit"
                        );
                    }
                    _ => {
                        tracing::error!(
                            outpoint = %out_point,
                            error = %e,
                            "consume_asset_lock failed unexpectedly after successful Platform submit; \
                             the lock row stays non-Consumed and will surface as Resumable. \
                             A user Resume on it will be rejected by Platform with 'lock already consumed'."
                        );
                    }
                }
            }
        }

        Ok(cs)
    }
}

/// Which recipient outputs must belong to the sender's own managed
/// platform account.
///
/// This is the sole behavioural difference between
/// [`PlatformAddressWallet::fund_from_asset_lock`] and
/// [`PlatformAddressWallet::fund_from_asset_lock_external`]. Consensus
/// itself never validates output ownership, so both modes produce
/// equally valid state transitions — the distinction exists purely to
/// keep an accidental third-party payment from being indistinguishable
/// from a deliberate one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum RecipientOwnership {
    /// Every recipient — explicit-amount outputs and the remainder
    /// alike — must be a member of the platform-payment account. This
    /// is the legacy "fund my own addresses" behaviour.
    AllOwned,
    /// Explicit-amount (`Some(credits)`) outputs may be any valid P2PKH
    /// address, including addresses this wallet knows nothing about.
    /// The single remainder (`None`) output must still be owned,
    /// because it is the change.
    ExternalExplicitOutputs,
}

/// Derive the fee strategy for an `AddressFundingFromAssetLock`
/// transition from the recipient map itself.
///
/// ## Why this cannot be a caller-supplied value
///
/// `AddressFundsFeeStrategyStep::ReduceOutput(i)` is POSITIONAL, and
/// consensus resolves `i` against the transition's `outputs`
/// `BTreeMap<PlatformAddress, Option<Credits>>` — i.e. against
/// `PlatformAddress`'s derived `Ord` (P2PKH before P2SH, then the
/// 20-byte hash ascending). See
/// `deduct_fee_from_outputs_or_remaining_balance_of_inputs_v0`, which
/// snapshots `outputs.keys()` before mutating, and
/// `AddressFundingFromAssetLockTransitionActionV0::resolved_outputs`,
/// which preserves those keys verbatim.
///
/// A caller holding a flat array therefore cannot name the fee-paying
/// output without reproducing `PlatformAddress`'s `Ord` exactly. Every
/// binding that tried to do so from array position (the Swift SDK, the
/// JNI `decode_funding_recipients`) got it wrong whenever the remainder
/// was not also first lexicographically — silently charging the fee to
/// an explicit-amount payee instead of the sender's change. With a
/// third-party payee in the set that is a real misallocation, not a
/// wash. Deriving it here, from the same map that becomes the outputs
/// map, is the only place the answer is knowable without duplicating a
/// consensus rule.
///
/// ## Why the remainder is always the right target
///
/// This flow never builds address inputs (`top_up_with_signers` passes
/// `BTreeMap::new()` for them), so `DeductFromInput(_)` has nothing to
/// resolve against and would leave the fee uncovered. Among the
/// outputs, the explicit-amount entries are exact amounts the caller
/// asked to deliver; only the `None` entry is a residual bucket
/// (`total_available - Σ explicit`), so it is the one output that can
/// absorb a fee without shortchanging a payee.
///
/// The `None` entry is unique — [`validate_recipient_shape`] rejects
/// any other cardinality — so the returned strategy is a single step.
fn remainder_fee_strategy(
    addresses: &BTreeMap<PlatformAddress, Option<Credits>>,
) -> Result<AddressFundsFeeStrategy, PlatformWalletError> {
    let index = addresses
        .values()
        .position(|amount| amount.is_none())
        .ok_or_else(|| {
            // Unreachable in the orchestrated flow: `validate_recipient_shape`
            // runs first and requires exactly one `None`. Kept as a typed
            // error rather than an `expect` so a future caller that skips
            // the pre-flight gets a diagnosable failure instead of a panic
            // across the FFI boundary.
            PlatformWalletError::AddressOperation(
                "fund_from_asset_lock requires exactly one remainder (None-amount) recipient to absorb the fee, found none"
                    .to_string(),
            )
        })?;

    // `ReduceOutput` carries a u16. Guard the narrowing so a
    // pathological recipient count cannot wrap the index onto a
    // different — and possibly third-party — output.
    let index: u16 = index.try_into().map_err(|_| {
        PlatformWalletError::AddressOperation(format!(
            "Too many funding recipients: remainder index {} exceeds u16::MAX",
            index
        ))
    })?;

    Ok(vec![AddressFundsFeeStrategyStep::ReduceOutput(index)])
}

fn enforce_asset_lock_amount_covers_address_funding_floor(
    funding: AssetLockFunding,
    output_count: usize,
    platform_version: &PlatformVersion,
) -> Result<AssetLockFunding, PlatformWalletError> {
    let required = minimum_address_funding_asset_lock_duffs(output_count, platform_version);

    match funding {
        AssetLockFunding::FromWalletBalance {
            amount_duffs,
            account_index,
        } => {
            if amount_duffs < required {
                return Err(PlatformWalletError::AssetLockInsufficientFunds {
                    available: amount_duffs,
                    required,
                });
            }
            Ok(AssetLockFunding::FromWalletBalance {
                amount_duffs,
                account_index,
            })
        }
        AssetLockFunding::DrainAccountBalance {
            account,
            minimum_lock_duffs,
        } => Ok(AssetLockFunding::DrainAccountBalance {
            account,
            minimum_lock_duffs: Some(
                minimum_lock_duffs.map_or(required, |floor| floor.max(required)),
            ),
        }),
        funding @ AssetLockFunding::FromExistingAssetLock { .. } => Ok(funding),
    }
}

/// Minimum L1 asset-lock value that can pass DPP's count-based
/// `AddressFundingFromAssetLock` admission-floor fee check.
fn minimum_address_funding_asset_lock_duffs(
    output_count: usize,
    platform_version: &PlatformVersion,
) -> u64 {
    ceil_credits_to_duffs(calculate_address_funding_from_asset_lock_min_required_fee(
        ADDRESS_FUNDING_FROM_ASSET_LOCK_INPUT_COUNT,
        output_count,
        platform_version,
    ))
}

fn ceil_credits_to_duffs(credits: Credits) -> u64 {
    credits / CREDITS_PER_DUFF + u64::from(!credits.is_multiple_of(CREDITS_PER_DUFF))
}

/// Pre-flight check for the recipient address map:
/// - Non-empty
/// - Exactly one `None`-amount entry (the remainder recipient)
/// - All addresses are P2PKH
/// - Ownership per `ownership` (see [`RecipientOwnership`])
///
/// Resolves the managed account once and delegates the actual rules to
/// the pure [`validate_recipient_map`], which is where the unit tests
/// live (no wallet construction needed to exercise the rules).
async fn validate_recipient_addresses(
    wallet: &PlatformAddressWallet,
    platform_account_index: u32,
    addresses: &BTreeMap<PlatformAddress, Option<Credits>>,
    ownership: RecipientOwnership,
) -> Result<(), PlatformWalletError> {
    // Shape checks before the lock. `validate_recipient_map` re-runs
    // them (it is self-contained so the tests can call it directly);
    // running them here too is what preserves the pre-existing error
    // precedence — a malformed recipient map is rejected with its own
    // typed error even when the wallet handle or the account is gone,
    // rather than being masked by `WalletNotFound` / `AddressSync`.
    validate_recipient_shape(addresses)?;

    let wm = wallet.wallet_manager.read().await;
    let info = wm.get_wallet_info(&wallet.wallet_id).ok_or_else(|| {
        PlatformWalletError::WalletNotFound(format!(
            "Wallet {:?} not found in wallet manager",
            hex::encode(wallet.wallet_id)
        ))
    })?;
    let account = info
        .core_wallet
        .platform_payment_managed_account_at_index(platform_account_index)
        .ok_or_else(|| {
            PlatformWalletError::AddressSync(format!(
                "No platform payment account at index {}",
                platform_account_index
            ))
        })?;

    validate_recipient_map(addresses, ownership, platform_account_index, |p2pkh| {
        account.contains_platform_address(p2pkh)
    })
}

/// Ownership-independent half of the pre-flight: cardinality and
/// address-type rules that hold for every funding mode.
///
/// P2SH stays rejected for ALL recipients, including pure third-party
/// payees under [`RecipientOwnership::ExternalExplicitOutputs`].
/// Relaxing that for recipient-only outputs is a deliberate follow-up:
/// the FFI's `TryFrom<PlatformAddressFFI> for PlatformAddress` rejects
/// the P2SH discriminant outright, so lifting the restriction here
/// alone would not actually make P2SH reachable from the SDKs.
fn validate_recipient_shape(
    addresses: &BTreeMap<PlatformAddress, Option<Credits>>,
) -> Result<(), PlatformWalletError> {
    if addresses.is_empty() {
        return Err(PlatformWalletError::AddressOperation(
            "fund_from_asset_lock requires at least one recipient address".to_string(),
        ));
    }

    let none_count = addresses.values().filter(|v| v.is_none()).count();
    if none_count != 1 {
        return Err(PlatformWalletError::AddressOperation(format!(
            "Exactly one address must have None balance (the funding recipient), found {}",
            none_count
        )));
    }

    for addr in addresses.keys() {
        if !matches!(addr, PlatformAddress::P2pkh(_)) {
            return Err(PlatformWalletError::AddressOperation(
                "Only P2PKH addresses are supported".to_string(),
            ));
        }
    }

    Ok(())
}

/// Pure recipient-map pre-flight, generic over the ownership oracle so
/// it can be unit-tested without standing up a `PlatformAddressWallet`.
///
/// `is_owned` answers "is this address a member of
/// `platform_account_index`?"; in production it is
/// `ManagedPlatformAccount::contains_platform_address`.
fn validate_recipient_map<F>(
    addresses: &BTreeMap<PlatformAddress, Option<Credits>>,
    ownership: RecipientOwnership,
    platform_account_index: u32,
    is_owned: F,
) -> Result<(), PlatformWalletError>
where
    F: Fn(&PlatformP2PKHAddress) -> bool,
{
    validate_recipient_shape(addresses)?;

    for (addr, amount) in addresses {
        let PlatformAddress::P2pkh(hash) = addr else {
            // Unreachable: `validate_recipient_shape` already rejected
            // every non-P2PKH address above. Kept as a total match
            // rather than an `unwrap` so a future third variant is a
            // compile-time prompt, not a panic.
            return Err(PlatformWalletError::AddressOperation(
                "Only P2PKH addresses are supported".to_string(),
            ));
        };
        let p2pkh = PlatformP2PKHAddress::new(*hash);

        // `None` == the remainder/change bucket. It must be ours in
        // BOTH modes: the asset lock is consumed in full, so this
        // output receives everything left over after the explicit
        // outputs and the fee.
        let is_remainder = amount.is_none();
        let must_be_owned = is_remainder || ownership == RecipientOwnership::AllOwned;
        if must_be_owned && !is_owned(&p2pkh) {
            return Err(PlatformWalletError::AddressNotFound(if is_remainder {
                format!(
                    "Remainder (change) address {} does not belong to platform account index {}; \
                     the remainder output absorbs the asset lock's leftover value and must be \
                     owned by the sender",
                    p2pkh, platform_account_index
                )
            } else {
                format!(
                    "Address {} does not belong to platform account index {}",
                    p2pkh, platform_account_index
                )
            }));
        }
    }
    Ok(())
}

/// Post-submit guard: confirm the proof-attested `address_infos` carry a
/// usable `AddressInfo` for every requested recipient.
///
/// A wholly-empty map, an entry whose value is `None`, or a recipient
/// not present in the map at all are each a DAPI / proof-verifier
/// contract violation — Platform accepted the transition, but the
/// proof omits credits for the caller's recipients. Returning `Ok`
/// here would let `consume_asset_lock` terminally destroy the only
/// resumable record for the L1 funding outpoint while one or more
/// recipients silently lose credits, which (since asset locks are
/// non-refundable) is permanent value loss.
fn validate_address_infos_complete(
    addresses: &BTreeMap<PlatformAddress, Option<Credits>>,
    address_infos: &AddressInfos,
) -> Result<(), PlatformWalletError> {
    let expected_recipient_count = addresses.len();
    if address_infos.is_empty() {
        return Err(PlatformWalletError::AddressSync(format!(
            "Address-funding ST succeeded but the proof returned no address infos \
             (expected {} recipient(s)); refusing to consume the asset lock with \
             no recorded credits",
            expected_recipient_count
        )));
    }

    let missing_recipients: Vec<String> = addresses
        .keys()
        .filter_map(|address| match address_infos.get(address) {
            Some(Some(_)) => None,
            _ => Some(format!("{address:?}")),
        })
        .collect();

    if !missing_recipients.is_empty() {
        return Err(PlatformWalletError::AddressSync(format!(
            "Address-funding ST succeeded but the proof omitted usable AddressInfo \
             for recipient(s): {}",
            missing_recipients.join(", ")
        )));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use dash_sdk::query_types::AddressInfo;
    use dpp::version::LATEST_PLATFORM_VERSION;

    #[derive(Debug)]
    struct NullAddressSigner;

    #[async_trait::async_trait]
    impl Signer<PlatformAddress> for NullAddressSigner {
        async fn sign(
            &self,
            _key: &PlatformAddress,
            _data: &[u8],
        ) -> Result<dpp::platform_value::BinaryData, dpp::ProtocolError> {
            unreachable!("underfunded fresh asset lock must fail before address signing")
        }

        async fn sign_create_witness(
            &self,
            _key: &PlatformAddress,
            _data: &[u8],
        ) -> Result<dpp::address_funds::AddressWitness, dpp::ProtocolError> {
            unreachable!("underfunded fresh asset lock must fail before address signing")
        }

        fn can_sign_with(&self, _key: &PlatformAddress) -> bool {
            false
        }
    }

    struct NullAssetLockSigner;

    #[async_trait::async_trait]
    impl key_wallet::signer::Signer for NullAssetLockSigner {
        type Error = String;

        fn supported_methods(&self) -> &[key_wallet::signer::SignerMethod] {
            &[]
        }

        async fn sign_ecdsa(
            &self,
            _path: &key_wallet::DerivationPath,
            _sighash: [u8; 32],
        ) -> Result<
            (
                dashcore::secp256k1::ecdsa::Signature,
                dashcore::secp256k1::PublicKey,
            ),
            Self::Error,
        > {
            unreachable!("underfunded fresh asset lock must fail before asset-lock signing")
        }

        async fn public_key(
            &self,
            _path: &key_wallet::DerivationPath,
        ) -> Result<dashcore::secp256k1::PublicKey, Self::Error> {
            unreachable!("underfunded fresh asset lock must fail before asset-lock signing")
        }
    }

    #[async_trait::async_trait]
    impl key_wallet::signer::ExtendedPubKeySigner for NullAssetLockSigner {
        async fn extended_public_key(
            &self,
            _path: &key_wallet::DerivationPath,
        ) -> Result<key_wallet::bip32::ExtendedPubKey, Self::Error> {
            unreachable!("underfunded fresh asset lock must fail before xpub export")
        }
    }

    fn p2pkh(b: u8) -> PlatformAddress {
        PlatformAddress::P2pkh([b; 20])
    }

    fn info(addr: PlatformAddress) -> AddressInfo {
        AddressInfo {
            address: addr,
            balance: 1_000,
            nonce: 0,
        }
    }

    /// Ownership oracle standing in for
    /// `ManagedPlatformAccount::contains_platform_address`: the byte
    /// tags in `owned` are the addresses the sender's account holds.
    fn owned(tags: &[u8]) -> impl Fn(&PlatformP2PKHAddress) -> bool + '_ {
        move |addr: &PlatformP2PKHAddress| {
            tags.iter()
                .any(|t| PlatformP2PKHAddress::new([*t; 20]) == *addr)
        }
    }

    fn check(
        addresses: &BTreeMap<PlatformAddress, Option<Credits>>,
        ownership: RecipientOwnership,
        owned_tags: &[u8],
    ) -> Result<(), PlatformWalletError> {
        validate_recipient_map(addresses, ownership, 0, owned(owned_tags))
    }

    /// Legacy behaviour is untouched: an explicit-amount output the
    /// sender does not own is still rejected before anything is
    /// broadcast. This is the safety property that motivated adding a
    /// second entry point rather than relaxing this one.
    #[test]
    fn all_owned_rejects_external_explicit_recipient() {
        let mut addresses = BTreeMap::new();
        addresses.insert(p2pkh(0xBB), Some(500)); // Bob — not ours
        addresses.insert(p2pkh(0xAA), None); // our change
        let err = check(&addresses, RecipientOwnership::AllOwned, &[0xAA])
            .expect_err("legacy mode must reject a third-party recipient");
        let msg = format!("{err}");
        assert!(
            msg.contains("does not belong to platform account index"),
            "unexpected error: {msg}"
        );
    }

    #[test]
    fn all_owned_accepts_fully_owned_recipient_set() {
        let mut addresses = BTreeMap::new();
        addresses.insert(p2pkh(0xAA), Some(500));
        addresses.insert(p2pkh(0xAB), None);
        check(&addresses, RecipientOwnership::AllOwned, &[0xAA, 0xAB])
            .expect("fully-owned set must pass in legacy mode");
    }

    /// The feature: Alice pays Bob an exact amount and keeps the
    /// remainder on an address of her own.
    #[test]
    fn external_accepts_external_explicit_with_owned_remainder() {
        let mut addresses = BTreeMap::new();
        addresses.insert(p2pkh(0xBB), Some(500)); // Bob — external
        addresses.insert(p2pkh(0xAA), None); // Alice's change
        check(
            &addresses,
            RecipientOwnership::ExternalExplicitOutputs,
            &[0xAA],
        )
        .expect("external explicit recipient with owned remainder must pass");
    }

    /// Multiple third-party payees in a single asset lock are fine —
    /// consensus places no cap on output count beyond the versioned
    /// maximum, and none of them need to be ours.
    #[test]
    fn external_accepts_several_external_explicit_recipients() {
        let mut addresses = BTreeMap::new();
        addresses.insert(p2pkh(0xBB), Some(500));
        addresses.insert(p2pkh(0xCC), Some(700));
        addresses.insert(p2pkh(0xAA), None);
        check(
            &addresses,
            RecipientOwnership::ExternalExplicitOutputs,
            &[0xAA],
        )
        .expect("multiple external recipients must pass");
    }

    /// The one ownership rule the external variant keeps: change comes
    /// home. A caller bug that put the third party in the remainder
    /// slot would hand them the WHOLE lock value, not the intended
    /// payment, so this stays a hard error.
    #[test]
    fn external_rejects_external_remainder() {
        let mut addresses = BTreeMap::new();
        addresses.insert(p2pkh(0xAA), Some(500)); // ours, explicit
        addresses.insert(p2pkh(0xBB), None); // Bob would get the change
        let err = check(
            &addresses,
            RecipientOwnership::ExternalExplicitOutputs,
            &[0xAA],
        )
        .expect_err("an unowned remainder must be rejected");
        let msg = format!("{err}");
        assert!(
            matches!(err, PlatformWalletError::AddressNotFound(_)),
            "expected AddressNotFound, got: {msg}"
        );
        assert!(
            msg.contains("Remainder (change) address"),
            "the error must name the remainder output: {msg}"
        );
    }

    /// A recipient set with no owned address at all still fails on the
    /// remainder rule — there is no path to "send everything away".
    #[test]
    fn external_rejects_when_nothing_is_owned() {
        let mut addresses = BTreeMap::new();
        addresses.insert(p2pkh(0xBB), Some(500));
        addresses.insert(p2pkh(0xCC), None);
        check(&addresses, RecipientOwnership::ExternalExplicitOutputs, &[])
            .expect_err("a wholly-external recipient set must be rejected");
    }

    #[test]
    fn rejects_p2sh_recipient_in_both_modes() {
        // P2SH explicit output, owned remainder. Rejected even in the
        // external mode, where ownership would otherwise not matter —
        // the address-type restriction is orthogonal to ownership and
        // relaxing it is a separate follow-up.
        let mut addresses = BTreeMap::new();
        addresses.insert(PlatformAddress::P2sh([0xBB; 20]), Some(500));
        addresses.insert(p2pkh(0xAA), None);
        for ownership in [
            RecipientOwnership::AllOwned,
            RecipientOwnership::ExternalExplicitOutputs,
        ] {
            let err = check(&addresses, ownership, &[0xAA])
                .expect_err("P2SH recipients must be rejected in every mode");
            assert!(
                format!("{err}").contains("Only P2PKH addresses are supported"),
                "{ownership:?}: unexpected error: {err}"
            );
        }
    }

    #[test]
    fn rejects_zero_remainders() {
        let mut addresses = BTreeMap::new();
        addresses.insert(p2pkh(0xAA), Some(500));
        addresses.insert(p2pkh(0xBB), Some(700));
        for ownership in [
            RecipientOwnership::AllOwned,
            RecipientOwnership::ExternalExplicitOutputs,
        ] {
            let err = check(&addresses, ownership, &[0xAA, 0xBB])
                .expect_err("a recipient set with no remainder must be rejected");
            assert!(
                format!("{err}").contains("found 0"),
                "unexpected error: {err}"
            );
        }
    }

    #[test]
    fn rejects_two_remainders() {
        let mut addresses = BTreeMap::new();
        addresses.insert(p2pkh(0xAA), None);
        addresses.insert(p2pkh(0xBB), None);
        for ownership in [
            RecipientOwnership::AllOwned,
            RecipientOwnership::ExternalExplicitOutputs,
        ] {
            let err = check(&addresses, ownership, &[0xAA, 0xBB])
                .expect_err("two remainders must be rejected");
            assert!(
                format!("{err}").contains("found 2"),
                "unexpected error: {err}"
            );
        }
    }

    #[test]
    fn rejects_empty_recipient_map() {
        let addresses = BTreeMap::new();
        for ownership in [
            RecipientOwnership::AllOwned,
            RecipientOwnership::ExternalExplicitOutputs,
        ] {
            let err = check(&addresses, ownership, &[])
                .expect_err("an empty recipient map must be rejected");
            assert!(
                format!("{err}").contains("at least one recipient address"),
                "unexpected error: {err}"
            );
        }
    }

    /// The consensus contract this whole flow hangs on:
    /// `ReduceOutput(i)` is resolved by consensus against the outputs
    /// `BTreeMap`'s key order (`PlatformAddress`'s derived `Ord`), and
    /// `remainder_fee_strategy` must name the `None` output's position
    /// in exactly that order.
    ///
    /// Both arrangements are pinned, because the hazard is asymmetric:
    /// a caller computing the index from its own array order happens to
    /// be right whenever the remainder is also first lexicographically,
    /// and silently wrong otherwise. The second case below is the one a
    /// list-order index gets wrong — with a third-party payee in the set
    /// it would charge the fee to the payee's explicit amount instead of
    /// the sender's change.
    #[test]
    fn remainder_fee_strategy_targets_the_remainder_output() {
        let alice = p2pkh(0x0A);
        let bob = p2pkh(0xBB);
        let carol = p2pkh(0xCC);

        // Case 1: the remainder sorts FIRST. Caller-supplied insertion
        // order deliberately lists it LAST.
        let mut addresses = BTreeMap::new();
        addresses.insert(bob, Some(500));
        addresses.insert(carol, Some(700));
        addresses.insert(alice, None);

        let keys: Vec<PlatformAddress> = addresses.keys().copied().collect();
        assert_eq!(
            keys,
            vec![alice, bob, carol],
            "BTreeMap must order by PlatformAddress's derived Ord"
        );

        let strategy = remainder_fee_strategy(&addresses).expect("one remainder");
        assert_eq!(strategy, vec![AddressFundsFeeStrategyStep::ReduceOutput(0)]);

        // Case 2: the remainder sorts LAST while the caller listed it
        // FIRST — the arrangement a naive array-position index gets
        // wrong (it would answer 0 and charge the fee to Alice's payee
        // output).
        let mut addresses = BTreeMap::new();
        addresses.insert(carol, None);
        addresses.insert(alice, Some(500));
        addresses.insert(bob, Some(700));

        let strategy = remainder_fee_strategy(&addresses).expect("one remainder");
        assert_eq!(strategy, vec![AddressFundsFeeStrategyStep::ReduceOutput(2)]);

        // Resolve the emitted index the way consensus does and confirm
        // it lands on the `None` bucket, not on a payee.
        let keys: Vec<PlatformAddress> = addresses.keys().copied().collect();
        let AddressFundsFeeStrategyStep::ReduceOutput(index) = strategy[0] else {
            panic!("expected a ReduceOutput step");
        };
        assert_eq!(keys[index as usize], carol);
        assert_eq!(addresses[&keys[index as usize]], None);
    }

    /// The remainder sitting in the MIDDLE of the lexicographic order —
    /// an index that is neither 0 nor `len - 1`, so it cannot be
    /// produced by an off-by-one or a "first"/"last" shortcut.
    #[test]
    fn remainder_fee_strategy_handles_a_middle_remainder() {
        let mut addresses = BTreeMap::new();
        addresses.insert(p2pkh(0xCC), Some(700));
        addresses.insert(p2pkh(0x0A), Some(500));
        addresses.insert(p2pkh(0xBB), None);
        addresses.insert(p2pkh(0xDD), Some(900));

        let strategy = remainder_fee_strategy(&addresses).expect("one remainder");
        assert_eq!(strategy, vec![AddressFundsFeeStrategyStep::ReduceOutput(1)]);

        let keys: Vec<PlatformAddress> = addresses.keys().copied().collect();
        assert_eq!(addresses[&keys[1]], None);
    }

    /// P2SH sorts after every P2PKH (variant discriminant first), which
    /// is part of the ordering contract even though the pre-flight
    /// rejects P2SH recipients today. Pinned directly on the ordering
    /// helper so the rule survives any future relaxation of the
    /// address-type restriction.
    #[test]
    fn remainder_fee_strategy_orders_p2pkh_before_p2sh() {
        let mut addresses = BTreeMap::new();
        addresses.insert(PlatformAddress::P2sh([0x01; 20]), Some(500));
        addresses.insert(p2pkh(0xFF), None);

        let keys: Vec<PlatformAddress> = addresses.keys().copied().collect();
        assert_eq!(keys, vec![p2pkh(0xFF), PlatformAddress::P2sh([0x01; 20])]);

        let strategy = remainder_fee_strategy(&addresses).expect("one remainder");
        assert_eq!(strategy, vec![AddressFundsFeeStrategyStep::ReduceOutput(0)]);
    }

    /// No remainder at all is a typed error, never a panic and never a
    /// silent `ReduceOutput(0)` aimed at whichever payee sorts first.
    #[test]
    fn remainder_fee_strategy_rejects_a_map_with_no_remainder() {
        let mut addresses = BTreeMap::new();
        addresses.insert(p2pkh(0x0A), Some(500));
        addresses.insert(p2pkh(0xBB), Some(700));

        let err = remainder_fee_strategy(&addresses).expect_err("no remainder must be rejected");
        assert!(
            format!("{err}").contains("exactly one remainder"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn address_funding_floor_rounds_credits_up_to_duffs() {
        assert_eq!(ceil_credits_to_duffs(0), 0);
        assert_eq!(ceil_credits_to_duffs(CREDITS_PER_DUFF), 1);
        assert_eq!(ceil_credits_to_duffs(CREDITS_PER_DUFF + 1), 2);
    }

    #[test]
    fn address_funding_floor_uses_current_dpp_min_fee_for_output_count() {
        assert_eq!(
            minimum_address_funding_asset_lock_duffs(1, LATEST_PLATFORM_VERSION),
            56_000
        );
        assert_eq!(
            minimum_address_funding_asset_lock_duffs(2, LATEST_PLATFORM_VERSION),
            62_000
        );
    }

    #[test]
    fn fresh_asset_lock_floor_rejects_undersized_exact_amount() {
        let required = minimum_address_funding_asset_lock_duffs(2, LATEST_PLATFORM_VERSION);
        let funding = AssetLockFunding::FromWalletBalance {
            amount_duffs: required - 1,
            account_index: 0,
        };

        let err = enforce_asset_lock_amount_covers_address_funding_floor(
            funding,
            2,
            LATEST_PLATFORM_VERSION,
        )
        .expect_err("fresh asset lock below the admission floor must be rejected");

        match err {
            PlatformWalletError::AssetLockInsufficientFunds {
                available,
                required: actual_required,
            } => {
                assert_eq!(available, required - 1);
                assert_eq!(actual_required, required);
            }
            other => panic!("expected AssetLockInsufficientFunds, got {other:?}"),
        }
    }

    #[test]
    fn fresh_asset_lock_floor_accepts_exact_minimum() {
        let required = minimum_address_funding_asset_lock_duffs(2, LATEST_PLATFORM_VERSION);
        let funding = AssetLockFunding::FromWalletBalance {
            amount_duffs: required,
            account_index: 0,
        };

        let funding = enforce_asset_lock_amount_covers_address_funding_floor(
            funding,
            2,
            LATEST_PLATFORM_VERSION,
        )
        .expect("DPP rejects only amounts below the admission floor");

        assert!(matches!(
            funding,
            AssetLockFunding::FromWalletBalance {
                amount_duffs,
                account_index: 0
            } if amount_duffs == required
        ));
    }

    #[test]
    fn address_funding_floor_is_installed_for_fresh_drain_builds() {
        use key_wallet::wallet::managed_wallet_info::asset_lock_builder::AssetLockFundingAccount;

        let required = minimum_address_funding_asset_lock_duffs(2, LATEST_PLATFORM_VERSION);
        let funding = AssetLockFunding::DrainAccountBalance {
            account: AssetLockFundingAccount::Bip44 { account_index: 0 },
            minimum_lock_duffs: None,
        };

        let funding = enforce_asset_lock_amount_covers_address_funding_floor(
            funding,
            2,
            LATEST_PLATFORM_VERSION,
        )
        .expect("drain builds should be floored before broadcast");

        match funding {
            AssetLockFunding::DrainAccountBalance {
                minimum_lock_duffs, ..
            } => assert_eq!(minimum_lock_duffs, Some(required)),
            other => panic!("expected DrainAccountBalance, got {other:?}"),
        }
    }

    #[test]
    fn address_funding_floor_preserves_higher_fresh_drain_floor() {
        use key_wallet::wallet::managed_wallet_info::asset_lock_builder::AssetLockFundingAccount;

        let required = minimum_address_funding_asset_lock_duffs(2, LATEST_PLATFORM_VERSION);
        let higher_floor = required + 1;
        let funding = AssetLockFunding::DrainAccountBalance {
            account: AssetLockFundingAccount::Bip44 { account_index: 0 },
            minimum_lock_duffs: Some(higher_floor),
        };

        let funding = enforce_asset_lock_amount_covers_address_funding_floor(
            funding,
            2,
            LATEST_PLATFORM_VERSION,
        )
        .expect("caller-supplied drain floors should survive when stricter");

        match funding {
            AssetLockFunding::DrainAccountBalance {
                minimum_lock_duffs, ..
            } => assert_eq!(minimum_lock_duffs, Some(higher_floor)),
            other => panic!("expected DrainAccountBalance, got {other:?}"),
        }
    }

    #[test]
    fn address_funding_floor_does_not_block_existing_asset_lock_resume() {
        let funding = AssetLockFunding::FromExistingAssetLock {
            out_point: dashcore::OutPoint::null(),
            consume_invitation_voucher: false,
        };

        let funding = enforce_asset_lock_amount_covers_address_funding_floor(
            funding,
            2,
            LATEST_PLATFORM_VERSION,
        )
        .expect("existing locks are resumed rather than pre-sized");

        assert!(matches!(
            funding,
            AssetLockFunding::FromExistingAssetLock {
                out_point,
                consume_invitation_voucher: false
            } if out_point == dashcore::OutPoint::null()
        ));
    }

    #[tokio::test]
    async fn fund_from_asset_lock_rejects_underfloor_fresh_lock_before_broadcast() {
        let (manager, wallet_id) = crate::test_support::test_platform_wallet_manager().await;
        let wallet = manager.get_wallet(&wallet_id).await.expect("wallet handle");
        let address = wallet
            .platform()
            .next_unused_receive_address(
                key_wallet::account::account_collection::PlatformPaymentAccountKey {
                    account: 0,
                    key_class: 0,
                },
            )
            .await
            .expect("owned platform payment address");
        let required = minimum_address_funding_asset_lock_duffs(1, LATEST_PLATFORM_VERSION);
        let mut addresses = BTreeMap::new();
        addresses.insert(address, None);

        let err = wallet
            .platform()
            .fund_from_asset_lock(
                AssetLockFunding::FromWalletBalance {
                    amount_duffs: required - 1,
                    account_index: 0,
                },
                0,
                addresses,
                vec![],
                &NullAddressSigner,
                &NullAssetLockSigner,
                None,
            )
            .await
            .expect_err("underfloor fresh asset lock must fail before broadcast");

        match err {
            PlatformWalletError::AssetLockInsufficientFunds {
                available,
                required: actual_required,
            } => {
                assert_eq!(available, required - 1);
                assert_eq!(actual_required, required);
            }
            other => panic!("expected AssetLockInsufficientFunds, got {other:?}"),
        }
    }

    #[test]
    fn rejects_empty_address_infos_for_non_empty_recipients() {
        let mut addresses = BTreeMap::new();
        addresses.insert(p2pkh(1), None);
        let address_infos: AddressInfos = AddressInfos::new();
        let err = validate_address_infos_complete(&addresses, &address_infos)
            .expect_err("empty address_infos must be rejected");
        let msg = format!("{}", err);
        assert!(
            msg.contains("returned no address infos"),
            "unexpected error: {msg}"
        );
    }

    #[test]
    fn rejects_recipient_with_none_address_info() {
        // The proof contract violation we actually hit in practice:
        // proof carries the recipient key but with a `None` value.
        // Pre-fix, this slipped past the inline `is_empty()` guard
        // and the asset lock got consumed regardless.
        let mut addresses = BTreeMap::new();
        addresses.insert(p2pkh(1), None);
        let mut address_infos: AddressInfos = AddressInfos::new();
        address_infos.insert(p2pkh(1), None);
        let err = validate_address_infos_complete(&addresses, &address_infos)
            .expect_err("None info must be rejected");
        let msg = format!("{}", err);
        assert!(
            msg.contains("omitted usable AddressInfo"),
            "unexpected: {msg}"
        );
    }

    #[test]
    fn rejects_recipient_absent_from_address_infos() {
        // Multi-recipient case: proof present for one address but
        // missing the other entirely. Without the per-recipient
        // check, this would also silently consume the lock with
        // partial crediting.
        let mut addresses = BTreeMap::new();
        addresses.insert(p2pkh(1), Some(500));
        addresses.insert(p2pkh(2), None);
        let mut address_infos: AddressInfos = AddressInfos::new();
        address_infos.insert(p2pkh(1), Some(info(p2pkh(1))));
        let err = validate_address_infos_complete(&addresses, &address_infos)
            .expect_err("missing recipient must be rejected");
        let msg = format!("{}", err);
        assert!(
            msg.contains("omitted usable AddressInfo"),
            "unexpected: {msg}"
        );
    }

    #[test]
    fn accepts_every_recipient_with_some_address_info() {
        let mut addresses = BTreeMap::new();
        addresses.insert(p2pkh(1), Some(500));
        addresses.insert(p2pkh(2), None);
        let mut address_infos: AddressInfos = AddressInfos::new();
        address_infos.insert(p2pkh(1), Some(info(p2pkh(1))));
        address_infos.insert(p2pkh(2), Some(info(p2pkh(2))));
        validate_address_infos_complete(&addresses, &address_infos)
            .expect("complete proof must pass");
    }
}
