use grovedb_commitment_tree::{
    Anchor, Builder, BundleType, DashMemo, FullViewingKey, NoteValue, PaymentAddress, Scope,
    SpendAuthorizingKey,
};

use crate::address_funds::OrchardAddress;
use crate::balances::credits::TokenAmount;
use crate::data_contract::TokenContractPosition;
use crate::fee::Credits;
use crate::shielded::{
    compute_token_purchase_from_shielded_pool_fee,
    compute_token_shielded_transfer_with_shielded_fee_fee,
    compute_token_unshield_with_shielded_fee_fee, token_pool_fee_bundle_extra_sighash_data,
    token_purchase_from_shielded_pool_extra_sighash_data,
    token_shielded_transfer_with_shielded_fee_extra_sighash_data,
    token_unshield_with_shielded_fee_extra_sighash_data, SerializedAction,
    TOKEN_PURCHASE_FROM_SHIELDED_POOL_TYPE, TOKEN_SHIELDED_TRANSFER_WITH_SHIELDED_FEE_TYPE,
    TOKEN_UNSHIELD_WITH_SHIELDED_FEE_TYPE,
};
use crate::state_transition::token_purchase_from_shielded_pool_transition::methods::TokenPurchaseFromShieldedPoolTransitionMethodsV0;
use crate::state_transition::token_purchase_from_shielded_pool_transition::TokenPurchaseFromShieldedPoolTransition;
use crate::state_transition::token_shielded_transfer_with_shielded_fee_transition::methods::TokenShieldedTransferWithShieldedFeeTransitionMethodsV0;
use crate::state_transition::token_shielded_transfer_with_shielded_fee_transition::TokenShieldedTransferWithShieldedFeeTransition;
use crate::state_transition::token_unshield_with_shielded_fee_transition::methods::TokenUnshieldWithShieldedFeeTransitionMethodsV0;
use crate::state_transition::token_unshield_with_shielded_fee_transition::TokenUnshieldWithShieldedFeeTransition;
use crate::state_transition::StateTransition;
use crate::ProtocolError;
use platform_value::Identifier;
use platform_version::version::PlatformVersion;

use super::{
    build_output_only_bundle, build_spend_bundle, prove_and_sign_bundle,
    serialize_authorized_bundle, OrchardProver, SerializedBundle, SpendableNote,
};

/// The credit pool side of an identity-less token pool transition: the wallet's credit pool
/// notes to spend, where the change goes, and the keys that authorize the spend.
pub struct ShieldedFeePayer<'a> {
    /// Credit pool notes to spend, with their Merkle paths.
    pub spends: Vec<SpendableNote>,
    /// Orchard address for the change note in the credit pool.
    pub change_address: &'a OrchardAddress,
    /// Full viewing key of the credit pool notes.
    pub fvk: &'a FullViewingKey,
    /// Spend authorizing key of the credit pool notes.
    pub ask: &'a SpendAuthorizingKey,
    /// Sinsemilla root of the credit pool's note commitment tree.
    pub anchor: Anchor,
}

/// The wallet's notes in a token's shielded pool and the keys that spend them.
pub struct TokenPoolSpender<'a> {
    /// Token pool notes to spend, with their Merkle paths.
    pub spends: Vec<SpendableNote>,
    /// Orchard address for the change note in the token pool.
    pub change_address: &'a OrchardAddress,
    /// Full viewing key of the token pool notes.
    pub fvk: &'a FullViewingKey,
    /// Spend authorizing key of the token pool notes.
    pub ask: &'a SpendAuthorizingKey,
    /// Sinsemilla root of the token pool's note commitment tree.
    pub anchor: Anchor,
}

fn total_value(spends: &[SpendableNote]) -> Result<u64, ProtocolError> {
    spends
        .iter()
        .try_fold(0u64, |total, spend| {
            total.checked_add(spend.note.value().inner())
        })
        .ok_or_else(|| {
            ProtocolError::ShieldedBuildError("total spendable value overflows u64".to_string())
        })
}

