use crate::address_funds::{OrchardAddress, PlatformAddress};
use crate::prelude::AssetLockProof;
use crate::state_transition::shield_from_asset_lock_transition::methods::ShieldFromAssetLockTransitionMethodsV0;
use crate::state_transition::shield_from_asset_lock_transition::ShieldFromAssetLockTransition;
use crate::state_transition::StateTransition;
use crate::ProtocolError;
use platform_version::version::PlatformVersion;

use super::{
    build_output_only_bundle, serialize_authorized_bundle, serialized_envelope_bytes,
    shielded_bundle_action_count, OrchardProver,
};

/// The asset-lock proof's envelope contribution BEYOND the measured baseline.
///
/// [`crate::shielded::SHIELDED_TRANSITION_WIRE_OVERHEAD_BYTES`] was calibrated from complete
/// `ShieldFromAssetLock` transitions that already carried a chain asset-lock proof, so the
/// baseline ALREADY contains one proof's worth of bytes. A supplied proof REPLACES that field
/// rather than adding to it; only the difference grows the transition. Modelling
/// `baseline + full proof` instead is conservative, but it can reject valid transitions at an
/// action boundary — the proof-size window between two action counts is only a few dozen bytes
/// wide (#4312 review finding b6f78dd76eb7).
///
/// Saturating: a chain proof at or below the baseline's own size yields `0` and leaves the
/// baseline ceiling untouched, while an instant proof's multi-KiB delta still tightens it.
///
/// Both builders below route their pre-proving size gate through this one function, so the gate
/// and its boundary tests cannot drift apart.
fn asset_lock_proof_envelope_delta_bytes(
    asset_lock_proof: &AssetLockProof,
) -> Result<u64, ProtocolError> {
    Ok(
        serialized_envelope_bytes(asset_lock_proof, "the asset-lock proof")?
            .saturating_sub(crate::shielded::SHIELDED_BASELINE_ASSET_LOCK_PROOF_BYTES),
    )
}

