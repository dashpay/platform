//! Preallocated index-path bindings.
//!
//! A `preallocated` index (see [`super::PREALLOCATED`]) promises that its
//! whole path is a pure function of one refersTo-referenced document: every
//! index property is either the referring property itself (whose value is the
//! referenced document's `$id`) or a key of that property's
//! `propertyAgreement` (whose value consensus enforces equal to a
//! referenced-document property at write time). This module derives that
//! function — the *binding* — from the index and the declaring document
//! type's properties, so the two consumers cannot drift:
//!
//! - contract validation (`apply_index_only` in `try_from_schema::common`)
//!   rejects the flag when no binding exists, and
//! - the rs-drive insert path resolves the binding against a referenced
//!   document being created to know which trees to preallocate.
//!
//! Only same-contract references qualify: preallocation happens inside the
//! referenced document's own insert, and a foreign contract's insert path
//! cannot know about referring types registered elsewhere (or later).

use crate::data_contract::document_type::property::{
    DocumentProperty, DocumentPropertyReferenceTarget, DocumentPropertyType,
};
use crate::data_contract::document_type::Index;
use indexmap::IndexMap;
use platform_value::Identifier;

/// Where one preallocated index-path key comes from, relative to the
/// referenced document being created.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PreallocatedKeySource<'a> {
    /// The index property is the referring property: its value for entries
    /// referencing the created document is that document's `$id`.
    ReferencedDocumentId,
    /// The index property is bound by the reference's `propertyAgreement`:
    /// its value is the named property of the referenced document. The two
    /// sides are validated to share one value kind, so encoding the
    /// referenced document's value yields the same key bytes the referring
    /// side would produce.
    ReferencedDocumentProperty(&'a str),
}

/// One way a preallocated index's path is determined by a referenced
/// document: the referring property, the (same-contract) document type it
/// references, and one key source per index property, in index order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreallocationBinding<'a> {
    /// The index property carrying the determining `refersTo` declaration.
    pub referring_property: &'a str,
    /// The referenced document type (in the declaring contract) whose
    /// document creation preallocates this index's trees.
    pub target_document_type_name: &'a str,
    /// One source per index property, in the index's property order —
    /// resolving each against a referenced document yields the full index
    /// path under the declaring document type's subtree.
    pub key_sources: Vec<PreallocatedKeySource<'a>>,
}

