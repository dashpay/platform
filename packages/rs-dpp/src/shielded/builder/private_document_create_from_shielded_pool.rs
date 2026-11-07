use grovedb_commitment_tree::{Anchor, FullViewingKey, SpendAuthorizingKey};

use crate::address_funds::OrchardAddress;
use crate::fee::Credits;
use crate::serialization::PlatformSerializable;
use crate::shielded::SerializedAction;
use crate::shielded::{
    compute_private_document_create_from_shielded_pool_fee,
    private_document_create_from_shielded_pool_extra_sighash_data,
};
use crate::state_transition::private_document_create_from_shielded_pool_transition::methods::PrivateDocumentCreateFromShieldedPoolTransitionMethodsV0;
use crate::state_transition::private_document_create_from_shielded_pool_transition::v0::validate_private_document_create_entries;
use crate::state_transition::private_document_create_from_shielded_pool_transition::PrivateDocumentCreateFromShieldedPoolTransition;
use crate::state_transition::state_transitions::shielded::common_validation::ENCRYPTED_NOTE_SIZE;
use crate::state_transition::{validate_state_transition_size, StateTransition};
use crate::ProtocolError;
use platform_value::{BinaryData, Identifier};
use platform_version::version::PlatformVersion;

use super::{build_spend_bundle, serialize_authorized_bundle, OrchardProver, SpendableNote};

/// Refuses `state_transition` if its serialized bytes are over the protocol's
/// `max_state_transition_size`, as every node and DAPI would.
fn check_size(
    state_transition: &StateTransition,
    platform_version: &PlatformVersion,
) -> Result<(), ProtocolError> {
    let size = state_transition.serialize_to_bytes()?.len() as u64;
    validate_state_transition_size(size, platform_version)
        .map_err(|error| ProtocolError::ShieldedBuildError(error.to_string()))
}