/// Builds a ShieldFromAssetLock state transition (core asset lock -> shielded pool).
///
/// Like Shield, constructs an output-only Orchard bundle. The funds come from
/// a core asset lock proof rather than platform address inputs.
///
/// # Parameters
/// - `recipient` - Orchard address to receive the shielded note
/// - `shield_amount` - Amount of credits to shield (from the asset lock)
/// - `asset_lock_proof` - Proof that funds are locked on core chain
/// - `asset_lock_private_key` - Private key for the asset lock (signs the transition)
/// - `prover` - Orchard prover (holds the Halo 2 proving key)
/// - `memo` - 36-byte structured memo for the recipient (4-byte type tag + 32-byte payload)
/// - `sender_ovk` - The sender's outgoing viewing key (External scope). With `Some`, the
///   recipient output's `out_ciphertext` is encrypted under it so the sender can later
///   recover the sent note (recipient, value, memo) from chain data via OVK recovery —
///   the Zcash outgoing-transaction-history convention. With `None`, a random outgoing
///   cipher key is used and the sent note is unrecoverable by anyone.
/// - `surplus_output` - Optional platform address that receives the asset-lock surplus
///   (`asset_lock_value − shield_amount − fee`); when `None`, the surplus is added to the fee
///   pools, capped at `shielded_implicit_fee_cap`
/// - `dummy_outputs` - Number of extra zero-value anonymity-set filler outputs to append after
///   the real recipient output (unrecoverable random addresses, `None` OVK, empty memo). `0`
///   reproduces the historical single-output bundle exactly. The on-wire action count becomes
///   `max(1 + dummy_outputs, 2)`, which consensus prices the fee from — see the pool-seeding flow.
/// - `platform_version` - Protocol version
#[allow(clippy::too_many_arguments)]
pub fn build_shield_from_asset_lock_transition<P: OrchardProver>(
    recipient: &OrchardAddress,
    shield_amount: u64,
    asset_lock_proof: AssetLockProof,
    asset_lock_private_key: &[u8],
    prover: &P,
    memo: [u8; 36],
    sender_ovk: Option<grovedb_commitment_tree::OutgoingViewingKey>,
    surplus_output: Option<PlatformAddress>,
    dummy_outputs: usize,
    platform_version: &PlatformVersion,
) -> Result<StateTransition, ProtocolError> {
    // Gate the on-wire action count (1 real output + the anonymity-set fillers) against both
    // consensus ceilings — the structural action cap and the transition-size-derived one —
    // BEFORE the ~30 s-per-bundle Halo 2 proof. The seeding flow's `MAX_ACTIONS_PER_BATCH`
    // stays within this, but the parameter is caller-controlled. Checked: `usize::MAX` dummies
    // must not wrap past the gate in release builds.
    //
    // The size side must price THIS transition's embedded asset-lock proof: an instant proof
    // carries its funding transaction and `InstantLock` (both hold input vectors — DPP admits
    // up to 100 inputs), which can consume the slack the baseline envelope leaves under
    // `max_state_transition_size`.
    //
    // Price the DELTA, not the whole proof: the baseline envelope was measured from
    // `ShieldFromAssetLock` transitions that already carried a chain asset-lock proof, and this
    // proof REPLACES that field rather than adding to it. Passing the full serialized size
    // would model `baseline + full proof` and reject valid transitions at an action boundary
    // (#4312 review finding b6f78dd76eb7).
    let num_outputs = dummy_outputs.checked_add(1).ok_or_else(|| {
        ProtocolError::ShieldedBuildError("dummy_outputs overflows the output count".to_string())
    })?;
    let proof_envelope_delta_bytes = asset_lock_proof_envelope_delta_bytes(&asset_lock_proof)?;
    shielded_bundle_action_count(0, num_outputs, proof_envelope_delta_bytes, platform_version)?;

    let bundle = build_output_only_bundle(
        recipient,
        shield_amount,
        memo,
        sender_ovk,
        dummy_outputs,
        prover,
    )?;
    let sb = serialize_authorized_bundle(&bundle);

    // For output-only bundles, Orchard value_balance is negative (value flowing in).
    // Convert to u64 (absolute amount entering the pool).
    let value_balance = sb
        .value_balance
        .checked_neg()
        .and_then(|v| u64::try_from(v).ok())
        .ok_or_else(|| {
            ProtocolError::ShieldedBuildError(
                "shield_from_asset_lock: bundle value_balance is not negative".to_string(),
            )
        })?;

    ShieldFromAssetLockTransition::try_from_asset_lock_with_bundle(
        asset_lock_proof,
        asset_lock_private_key,
        sb.actions,
        value_balance,
        sb.anchor,
        sb.proof,
        sb.binding_signature,
        surplus_output,
        platform_version,
    )
}

