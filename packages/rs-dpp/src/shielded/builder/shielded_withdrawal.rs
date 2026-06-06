use grovedb_commitment_tree::{Anchor, FullViewingKey, SpendAuthorizingKey};

use crate::address_funds::OrchardAddress;
use crate::fee::Credits;
use crate::identity::core_script::CoreScript;
use crate::shielded::compute_minimum_shielded_fee;
use crate::state_transition::shielded_withdrawal_transition::methods::ShieldedWithdrawalTransitionMethodsV0;
use crate::state_transition::shielded_withdrawal_transition::ShieldedWithdrawalTransition;
use crate::state_transition::StateTransition;
use crate::withdrawal::Pooling;
use crate::ProtocolError;
use platform_version::version::PlatformVersion;

use super::{build_spend_bundle, serialize_authorized_bundle, OrchardProver, SpendableNote};

/// Builds a ShieldedWithdrawal state transition (shielded pool -> core L1 address).
///
/// Spends existing notes and withdraws value to a core chain script output.
/// The shielded fee is deducted from the spent notes. Any remaining value is
/// returned to the shielded `change_address`.
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
/// - `fee` - Optional fee. Consensus always charges exactly `compute_minimum_shielded_fee`
///   and ignores any surplus, so this must be `None` (use the minimum) or `Some(min_fee)`;
///   any other value is rejected.
/// - `platform_version` - Protocol version
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
    fee: Option<Credits>,
    platform_version: &PlatformVersion,
) -> Result<StateTransition, ProtocolError> {
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
    let num_actions = spends.len().max(2);
    let min_fee = compute_minimum_shielded_fee(num_actions, platform_version)?;
    // Consensus always carves exactly `compute_minimum_shielded_fee` from the pool and never
    // reads the embedded value_balance surplus, so any `fee > min_fee` is NOT charged as fee —
    // it would silently inflate the Core withdrawal amount instead. Pin the fee to the minimum
    // (matching `build_shielded_transfer_transition`) so the builder fails fast rather than
    // producing a transition whose effective payout differs from `withdrawal_amount`.
    let effective_fee = match fee {
        None => min_fee,
        Some(f) if f == min_fee => min_fee,
        Some(f) => {
            return Err(ProtocolError::ShieldedBuildError(format!(
                "withdrawal fee must equal the minimum fee {} exactly (consensus charges the \
                 minimum; any surplus would inflate the withdrawal amount, not the fee); got {}",
                min_fee, f
            )));
        }
    };

    let required = withdrawal_amount
        .checked_add(effective_fee)
        .ok_or_else(|| {
            ProtocolError::ShieldedBuildError("fee + withdrawal_amount overflows u64".to_string())
        })?;
    if required > total_spent {
        return Err(ProtocolError::ShieldedBuildError(format!(
            "withdrawal amount {} + fee {} = {} exceeds total spendable value {}",
            withdrawal_amount, effective_fee, required, total_spent
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
    );

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

    ShieldedWithdrawalTransition::try_from_bundle(
        sb.actions,
        sb.value_balance as u64,
        sb.anchor,
        sb.proof,
        sb.binding_signature,
        core_fee_per_byte,
        pooling,
        output_script,
        platform_version,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shielded::builder::test_helpers::{
        test_orchard_address, test_spendable_note, TestProver,
    };

    #[test]
    fn test_shielded_withdrawal_fee_below_minimum() {
        let platform_version = PlatformVersion::latest();
        let change_address = test_orchard_address();

        let note = test_spendable_note(1_000_000);
        let spends = vec![note];

        let sk = grovedb_commitment_tree::SpendingKey::from_bytes([42u8; 32])
            .expect("valid spending key bytes");
        let fvk = FullViewingKey::from(&sk);
        let ask = SpendAuthorizingKey::from(&sk);

        let result = build_shielded_withdrawal_transition(
            spends,
            100,
            CoreScript::new_p2pkh([1u8; 20]), // minimal P2PKH prefix
            1,
            Pooling::Never,
            &change_address,
            &fvk,
            &ask,
            Anchor::empty_tree(),
            &TestProver,
            [0u8; 36],
            Some(1), // fee = 1 != min_fee → rejected (fee is pinned to the minimum)
            platform_version,
        );

        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("must equal the minimum fee"),
            "unexpected error: {}",
            err
        );
    }

    #[test]
    fn test_shielded_withdrawal_fee_above_min_rejected() {
        let platform_version = PlatformVersion::latest();
        let change_address = test_orchard_address();

        let note = test_spendable_note(u64::MAX);
        let spends = vec![note];

        let sk = grovedb_commitment_tree::SpendingKey::from_bytes([42u8; 32])
            .expect("valid spending key bytes");
        let fvk = FullViewingKey::from(&sk);
        let ask = SpendAuthorizingKey::from(&sk);

        // The fee is pinned to the minimum; any value above it is rejected (the surplus would
        // inflate the withdrawal amount, not the fee). Builder pads to max(spends, 2).
        let num_actions = 2usize;
        let min_fee = crate::shielded::compute_minimum_shielded_fee(num_actions, platform_version)
            .expect("fee computation should not overflow");
        let above_min = min_fee + 1;

        let result = build_shielded_withdrawal_transition(
            spends,
            100,
            CoreScript::new_p2pkh([1u8; 20]),
            1,
            Pooling::Never,
            &change_address,
            &fvk,
            &ask,
            Anchor::empty_tree(),
            &TestProver,
            [0u8; 36],
            Some(above_min),
            platform_version,
        );

        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("must equal the minimum fee"),
            "unexpected error: {}",
            err
        );
    }

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
            None,
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
            None,
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
    fn test_shielded_withdrawal_fee_far_above_min_rejected() {
        // A fee far above the minimum is rejected like any other value != min_fee.
        let platform_version = PlatformVersion::latest();
        let change_address = test_orchard_address();

        let note = test_spendable_note(u64::MAX);
        let spends = vec![note];

        let sk = grovedb_commitment_tree::SpendingKey::from_bytes([42u8; 32]).expect("valid sk");
        let fvk = FullViewingKey::from(&sk);
        let ask = SpendAuthorizingKey::from(&sk);

        let min_fee = crate::shielded::compute_minimum_shielded_fee(2, platform_version)
            .expect("fee computation should not overflow");
        let far_above_min = min_fee.saturating_mul(1000);

        let result = build_shielded_withdrawal_transition(
            spends,
            100,
            CoreScript::new_p2pkh([1u8; 20]),
            1,
            Pooling::Never,
            &change_address,
            &fvk,
            &ask,
            Anchor::empty_tree(),
            &TestProver,
            [0u8; 36],
            Some(far_above_min),
            platform_version,
        );

        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("must equal the minimum fee"),
            "unexpected error: {}",
            err
        );
    }

    #[test]
    fn test_shielded_withdrawal_fee_at_exact_min_accepted() {
        // Boundary test: fee == min_fee should be accepted (strictly `<`
        // is rejected).
        let platform_version = PlatformVersion::latest();
        let change_address = test_orchard_address();

        let note = test_spendable_note(1_000_000);
        let spends = vec![note];

        let sk = grovedb_commitment_tree::SpendingKey::from_bytes([42u8; 32]).expect("valid sk");
        let fvk = FullViewingKey::from(&sk);
        let ask = SpendAuthorizingKey::from(&sk);

        let min_fee = crate::shielded::compute_minimum_shielded_fee(2, platform_version)
            .expect("fee computation should not overflow");

        let result = build_shielded_withdrawal_transition(
            spends,
            100,
            CoreScript::new_p2pkh([1u8; 20]),
            1,
            Pooling::Never,
            &change_address,
            &fvk,
            &ask,
            Anchor::empty_tree(),
            &TestProver,
            [0u8; 36],
            Some(min_fee),
            platform_version,
        );
        // Fee == min_fee is accepted by the pin; a successful build is fine. Only a
        // later-stage failure should surface — never the fee-mismatch error.
        if let Err(err) = result {
            let err = err.to_string();
            assert!(
                !err.contains("must equal the minimum fee"),
                "fee == min must not be rejected as a fee mismatch: {}",
                err
            );
        }
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
            None,
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
