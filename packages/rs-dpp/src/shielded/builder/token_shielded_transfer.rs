use grovedb_commitment_tree::{
    Anchor, Builder, BundleType, DashMemo, FullViewingKey, NoteValue, PaymentAddress, Scope,
    SpendAuthorizingKey,
};

use crate::address_funds::OrchardAddress;
use crate::balances::credits::TokenAmount;
use crate::identity::signer::Signer;
use crate::identity::IdentityPublicKey;
use crate::prelude::{Identifier, IdentityNonce, UserFeeIncrease};
use crate::shielded::{token_shielded_transfer_extra_sighash_data, OrchardBundleParams};
use crate::state_transition::batch_transition::methods::v1::DocumentsBatchTransitionMethodsV1;
use crate::state_transition::batch_transition::BatchTransition;
use crate::state_transition::StateTransition;
use crate::ProtocolError;
use platform_version::version::PlatformVersion;

use super::{prove_and_sign_bundle, serialize_authorized_bundle, OrchardProver, SpendableNote};

/// Builds a `TokenShieldedTransfer` batch transition: spends `spends` inside the token's
/// shielded pool, pays `transfer_amount` to `recipient` and any remainder to `change_address`,
/// and wraps the bundle in a batch transition signed by `owner_id`, which pays the fee in
/// credits.
///
/// Nothing leaves the pool, so the bundle's value balance is exactly zero; consensus rejects
/// anything else. Both outputs are encrypted under the sender's external outgoing viewing key so
/// the sending wallet can recover its own send history. The token id and owner id are bound into
/// the Orchard sighash, so the bundle is tied to this token and this signing identity.
#[allow(clippy::too_many_arguments)]
pub async fn build_token_shielded_transfer_transition<
    S: Signer<IdentityPublicKey>,
    P: OrchardProver,
>(
    token_id: Identifier,
    owner_id: Identifier,
    data_contract_id: Identifier,
    token_contract_position: u16,
    spends: Vec<SpendableNote>,
    recipient: &OrchardAddress,
    transfer_amount: TokenAmount,
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
    if transfer_amount == 0 {
        return Err(ProtocolError::ShieldedBuildError(
            "token shielded transfer amount must be greater than zero".to_string(),
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
    if transfer_amount > total_spent {
        return Err(ProtocolError::ShieldedBuildError(format!(
            "token shielded transfer amount {} exceeds total spendable value {}",
            transfer_amount, total_spent
        )));
    }
    let change_amount = total_spent - transfer_amount;

    let mut builder = Builder::<DashMemo>::new(BundleType::DEFAULT, anchor);

    for spend in spends {
        builder
            .add_spend(fvk.clone(), spend.note, spend.merkle_path)
            .map_err(|e| {
                ProtocolError::ShieldedBuildError(format!("failed to add spend: {:?}", e))
            })?;
    }

    let sender_ovk = fvk.to_ovk(Scope::External);

    builder
        .add_output(
            Some(sender_ovk.clone()),
            PaymentAddress::from(recipient),
            NoteValue::from_raw(transfer_amount),
            memo,
        )
        .map_err(|e| ProtocolError::ShieldedBuildError(format!("failed to add output: {:?}", e)))?;

    if change_amount > 0 {
        builder
            .add_output(
                Some(sender_ovk),
                PaymentAddress::from(change_address),
                NoteValue::from_raw(change_amount),
                [0u8; 36],
            )
            .map_err(|e| {
                ProtocolError::ShieldedBuildError(format!("failed to add change output: {:?}", e))
            })?;
    }

    let extra_sighash_data = token_shielded_transfer_extra_sighash_data(
        &token_id.to_buffer(),
        &owner_id.to_buffer(),
        platform_version,
    )?;

    let bundle = prove_and_sign_bundle(
        builder,
        prover,
        std::slice::from_ref(ask),
        &extra_sighash_data,
    )?;
    let sb = serialize_authorized_bundle(&bundle);

    if sb.value_balance != 0 {
        return Err(ProtocolError::ShieldedBuildError(format!(
            "token shielded transfer bundle value balance must be zero, got {}",
            sb.value_balance
        )));
    }

    BatchTransition::new_token_shielded_transfer_transition(
        token_id,
        owner_id,
        data_contract_id,
        token_contract_position,
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

    #[tokio::test]
    async fn rejects_amount_above_spendable_value() {
        let sk = SpendingKey::from_bytes([42u8; 32]).expect("valid spending key bytes");
        let fvk = FullViewingKey::from(&sk);
        let ask = SpendAuthorizingKey::from(&sk);
        let key = test_identity_key();
        let err = build_token_shielded_transfer_transition(
            Identifier::from([1u8; 32]),
            Identifier::from([2u8; 32]),
            Identifier::from([3u8; 32]),
            0,
            vec![test_spendable_note(100)],
            &test_orchard_address(),
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
}
