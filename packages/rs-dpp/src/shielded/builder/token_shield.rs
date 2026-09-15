use grovedb_commitment_tree::OutgoingViewingKey;

use crate::address_funds::OrchardAddress;
use crate::balances::credits::TokenAmount;
use crate::identity::signer::Signer;
use crate::identity::IdentityPublicKey;
use crate::prelude::{Identifier, IdentityNonce, UserFeeIncrease};
use crate::shielded::OrchardBundleParams;
use crate::state_transition::batch_transition::methods::v1::DocumentsBatchTransitionMethodsV1;
use crate::state_transition::batch_transition::BatchTransition;
use crate::state_transition::StateTransition;
use crate::ProtocolError;
use platform_version::version::PlatformVersion;

use super::{build_output_only_bundle, serialize_authorized_bundle, OrchardProver};

/// Builds a `TokenShield` batch transition: proves an outputs-only Orchard bundle paying
/// `amount` of the token to `recipient` inside the token's own shielded pool, then wraps it in
/// a batch transition signed by `owner_id`, the identity whose token balance funds the shield.
///
/// The identity pays the fee in credits; `amount` tokens leave its balance for the pool at
/// execution. The bundle has no spends, so it carries no anchor of the pool and no extra
/// sighash data: the identity signature over the whole batch binds it. `sender_ovk` lets the
/// sending wallet recover the note it created (recipient, value, memo) from chain data.
#[allow(clippy::too_many_arguments)]
pub async fn build_token_shield_transition<S: Signer<IdentityPublicKey>, P: OrchardProver>(
    token_id: Identifier,
    owner_id: Identifier,
    data_contract_id: Identifier,
    token_contract_position: u16,
    recipient: &OrchardAddress,
    amount: TokenAmount,
    memo: [u8; 36],
    sender_ovk: Option<OutgoingViewingKey>,
    identity_public_key: &IdentityPublicKey,
    identity_contract_nonce: IdentityNonce,
    user_fee_increase: UserFeeIncrease,
    signer: &S,
    prover: &P,
    platform_version: &PlatformVersion,
) -> Result<StateTransition, ProtocolError> {
    if amount == 0 {
        return Err(ProtocolError::ShieldedBuildError(
            "token shield amount must be greater than zero".to_string(),
        ));
    }

    let bundle = build_output_only_bundle(recipient, amount, memo, sender_ovk, 0, prover)?;
    let sb = serialize_authorized_bundle(&bundle);

    BatchTransition::new_token_shield_transition(
        token_id,
        owner_id,
        data_contract_id,
        token_contract_position,
        sb.value_balance.unsigned_abs(),
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
        test_identity_key, test_orchard_address, DummyIdentitySigner, TestProver,
    };
    use crate::state_transition::batch_transition::accessors::DocumentsBatchTransitionAccessorsV0;
    use crate::state_transition::batch_transition::batched_transition::token_transition::TokenTransition;
    use crate::state_transition::batch_transition::batched_transition::BatchedTransitionRef;
    use crate::state_transition::batch_transition::token_base_transition::token_base_transition_accessors::TokenBaseTransitionAccessors;
    use crate::state_transition::batch_transition::token_base_transition::v0::v0_methods::TokenBaseTransitionV0Methods;
    use crate::state_transition::batch_transition::token_shield_transition::v0::v0_methods::TokenShieldTransitionV0Methods;

    #[tokio::test]
    async fn rejects_zero_amount() {
        let key = test_identity_key();
        let err = build_token_shield_transition(
            Identifier::from([1u8; 32]),
            Identifier::from([2u8; 32]),
            Identifier::from([3u8; 32]),
            0,
            &test_orchard_address(),
            0,
            [0u8; 36],
            None,
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

    #[tokio::test]
    async fn builds_a_signed_batch_carrying_the_proved_bundle() {
        let key = test_identity_key();
        let token_id = Identifier::from([1u8; 32]);
        let owner_id = Identifier::from([2u8; 32]);
        let state_transition = build_token_shield_transition(
            token_id,
            owner_id,
            Identifier::from([3u8; 32]),
            0,
            &test_orchard_address(),
            50_000,
            [0u8; 36],
            None,
            &key,
            7,
            0,
            &DummyIdentitySigner,
            &TestProver,
            PlatformVersion::latest(),
        )
        .await
        .expect("shield transition");

        assert_eq!(state_transition.owner_id(), Some(owner_id));
        let StateTransition::Batch(batch) = state_transition else {
            panic!("expected a batch transition");
        };
        let transitions: Vec<_> = batch.transitions_iter().collect();
        assert_eq!(transitions.len(), 1);
        let BatchedTransitionRef::Token(TokenTransition::Shield(shield)) = transitions[0] else {
            panic!("expected a token shield transition");
        };
        assert_eq!(shield.amount(), 50_000);
        assert_eq!(shield.base().token_id(), token_id);
        assert_eq!(shield.base().identity_contract_nonce(), 7);
        // Orchard pads an outputs-only bundle to its two-action minimum.
        assert_eq!(shield.actions().len(), 2);
        assert!(!shield.proof().is_empty());
    }
}
