use grovedb_commitment_tree::{Anchor, FullViewingKey, SpendAuthorizingKey};

use crate::address_funds::OrchardAddress;
use crate::balances::credits::TokenAmount;
use crate::group::GroupStateTransitionInfoStatus;
use crate::identity::signer::Signer;
use crate::identity::IdentityPublicKey;
use crate::prelude::{Identifier, IdentityNonce, UserFeeIncrease};
use crate::shielded::{token_burn_from_pool_extra_sighash_data, OrchardBundleParams};
use crate::state_transition::batch_transition::methods::v1::DocumentsBatchTransitionMethodsV1;
use crate::state_transition::batch_transition::BatchTransition;
use crate::state_transition::StateTransition;
use crate::ProtocolError;
use platform_version::version::PlatformVersion;

use super::{build_spend_bundle, serialize_authorized_bundle, OrchardProver, SpendableNote};

/// Builds a `TokenBurnFromPool` batch transition: spends `spends` inside the token's shielded
/// pool, destroys `amount` and returns the remainder to `change_address`, then wraps the bundle
/// in a batch transition signed by `owner_id`, who must be authorized to burn and pays the fee
/// in credits. The token id, owner id and amount are bound into the Orchard sighash.
///
/// A group action burn is proven once, by the proposer
/// (`GroupStateTransitionInfoProposer`): the group action pins the digest of the actions and
/// the sighash binds the proposer, so every other signer submits the proposer's bundle unchanged
/// through `new_token_burn_from_pool_transition` with `GroupStateTransitionInfoOtherSigner`.
/// Asking this builder for a fresh bundle on behalf of another signer is refused, since consensus
/// would reject it as a modification of the group action.
#[allow(clippy::too_many_arguments)]
pub async fn build_token_burn_from_pool_transition<
    S: Signer<IdentityPublicKey>,
    P: OrchardProver,
>(
    token_id: Identifier,
    owner_id: Identifier,
    data_contract_id: Identifier,
    token_contract_position: u16,
    spends: Vec<SpendableNote>,
    amount: TokenAmount,
    change_address: &OrchardAddress,
    fvk: &FullViewingKey,
    ask: &SpendAuthorizingKey,
    anchor: Anchor,
    memo: [u8; 36],
    public_note: Option<String>,
    using_group_info: Option<GroupStateTransitionInfoStatus>,
    identity_public_key: &IdentityPublicKey,
    identity_contract_nonce: IdentityNonce,
    user_fee_increase: UserFeeIncrease,
    signer: &S,
    prover: &P,
    platform_version: &PlatformVersion,
) -> Result<StateTransition, ProtocolError> {
    if amount == 0 {
        return Err(ProtocolError::ShieldedBuildError(
            "token burn from pool amount must be greater than zero".to_string(),
        ));
    }
    if amount > i64::MAX as u64 {
        return Err(ProtocolError::ShieldedBuildError(format!(
            "token burn from pool amount {} exceeds maximum allowed value {}",
            amount,
            i64::MAX as u64
        )));
    }
    if matches!(
        using_group_info,
        Some(GroupStateTransitionInfoStatus::GroupStateTransitionInfoOtherSigner(_))
    ) {
        return Err(ProtocolError::ShieldedBuildError(
            "a group action burn from pool is proven once by the proposer; another signer submits the proposer's bundle unchanged instead of building its own".to_string(),
        ));
    }

    let total_spent: u64 = spends
        .iter()
        .try_fold(0u64, |total, spend| {
            total.checked_add(spend.note.value().inner())
        })
        .ok_or_else(|| {
            ProtocolError::ShieldedBuildError("total spendable value overflows u64".to_string())
        })?;
    if amount > total_spent {
        return Err(ProtocolError::ShieldedBuildError(format!(
            "token burn from pool amount {} exceeds total spendable value {}",
            amount, total_spent
        )));
    }
    let change_amount = total_spent - amount;

    let extra_sighash_data = token_burn_from_pool_extra_sighash_data(
        &token_id.to_buffer(),
        &owner_id.to_buffer(),
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
            "token burn from pool bundle value balance {} does not equal the amount {}",
            sb.value_balance, amount
        )));
    }

    BatchTransition::new_token_burn_from_pool_transition(
        token_id,
        owner_id,
        data_contract_id,
        token_contract_position,
        amount,
        OrchardBundleParams {
            actions: sb.actions,
            anchor: sb.anchor,
            proof: sb.proof,
            binding_signature: sb.binding_signature,
        },
        public_note,
        using_group_info,
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
    use crate::group::GroupStateTransitionInfo;
    use crate::shielded::builder::test_helpers::{
        test_identity_key, test_orchard_address, test_spendable_note, DummyIdentitySigner,
        TestProver,
    };
    use grovedb_commitment_tree::SpendingKey;

    #[tokio::test]
    async fn rejects_amount_above_spendable_value() {
        let sk = SpendingKey::from_bytes([42u8; 32]).expect("valid spending key bytes");
        let fvk = FullViewingKey::from(&sk);
        let ask = SpendAuthorizingKey::from(&sk);
        let key = test_identity_key();
        let err = build_token_burn_from_pool_transition(
            Identifier::from([1u8; 32]),
            Identifier::from([2u8; 32]),
            Identifier::from([3u8; 32]),
            0,
            vec![test_spendable_note(100)],
            1_000,
            &test_orchard_address(),
            &fvk,
            &ask,
            Anchor::empty_tree(),
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
        .expect_err("overspend must be rejected")
        .to_string();
        assert!(
            err.contains("exceeds total spendable value"),
            "unexpected error: {err}"
        );
    }

    #[tokio::test]
    async fn rejects_a_fresh_bundle_for_another_group_signer() {
        let sk = SpendingKey::from_bytes([42u8; 32]).expect("valid spending key bytes");
        let fvk = FullViewingKey::from(&sk);
        let ask = SpendAuthorizingKey::from(&sk);
        let key = test_identity_key();
        let err = build_token_burn_from_pool_transition(
            Identifier::from([1u8; 32]),
            Identifier::from([2u8; 32]),
            Identifier::from([3u8; 32]),
            0,
            vec![test_spendable_note(1_000)],
            100,
            &test_orchard_address(),
            &fvk,
            &ask,
            Anchor::empty_tree(),
            [0u8; 36],
            None,
            Some(
                GroupStateTransitionInfoStatus::GroupStateTransitionInfoOtherSigner(
                    GroupStateTransitionInfo {
                        group_contract_position: 0,
                        action_id: Identifier::from([4u8; 32]),
                        action_is_proposer: false,
                    },
                ),
            ),
            &key,
            1,
            0,
            &DummyIdentitySigner,
            &TestProver,
            PlatformVersion::latest(),
        )
        .await
        .expect_err("another signer must reuse the proposer's bundle")
        .to_string();
        assert!(
            err.contains("proven once by the proposer"),
            "unexpected error: {err}"
        );
    }
}
