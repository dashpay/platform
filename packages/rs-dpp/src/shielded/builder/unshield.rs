use grovedb_commitment_tree::{Anchor, FullViewingKey, SpendAuthorizingKey};

use crate::address_funds::{OrchardAddress, PlatformAddress};
use crate::fee::Credits;
use crate::shielded::compute_shielded_unshield_fee;
use crate::state_transition::unshield_transition::methods::UnshieldTransitionMethodsV0;
use crate::state_transition::unshield_transition::UnshieldTransition;
use crate::state_transition::StateTransition;
use crate::ProtocolError;
use platform_version::version::PlatformVersion;

use super::{build_spend_bundle, serialize_authorized_bundle, OrchardProver, SpendableNote};

/// Builds an Unshield state transition (shielded pool -> platform address).
///
/// Spends existing notes and sends part of the value to a transparent platform
/// address. The shielded fee is deducted from the spent notes. Any remaining
/// value is returned to the shielded `change_address`; the change note is
/// encrypted with the sender's External-scope OVK (derived from `fvk`) so the
/// wallet can recover it — including the structured memo — via OVK recovery.
///
/// # Parameters
/// - `spends` - Notes to spend with their Merkle paths
/// - `output_address` - Platform address to receive the unshielded funds
/// - `unshield_amount` - Amount to unshield to the platform address
/// - `change_address` - Orchard address for change output
/// - `fvk` - Full viewing key for spend authorization
/// - `ask` - Spend authorizing key for RedPallas signatures
/// - `anchor` - Sinsemilla root of the note commitment tree (Orchard Anchor)
/// - `prover` - Orchard prover (holds the Halo 2 proving key)
/// - `memo` - 36-byte structured memo for the change output (4-byte type tag + 32-byte payload)
/// - `platform_version` - Protocol version
///
/// The fee is not a parameter: consensus always charges exactly
/// `compute_shielded_unshield_fee` (the base shielded minimum fee PLUS the flat storage cost of the
/// single `AddBalanceToAddress` write this transition performs crediting the net to the output
/// address) and ignores any surplus. Returns the built transition together with the fee (in
/// credits) that was applied.
#[allow(clippy::too_many_arguments)]
pub fn build_unshield_transition<P: OrchardProver>(
    spends: Vec<SpendableNote>,
    output_address: PlatformAddress,
    unshield_amount: u64,
    change_address: &OrchardAddress,
    fvk: &FullViewingKey,
    ask: &SpendAuthorizingKey,
    anchor: Anchor,
    prover: &P,
    memo: [u8; 36],
    platform_version: &PlatformVersion,
) -> Result<(StateTransition, Credits), ProtocolError> {
    if unshield_amount > i64::MAX as u64 {
        return Err(ProtocolError::ShieldedBuildError(format!(
            "unshield amount {} exceeds maximum allowed value {}",
            unshield_amount,
            i64::MAX as u64
        )));
    }

    let total_spent: u64 = spends.iter().map(|s| s.note.value().inner()).sum();

    // Orchard's BundleType::DEFAULT pads every bundle to a 2-action minimum
    // (MIN_ACTIONS), so even a single-spend unshield is serialized and proven with 2
    // actions. Price the fee against that same floor (matching shielded_transfer);
    // otherwise consensus recomputes min_fee from the on-wire actions.len() == 2 and
    // rejects an honest single-spend unshield with InsufficientShieldedFeeError.
    let num_actions = spends.len().max(2);
    // The fee is fixed at the unshield minimum: consensus always carves exactly
    // `compute_shielded_unshield_fee` from the pool — the base shielded minimum fee PLUS the flat
    // storage cost of the single `AddBalanceToAddress` write this transition performs — and the net
    // (`unshield_amount`) is credited to the output address.
    let fee = compute_shielded_unshield_fee(num_actions, platform_version)?;

    let required = unshield_amount.checked_add(fee).ok_or_else(|| {
        ProtocolError::ShieldedBuildError("fee + unshield_amount overflows u64".to_string())
    })?;
    if required > total_spent {
        return Err(ProtocolError::ShieldedBuildError(format!(
            "unshield amount {} + fee {} = {} exceeds total spendable value {}",
            unshield_amount, fee, required, total_spent
        )));
    }

    let change_amount = total_spent - required;

    // Bind the transparent fields (output_address, unshielding_amount == required) into the
    // Orchard sighash. Shared with the consensus verifier in shielded_proof.rs so the signed
    // and verified bytes cannot diverge.
    let extra_sighash_data = crate::shielded::unshield_extra_sighash_data(
        &output_address.to_bytes(),
        required,
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

    let state_transition = UnshieldTransition::try_from_bundle(
        output_address,
        sb.actions,
        sb.value_balance as u64,
        sb.anchor,
        sb.proof,
        sb.binding_signature,
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
    fn test_unshield_insufficient_funds() {
        let platform_version = PlatformVersion::latest();
        let change_address = test_orchard_address();
        let output_address = PlatformAddress::P2pkh([1u8; 20]);

        let note = test_spendable_note(100);
        let spends = vec![note];

        let sk = grovedb_commitment_tree::SpendingKey::from_bytes([42u8; 32])
            .expect("valid spending key bytes");
        let fvk = FullViewingKey::from(&sk);
        let ask = SpendAuthorizingKey::from(&sk);

        let result = build_unshield_transition(
            spends,
            output_address,
            1_000_000,
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
    // Extra coverage — bounds / overflow / empty-spends branches
    // --------------------------------------------------------------

    #[test]
    fn test_unshield_amount_exceeds_i64_max_errors() {
        let platform_version = PlatformVersion::latest();
        let change_address = test_orchard_address();
        let output_address = PlatformAddress::P2pkh([1u8; 20]);

        let note = test_spendable_note(u64::MAX);
        let spends = vec![note];

        let sk = grovedb_commitment_tree::SpendingKey::from_bytes([42u8; 32]).expect("valid sk");
        let fvk = FullViewingKey::from(&sk);
        let ask = SpendAuthorizingKey::from(&sk);

        let result = build_unshield_transition(
            spends,
            output_address,
            (i64::MAX as u64) + 1, // overflow the i64 cap
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
    fn test_unshield_amount_exceeds_spendable_with_default_fee() {
        // unshield_amount + default_min_fee > total_spent should surface
        // the "exceeds total spendable value" branch.
        let platform_version = PlatformVersion::latest();
        let change_address = test_orchard_address();
        let output_address = PlatformAddress::P2pkh([1u8; 20]);

        let note = test_spendable_note(5_000);
        let spends = vec![note];

        let sk = grovedb_commitment_tree::SpendingKey::from_bytes([42u8; 32]).expect("valid sk");
        let fvk = FullViewingKey::from(&sk);
        let ask = SpendAuthorizingKey::from(&sk);

        let result = build_unshield_transition(
            spends,
            output_address,
            6_000, // more than the note's 5_000
            &change_address,
            &fvk,
            &ask,
            Anchor::empty_tree(),
            &TestProver,
            [0u8; 36],
            platform_version,
        );
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("exceeds total spendable value"),
            "unexpected error: {}",
            err
        );
    }

    #[test]
    fn test_unshield_zero_spends_errors() {
        let platform_version = PlatformVersion::latest();
        let change_address = test_orchard_address();
        let output_address = PlatformAddress::P2pkh([1u8; 20]);

        let sk = grovedb_commitment_tree::SpendingKey::from_bytes([42u8; 32]).expect("valid sk");
        let fvk = FullViewingKey::from(&sk);
        let ask = SpendAuthorizingKey::from(&sk);

        let result = build_unshield_transition(
            vec![],
            output_address,
            1,
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

    #[test]
    fn test_unshield_fee_default_sufficient_value_reaches_add_spend() {
        // When fee=None and total_spent exactly covers (amount + min_fee),
        // we bypass all amount checks and hit the downstream add_spend
        // AnchorMismatch. This exercises the default-fee branch.
        let platform_version = PlatformVersion::latest();
        let change_address = test_orchard_address();
        let output_address = PlatformAddress::P2pkh([1u8; 20]);

        let min_fee = crate::shielded::compute_shielded_unshield_fee(2, platform_version)
            .expect("fee computation should not overflow");
        let unshield_amount = 42u64;
        let note = test_spendable_note(unshield_amount + min_fee);
        let spends = vec![note];

        let sk = grovedb_commitment_tree::SpendingKey::from_bytes([42u8; 32]).expect("valid sk");
        let fvk = FullViewingKey::from(&sk);
        let ask = SpendAuthorizingKey::from(&sk);

        let result = build_unshield_transition(
            spends,
            output_address,
            unshield_amount,
            &change_address,
            &fvk,
            &ask,
            Anchor::empty_tree(),
            &TestProver,
            [0u8; 36],
            platform_version,
        );
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("failed to add spend")
                || err_msg.contains("anchor")
                || err_msg.contains("AnchorMismatch"),
            "expected downstream add_spend error, got: {}",
            err_msg
        );
    }
}
