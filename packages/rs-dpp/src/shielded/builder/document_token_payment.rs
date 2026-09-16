use grovedb_commitment_tree::{Anchor, FullViewingKey, SpendAuthorizingKey};

use crate::address_funds::OrchardAddress;
use crate::balances::credits::TokenAmount;
use crate::prelude::Identifier;
use crate::shielded::document_token_payment_extra_sighash_data;
use crate::tokens::token_payment_info::v1::TokenShieldedPayment;
use crate::ProtocolError;
use platform_version::version::PlatformVersion;

use super::{build_spend_bundle, serialize_authorized_bundle, OrchardProver, SpendableNote};

/// Builds the shielded payment of a document action whose token cost is `amount`: spends
/// `spends` from the token's shielded pool, returns the remainder to `change_address` as a
/// new note and binds the token id, the batch owner, the document's contract and id and the
/// amount into the Orchard sighash. Put the result into a `TokenPaymentInfo::V1` on the
/// document transition's base; the identity signing the batch still pays the credit fee.
///
/// `value_balance == amount` exactly; the pool pays the cost, nothing is carved for fees.
#[allow(clippy::too_many_arguments)]
pub fn build_document_shielded_token_payment<P: OrchardProver>(
    token_id: Identifier,
    owner_id: Identifier,
    data_contract_id: Identifier,
    document_id: Identifier,
    spends: Vec<SpendableNote>,
    amount: TokenAmount,
    change_address: &OrchardAddress,
    fvk: &FullViewingKey,
    ask: &SpendAuthorizingKey,
    anchor: Anchor,
    memo: [u8; 36],
    prover: &P,
    platform_version: &PlatformVersion,
) -> Result<TokenShieldedPayment, ProtocolError> {
    if amount == 0 {
        return Err(ProtocolError::ShieldedBuildError(
            "document token payment amount must be greater than zero".to_string(),
        ));
    }
    if amount > i64::MAX as u64 {
        return Err(ProtocolError::ShieldedBuildError(format!(
            "document token payment amount {} exceeds maximum allowed value {}",
            amount,
            i64::MAX as u64
        )));
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
            "document token payment amount {} exceeds total spendable value {}",
            amount, total_spent
        )));
    }
    let change_amount = total_spent - amount;

    let extra_sighash_data = document_token_payment_extra_sighash_data(
        &token_id.to_buffer(),
        &owner_id.to_buffer(),
        &data_contract_id.to_buffer(),
        &document_id.to_buffer(),
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
            "document token payment bundle value balance {} does not equal the amount {}",
            sb.value_balance, amount
        )));
    }

    Ok(TokenShieldedPayment {
        amount,
        actions: sb.actions,
        anchor: sb.anchor,
        proof: sb.proof,
        binding_signature: sb.binding_signature,
    })
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
    fn rejects_amount_above_spendable_value() {
        let (fvk, ask) = keys();
        let err = build_document_shielded_token_payment(
            Identifier::from([1u8; 32]),
            Identifier::from([2u8; 32]),
            Identifier::from([3u8; 32]),
            Identifier::from([4u8; 32]),
            vec![test_spendable_note(100)],
            1_000,
            &test_orchard_address(),
            &fvk,
            &ask,
            Anchor::empty_tree(),
            [0u8; 36],
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
    fn rejects_zero_amount() {
        let (fvk, ask) = keys();
        let err = build_document_shielded_token_payment(
            Identifier::from([1u8; 32]),
            Identifier::from([2u8; 32]),
            Identifier::from([3u8; 32]),
            Identifier::from([4u8; 32]),
            vec![test_spendable_note(100)],
            0,
            &test_orchard_address(),
            &fvk,
            &ask,
            Anchor::empty_tree(),
            [0u8; 36],
            &TestProver,
            PlatformVersion::latest(),
        )
        .expect_err("zero amount must be rejected")
        .to_string();
        assert!(err.contains("greater than zero"), "unexpected error: {err}");
    }
}
