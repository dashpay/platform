use grovedb_commitment_tree::{Anchor, FullViewingKey, SpendAuthorizingKey};

use crate::address_funds::OrchardAddress;
use crate::fee::Credits;
use crate::shielded::{
    compute_private_document_create_from_shielded_pool_fee,
    private_document_create_from_shielded_pool_extra_sighash_data,
};
use crate::state_transition::private_document_create_from_shielded_pool_transition::methods::PrivateDocumentCreateFromShieldedPoolTransitionMethodsV0;
use crate::state_transition::private_document_create_from_shielded_pool_transition::PrivateDocumentCreateFromShieldedPoolTransition;
use crate::state_transition::StateTransition;
use crate::ProtocolError;
use platform_value::{BinaryData, Identifier};
use platform_version::version::PlatformVersion;

use super::{build_spend_bundle, serialize_authorized_bundle, OrchardProver, SpendableNote};

/// Build a `PrivateDocumentCreateFromShieldedPool` transition (protocol version 15): append
/// `entries` to the private document type `document_type_name` of `data_contract_id`, paying the
/// fee from `spends` and sending the change back to `change_address`.
///
/// The fee is not a parameter: the bundle's value balance is the fee, and consensus pins it to
/// exactly `compute_private_document_create_from_shielded_pool_fee` for the bundle's action
/// count and the entries' number and size, so there is nothing for the caller to choose. The
/// fee is priced at the size of the first entry; every entry must take the type's `entrySize`,
/// which consensus checks. The contract, the type name and every entry are bound into the
/// bundle's sighash. Returns the built transition together with the fee it pays.
#[allow(clippy::too_many_arguments)]
pub fn build_private_document_create_from_shielded_pool_transition<P: OrchardProver>(
    spends: Vec<SpendableNote>,
    data_contract_id: Identifier,
    document_type_name: String,
    entries: Vec<BinaryData>,
    change_address: &OrchardAddress,
    fvk: &FullViewingKey,
    ask: &SpendAuthorizingKey,
    anchor: Anchor,
    prover: &P,
    platform_version: &PlatformVersion,
) -> Result<(StateTransition, Credits), ProtocolError> {
    let entry_size = entries.first().map(BinaryData::len).ok_or_else(|| {
        ProtocolError::ShieldedBuildError("a private document create needs an entry".to_string())
    })?;
    let entry_size = u32::try_from(entry_size).map_err(|_| {
        ProtocolError::ShieldedBuildError(format!(
            "a private document entry of {entry_size} bytes is too large"
        ))
    })?;

    let total_spent = spends
        .iter()
        .try_fold(0u64, |total, spend| {
            total.checked_add(spend.note.value().inner())
        })
        .ok_or_else(|| {
            ProtocolError::ShieldedBuildError("the spent notes' total overflows u64".to_string())
        })?;

    // The bundle holds the spends and one change output, padded to at least two actions.
    let num_actions = spends.len().max(2);
    let fee = compute_private_document_create_from_shielded_pool_fee(
        num_actions,
        entries.len(),
        entry_size,
        platform_version,
    )?;
    if fee > total_spent {
        return Err(ProtocolError::ShieldedBuildError(format!(
            "private document create fee {fee} exceeds total spendable value {total_spent}"
        )));
    }
    let change_amount = total_spent - fee;

    let extra_sighash_data = private_document_create_from_shielded_pool_extra_sighash_data(
        &data_contract_id.to_buffer(),
        &document_type_name,
        &entries,
        platform_version,
    )?;

    let bundle = build_spend_bundle(
        spends,
        change_address,
        change_amount,
        [0u8; 36],
        fvk,
        ask,
        anchor,
        prover,
        &extra_sighash_data,
    )?;

    let sb = serialize_authorized_bundle(&bundle);

    let state_transition = PrivateDocumentCreateFromShieldedPoolTransition::try_from_bundle(
        data_contract_id,
        document_type_name,
        entries,
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

    fn keys() -> (FullViewingKey, SpendAuthorizingKey) {
        let sk = grovedb_commitment_tree::SpendingKey::from_bytes([42u8; 32])
            .expect("valid spending key bytes");
        (FullViewingKey::from(&sk), SpendAuthorizingKey::from(&sk))
    }

    #[test]
    fn should_refuse_to_build_without_an_entry() {
        let (fvk, ask) = keys();
        let err = build_private_document_create_from_shielded_pool_transition(
            vec![test_spendable_note(1_000_000_000)],
            Identifier::from([1u8; 32]),
            "privateContactRequest".to_string(),
            vec![],
            &test_orchard_address(),
            &fvk,
            &ask,
            Anchor::empty_tree(),
            &TestProver,
            PlatformVersion::latest(),
        )
        .expect_err("an empty entry list has no size to price");
        assert!(err.to_string().contains("needs an entry"), "{err}");
    }

    #[test]
    fn should_refuse_to_build_when_the_notes_do_not_cover_the_fee() {
        let (fvk, ask) = keys();
        let err = build_private_document_create_from_shielded_pool_transition(
            vec![test_spendable_note(100)],
            Identifier::from([1u8; 32]),
            "privateContactRequest".to_string(),
            vec![BinaryData::new(vec![7u8; 585])],
            &test_orchard_address(),
            &fvk,
            &ask,
            Anchor::empty_tree(),
            &TestProver,
            PlatformVersion::latest(),
        )
        .expect_err("100 credits do not cover the fee");
        assert!(
            err.to_string().contains("exceeds total spendable value"),
            "{err}"
        );
    }
}
