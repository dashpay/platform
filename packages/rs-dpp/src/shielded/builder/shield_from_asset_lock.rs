use crate::address_funds::{OrchardAddress, PlatformAddress};
use crate::prelude::AssetLockProof;
use crate::state_transition::shield_from_asset_lock_transition::methods::ShieldFromAssetLockTransitionMethodsV0;
use crate::state_transition::shield_from_asset_lock_transition::ShieldFromAssetLockTransition;
use crate::state_transition::StateTransition;
use crate::ProtocolError;
use platform_version::version::PlatformVersion;

use super::{
    build_output_only_bundle, serialize_authorized_bundle, shielded_bundle_action_count,
    OrchardProver,
};

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
    let num_outputs = dummy_outputs.checked_add(1).ok_or_else(|| {
        ProtocolError::ShieldedBuildError("dummy_outputs overflows the output count".to_string())
    })?;
    shielded_bundle_action_count(0, num_outputs, platform_version)?;

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
    // proof, with the same checked output-count arithmetic.
    let num_outputs = dummy_outputs.checked_add(1).ok_or_else(|| {
        ProtocolError::ShieldedBuildError("dummy_outputs overflows the output count".to_string())
    })?;
    shielded_bundle_action_count(0, num_outputs, platform_version)?;

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
