use grovedb_commitment_tree::OutgoingViewingKey;

use crate::address_funds::OrchardAddress;
use crate::balances::credits::TokenAmount;
use crate::fee::Credits;
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

/// Builds a `TokenDirectPurchaseToPool` batch transition: proves an outputs-only Orchard bundle
/// creating `token_count` of the token as a note for `recipient` inside the token's shielded
/// pool, then wraps it in a batch transition signed by `owner_id`, the buyer, who pays at most
/// `total_agreed_price` credits to the contract owner plus the fee.
#[allow(clippy::too_many_arguments)]
pub async fn build_token_direct_purchase_to_pool_transition<
    S: Signer<IdentityPublicKey>,
    P: OrchardProver,
>(
    token_id: Identifier,
    owner_id: Identifier,
    data_contract_id: Identifier,
    token_contract_position: u16,
    recipient: &OrchardAddress,
    token_count: TokenAmount,
    total_agreed_price: Credits,
    memo: [u8; 36],
    sender_ovk: Option<OutgoingViewingKey>,
    identity_public_key: &IdentityPublicKey,
    identity_contract_nonce: IdentityNonce,
    user_fee_increase: UserFeeIncrease,
    signer: &S,
    prover: &P,
    platform_version: &PlatformVersion,
) -> Result<StateTransition, ProtocolError> {
    if token_count == 0 {
        return Err(ProtocolError::ShieldedBuildError(
            "token direct purchase to pool count must be greater than zero".to_string(),
        ));
    }
    if token_count > i64::MAX as u64 {
        return Err(ProtocolError::ShieldedBuildError(format!(
            "token direct purchase to pool count {} exceeds maximum allowed value {}",
            token_count,
            i64::MAX as u64
        )));
    }

    let bundle = build_output_only_bundle(recipient, token_count, memo, sender_ovk, 0, prover)?;
    let sb = serialize_authorized_bundle(&bundle);

    if sb.value_balance != -(token_count as i64) {
        return Err(ProtocolError::ShieldedBuildError(format!(
            "token direct purchase to pool count bundle value balance {} does not equal -{}",
            sb.value_balance, token_count
        )));
    }

    BatchTransition::new_token_direct_purchase_to_pool_transition(
        token_id,
        owner_id,
        data_contract_id,
        token_contract_position,
        token_count,
        total_agreed_price,
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

    #[tokio::test]
    async fn rejects_zero_count() {
        let key = test_identity_key();
        let err = build_token_direct_purchase_to_pool_transition(
            Identifier::from([1u8; 32]),
            Identifier::from([2u8; 32]),
            Identifier::from([3u8; 32]),
            0,
            &test_orchard_address(),
            0,
            1_000,
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
        .expect_err("zero count must be rejected")
        .to_string();
        assert!(err.contains("greater than zero"), "unexpected error: {err}");
    }
}