/// Build a `PrivateDocumentCreateFromShieldedPool` transition (protocol version 15): append
/// `entries` to the private document type `document_type_name` of `data_contract_id`, paying the
/// fee from `spends` and sending the change back to `change_address`.
///
/// The fee is not a parameter: the bundle's value balance is the fee, and consensus pins it to
/// exactly `compute_private_document_create_from_shielded_pool_fee` for the bundle's action
/// count and the entries' number and size, so there is nothing for the caller to choose. The
/// entries must pass the checks basic structure makes (from 1 to 16, of one size within the cap,
/// no two the same), which run before anything is proved; that they take the type's `entrySize`
/// is for the caller to know, as consensus checks it against the contract. The transition must
/// fit `max_state_transition_size`: one whose entries and actions alone do not is refused before
/// proving, and the built one is checked again with its proof. The contract, the type name and
/// every entry are bound into the bundle's sighash. Returns the built transition together with
/// the fee it pays.
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
    // The entries consensus refuses are refused here, before any proving.
    if let Some(error) = validate_private_document_create_entries(&entries, platform_version)
        .errors
        .into_iter()
        .next()
    {
        return Err(ProtocolError::ShieldedBuildError(error.to_string()));
    }
    // Every entry now takes the first one's size, at most `max_private_document_entry_size`.
    let entry_size = entries.first().map(|entry| entry.len() as u32).unwrap_or(0);

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

    // Every node refuses a transition over `max_state_transition_size`, and so does DAPI. The
    // transition with these entries and the bundle's actions but no proof is a lower bound of
    // the one this builds, so one already over the limit is refused before anything is proved.
    let placeholder_action = SerializedAction {
        nullifier: [0; 32],
        rk: [0; 32],
        cmx: [0; 32],
        encrypted_note: vec![0; ENCRYPTED_NOTE_SIZE],
        cv_net: [0; 32],
        spend_auth_sig: [0; 64],
    };
    let without_proof = PrivateDocumentCreateFromShieldedPoolTransition::try_from_bundle(
        data_contract_id,
        document_type_name.clone(),
        entries.clone(),
        vec![placeholder_action; num_actions],
        0,
        [0; 32],
        vec![],
        [0; 64],
        platform_version,
    )?;
    check_size(&without_proof, platform_version)?;
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
    // The proof's own bytes can still take the transition over the limit.
    check_size(&state_transition, platform_version)?;
    Ok((state_transition, fee))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shielded::builder::test_helpers::{
        test_orchard_address, test_spendable_note, TestProver,
    };
    use crate::shielded::builder::OrchardProver;
    use grovedb_commitment_tree::ExtractedNoteCommitment;

    fn keys() -> (FullViewingKey, SpendAuthorizingKey) {
        let sk = grovedb_commitment_tree::SpendingKey::from_bytes([42u8; 32])
            .expect("valid spending key bytes");
        (FullViewingKey::from(&sk), SpendAuthorizingKey::from(&sk))
    }

    /// A prover that fails the test if a build ever reaches the proof.
    struct UnreachableProver;

    impl OrchardProver for UnreachableProver {
        fn proving_key(&self) -> &grovedb_commitment_tree::ProvingKey {
            panic!("the build must refuse the entries before proving")
        }
    }

    /// A funded note and the anchor its witness proves it under, so a build gets past
    /// `add_spend` and reaches the prover unless the entries are refused first.
    fn spend_with_its_anchor() -> (SpendableNote, Anchor) {
        let spend = test_spendable_note(1_000_000_000_000);
        let anchor = spend
            .merkle_path
            .root(ExtractedNoteCommitment::from(spend.note.commitment()));
        (spend, anchor)
    }

    fn build_with(entries: Vec<BinaryData>) -> Result<(StateTransition, Credits), ProtocolError> {
        let (fvk, ask) = keys();
        let (spend, anchor) = spend_with_its_anchor();
        build_private_document_create_from_shielded_pool_transition(
            vec![spend],
            Identifier::from([1u8; 32]),
            "privateContactRequest".to_string(),
            entries,
            &test_orchard_address(),
            &fvk,
            &ask,
            anchor,
            &UnreachableProver,
            PlatformVersion::latest(),
        )
    }

    /// The control for the refusals below: with valid entries the fixture reaches the prover.
    #[test]
    #[should_panic(expected = "the build must refuse the entries before proving")]
    fn should_reach_the_prover_with_entries_consensus_accepts() {
        let _ = build_with(vec![BinaryData::new(vec![1u8; 585])]);
    }

    #[test]
    fn should_refuse_entries_consensus_refuses_before_proving() {
        let platform_version = PlatformVersion::latest();
        let limits = &platform_version.system_limits;
        let max_count = limits.max_private_document_entries_per_transition.unwrap();
        let max_size = limits.max_private_document_entry_size.unwrap() as usize;
        let cases: Vec<(&str, Vec<BinaryData>)> = vec![
            ("no entry", vec![]),
            (
                "one entry too many",
                (0..=max_count)
                    .map(|i| BinaryData::new(vec![i as u8; 585]))
                    .collect(),
            ),
            ("an empty entry", vec![BinaryData::new(vec![])]),
            (
                "an entry over the cap",
                vec![BinaryData::new(vec![1u8; max_size + 1])],
            ),
            (
                "entries of two sizes",
                vec![
                    BinaryData::new(vec![1u8; 585]),
                    BinaryData::new(vec![2u8; 584]),
                ],
            ),
            (
                "two identical entries",
                vec![
                    BinaryData::new(vec![1u8; 585]),
                    BinaryData::new(vec![1u8; 585]),
                ],
            ),
        ];
        for (case, entries) in cases {
            let expected = validate_private_document_create_entries(&entries, platform_version)
                .errors
                .into_iter()
                .next()
                .unwrap_or_else(|| panic!("{case}: basic structure refuses it"))
                .to_string();
            match build_with(entries) {
                Err(ProtocolError::ShieldedBuildError(message)) => {
                    assert_eq!(message, expected, "{case}")
                }
                other => panic!("{case}: expected the entry refusal, got {other:?}"),
            }
        }
    }

    /// Sixteen entries of the largest size are valid entries, but 64 KiB is far over the
    /// transition size limit, so the build is refused before proving.
    #[test]
    fn should_refuse_a_transition_over_the_size_limit_before_proving() {
        let platform_version = PlatformVersion::latest();
        let limits = &platform_version.system_limits;
        let entries: Vec<BinaryData> =
            (0..limits.max_private_document_entries_per_transition.unwrap())
                .map(|i| {
                    BinaryData::new(vec![
                        i as u8;
                        limits.max_private_document_entry_size.unwrap() as usize
                    ])
                })
                .collect();
        assert!(validate_private_document_create_entries(&entries, platform_version).is_valid());
        match build_with(entries) {
            Err(ProtocolError::ShieldedBuildError(message)) => assert!(
                message.contains(&format!("maximum {}", limits.max_state_transition_size)),
                "{message}"
            ),
            other => panic!("expected the size refusal, got {other:?}"),
        }
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