/// Builds the credit pool fee bundle: spends `payer.spends`, carves `credits_leaving` (the fee,
/// plus the price for a purchase) and returns the remainder as change, bound to the token
/// bundle through `token_actions`.
#[allow(clippy::too_many_arguments)]
fn build_fee_bundle<P: OrchardProver>(
    state_transition_type: u8,
    token_id: &Identifier,
    token_actions: &[SerializedAction],
    payer: ShieldedFeePayer<'_>,
    credits_leaving: Credits,
    expected_actions: usize,
    prover: &P,
    platform_version: &PlatformVersion,
) -> Result<SerializedBundle, ProtocolError> {
    let total_spent = total_value(&payer.spends)?;
    if credits_leaving > total_spent {
        return Err(ProtocolError::ShieldedBuildError(format!(
            "credits leaving the pool {} exceed the total spendable value {}",
            credits_leaving, total_spent
        )));
    }
    let change_amount = total_spent - credits_leaving;
    let extra_sighash_data = token_pool_fee_bundle_extra_sighash_data(
        state_transition_type,
        &token_id.to_buffer(),
        token_actions,
        platform_version,
    )?;
    let bundle = build_spend_bundle(
        payer.spends,
        payer.change_address,
        change_amount,
        [0u8; 36],
        payer.fvk,
        payer.ask,
        payer.anchor,
        prover,
        &extra_sighash_data,
    )?;
    let sb = serialize_authorized_bundle(&bundle);
    if sb.value_balance != credits_leaving as i64 {
        return Err(ProtocolError::ShieldedBuildError(format!(
            "fee bundle value balance {} does not equal the credits leaving the pool {}",
            sb.value_balance, credits_leaving
        )));
    }
    if sb.actions.len() != expected_actions {
        return Err(ProtocolError::ShieldedBuildError(format!(
            "fee bundle has {} actions, the fee was computed for {}",
            sb.actions.len(),
            expected_actions
        )));
    }
    Ok(sb)
}

/// Builds a `TokenShieldedTransferWithShieldedFee`: pays `transfer_amount` of the token to
/// `recipient` inside the token's shielded pool (value balance zero) and the fee out of the
/// credit shielded pool. No identity is involved. Returns the transition and the fee paid.
#[allow(clippy::too_many_arguments)]
pub fn build_token_shielded_transfer_with_shielded_fee_transition<P: OrchardProver>(
    token_id: Identifier,
    data_contract_id: Identifier,
    token_contract_position: TokenContractPosition,
    spender: TokenPoolSpender<'_>,
    recipient: &OrchardAddress,
    transfer_amount: TokenAmount,
    memo: [u8; 36],
    fee_payer: ShieldedFeePayer<'_>,
    prover: &P,
    platform_version: &PlatformVersion,
) -> Result<(StateTransition, Credits), ProtocolError> {
    if transfer_amount == 0 {
        return Err(ProtocolError::ShieldedBuildError(
            "token shielded transfer amount must be greater than zero".to_string(),
        ));
    }
    let total_spent = total_value(&spender.spends)?;
    if transfer_amount > total_spent {
        return Err(ProtocolError::ShieldedBuildError(format!(
            "token shielded transfer amount {} exceeds total spendable value {}",
            transfer_amount, total_spent
        )));
    }
    let change_amount = total_spent - transfer_amount;
    let mut builder = Builder::<DashMemo>::new(BundleType::DEFAULT, spender.anchor);
    for spend in spender.spends {
        builder
            .add_spend(spender.fvk.clone(), spend.note, spend.merkle_path)
            .map_err(|e| {
                ProtocolError::ShieldedBuildError(format!("failed to add spend: {:?}", e))
            })?;
    }
    let sender_ovk = spender.fvk.to_ovk(Scope::External);
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
                PaymentAddress::from(spender.change_address),
                NoteValue::from_raw(change_amount),
                [0u8; 36],
            )
            .map_err(|e| {
                ProtocolError::ShieldedBuildError(format!("failed to add change output: {:?}", e))
            })?;
    }
    let token_extra = token_shielded_transfer_with_shielded_fee_extra_sighash_data(
        &token_id.to_buffer(),
        platform_version,
    )?;
    let token_bundle = prove_and_sign_bundle(
        builder,
        prover,
        std::slice::from_ref(spender.ask),
        &token_extra,
    )?;
    let token_sb = serialize_authorized_bundle(&token_bundle);
    if token_sb.value_balance != 0 {
        return Err(ProtocolError::ShieldedBuildError(format!(
            "token shielded transfer bundle value balance must be zero, got {}",
            token_sb.value_balance
        )));
    }
    let fee_actions = fee_payer.spends.len().max(2);
    let fee = compute_token_shielded_transfer_with_shielded_fee_fee(
        token_sb.actions.len(),
        fee_actions,
        platform_version,
    )?;
    let fee_sb = build_fee_bundle(
        TOKEN_SHIELDED_TRANSFER_WITH_SHIELDED_FEE_TYPE,
        &token_id,
        &token_sb.actions,
        fee_payer,
        fee,
        fee_actions,
        prover,
        platform_version,
    )?;
    let transition = TokenShieldedTransferWithShieldedFeeTransition::try_from_bundles(
        data_contract_id,
        token_contract_position,
        token_id,
        token_sb.actions,
        token_sb.anchor,
        token_sb.proof,
        token_sb.binding_signature,
        fee_sb.actions,
        fee_sb.anchor,
        fee_sb.proof,
        fee_sb.binding_signature,
        fee,
        platform_version,
    )?;
    Ok((transition, fee))
}

