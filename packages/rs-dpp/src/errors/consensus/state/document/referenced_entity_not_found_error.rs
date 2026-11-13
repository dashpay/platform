use crate::consensus::state::state_error::StateError;
use crate::consensus::ConsensusError;
use crate::data_contract::document_type::DocumentPropertyReferenceTarget;
use crate::ProtocolError;
use bincode::{Decode, DecodeUntrusted, Encode};
use platform_serialization_derive::{
    PlatformDeserializeTrusted, PlatformDeserializeUntrusted, PlatformSerialize,
};
use platform_value::Identifier;
use thiserror::Error;

#[derive(
    Error,
    Debug,
    Clone,
    PartialEq,
    Eq,
    Encode,
    Decode,
    PlatformSerialize,
    PlatformDeserializeTrusted,
    PlatformDeserializeUntrusted,
    DecodeUntrusted,
)]
#[error("referenced {entity_type} {entity_id} not found for path {path}")]
#[platform_serialize(unversioned)]
pub struct ReferencedEntityNotFoundError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    entity_id: Identifier,
    entity_type: DocumentPropertyReferenceTarget,
    path: String,
}

impl ReferencedEntityNotFoundError {
    pub fn new(
        entity_id: Identifier,
        entity_type: DocumentPropertyReferenceTarget,
        path: String,
    ) -> Self {
        Self {
            entity_id,
            // Without the requirements of a reference naming its target through the
            // referring document, which could outgrow the budget an error is decoded in
            entity_type: entity_type.named_in_an_error(),
            path,
        }
    }

    pub fn entity_id(&self) -> &Identifier {
        &self.entity_id
    }

    pub fn entity_type(&self) -> &DocumentPropertyReferenceTarget {
        &self.entity_type
    }

    pub fn path(&self) -> &str {
        &self.path
    }
}

impl From<ReferencedEntityNotFoundError> for ConsensusError {
    fn from(err: ReferencedEntityNotFoundError) -> Self {
        Self::StateError(StateError::ReferencedEntityNotFoundError(err))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data_contract::document_type::{
        DocumentTypeRequirements, DynamicDocumentReference, DynamicDocumentReferenceKind,
        RequiredPropertySlot, SlotPropertyName, SlotRequirement, SlotRequirementTerm, SlotValue,
    };
    use crate::serialization::{
        PlatformDeserializableUntrusted, PlatformSerializableWithPlatformVersion,
    };
    use platform_version::version::LATEST_PLATFORM_VERSION;
    use std::collections::{BTreeMap, BTreeSet};

    /// A reference naming its target through the referring document, with requirements as
    /// large as a contract may declare them: four slots of 64-character names, each named
    /// property as long, every slot distinct from the others, and a `require` of groups.
    fn largest_dynamic_reference() -> DynamicDocumentReference {
        let name = |letter: char| letter.to_string().repeat(64);
        let slots: Vec<String> = ['a', 'b', 'c', 'd'].into_iter().map(name).collect();
        DynamicDocumentReference {
            kind: DynamicDocumentReferenceKind::Moderated,
            contract_id_property: name('x'),
            document_type_property: name('y'),
            requirements: Some(DocumentTypeRequirements {
                slots: slots
                    .iter()
                    .map(|slot| {
                        (
                            slot.clone(),
                            RequiredPropertySlot {
                                name: SlotPropertyName::Named(name('p')),
                                value: SlotValue::String,
                                max_bytes: Some(u32::MAX),
                                distinct_from: slots
                                    .iter()
                                    .filter(|other| *other != slot)
                                    .cloned()
                                    .collect::<BTreeSet<_>>(),
                                when: None,
                            },
                        )
                    })
                    .collect::<BTreeMap<_, _>>(),
                require: Some(SlotRequirement::AnyOf(
                    (0..slots.len())
                        .flat_map(|first| {
                            ((first + 1)..slots.len()).map(move |second| (first, second))
                        })
                        .map(|(first, second)| {
                            SlotRequirementTerm::Group(vec![
                                slots[first].clone(),
                                slots[second].clone(),
                            ])
                        })
                        .collect(),
                )),
                when: None,
            }),
        }
    }

    /// A client decodes a consensus error within a fixed budget. The declaration of a
    /// reference naming its target through the referring document can outgrow it, so the
    /// error carries the target without its requirements and always decodes.
    #[test]
    fn should_carry_a_dynamic_target_without_its_requirements() {
        let reference = largest_dynamic_reference();
        let declared = DocumentPropertyReferenceTarget::DynamicDocument(reference.clone());
        let declared_size = bincode::encode_to_vec(&declared, bincode::config::standard())
            .expect("encodes")
            .len();
        assert!(
            declared_size > 2000,
            "the declaration alone, {declared_size} bytes, outgrows the budget"
        );

        let error = ReferencedEntityNotFoundError::new(
            Identifier::new([7; 32]),
            declared,
            "descriptionId".to_string(),
        );
        assert_eq!(
            error.entity_type(),
            &DocumentPropertyReferenceTarget::DynamicDocument(DynamicDocumentReference {
                requirements: None,
                ..reference
            })
        );
        // The message names the kind and the properties read, as before
        assert!(error.to_string().starts_with(&format!(
            "referenced moderated document (contract named by {}",
            "x".repeat(64)
        )));

        let consensus_error: ConsensusError = error.into();
        let bytes = consensus_error
            .serialize_to_bytes_with_platform_version(LATEST_PLATFORM_VERSION)
            .expect("the error serializes");
        assert!(bytes.len() < 2000, "{} bytes", bytes.len());
        assert_eq!(
            ConsensusError::deserialize_from_bytes_untrusted(&bytes)
                .expect("the error decodes within the budget"),
            consensus_error
        );
    }

    #[test]
    fn should_carry_every_other_target_whole() {
        let target = DocumentPropertyReferenceTarget::PermanentDocument {
            contract_id: None,
            document_type_name: "post".to_string(),
            where_conditions: Default::default(),
            property_agreement: Default::default(),
        };
        let error = ReferencedEntityNotFoundError::new(
            Identifier::new([7; 32]),
            target.clone(),
            "postId".to_string(),
        );
        assert_eq!(error.entity_type(), &target);
    }
}
