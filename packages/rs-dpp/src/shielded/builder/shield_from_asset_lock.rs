use crate::address_funds::OrchardAddress;
use crate::prelude::{AssetLockProof, UserFeeIncrease};
use crate::state_transition::shield_from_asset_lock_transition::methods::ShieldFromAssetLockTransitionMethodsV0;
use crate::state_transition::shield_from_asset_lock_transition::ShieldFromAssetLockTransition;
use crate::state_transition::StateTransition;
use crate::ProtocolError;
use platform_version::version::PlatformVersion;

use super::{build_output_only_bundle, serialize_authorized_bundle, OrchardProver};

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
/// - `user_fee_increase` - Fee multiplier (0 = 100% base fee)
/// - `prover` - Orchard prover (holds the Halo 2 proving key)
/// - `memo` - 36-byte structured memo for the recipient (4-byte type tag + 32-byte payload)
/// - `platform_version` - Protocol version
#[allow(clippy::too_many_arguments)]
pub fn build_shield_from_asset_lock_transition<P: OrchardProver>(
    recipient: &OrchardAddress,
    shield_amount: u64,
    asset_lock_proof: AssetLockProof,
    asset_lock_private_key: &[u8],
    user_fee_increase: UserFeeIncrease,
    prover: &P,
    memo: [u8; 36],
    platform_version: &PlatformVersion,
) -> Result<StateTransition, ProtocolError> {
    let bundle = build_output_only_bundle(recipient, shield_amount, memo, prover)?;
    let sb = serialize_authorized_bundle(&bundle);

    // For output-only bundles, Orchard value_balance is negative (value flowing in).
    // Convert to u64 (absolute amount entering the pool).
    let value_balance = sb
        .value_balance
        .checked_neg()
        .and_then(|v| u64::try_from(v).ok())
        .ok_or_else(|| {
            ProtocolError::Generic(
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
        user_fee_increase,
        platform_version,
    )
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

        let bundle = build_output_only_bundle(&recipient, amount, [0u8; 36], &TestProver)
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
}