/// Builds a ShieldFromAssetLock state transition where the
/// asset-lock-proof signature is produced by an external
/// [`key_wallet::signer::Signer`] (Swift / hardware-wallet / HSM
/// flow). The raw private key never crosses the FFI boundary;
/// derive + sign + zeroise happen inside the signer.
///
/// # Parameters
/// - `recipient` - Orchard address to receive the shielded note
/// - `shield_amount` - Amount of credits to shield (from the asset lock)
/// - `asset_lock_proof` - Proof that funds are locked on core chain
/// - `asset_lock_proof_path` - BIP32 path to the asset-lock key inside `asset_lock_signer`
/// - `asset_lock_signer` - External signer that produces the outer ECDSA signature
/// - `prover` - Orchard prover (holds the Halo 2 proving key)
/// - `memo` - 36-byte structured memo for the recipient (4-byte type tag + 32-byte payload)
/// - `sender_ovk` - The sender's outgoing viewing key (External scope). With `Some`, the
///   recipient output's `out_ciphertext` is encrypted under it so the sender can later
///   recover the sent note (recipient, value, memo) from chain data via OVK recovery —
///   the Zcash outgoing-transaction-history convention. With `None`, a random outgoing
///   cipher key is used and the sent note is unrecoverable by anyone.
/// - `surplus_output` - Optional platform address that receives the asset-lock surplus
///   (`asset_lock_value − shield_amount − fee`); when `None`, the surplus is added to the fee
///   pools, capped at `shielded_implicit_fee_cap`
/// - `dummy_outputs` - Number of extra zero-value anonymity-set filler outputs to append after
///   the real recipient output (unrecoverable random addresses, `None` OVK, empty memo). `0`
///   reproduces the historical single-output bundle exactly. The on-wire action count becomes
///   `max(1 + dummy_outputs, 2)`, which consensus prices the fee from — see the pool-seeding flow.
/// - `platform_version` - Protocol version
#[cfg(feature = "core_key_wallet")]
#[allow(clippy::too_many_arguments)]
pub async fn build_shield_from_asset_lock_transition_with_signer<P, AS>(
    recipient: &OrchardAddress,
    shield_amount: u64,
    asset_lock_proof: AssetLockProof,
    asset_lock_proof_path: &::key_wallet::bip32::DerivationPath,
    asset_lock_signer: &AS,
    prover: &P,
    memo: [u8; 36],
    sender_ovk: Option<grovedb_commitment_tree::OutgoingViewingKey>,
    surplus_output: Option<PlatformAddress>,
    dummy_outputs: usize,
    platform_version: &PlatformVersion,
) -> Result<StateTransition, ProtocolError>
where
    P: OrchardProver,
    AS: ::key_wallet::signer::Signer,
{
    // Same pre-proving gate as the non-signer sibling: both consensus ceilings, before the
    // proof, with the same checked output-count arithmetic and the same
    // transition-specific proof envelope DELTA priced into the size side (the baseline already
    // carries a chain proof — see the sibling).
    let num_outputs = dummy_outputs.checked_add(1).ok_or_else(|| {
        ProtocolError::ShieldedBuildError("dummy_outputs overflows the output count".to_string())
    })?;
    let proof_envelope_delta_bytes = asset_lock_proof_envelope_delta_bytes(&asset_lock_proof)?;
    shielded_bundle_action_count(0, num_outputs, proof_envelope_delta_bytes, platform_version)?;

    let bundle = build_output_only_bundle(
        recipient,
        shield_amount,
        memo,
        sender_ovk,
        dummy_outputs,
        prover,
    )?;
    let sb = serialize_authorized_bundle(&bundle);

    // For output-only bundles, Orchard value_balance is negative (value flowing in).
    // Convert to u64 (absolute amount entering the pool).
    let value_balance = sb
        .value_balance
        .checked_neg()
        .and_then(|v| u64::try_from(v).ok())
        .ok_or_else(|| {
            ProtocolError::ShieldedBuildError(
                "shield_from_asset_lock: bundle value_balance is not negative".to_string(),
            )
        })?;

    ShieldFromAssetLockTransition::try_from_asset_lock_with_bundle_and_signer(
        asset_lock_proof,
        asset_lock_proof_path,
        asset_lock_signer,
        sb.actions,
        value_balance,
        sb.anchor,
        sb.proof,
        sb.binding_signature,
        surplus_output,
        platform_version,
    )
    .await
}

#[cfg(test)]
mod envelope_gate_tests {
    //! Serialized-size boundary coverage for the transition-specific envelope
    //! in the pre-proving gate (#4312 review finding e90e9cf15f52): a chain
    //! proof keeps the baseline ceiling, while a realistic multi-input
    //! instant proof consumes the slack and must tighten the ceiling BEFORE
    //! any Halo 2 work.
    use std::str::FromStr;

    use dashcore::bls_sig_utils::BLSSignature;
    use dashcore::hash_types::CycleHash;
    use dashcore::transaction::special_transaction::asset_lock::AssetLockPayload;
    use dashcore::transaction::special_transaction::TransactionPayload;
    use dashcore::{InstantLock, OutPoint, ScriptBuf, Transaction, TxIn, TxOut, Txid};
    use platform_version::version::PlatformVersion;

