use grovedb_commitment_tree::{Anchor, FullViewingKey, SpendAuthorizingKey};

use crate::address_funds::OrchardAddress;
use crate::fee::Credits;
use crate::shielded::compute_shielded_identity_top_up_fee;
use crate::state_transition::identity_top_up_from_shielded_pool_transition::methods::IdentityTopUpFromShieldedPoolTransitionMethodsV0;
use crate::state_transition::identity_top_up_from_shielded_pool_transition::IdentityTopUpFromShieldedPoolTransition;
use crate::state_transition::StateTransition;
use crate::ProtocolError;
use platform_value::Identifier;
use platform_version::version::PlatformVersion;

use super::{build_spend_bundle, serialize_authorized_bundle, OrchardProver, SpendableNote};

/// Build an `IdentityTopUpFromShieldedPool` transition: spend `spends` so that exactly
/// `top_up_amount + fee` leaves the pool, sending change back to `change_address`.
/// The identity receives `top_up_amount`; the flat fee is carved from the value
/// balance exactly as `Unshield` does. Returns the transition and the fee used.
#[allow(clippy::too_many_arguments)]
pub fn build_identity_top_up_from_shielded_pool_transition<P: OrchardProver>(
    spends: Vec<SpendableNote>,
    identity_id: Identifier,
    top_up_amount: u64,
    change_address: &OrchardAddress,
    fvk: &FullViewingKey,
    ask: &SpendAuthorizingKey,
    anchor: Anchor,
    prover: &P,
    memo: [u8; 36],
    platform_version: &PlatformVersion,
) -> Result<(StateTransition, Credits), ProtocolError> {
    if top_up_amount > i64::MAX as u64 {
        return Err(ProtocolError::ShieldedBuildError(format!(
            "top up amount {} exceeds maximum allowed value {}",
            top_up_amount,
            i64::MAX as u64
        )));
    }

    let total_spent: u64 = spends.iter().map(|s| s.note.value().inner()).sum();

    let num_actions = spends.len().max(2);
    let fee = compute_shielded_identity_top_up_fee(num_actions, platform_version)?;

    let required = top_up_amount.checked_add(fee).ok_or_else(|| {
        ProtocolError::ShieldedBuildError("fee + top_up_amount overflows u64".to_string())
    })?;
    if required > total_spent {
        return Err(ProtocolError::ShieldedBuildError(format!(
            "top up amount {} + fee {} = {} exceeds total spendable value {}",
            top_up_amount, fee, required, total_spent
        )));
    }

    let change_amount = total_spent - required;

    let extra_sighash_data = crate::shielded::identity_top_up_from_shielded_extra_sighash_data(
        &identity_id.to_buffer(),
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

    let state_transition = IdentityTopUpFromShieldedPoolTransition::try_from_bundle(
        identity_id,
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
    fn test_identity_top_up_insufficient_funds() {
        let platform_version = PlatformVersion::latest();
        let change_address = test_orchard_address();
        let spends = vec![test_spendable_note(100)];
        let sk = grovedb_commitment_tree::SpendingKey::from_bytes([42u8; 32])
            .expect("valid spending key bytes");
        let fvk = FullViewingKey::from(&sk);
        let ask = SpendAuthorizingKey::from(&sk);

        let result = build_identity_top_up_from_shielded_pool_transition(
            spends,
            Identifier::from([1u8; 32]),
            1_000_000,
            &change_address,
            &fvk,
            &ask,
            Anchor::empty_tree(),
            &TestProver,
            [0u8; 36],
            platform_version,
        );
        let err = result
            .expect_err("must fail on insufficient funds")
            .to_string();
        assert!(
            err.contains("exceeds total spendable value"),
            "unexpected error: {err}"
        );
    }
}
