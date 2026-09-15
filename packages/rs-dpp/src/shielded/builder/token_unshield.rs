use grovedb_commitment_tree::{Anchor, FullViewingKey, SpendAuthorizingKey};

use crate::address_funds::OrchardAddress;
use crate::balances::credits::TokenAmount;
use crate::identity::signer::Signer;
use crate::identity::IdentityPublicKey;
use crate::prelude::{Identifier, IdentityNonce, UserFeeIncrease};
use crate::shielded::{token_unshield_extra_sighash_data, OrchardBundleParams};
use crate::state_transition::batch_transition::methods::v1::DocumentsBatchTransitionMethodsV1;
use crate::state_transition::batch_transition::BatchTransition;
use crate::state_transition::StateTransition;
use crate::ProtocolError;
use platform_version::version::PlatformVersion;

use super::{build_spend_bundle, serialize_authorized_bundle, OrchardProver, SpendableNote};

/// Builds a `TokenUnshield` batch transition: spends `spends` from the token's shielded pool,
/// credits `amount` tokens to `recipient_id`, returns the remainder to `change_address` as a
/// new note, and wraps the bundle in a batch transition signed by `owner_id`.
///
/// Tokens cannot pay fees, so unlike the credit pool's `Unshield` nothing is carved from the
/// value balance: `value_balance == amount` exactly, and the signing identity pays the fee in
/// credits. The token id, owner id, recipient id and amount are bound into the Orchard sighash
/// so the bundle cannot be replayed against a different token, recipient or amount.
#[allow(clippy::too_many_arguments)]
pub async fn build_token_unshield_transition<S: Signer<IdentityPublicKey>, P: OrchardProver>(
    token_id: Identifier,
    owner_id: Identifier,
    data_contract_id: Identifier,
    token_contract_position: u16,
    spends: Vec<SpendableNote>,
    recipient_id: Identifier,
    amount: TokenAmount,
    change_address: &OrchardAddress,
    fvk: &FullViewingKey,
    ask: &SpendAuthorizingKey,
    anchor: Anchor,
    memo: [u8; 36],
    identity_public_key: &IdentityPublicKey,
    identity_contract_nonce: IdentityNonce,
    user_fee_increase: UserFeeIncrease,
    signer: &S,
    prover: &P,
    platform_version: &PlatformVersion,
) -> Result<StateTransition, ProtocolError> {
    if amount == 0 {
        return Err(ProtocolError::ShieldedBuildError(
            "token unshield amount must be greater than zero".to_string(),
        ));
    }
    if amount > i64::MAX as u64 {
        return Err(ProtocolError::ShieldedBuildError(format!(
            "token unshield amount {} exceeds maximum allowed value {}",
            amount,
            i64::MAX as u64
        )));
    }

    let total_spent: u64 = spends.iter().map(|s| s.note.value().inner()).sum();
    if amount > total_spent {
        return Err(ProtocolError::ShieldedBuildError(format!(
            "token unshield amount {} exceeds total spendable value {}",
            amount, total_spent
        )));
    }
    let change_amount = total_spent - amount;

    let extra_sighash_data = token_unshield_extra_sighash_data(
        &token_id.to_buffer(),
        &owner_id.to_buffer(),
        &recipient_id.to_buffer(),
        amount,
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

    if sb.value_balance != amount as i64 {
        return Err(ProtocolError::ShieldedBuildError(format!(
            "token unshield bundle value balance {} does not equal the amount {}",
            sb.value_balance, amount
        )));
    }

    BatchTransition::new_token_unshield_transition(
        token_id,
        owner_id,
        data_contract_id,
        token_contract_position,
        amount,
        recipient_id,
        OrchardBundleParams {
            actions: sb.actions,
            anchor: sb.anchor,
            proof: sb.proof,
            binding_signature: sb.binding_signature,
        },
        identity_public_key,
        identity_contract_nonce,
        user_fee_increase,
        signer,
        platform_version,
        None,
    )
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shielded::builder::test_helpers::{
        test_identity_key, test_orchard_address, test_spendable_note, DummyIdentitySigner,
        TestProver,
    };
    use grovedb_commitment_tree::SpendingKey;

    fn keys() -> (FullViewingKey, SpendAuthorizingKey) {
        let sk = SpendingKey::from_bytes([42u8; 32]).expect("valid spending key bytes");
        (FullViewingKey::from(&sk), SpendAuthorizingKey::from(&sk))
    }

    #[tokio::test]
    async fn rejects_amount_above_spendable_value() {
        let (fvk, ask) = keys();
        let key = test_identity_key();
        let err = build_token_unshield_transition(
            Identifier::from([1u8; 32]),
            Identifier::from([2u8; 32]),
            Identifier::from([3u8; 32]),
            0,
            vec![test_spendable_note(100)],
            Identifier::from([4u8; 32]),
            1_000,
            &test_orchard_address(),
            &fvk,
            &ask,
            Anchor::empty_tree(),
            [0u8; 36],
            &key,
            1,
            0,
            &DummyIdentitySigner,
            &TestProver,
            PlatformVersion::latest(),
        )
        .await
        .expect_err("overspend must be rejected")
        .to_string();
        assert!(
            err.contains("exceeds total spendable value"),
            "unexpected error: {err}"
        );
    }

    #[tokio::test]
    async fn rejects_zero_amount() {
        let (fvk, ask) = keys();
        let key = test_identity_key();
        let err = build_token_unshield_transition(
            Identifier::from([1u8; 32]),
            Identifier::from([2u8; 32]),
            Identifier::from([3u8; 32]),
            0,
            vec![test_spendable_note(100)],
            Identifier::from([4u8; 32]),
            0,
            &test_orchard_address(),
            &fvk,
            &ask,
            Anchor::empty_tree(),
            [0u8; 36],
            &key,
            1,
            0,
            &DummyIdentitySigner,
            &TestProver,
            PlatformVersion::latest(),
        )
        .await
        .expect_err("zero amount must be rejected")
        .to_string();
        assert!(err.contains("greater than zero"), "unexpected error: {err}");
    }
}