    use super::asset_lock_proof_envelope_delta_bytes;
    use crate::identity::state_transition::asset_lock_proof::chain::ChainAssetLockProof;
    use crate::identity::state_transition::asset_lock_proof::{
        AssetLockProof, InstantAssetLockProof,
    };
    use crate::shielded::builder::{serialized_envelope_bytes, shielded_bundle_action_count};
    use crate::shielded::{
        max_shielded_actions_for_envelope, max_shielded_actions_per_transition,
        SHIELDED_BASELINE_ASSET_LOCK_PROOF_BYTES,
    };

    fn txid() -> Txid {
        Txid::from_str("a477af6b2667c29670467e4e0728b685ee07b240235771862318e29ddbe58458").unwrap()
    }

    /// An instant asset-lock proof whose funding transaction and
    /// `InstantLock` both carry `num_inputs` inputs — the DPP-admitted shape
    /// (up to 100 inputs) that the fixed baseline envelope cannot cover.
    /// Script sigs are sized like real signed P2PKH inputs (~107 bytes).
    fn instant_proof_with_inputs(num_inputs: usize) -> AssetLockProof {
        instant_proof_with_padded_input(num_inputs, 0)
    }

    /// [`instant_proof_with_inputs`] with `pad` extra bytes on the FIRST input's script sig, so
    /// the proof's serialized size can be tuned a byte at a time. Whole inputs move the size in
    /// ~190-byte steps, which is far coarser than the boundary windows the gate tests probe.
    fn instant_proof_with_padded_input(num_inputs: usize, pad: usize) -> AssetLockProof {
        let inputs: Vec<TxIn> = (0..num_inputs)
            .map(|i| TxIn {
                previous_output: OutPoint::new(txid(), i as u32),
                script_sig: ScriptBuf::from(vec![0u8; 107 + if i == 0 { pad } else { 0 }]),
                sequence: 0,
                witness: Default::default(),
            })
            .collect();
        let transaction = Transaction {
            version: 3,
            lock_time: 0,
            input: inputs,
            output: vec![TxOut {
                value: 100_000_000,
                script_pubkey: ScriptBuf::new_op_return(&[]),
            }],
            special_transaction_payload: Some(TransactionPayload::AssetLockPayloadType(
                AssetLockPayload {
                    version: 0,
                    credit_outputs: vec![TxOut {
                        value: 100_000_000,
                        script_pubkey: ScriptBuf::from(vec![0u8; 25]),
                    }],
                },
            )),
        };
        let instant_lock = InstantLock {
            version: 1,
            inputs: (0..num_inputs)
                .map(|i| OutPoint::new(txid(), i as u32))
                .collect(),
            txid: transaction.txid(),
            cyclehash: CycleHash::from_str(
                "7c30826123d0f29fe4c4a8895d7ba4eb469b1fafa6ad7b23896a1a591766a536",
            )
            .unwrap(),
            signature: BLSSignature::from_str(
                "8967c46529a967b3822e1ba8a173066296d02593f0f59b3a78a30a7eef9c8a120847729e62e\
                 4a32954339286b79fe7590221331cd28d576887a263f45b595d499272f656c3f5176987c976\
                 239cac16f972d796ad82931d532102a4f95eec7d80",
            )
            .unwrap(),
        };
        AssetLockProof::Instant(InstantAssetLockProof::new(instant_lock, transaction, 0))
    }

    /// A chain proof serializes to a few dozen bytes, so pricing it into the
    /// gate must NOT move the ceiling off the baseline.
    #[test]
    fn chain_proof_envelope_keeps_the_baseline_ceiling() {
        let platform_version = PlatformVersion::latest();
        let baseline = max_shielded_actions_per_transition(platform_version);

        let proof = AssetLockProof::Chain(ChainAssetLockProof {
            core_chain_locked_height: 1_042_000,
            out_point: OutPoint::new(txid(), 0),
        });
        let bytes =
            serialized_envelope_bytes(&proof, "the asset-lock proof").expect("measurable proof");
        assert!(
            bytes < 128,
            "a chain proof must serialize to a few dozen bytes, got {bytes}"
        );
        assert_eq!(
            max_shielded_actions_for_envelope(platform_version, bytes),
            baseline,
            "a chain-proof envelope must keep the baseline ceiling"
        );
        // The full baseline-ceiling output shape still passes the gate.
        shielded_bundle_action_count(0, baseline, bytes, platform_version)
            .expect("a chain-proof shield at the baseline ceiling must pass the pre-proving gate");
    }