/// Builds a `TokenUnshieldWithShieldedFee`: `amount` of the token leaves the token's shielded
/// pool into `recipient_id`'s balance, the fee is paid out of the credit shielded pool. Returns
/// the transition and the fee paid.
#[allow(clippy::too_many_arguments)]
pub fn build_token_unshield_with_shielded_fee_transition<P: OrchardProver>(
    token_id: Identifier,
    data_contract_id: Identifier,
    token_contract_position: TokenContractPosition,
    spender: TokenPoolSpender<'_>,
    recipient_id: Identifier,
    amount: TokenAmount,
    memo: [u8; 36],
    fee_payer: ShieldedFeePayer<'_>,
    prover: &P,
    platform_version: &PlatformVersion,
) -> Result<(StateTransition, Credits), ProtocolError> {
    if amount == 0 {
        return Err(ProtocolError::ShieldedBuildError(
            "token unshield amount must be greater than zero".to_string(),
        ));
    }
    let total_spent = total_value(&spender.spends)?;
    if amount > total_spent {
        return Err(ProtocolError::ShieldedBuildError(format!(
            "token unshield amount {} exceeds total spendable value {}",
            amount, total_spent
        )));
    }
    let change_amount = total_spent - amount;
    let token_extra = token_unshield_with_shielded_fee_extra_sighash_data(
        &token_id.to_buffer(),
        &recipient_id.to_buffer(),
        amount,
        platform_version,
    )?;
    let token_bundle = build_spend_bundle(
        spender.spends,
        spender.change_address,
        change_amount,
        memo,
        spender.fvk,
        spender.ask,
        spender.anchor,
        prover,
        &token_extra,
    )?;
    let token_sb = serialize_authorized_bundle(&token_bundle);
    if token_sb.value_balance != amount as i64 {
        return Err(ProtocolError::ShieldedBuildError(format!(
            "token unshield bundle value balance {} does not equal the amount {}",
            token_sb.value_balance, amount
        )));
    }
    let fee_actions = fee_payer.spends.len().max(2);
    let fee = compute_token_unshield_with_shielded_fee_fee(
        token_sb.actions.len(),
        fee_actions,
        platform_version,
    )?;
    let fee_sb = build_fee_bundle(
        TOKEN_UNSHIELD_WITH_SHIELDED_FEE_TYPE,
        &token_id,
        &token_sb.actions,
        fee_payer,
        fee,
        fee_actions,
        prover,
        platform_version,
    )?;
    let transition = TokenUnshieldWithShieldedFeeTransition::try_from_bundles(
        data_contract_id,
        token_contract_position,
        token_id,
        recipient_id,
        amount,
        token_sb.actions,
        token_sb.anchor,
        token_sb.proof,
        token_sb.binding_signature,
        fee_sb.actions,
        fee_sb.anchor,
        fee_sb.proof,
        fee_sb.binding_signature,
        fee,
        platform_version,
    )?;
    Ok((transition, fee))
}

