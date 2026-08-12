use grovedb_commitment_tree::{Anchor, FullViewingKey, SpendAuthorizingKey};

use crate::address_funds::OrchardAddress;
use crate::fee::Credits;
use crate::identity::core_script::CoreScript;
use crate::shielded::compute_shielded_withdrawal_fee;
use crate::state_transition::shielded_withdrawal_transition::methods::ShieldedWithdrawalTransitionMethodsV0;
use crate::state_transition::shielded_withdrawal_transition::ShieldedWithdrawalTransition;
use crate::state_transition::StateTransition;
use crate::withdrawal::Pooling;
use crate::ProtocolError;
use platform_version::version::PlatformVersion;

use super::{
    build_spend_bundle, serialize_authorized_bundle, shielded_bundle_action_count, OrchardProver,
    SpendableNote,
};

/// Builds a ShieldedWithdrawal state transition (shielded pool -> core L1 address).
///
/// Spends existing notes and withdraws value to a core chain script output.
/// The shielded fee is deducted from the spent notes. Any remaining value is
/// returned to the shielded `change_address`; the change note is encrypted
/// with the sender's External-scope OVK (derived from `fvk`) so the wallet
/// can recover it — including the structured memo — via OVK recovery.
///
/// # Parameters
/// - `spends` - Notes to spend with their Merkle paths
/// - `withdrawal_amount` - Amount to withdraw to the core chain
/// - `output_script` - Core chain script to receive the funds
/// - `core_fee_per_byte` - Core chain fee rate
/// - `pooling` - Withdrawal pooling strategy
/// - `change_address` - Orchard address for change output
/// - `fvk` - Full viewing key for spend authorization
/// - `ask` - Spend authorizing key for RedPallas signatures
/// - `anchor` - Sinsemilla root of the note commitment tree (Orchard Anchor)
/// - `prover` - Orchard prover (holds the Halo 2 proving key)
/// - `memo` - 36-byte structured memo for the change output (4-byte type tag + 32-byte payload)
/// - `platform_version` - Protocol version
///
/// The fee is not a parameter: consensus always charges exactly
/// `compute_shielded_withdrawal_fee` (the base shielded minimum fee PLUS the flat storage cost of
/// the Core withdrawal document this transition inserts) and ignores any surplus. Returns the built
/// transition together with the fee (in credits) that was applied.
#[allow(clippy::too_many_arguments)]
pub fn build_shielded_withdrawal_transition<P: OrchardProver>(
    spends: Vec<SpendableNote>,
    withdrawal_amount: u64,
    output_script: CoreScript,
    core_fee_per_byte: u32,
    pooling: Pooling,
    change_address: &OrchardAddress,
    fvk: &FullViewingKey,
    ask: &SpendAuthorizingKey,
    anchor: Anchor,
    prover: &P,
    memo: [u8; 36],
    platform_version: &PlatformVersion,
) -> Result<(StateTransition, Credits), ProtocolError> {
    if withdrawal_amount > i64::MAX as u64 {
        return Err(ProtocolError::ShieldedBuildError(format!(
            "withdrawal amount {} exceeds maximum allowed value {}",
            withdrawal_amount,
            i64::MAX as u64
        )));
    }

    let total_spent: u64 = spends.iter().map(|s| s.note.value().inner()).sum();

    // Orchard's BundleType::DEFAULT pads every bundle to a 2-action minimum
    // (MIN_ACTIONS), so even a single-spend withdrawal is serialized and proven with 2
    // actions. Price the fee against that same floor (matching shielded_transfer);
    // otherwise consensus recomputes min_fee from the on-wire actions.len() == 2 and
    // rejects an honest single-spend withdrawal with InsufficientShieldedFeeError (or,
    // post-fee, WithdrawalBelowMinAmountError).
    //
    // Routed through the shared predictor (1 shielded output — the change note), which is
    // numerically `spends.len().max(2)` AND enforces both consensus ceilings (the structural
    // action cap and the transition-size-derived one) BEFORE the ~30 s Halo 2 proof.
    let num_actions = shielded_bundle_action_count(spends.len(), 1, platform_version)?;
    // The fee is fixed at the withdrawal minimum: consensus always carves exactly
    // `compute_shielded_withdrawal_fee` from the pool — the base shielded minimum fee PLUS the
    // flat storage cost of the Core withdrawal document this transition inserts — and the net
    // (`withdrawal_amount`) goes to that Core withdrawal document.
    let fee = compute_shielded_withdrawal_fee(num_actions, platform_version)?;

    let required = withdrawal_amount.checked_add(fee).ok_or_else(|| {
        ProtocolError::ShieldedBuildError("fee + withdrawal_amount overflows u64".to_string())
    })?;
    if required > total_spent {
        return Err(ProtocolError::ShieldedBuildError(format!(
            "withdrawal amount {} + fee {} = {} exceeds total spendable value {}",
            withdrawal_amount, fee, required, total_spent
        )));
    }

    let change_amount = total_spent - required;

    // Bind every Core-facing withdrawal field into the Orchard sighash (output_script,
    // unshielding_amount == required, core_fee_per_byte, pooling) so the binding signature
    // authorizes them. Shared with the consensus verifier in shielded_proof.rs.
    let extra_sighash_data = crate::shielded::shielded_withdrawal_extra_sighash_data(
        output_script.as_bytes(),
        required,
        core_fee_per_byte,
        pooling,
        platform_version,
    )?;

    let bundle = build_spend_bundle(
        spends,
        change_address,
        change_amount,
        memo,
        fvk,
        ask,
        anchor,
        prover,
        &extra_sighash_data,
    )?;

    let sb = serialize_authorized_bundle(&bundle);

    let state_transition = ShieldedWithdrawalTransition::try_from_bundle(
        sb.actions,
        sb.value_balance as u64,
        sb.anchor,
        sb.proof,
        sb.binding_signature,
        core_fee_per_byte,
        pooling,
        output_script,
        platform_version,
    )?;
    Ok((state_transition, fee))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shielded::builder::test_helpers::{
        test_orchard_address, test_spendable_note, TestProver,
    };

    #[test]
    fn test_shielded_withdrawal_insufficient_funds() {
        let platform_version = PlatformVersion::latest();
        let change_address = test_orchard_address();

        let note = test_spendable_note(100);
        let spends = vec![note];

        let sk = grovedb_commitment_tree::SpendingKey::from_bytes([42u8; 32])
            .expect("valid spending key bytes");
        let fvk = FullViewingKey::from(&sk);
        let ask = SpendAuthorizingKey::from(&sk);

        let result = build_shielded_withdrawal_transition(
            spends,
            1_000_000,
            CoreScript::new_p2pkh([1u8; 20]),
            1,
            Pooling::Never,
            &change_address,
            &fvk,
            &ask,
            Anchor::empty_tree(),
            &TestProver,
            [0u8; 36],
            platform_version,
        );

        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("exceeds total spendable value"),
            "unexpected error: {}",
            err
        );
    }

    // --------------------------------------------------------------
    // Extra coverage — upper-bound / overflow / default branches
    // --------------------------------------------------------------

    #[test]
    fn test_shielded_withdrawal_amount_exceeds_i64_max_errors() {
        // `withdrawal_amount > i64::MAX as u64` is the first check and has
        // its own error branch.
        let platform_version = PlatformVersion::latest();
        let change_address = test_orchard_address();

        let note = test_spendable_note(u64::MAX);
        let spends = vec![note];

        let sk = grovedb_commitment_tree::SpendingKey::from_bytes([42u8; 32]).expect("valid sk");
        let fvk = FullViewingKey::from(&sk);
        let ask = SpendAuthorizingKey::from(&sk);

        let result = build_shielded_withdrawal_transition(
            spends,
            (i64::MAX as u64) + 1, // exceeds the i64 limit
            CoreScript::new_p2pkh([1u8; 20]),
            1,
            Pooling::Never,
            &change_address,
            &fvk,
            &ask,
            Anchor::empty_tree(),
            &TestProver,
            [0u8; 36],
            platform_version,
        );
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("exceeds maximum allowed value"),
            "unexpected error: {}",
            err
        );
    }

    #[test]
    fn test_shielded_withdrawal_zero_spends_errors() {
        // Empty spends vec → total_spent = 0 and num_actions = 2 (max floor).
        let platform_version = PlatformVersion::latest();
        let change_address = test_orchard_address();

        let sk = grovedb_commitment_tree::SpendingKey::from_bytes([42u8; 32]).expect("valid sk");
        let fvk = FullViewingKey::from(&sk);
        let ask = SpendAuthorizingKey::from(&sk);

        let result = build_shielded_withdrawal_transition(
            vec![],
            1,
            CoreScript::new_p2pkh([1u8; 20]),
            1,
            Pooling::Never,
            &change_address,
            &fvk,
            &ask,
            Anchor::empty_tree(),
            &TestProver,
            [0u8; 36],
            platform_version,
        );
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("exceeds total spendable value"),
            "unexpected error: {}",
            err
        );
    }
}