    /// A realistic multi-input instant proof consumes more than the slack the
    /// baseline ceiling leaves under `max_state_transition_size`, so the gate
    /// must reject the baseline-ceiling shape BEFORE proving — the exact
    /// scenario the fixed envelope let through.
    #[test]
    fn multi_input_instant_proof_envelope_tightens_the_ceiling() {
        let platform_version = PlatformVersion::latest();
        let baseline = max_shielded_actions_per_transition(platform_version);

        // 20 inputs is well inside DPP's 100-input admission and already
        // costs ~3.5 KiB of envelope — more than the ~1.4 KiB slack.
        let proof = instant_proof_with_inputs(20);
        let bytes =
            serialized_envelope_bytes(&proof, "the asset-lock proof").expect("measurable proof");
        let ceiling = max_shielded_actions_for_envelope(platform_version, bytes);
        assert!(
            ceiling < baseline,
            "a {bytes}-byte instant-proof envelope must tighten the ceiling below the \
             baseline {baseline}"
        );

        // The shape that passes with a chain proof must now fail, pre-proving,
        // with the size-derived message naming the envelope.
        let err = shielded_bundle_action_count(0, baseline, bytes, platform_version).expect_err(
            "the baseline-ceiling shape must be rejected pre-proving under a multi-input \
             instant-proof envelope",
        );
        let msg = err.to_string();
        assert!(
            msg.contains("max_state_transition_size") && msg.contains("transition-specific"),
            "unexpected error: {msg}"
        );

        // At the tightened ceiling the gate accepts again — the gate tightens,
        // it does not shut.
        shielded_bundle_action_count(0, ceiling.max(1), bytes, platform_version)
            .expect("the tightened ceiling itself must pass");

        // And a DPP-maximal 100-input proof must tighten further, never panic.
        let max_proof = instant_proof_with_inputs(100);
        let max_bytes = serialized_envelope_bytes(&max_proof, "the asset-lock proof")
            .expect("measurable proof");
        assert!(max_bytes > bytes);
        assert!(max_shielded_actions_for_envelope(platform_version, max_bytes) <= ceiling);
    }

    /// `SHIELDED_BASELINE_ASSET_LOCK_PROOF_BYTES` must equal the encoded size of the very proof
    /// the baseline envelope was calibrated with — the chain proof from
    /// `shield_from_asset_lock_transition/signing_tests.rs::make_chain_asset_lock_proof`, whose
    /// transitions produced the 2-, 6- and 7-action measurements behind
    /// `SHIELDED_TRANSITION_WIRE_OVERHEAD_BYTES`. If the encoding moves, this fails instead of
    /// silently biasing the gate.
    #[test]
    fn baseline_asset_lock_proof_bytes_matches_the_calibration_proof() {
        let calibration_proof = AssetLockProof::Chain(ChainAssetLockProof {
            core_chain_locked_height: 100,
            out_point: OutPoint::from([11u8; 36]),
        });
        let measured = serialized_envelope_bytes(&calibration_proof, "the asset-lock proof")
            .expect("measurable proof");
        assert_eq!(
            measured, SHIELDED_BASELINE_ASSET_LOCK_PROOF_BYTES,
            "the baseline proof allowance must equal the calibration proof's encoded size"
        );

        // The calibration proof itself must therefore cost NOTHING extra: it is exactly what the
        // baseline already models.
        assert_eq!(
            asset_lock_proof_envelope_delta_bytes(&calibration_proof).expect("measurable proof"),
            0,
            "the calibration proof must not be charged twice"
        );
        assert_eq!(
            max_shielded_actions_for_envelope(PlatformVersion::latest(), 0),
            max_shielded_actions_per_transition(PlatformVersion::latest()),
        );
    }