/// Builds a `TokenPurchaseFromShieldedPool`: `token_count` tokens are bought at
/// `total_agreed_price` credits (which must match the token's direct purchase price) paid out
/// of the credit shielded pool together with the fee, and minted into a note for `recipient`
/// in the token's shielded pool. Returns the transition and the fee paid.
#[allow(clippy::too_many_arguments)]
pub fn build_token_purchase_from_shielded_pool_transition<P: OrchardProver>(
    token_id: Identifier,
    data_contract_id: Identifier,
    token_contract_position: TokenContractPosition,
    recipient: &OrchardAddress,
    recipient_fvk: &FullViewingKey,
    token_count: TokenAmount,
    total_agreed_price: Credits,
    memo: [u8; 36],
    fee_payer: ShieldedFeePayer<'_>,
    prover: &P,
    platform_version: &PlatformVersion,
) -> Result<(StateTransition, Credits), ProtocolError> {
    if token_count == 0 {
        return Err(ProtocolError::ShieldedBuildError(
            "token purchase count must be greater than zero".to_string(),
        ));
    }
    let token_bundle = build_output_only_bundle(
        recipient,
        token_count,
        memo,
        Some(recipient_fvk.to_ovk(Scope::External)),
        0,
        prover,
    )?;
    // An outputs-only bundle carries no spend-auth signatures; its binding signature must
    // still commit to the token id, count and price, so re-sign it over that sighash.
    let token_sb = {
        let mut builder = Builder::<DashMemo>::new(
            BundleType::Transactional {
                flags: grovedb_commitment_tree::Flags::SPENDS_DISABLED,
                bundle_required: false,
            },
            Anchor::empty_tree(),
        );
        builder
            .add_output(
                Some(recipient_fvk.to_ovk(Scope::External)),
                PaymentAddress::from(recipient),
                NoteValue::from_raw(token_count),
                memo,
            )
            .map_err(|e| {
                ProtocolError::ShieldedBuildError(format!("failed to add output: {:?}", e))
            })?;
        let token_extra = token_purchase_from_shielded_pool_extra_sighash_data(
            &token_id.to_buffer(),
            token_count,
            total_agreed_price,
            platform_version,
        )?;
        drop(token_bundle);
        serialize_authorized_bundle(&prove_and_sign_bundle(builder, prover, &[], &token_extra)?)
    };
    if token_sb.value_balance != -(token_count as i64) {
        return Err(ProtocolError::ShieldedBuildError(format!(
            "token purchase bundle value balance {} does not equal minus the token count {}",
            token_sb.value_balance, token_count
        )));
    }
    let fee_actions = fee_payer.spends.len().max(2);
    let fee = compute_token_purchase_from_shielded_pool_fee(
        token_sb.actions.len(),
        fee_actions,
        platform_version,
    )?;
    let credits_leaving = total_agreed_price.checked_add(fee).ok_or_else(|| {
        ProtocolError::ShieldedBuildError("price + fee overflows u64".to_string())
    })?;
    let fee_sb = build_fee_bundle(
        TOKEN_PURCHASE_FROM_SHIELDED_POOL_TYPE,
        &token_id,
        &token_sb.actions,
        fee_payer,
        credits_leaving,
        fee_actions,
        prover,
        platform_version,
    )?;
    let transition = TokenPurchaseFromShieldedPoolTransition::try_from_bundles(
        data_contract_id,
        token_contract_position,
        token_id,
        token_count,
        total_agreed_price,
        token_sb.actions,
        token_sb.anchor,
        token_sb.proof,
        token_sb.binding_signature,
        fee_sb.actions,
        fee_sb.anchor,
        fee_sb.proof,
        fee_sb.binding_signature,
        credits_leaving,
        platform_version,
    )?;
    Ok((transition, fee))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shielded::builder::test_helpers::{
        test_orchard_address, test_spendable_note, TestProver,
    };
    use grovedb_commitment_tree::SpendingKey;

    fn keys() -> (FullViewingKey, SpendAuthorizingKey) {
        let sk = SpendingKey::from_bytes([42u8; 32]).expect("valid spending key bytes");
        (FullViewingKey::from(&sk), SpendAuthorizingKey::from(&sk))
    }

    #[test]
    fn rejects_a_transfer_above_the_spendable_token_value() {
        let (fvk, ask) = keys();
        let address = test_orchard_address();
        let err = build_token_shielded_transfer_with_shielded_fee_transition(
            Identifier::from([1u8; 32]),
            Identifier::from([2u8; 32]),
            0,
            TokenPoolSpender {
                spends: vec![test_spendable_note(100)],
                change_address: &address,
                fvk: &fvk,
                ask: &ask,
                anchor: Anchor::empty_tree(),
            },
            &address,
            1_000,
            [0u8; 36],
            ShieldedFeePayer {
                spends: vec![test_spendable_note(1_000_000_000)],
                change_address: &address,
                fvk: &fvk,
                ask: &ask,
                anchor: Anchor::empty_tree(),
            },
            &TestProver,
            PlatformVersion::latest(),
        )
        .expect_err("overspend must be rejected")
        .to_string();
        assert!(
            err.contains("exceeds total spendable value"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn rejects_a_zero_unshield_amount() {
        let (fvk, ask) = keys();
        let address = test_orchard_address();
        let err = build_token_unshield_with_shielded_fee_transition(
            Identifier::from([1u8; 32]),
            Identifier::from([2u8; 32]),
            0,
            TokenPoolSpender {
                spends: vec![test_spendable_note(100)],
                change_address: &address,
                fvk: &fvk,
                ask: &ask,
                anchor: Anchor::empty_tree(),
            },
            Identifier::from([3u8; 32]),
            0,
            [0u8; 36],
            ShieldedFeePayer {
                spends: vec![test_spendable_note(1_000_000_000)],
                change_address: &address,
                fvk: &fvk,
                ask: &ask,
                anchor: Anchor::empty_tree(),
            },
            &TestProver,
            PlatformVersion::latest(),
        )
        .expect_err("zero amount must be rejected")
        .to_string();
        assert!(err.contains("greater than zero"), "unexpected error: {err}");
    }

    #[test]
    fn rejects_a_purchase_the_fee_notes_cannot_pay() {
        let (fvk, ask) = keys();
        let address = test_orchard_address();
        let err = build_token_purchase_from_shielded_pool_transition(
            Identifier::from([1u8; 32]),
            Identifier::from([2u8; 32]),
            0,
            &address,
            &fvk,
            5,
            1_000,
            [0u8; 36],
            ShieldedFeePayer {
                spends: vec![test_spendable_note(10)],
                change_address: &address,
                fvk: &fvk,
                ask: &ask,
                anchor: Anchor::empty_tree(),
            },
            &TestProver,
            PlatformVersion::latest(),
        )
        .expect_err("the fee notes cannot cover price plus fee")
        .to_string();
        assert!(
            err.contains("exceed the total spendable value"),
            "unexpected error: {err}"
        );
    }
}
