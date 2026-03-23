use grovedb_commitment_tree::{Anchor, FullViewingKey, SpendAuthorizingKey};

use crate::address_funds::{OrchardAddress, PlatformAddress};
use crate::fee::Credits;
use crate::shielded::compute_minimum_shielded_fee;
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
/// value is returned to the shielded `change_address`.
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
/// - `fee` - Optional fee override; if `None`, the minimum fee is computed automatically.
///   If `Some`, must be >= the minimum fee.
/// - `platform_version` - Protocol version
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
    fee: Option<Credits>,
    platform_version: &PlatformVersion,
) -> Result<StateTransition, ProtocolError> {
    if unshield_amount > i64::MAX as u64 {
        return Err(ProtocolError::ShieldedBuildError(format!(
            "unshield amount {} exceeds maximum allowed value {}",
            unshield_amount,
            i64::MAX as u64
        )));
    }

    let total_spent: u64 = spends.iter().map(|s| s.note.value().inner()).sum();

    // Conservative action count: at least (spends, 1) since we have a change output.
    let num_actions = spends.len().max(1);
    let min_fee = compute_minimum_shielded_fee(num_actions, platform_version);
    let effective_fee = match fee {
        Some(f) if f < min_fee => {
            return Err(ProtocolError::ShieldedBuildError(format!(
                "fee {} is below minimum required fee {}",
                f, min_fee
            )));
        }
        Some(f) if f > min_fee.saturating_mul(1000) => {
            return Err(ProtocolError::ShieldedBuildError(format!(
                "fee {} exceeds 1000x the minimum fee {}",
                f, min_fee
            )));
        }
        Some(f) => f,
        None => min_fee,
    };

    let required = unshield_amount.checked_add(effective_fee).ok_or_else(|| {
        ProtocolError::ShieldedBuildError("fee + unshield_amount overflows u64".to_string())
    })?;
    if required > total_spent {
        return Err(ProtocolError::ShieldedBuildError(format!(
            "unshield amount {} + fee {} = {} exceeds total spendable value {}",
            unshield_amount, effective_fee, required, total_spent
        )));
    }

    let change_amount = total_spent - required;

    // Unshield extra_data = output_address || value_balance (le bytes)
    // value_balance = unshield_amount + fee, becomes v0.unshielding_amount in the state transition
    // Must match server-side sighash in shielded_proof.rs
    let mut extra_sighash_data = output_address.to_bytes();
    extra_sighash_data.extend_from_slice(&required.to_le_bytes());

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

    UnshieldTransition::try_from_bundle(
        output_address,
        sb.actions,
        sb.value_balance as u64,
        sb.anchor,
        sb.proof,
        sb.binding_signature,
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
    fn test_unshield_fee_below_minimum() {
        let platform_version = PlatformVersion::latest();
        let change_address = test_orchard_address();
        let output_address = PlatformAddress::P2pkh([1u8; 20]);

        let note = test_spendable_note(1_000_000);
        let spends = vec![note];

        let sk = grovedb_commitment_tree::SpendingKey::from_bytes([42u8; 32])
            .expect("valid spending key bytes");
        let fvk = FullViewingKey::from(&sk);
        let ask = SpendAuthorizingKey::from(&sk);

        let result = build_unshield_transition(
            spends,
            output_address,
            100,
            &change_address,
            &fvk,
            &ask,
            Anchor::empty_tree(),
            &TestProver,
            [0u8; 36],
            Some(1), // fee = 1, should be below minimum
            platform_version,
        );

        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("below minimum required fee"),
            "unexpected error: {}",
            err
        );
    }

    #[test]
    fn test_unshield_fee_above_upper_bound() {
        let platform_version = PlatformVersion::latest();
        let change_address = test_orchard_address();
        let output_address = PlatformAddress::P2pkh([1u8; 20]);

        let note = test_spendable_note(u64::MAX);
        let spends = vec![note];

        let sk = grovedb_commitment_tree::SpendingKey::from_bytes([42u8; 32])
            .expect("valid spending key bytes");
        let fvk = FullViewingKey::from(&sk);
        let ask = SpendAuthorizingKey::from(&sk);

        // Compute the minimum fee so we can exceed the 1000x bound
        let num_actions = 1usize;
        let min_fee = crate::shielded::compute_minimum_shielded_fee(num_actions, platform_version);
        let excessive_fee = min_fee.saturating_mul(1000) + 1;

        let result = build_unshield_transition(
            spends,
            output_address,
            100,
            &change_address,
            &fvk,
            &ask,
            Anchor::empty_tree(),
            &TestProver,
            [0u8; 36],
            Some(excessive_fee),
            platform_version,
        );

        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("exceeds 1000x the minimum fee"),
            "unexpected error: {}",
            err
        );
    }

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