impl Index {
    /// Every binding through which this index's path is fully determined by
    /// a same-contract `permanentDocument` reference. Empty when the index
    /// cannot be preallocated: validation requires at least one binding for
    /// `preallocated: true`, and the insert path preallocates once per
    /// binding whose target is the document type being created.
    ///
    /// `flattened_properties` are the declaring document type's flattened
    /// properties; `own_contract_id` is the declaring contract's id (a
    /// reference naming it explicitly counts as same-contract).
    pub fn preallocation_bindings<'a>(
        &'a self,
        flattened_properties: &'a IndexMap<String, DocumentProperty>,
        own_contract_id: Identifier,
    ) -> Vec<PreallocationBinding<'a>> {
        self.preallocation_bindings_impl(flattened_properties, own_contract_id, None)
    }

    /// [`Self::preallocation_bindings`] restricted to bindings whose target
    /// is `target_document_type_name`. The write path calls this once per
    /// document insert for every preallocated index of the contract, so
    /// candidates naming other target types are rejected before their
    /// key-source vectors are ever allocated.
    pub fn preallocation_bindings_for_target<'a>(
        &'a self,
        flattened_properties: &'a IndexMap<String, DocumentProperty>,
        own_contract_id: Identifier,
        target_document_type_name: &str,
    ) -> Vec<PreallocationBinding<'a>> {
        self.preallocation_bindings_impl(
            flattened_properties,
            own_contract_id,
            Some(target_document_type_name),
        )
    }

    fn preallocation_bindings_impl<'a>(
        &'a self,
        flattened_properties: &'a IndexMap<String, DocumentProperty>,
        own_contract_id: Identifier,
        only_target_document_type_name: Option<&str>,
    ) -> Vec<PreallocationBinding<'a>> {
        let mut bindings = Vec::new();
        for candidate in &self.properties {
            let Some(property) = flattened_properties.get(&candidate.name) else {
                continue;
            };
            let DocumentPropertyType::IdentifierWithReference(
                DocumentPropertyReferenceTarget::PermanentDocument {
                    contract_id,
                    document_type_name,
                    property_agreement,
                },
            ) = &property.property_type
            else {
                continue;
            };
            if contract_id.is_some_and(|id| id != own_contract_id) {
                continue;
            }
            if only_target_document_type_name
                .is_some_and(|target| target != document_type_name.as_str())
            {
                continue;
            }
            let key_sources: Option<Vec<_>> = self
                .properties
                .iter()
                .map(|index_property| {
                    if index_property.name == candidate.name {
                        Some(PreallocatedKeySource::ReferencedDocumentId)
                    } else {
                        property_agreement
                            .get(&index_property.name)
                            .map(|referenced| {
                                PreallocatedKeySource::ReferencedDocumentProperty(
                                    referenced.as_str(),
                                )
                            })
                    }
                })
                .collect();
            if let Some(key_sources) = key_sources {
                bindings.push(PreallocationBinding {
                    referring_property: candidate.name.as_str(),
                    target_document_type_name: document_type_name.as_str(),
                    key_sources,
                });
            }
        }
        bindings
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data_contract::document_type::IndexProperty;
    use std::collections::BTreeMap;

    fn identifier_reference_property(
        target_type: &str,
        contract_id: Option<Identifier>,
        agreement: &[(&str, &str)],
    ) -> DocumentProperty {
        DocumentProperty {
            property_type: DocumentPropertyType::IdentifierWithReference(
                DocumentPropertyReferenceTarget::PermanentDocument {
                    contract_id,
                    document_type_name: target_type.to_string(),
                    property_agreement: agreement
                        .iter()
                        .map(|(referring, referenced)| {
                            (referring.to_string(), referenced.to_string())
                        })
                        .collect::<BTreeMap<_, _>>(),
                },
            ),
            required: true,
            required_since: None,
            transient: false,
        }
    }

    fn string_property() -> DocumentProperty {
        use crate::data_contract::document_type::property::StringPropertySizes;
        DocumentProperty {
            property_type: DocumentPropertyType::String(StringPropertySizes {
                min_length: None,
                max_length: None,
            }),
            required: true,
            required_since: None,
            transient: false,
        }
    }

    fn index_on(properties: &[&str]) -> Index {
        Index {
            name: "test".to_string(),
            properties: properties
                .iter()
                .map(|name| IndexProperty {
                    name: name.to_string(),
                    ascending: true,
                })
                .collect(),
            unique: false,
            null_searchable: true,
            contested_index: None,
            countable: Default::default(),
            range_countable: false,
            summable: None,
            range_summable: false,
            ranked_countable: false,
            ranked_countable_at: None,
            ranked_summable: false,
            ranked_averageable: false,
            time_range: None,
            terminal: Some("$ownerId".to_string()),
            preallocated: true,
            skip_if_absent: false,
        }
    }

    #[test]
    fn binds_agreement_prefix_and_referring_property() {
        let own_contract_id = Identifier::from([1u8; 32]);
        let mut properties = IndexMap::new();
        properties.insert("hashtag".to_string(), string_property());
        properties.insert(
            "postId".to_string(),
            identifier_reference_property("post", None, &[("hashtag", "hashtag")]),
        );

        let index = index_on(&["hashtag", "postId"]);
        let bindings = index.preallocation_bindings(&properties, own_contract_id);
        assert_eq!(
            bindings,
            vec![PreallocationBinding {
                referring_property: "postId",
                target_document_type_name: "post",
                key_sources: vec![
                    PreallocatedKeySource::ReferencedDocumentProperty("hashtag"),
                    PreallocatedKeySource::ReferencedDocumentId,
                ],
            }]
        );
    }

    #[test]
    fn no_binding_when_a_property_is_not_determined() {
        let own_contract_id = Identifier::from([1u8; 32]);
        let mut properties = IndexMap::new();
        properties.insert("hashtag".to_string(), string_property());
        properties.insert(
            "postId".to_string(),
            identifier_reference_property("post", None, &[]),
        );

        // `hashtag` is neither the referring property nor in the agreement.
        let index = index_on(&["hashtag", "postId"]);
        assert!(index
            .preallocation_bindings(&properties, own_contract_id)
            .is_empty());
    }

    #[test]
    fn foreign_contract_reference_does_not_bind_but_own_id_does() {
        let own_contract_id = Identifier::from([1u8; 32]);
        let mut foreign = IndexMap::new();
        foreign.insert(
            "postId".to_string(),
            identifier_reference_property("post", Some(Identifier::from([2u8; 32])), &[]),
        );
        let index = index_on(&["postId"]);
        assert!(index
            .preallocation_bindings(&foreign, own_contract_id)
            .is_empty());

        let mut own = IndexMap::new();
        own.insert(
            "postId".to_string(),
            identifier_reference_property("post", Some(own_contract_id), &[]),
        );
        assert_eq!(index.preallocation_bindings(&own, own_contract_id).len(), 1);
    }
}