    /// The fix, at the boundary it actually matters: a proof whose FULL serialized size pushes
    /// the action ceiling down by one, but whose DELTA above the baseline does not.
    ///
    /// Under the old `baseline + full proof` model such a transition was rejected pre-proving
    /// even though the real transition — which carries the proof INSTEAD of the baseline's own
    /// chain proof — fits under `max_state_transition_size`. The window is only
    /// `SHIELDED_BASELINE_ASSET_LOCK_PROOF_BYTES` wide, so it is searched for rather than
    /// hardcoded: a limits change moves it, and the test follows.
    #[test]
    fn proof_envelope_prices_only_the_delta_above_the_baseline() {
        let platform_version = PlatformVersion::latest();
        let baseline = max_shielded_actions_per_transition(platform_version);

        // Whole inputs step ~190 B at a time — far coarser than the ~40 B window where the two
        // models disagree — so widen by whole inputs first, then by SINGLE script bytes. Take
        // the largest input count still below the ceiling drop, then pad one byte at a time
        // until the FULL size first costs an action; that first crossing is by construction
        // within a byte or two of the threshold, hence inside the window.
        let full_bytes = |proof: &AssetLockProof| {
            serialized_envelope_bytes(proof, "the asset-lock proof").expect("measurable proof")
        };
        let costs_an_action =
            |bytes: u64| max_shielded_actions_for_envelope(platform_version, bytes) < baseline;

        let mut num_inputs = 1;
        while num_inputs < 100
            && !costs_an_action(full_bytes(&instant_proof_with_inputs(num_inputs + 1)))
        {
            num_inputs += 1;
        }
        assert!(
            !costs_an_action(full_bytes(&instant_proof_with_inputs(num_inputs))),
            "the starting input count must still afford the baseline ceiling"
        );

        let mut boundary = None;
        for pad in 0..1_024 {
            let proof = instant_proof_with_padded_input(num_inputs, pad);
            let full = full_bytes(&proof);
            if costs_an_action(full) {
                boundary = Some((proof, full));
                break;
            }
        }
        let (proof, full) = boundary.expect("byte-level padding must cross the threshold");
        let delta = asset_lock_proof_envelope_delta_bytes(&proof).expect("measurable proof");

        assert_eq!(
            delta,
            full - SHIELDED_BASELINE_ASSET_LOCK_PROOF_BYTES,
            "the delta must be the full proof minus the baseline's own proof"
        );
        assert_eq!(
            max_shielded_actions_for_envelope(platform_version, delta),
            baseline,
            "at this boundary the delta must still afford the baseline ceiling ({baseline}), \
             while the full proof ({full} bytes) does not — that gap is the double-count"
        );

        // The gate as the BUILDERS call it (same helper) now accepts the baseline-ceiling shape
        // that the old full-proof model rejected pre-proving.
        shielded_bundle_action_count(0, baseline, delta, platform_version).expect(
            "the boundary transition must pass the pre-proving gate once the proof is priced \
             as a delta",
        );
        shielded_bundle_action_count(0, baseline, full, platform_version).expect_err(
            "the old full-proof model rejected this exact shape — that is the regression this \
             test pins",
        );

        // The correction is a bounded credit, not a hole: a proof large enough on its own still
        // tightens the ceiling even after the baseline is subtracted.
        let big = instant_proof_with_inputs(100);
        let big_delta = asset_lock_proof_envelope_delta_bytes(&big).expect("measurable proof");
        assert!(
            max_shielded_actions_for_envelope(platform_version, big_delta) < baseline,
            "a DPP-maximal instant proof must still tighten the ceiling"
        );
        shielded_bundle_action_count(0, baseline, big_delta, platform_version)
            .expect_err("a DPP-maximal instant proof must still be rejected at the baseline shape");
    }
}

