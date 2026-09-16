use grovedb_commitment_tree::OutgoingViewingKey;

use crate::address_funds::OrchardAddress;
use crate::balances::credits::TokenAmount;
use crate::data_contract::associated_token::token_distribution_key::TokenDistributionType;
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

/// Builds a `TokenClaimToPool` batch transition: proves an outputs-only Orchard bundle creating
/// `amount` of the token as a note for `recipient` inside the token's shielded pool, then wraps
/// it in a batch transition signed by `owner_id`, the claimant, who pays the fee in credits.
///
/// `amount` must be exactly what consensus will pay out: the next pre-programmed release, or for
/// a perpetual distribution the rewards from the last claim up to `claim_up_to` (a cycle-aligned
/// moment in the distribution's own unit, not after the current interval). A mismatch is a paid
/// failure, so the client should compute the amount from the same state it read the moment from.
#[allow(clippy::too_many_arguments)]
pub async fn build_token_claim_to_pool_transition<
    S: Signer<IdentityPublicKey>,
    P: OrchardProver,
>(
    token_id: Identifier,
    owner_id: Identifier,
    data_contract_id: Identifier,
    token_contract_position: u16,
    recipient: &OrchardAddress,
    amount: TokenAmount,
    distribution_type: TokenDistributionType,
    claim_up_to: Option<u64>,
    memo: [u8; 36],
    sender_ovk: Option<OutgoingViewingKey>,
    public_note: Option<String>,
    identity_public_key: &IdentityPublicKey,
    identity_contract_nonce: IdentityNonce,
    user_fee_increase: UserFeeIncrease,
    signer: &S,
    prover: &P,
    platform_version: &PlatformVersion,
) -> Result<StateTransition, ProtocolError> {
    if amount == 0 {
        return Err(ProtocolError::ShieldedBuildError(
            "token claim to pool amount must be greater than zero".to_string(),
        ));
    }
    if amount > i64::MAX as u64 {
        return Err(ProtocolError::ShieldedBuildError(format!(
            "token claim to pool amount {} exceeds maximum allowed value {}",
            amount,
            i64::MAX as u64
        )));
    }
    if distribution_type == TokenDistributionType::Perpetual && claim_up_to.is_none() {
        return Err(ProtocolError::ShieldedBuildError(
            "a perpetual claim into the pool must name the moment it claims up to".to_string(),
        ));
    }

    let bundle = build_output_only_bundle(recipient, amount, memo, sender_ovk, 0, prover)?;
    let sb = serialize_authorized_bundle(&bundle);

    if sb.value_balance != -(amount as i64) {
        return Err(ProtocolError::ShieldedBuildError(format!(
            "token claim to pool amount bundle value balance {} does not equal -{}",
            sb.value_balance, amount
        )));
    }

    BatchTransition::new_token_claim_to_pool_transition(
        token_id,
        owner_id,
        data_contract_id,
        token_contract_position,
        distribution_type,
        claim_up_to,
        OrchardBundleParams {
            actions: sb.actions,
            anchor: sb.anchor,
            proof: sb.proof,
            binding_signature: sb.binding_signature,
        },
        public_note,
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

    #[tokio::test]
    async fn perpetual_claim_needs_a_moment() {
        let key = test_identity_key();
        let err = build_token_claim_to_pool_transition(
            Identifier::from([1u8; 32]),
            Identifier::from([2u8; 32]),
            Identifier::from([3u8; 32]),
            0,
            &test_orchard_address(),
            100,
            TokenDistributionType::Perpetual,
            None,
            [0u8; 36],
            None,
            None,
            &key,
            1,
            0,
            &DummyIdentitySigner,
            &TestProver,
            PlatformVersion::latest(),
        )
        .await
        .expect_err("perpetual claim without a moment must be rejected")
        .to_string();
        assert!(err.contains("claims up to"), "unexpected error: {err}");
    }
}