#[cfg(test)]
mod tests {
    use super::super::{build_output_only_bundle, serialize_authorized_bundle};
    use crate::shielded::builder::test_helpers::{test_orchard_address, TestProver};

    /// Verifies that an output-only bundle produces a negative value_balance
    /// (value flowing into the pool), which is the precondition for
    /// shield_from_asset_lock's value_balance conversion.
    #[test]
    fn test_output_only_bundle_value_balance_is_negative() {
        let recipient = test_orchard_address();
        let amount = 50_000u64;

        let bundle = build_output_only_bundle(&recipient, amount, [0u8; 36], None, 0, &TestProver)
            .expect("bundle should build successfully");
        let sb = serialize_authorized_bundle(&bundle);

        // Output-only bundles have negative value_balance (value entering the pool)
        assert!(
            sb.value_balance < 0,
            "expected negative value_balance, got {}",
            sb.value_balance
        );

        // The absolute value should match the shield amount
        let abs_balance = sb
            .value_balance
            .checked_neg()
            .and_then(|v| u64::try_from(v).ok())
            .expect("value_balance should be safely negatable");
        assert_eq!(abs_balance, amount);
    }

    /// Consensus prices the shielded fee from the on-wire `actions.len()`, and the wallet reserves
    /// the fee for exactly 2 actions (Orchard's `MIN_ACTIONS`). A single-output, spends-disabled
    /// bundle must therefore serialize to exactly 2 on-wire actions. If a future Orchard or builder
    /// change alters that padding, the hardcoded wallet reservation would diverge from what consensus
    /// charges (a valid client tx would be rejected); this test fails loudly if that invariant breaks.
    #[test]
    fn test_output_only_bundle_serializes_to_min_actions() {
        let recipient = test_orchard_address();
        let bundle =
            build_output_only_bundle(&recipient, 50_000u64, [0u8; 36], None, 0, &TestProver)
                .expect("bundle should build");
        let sb = serialize_authorized_bundle(&bundle);
        assert_eq!(
            sb.actions.len(),
            2,
            "single-output shield bundle must pad to exactly 2 on-wire actions"
        );
    }

    // -------------------------------------------------------------
    // Arithmetic edge cases on the value_balance conversion branch
    // (the `checked_neg().and_then(u64::try_from)` chain).
    // -------------------------------------------------------------

    #[test]
    fn test_value_balance_positive_would_fail_conversion() {
        // This is a regression-guard: if a *positive* value_balance ever
        // reached the conversion path, `checked_neg` on i64::MIN would
        // overflow and the `.try_from::<u64>` on a negative value would
        // fail. We simulate by constructing a hypothetical value_balance
        // scenario rather than calling the high-level builder (which
        // requires a real AssetLockProof).
        let positive: i64 = 123;
        let converted = positive.checked_neg().and_then(|v| u64::try_from(v).ok());
        assert!(converted.is_none(), "negative result cannot be u64");

        let zero: i64 = 0;
        let converted_zero = zero.checked_neg().and_then(|v| u64::try_from(v).ok());
        assert_eq!(converted_zero, Some(0));

        let negative: i64 = -42;
        let converted_neg = negative.checked_neg().and_then(|v| u64::try_from(v).ok());
        assert_eq!(converted_neg, Some(42));
    }

    #[test]
    fn test_output_only_various_amounts_negative_balance() {
        // Try several amounts to ensure the helper consistently produces a
        // negative value_balance equal in magnitude to the requested amount.
        for amount in [1u64, 100, 1_000_000, u32::MAX as u64] {
            let recipient = test_orchard_address();
            let bundle =
                build_output_only_bundle(&recipient, amount, [0u8; 36], None, 0, &TestProver)
                    .expect("bundle should build");
            let sb = serialize_authorized_bundle(&bundle);
            assert_eq!(
                sb.value_balance,
                -(amount as i64),
                "value_balance mismatch for amount {}",
                amount
            );
        }
    }
}
