//! Preallocated index-path bindings.
//!
//! A `preallocated` index (see [`super::PREALLOCATED`]) promises that its
//! whole path is a pure function of one refersTo-referenced document: every
//! index property is either the referring property itself (whose value is the
//! referenced document's `$id`) or a referring value of that property's
//! `where` (whose value consensus enforces equal to a
//! referenced-document property at write time). A referring typed array bound
//! by an equality with a referenced typed array (protocol version 14) holds
//! exactly the referenced document's list, so its level of the path takes one
//! key per element of that list. This module derives that function — the
//! *binding* — from the index and the declaring document type's properties,
//! so the two consumers cannot drift:
//!
//! - contract validation (`apply_index_only` in `try_from_schema::common`)
//!   rejects the flag when no binding exists, and
//! - the rs-drive insert path resolves the binding against a referenced
//!   document being created to know which trees to preallocate.
//!
//! Only same-contract references qualify: preallocation happens inside the
//! referenced document's own insert, and a foreign contract's insert path
//! cannot know about referring types registered elsewhere (or later).
//!
//! Only references to documents that never leave state without a trace
//! qualify: a `permanentDocument` one, whose document never leaves it, and a
//! `moderatedDocument` one, whose document leaves it only through a
//! moderator's removal, which keeps a record of it for good. Through the
//! latter a binding holds only when the record keeps every key of the path
//! (see [`PreallocationBinding::is_kept_on_removal`]), so a removed
//! document's trees stay keyed by values its record still shows, and a
//! restore of the document as it was removed finds them in place. As through
//! a `permanentDocument` reference, the trees are keyed by the values the
//! document was created with: a key that changes afterwards (a mutable
//! property, a moderator's `changeFields`, a transferred `$ownerId`) leaves
//! them empty and the first entry under the new value builds its own, as
//! without preallocation, which stays an optimization.

use crate::data_contract::document_type::accessors::{
    DocumentTypeV0Getters, DocumentTypeV2Getters,
};
use crate::data_contract::document_type::property::{
    is_path_listed, typed_array_property, DocumentProperty, DocumentPropertyType,
    DocumentReferenceDeclaration, DocumentReferenceKind,
};
use crate::data_contract::document_type::{DocumentTypeRef, Index};
use crate::document::property_names::{ID, OWNER_ID};
use indexmap::IndexMap;
use platform_value::Identifier;
use std::collections::BTreeSet;

/// Where one preallocated index-path key comes from, relative to the
/// referenced document being created.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PreallocatedKeySource<'a> {
    /// The index property is the referring property: its value for entries
    /// referencing the created document is that document's `$id`.
    ReferencedDocumentId,
    /// The index property is bound by the reference's `where`:
    /// its value is the named property of the referenced document, which may
    /// be one of its `$ownerId` and `$creatorId` system identifiers as well
    /// as a schema property. The two sides are validated to share one value
    /// kind; the insert path resolves the name through the same document
    /// accessor the entry walkers key by, which serves the two system names
    /// alongside the schema properties, then keys the value as the referring
    /// type keys its property (an unsigned integer can be keyed in a
    /// different generation on each side).
    ReferencedDocumentProperty(&'a str),
    /// The index property is a typed array bound by the reference's `where`
    /// to the named typed array of the referenced document, an equality
    /// between two lists (protocol version 14): every entry referencing the
    /// created document holds exactly its list, in order, and the index enters
    /// it once per distinct element, so the level takes one key per distinct
    /// element of the referenced list (none for an empty or absent list, which
    /// the index skips). Each element is keyed as the referring type keys the
    /// elements of its property, as for
    /// [`Self::ReferencedDocumentProperty`].
    ReferencedDocumentListElements(&'a str),
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
    /// The kind of the determining reference:
    /// [`DocumentReferenceKind::Permanent`] or
    /// [`DocumentReferenceKind::Moderated`].
    pub kind: DocumentReferenceKind,
    /// One source per index property, in the index's property order —
    /// resolving each against a referenced document yields the full index
    /// path under the declaring document type's subtree.
    pub key_sources: Vec<PreallocatedKeySource<'a>>,
}

impl<'a> PreallocationBinding<'a> {
    /// Whether every key of the path stays on record when the referenced
    /// document leaves state. Always through a `permanentDocument`
    /// reference, whose document never does. Through a `moderatedDocument`
    /// one, whose document leaves it only through a moderator's removal,
    /// when each key the `where` binds is the document's `$id`, its
    /// `$ownerId` or a field its removal record keeps: one of `kept_fields`, the referenced
    /// type's `moderatorAbilities.deleteKeepsFields`, or inside an object
    /// listed there ([`is_path_listed`]). The record always keeps the
    /// document's id and owner, never its creator. A list keying a level by
    /// its elements is kept as a whole, as a single value is.
    pub fn is_kept_on_removal(&self, kept_fields: &BTreeSet<String>) -> bool {
        self.first_key_dropped_on_removal(kept_fields).is_none()
    }

    /// The first referenced property keying the path that the removal record
    /// of the referenced document drops, or `None` when
    /// [`Self::is_kept_on_removal`].
    pub fn first_key_dropped_on_removal(&self, kept_fields: &BTreeSet<String>) -> Option<&'a str> {
        self.key_sources
            .iter()
            .find_map(|key_source| match *key_source {
                PreallocatedKeySource::ReferencedDocumentProperty(referenced)
                | PreallocatedKeySource::ReferencedDocumentListElements(referenced)
                    if !referenced_value_kept_on_removal(self.kind, referenced, kept_fields) =>
                {
                    Some(referenced)
                }
                _ => None,
            })
    }
}

/// The same-contract, by-id `permanentDocument` or `moderatedDocument`
/// reference `property` declares, `None` for every other property: the only
/// references whose `where` values fix an index property from the referenced
/// document's values. Shared by preallocation and the `summableOffCountIndex`
/// lossless rule ([`Index::count_index_derivations`]), so the two agree on what
/// binds.
pub(crate) fn same_contract_record_reference(
    property: &DocumentProperty,
    own_contract_id: Identifier,
) -> Option<DocumentReferenceDeclaration<'_>> {
    let DocumentPropertyType::IdentifierWithReference(target) = &property.property_type else {
        return None;
    };
    target
        .as_document_reference()
        .filter(|reference| {
            matches!(
                reference.kind,
                DocumentReferenceKind::Permanent | DocumentReferenceKind::Moderated
            )
        })
        .filter(|reference| {
            !reference
                .contract_id
                .is_some_and(|id| id != own_contract_id)
        })
}

/// Whether the referenced document's value `referenced` can still be read
/// once the document leaves through a reference of `kind`: a reference that
/// is not moderated keeps its document, and a moderator's removal record
/// keeps the document's id, its owner and the fields its type lists under
/// `deleteKeepsFields` (`kept_fields`), never its creator. Preallocation and
/// the `summableOffCountIndex` lossless check both judge by it.
pub fn referenced_value_kept_on_removal(
    kind: DocumentReferenceKind,
    referenced: &str,
    kept_fields: &BTreeSet<String>,
) -> bool {
    kind != DocumentReferenceKind::Moderated
        || referenced == ID
        || referenced == OWNER_ID
        || is_path_listed(kept_fields, referenced)
}

impl Index {
    /// Every binding through which this index's path is fully determined by
    /// a same-contract `permanentDocument` or `moderatedDocument` reference.
    /// Empty when the index cannot be preallocated: validation requires at
    /// least one binding for `preallocated: true` (and, once every document
    /// type of the contract is parsed, one that
    /// [`PreallocationBinding::is_kept_on_removal`]), and the insert path
    /// preallocates once per binding whose target is the document type being
    /// created ([`Self::preallocation_bindings_for_target`]).
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

    /// The bindings the insert of a document of `target_document_type`
    /// preallocates through: those of [`Self::preallocation_bindings`] whose
    /// target it is and whose path its removal record would keep
    /// ([`PreallocationBinding::is_kept_on_removal`], always true through a
    /// `permanentDocument` reference). The write path calls this once per
    /// document insert for every preallocated index of the contract, so
    /// candidates naming other target types are rejected before their
    /// key-source vectors are ever allocated.
    pub fn preallocation_bindings_for_target<'a>(
        &'a self,
        flattened_properties: &'a IndexMap<String, DocumentProperty>,
        own_contract_id: Identifier,
        target_document_type: DocumentTypeRef,
    ) -> Vec<PreallocationBinding<'a>> {
        let kept_fields = target_document_type.moderator_deletion_kept_fields();
        let mut bindings = self.preallocation_bindings_impl(
            flattened_properties,
            own_contract_id,
            Some(target_document_type.name().as_str()),
        );
        bindings.retain(|binding| binding.is_kept_on_removal(kept_fields));
        bindings
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
            // Only a scalar reference can bind: an element reference of a
            // typed array is no scalar identifier, and its value is no
            // referenced document's `$id`. A
            // lookup reference of either kind never matches either: its value
            // is not the referenced document's `$id`. Nor
            // does a reference expression (`anyOf` / `allOf`), even of
            // permanentDocument leaves only: an `anyOf` value may be the id of
            // a document of any of them, and binding an `allOf` would have to
            // pick one leaf's agreement over the others'. Nor does a
            // deletableDocument reference: its document can leave state
            // without a record, and the trees would outlive it with nothing
            // left to say what they were keyed by
            let Some(reference) = same_contract_record_reference(property, own_contract_id) else {
                continue;
            };
            if only_target_document_type_name
                .is_some_and(|target| target != reference.document_type_name)
            {
                continue;
            }
            let key_sources: Option<Vec<_>> =
                self.properties
                    .iter()
                    .map(|index_property| {
                        if index_property.name == candidate.name {
                            Some(PreallocatedKeySource::ReferencedDocumentId)
                        } else if index_property.name.starts_with('$') {
                            // The referring document's own system properties are
                            // never derived from the referenced document, even
                            // when an agreement binds the writer's `$ownerId` to
                            // it: that agreement is a write gate, and deriving an
                            // owner-prefixed path from it is left for later.
                            None
                        } else if typed_array_property(flattened_properties, &index_property.name)
                            .is_some()
                        {
                            // A typed array is bound only by an equality with a
                            // referenced typed array: contract registration refuses
                            // an equality pairing a list with anything else, and
                            // `contains` / `containsAll` are no equalities and bind
                            // nothing (the elements of a document whose list only
                            // holds, or is held by, the referenced list are not
                            // fixed by it). An entry then holds the referenced
                            // list, and the index enters it under each element
                            reference.property_agreement.get(&index_property.name).map(
                                |referenced| {
                                    PreallocatedKeySource::ReferencedDocumentListElements(
                                        referenced.as_str(),
                                    )
                                },
                            )
                        } else {
                            reference.property_agreement.get(&index_property.name).map(
                                |referenced| {
                                    PreallocatedKeySource::ReferencedDocumentProperty(
                                        referenced.as_str(),
                                    )
                                },
                            )
                        }
                    })
                    .collect();
            if let Some(key_sources) = key_sources {
                bindings.push(PreallocationBinding {
                    referring_property: candidate.name.as_str(),
                    target_document_type_name: reference.document_type_name,
                    kind: reference.kind,
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
    use crate::data_contract::document_type::index::summable_off_count_index::CountIndexDerivation;
    use crate::data_contract::document_type::property::DocumentPropertyReferenceTarget;
    use crate::data_contract::document_type::{
        DocumentReferenceLookup, IndexProperty, LookupKeySource, ReferenceOperands,
    };
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
                    where_conditions: Default::default(),
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
            distinct_from: None,
            encrypted_for: None,
            generated_from: None,
            revealed_reference: None,
            transient: false,
        }
    }

    fn string_property() -> DocumentProperty {
        use crate::data_contract::document_type::property::StringPropertySizes;
        DocumentProperty {
            property_type: DocumentPropertyType::String(StringPropertySizes {
                min_length: None,
                max_length: None,
                max_bytes: None,
            }),
            required: true,
            required_since: None,
            distinct_from: None,
            encrypted_for: None,
            generated_from: None,
            revealed_reference: None,
            transient: false,
        }
    }

    fn identifier_property() -> DocumentProperty {
        DocumentProperty {
            property_type: DocumentPropertyType::Identifier,
            required: true,
            required_since: None,
            distinct_from: None,
            encrypted_for: None,
            generated_from: None,
            revealed_reference: None,
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
            ranked_countable_at: vec![],
            ranked_summable_at: Vec::new(),
            ranked_averageable_at: Vec::new(),
            ranked_summable: false,
            ranked_averageable: false,
            time_range: None,
            integer_range: None,
            terminal: Some(vec!["$ownerId".to_string()]),
            preallocated: true,
            outlives_delete: false,
            skip_if_absent: false,
            skip_if_absent_properties: Vec::new(),
            summable_off_count_index: None,
            indexed_array: None,
        }
    }

    fn tags_property() -> DocumentProperty {
        use crate::data_contract::document_type::array::TypedArrayProperty;
        use crate::data_contract::document_type::property::StringPropertySizes;
        DocumentProperty {
            property_type: DocumentPropertyType::TypedArray(TypedArrayProperty {
                item_type: Box::new(DocumentPropertyType::String(StringPropertySizes {
                    min_length: Some(1),
                    max_length: Some(63),
                    max_bytes: None,
                })),
                item_constraints: Default::default(),
                min_items: None,
                max_items: 4,
                unique_items: true,
            }),
            ..string_property()
        }
    }

    /// A `where` equality between two typed arrays fixes the referring list to
    /// the referenced one: a preallocated path takes a key per element of the
    /// referenced list at the array's level, and a `summableOffCountIndex`
    /// group is fixed through it, at whatever position the array sits.
    #[test]
    fn should_bind_a_typed_array_through_a_pair_of_typed_arrays() {
        let own_contract_id = Identifier::from([1u8; 32]);
        let mut properties = IndexMap::new();
        properties.insert("tags".to_string(), tags_property());
        properties.insert(
            "postId".to_string(),
            identifier_reference_property("post", None, &[("tags", "tags")]),
        );

        for (index_properties, key_sources) in [
            (
                ["tags", "postId"],
                vec![
                    PreallocatedKeySource::ReferencedDocumentListElements("tags"),
                    PreallocatedKeySource::ReferencedDocumentId,
                ],
            ),
            (
                ["postId", "tags"],
                vec![
                    PreallocatedKeySource::ReferencedDocumentId,
                    PreallocatedKeySource::ReferencedDocumentListElements("tags"),
                ],
            ),
        ] {
            let index = index_on(&index_properties);
            assert_eq!(
                index.preallocation_bindings(&properties, own_contract_id),
                vec![PreallocationBinding {
                    referring_property: "postId",
                    target_document_type_name: "post",
                    kind: DocumentReferenceKind::Permanent,
                    key_sources,
                }]
            );

            let source = index_on(&["postId"]);
            let mut counts = index;
            counts.summable_off_count_index = Some(source.name.clone());
            assert_eq!(
                counts.count_index_derivations(&source, &properties, own_contract_id),
                Ok(vec![CountIndexDerivation {
                    property: "tags",
                    referring_property: "postId",
                    target_document_type_name: "post",
                    kind: DocumentReferenceKind::Permanent,
                    referenced: "tags",
                }])
            );
        }
    }

    /// A typed array no `where` equality binds fixes nothing: no binding and
    /// no derivation.
    #[test]
    fn should_bind_no_typed_array_without_a_where_equality() {
        let own_contract_id = Identifier::from([1u8; 32]);
        let mut properties = IndexMap::new();
        properties.insert("tags".to_string(), tags_property());
        properties.insert(
            "postId".to_string(),
            identifier_reference_property("post", None, &[]),
        );

        let index = index_on(&["tags", "postId"]);
        assert!(index
            .preallocation_bindings(&properties, own_contract_id)
            .is_empty());

        let source = index_on(&["postId"]);
        let mut counts = index;
        counts.summable_off_count_index = Some(source.name.clone());
        assert_eq!(
            counts.count_index_derivations(&source, &properties, own_contract_id),
            Err("tags")
        );
    }

    /// Through a moderatedDocument reference a list binding holds when the
    /// removal record keeps the list, as a single value's does.
    #[test]
    fn should_keep_a_moderated_list_binding_only_when_its_record_keeps_the_list() {
        let binding = PreallocationBinding {
            referring_property: "postId",
            target_document_type_name: "post",
            kind: DocumentReferenceKind::Moderated,
            key_sources: vec![
                PreallocatedKeySource::ReferencedDocumentListElements("tags"),
                PreallocatedKeySource::ReferencedDocumentId,
            ],
        };
        assert!(binding.is_kept_on_removal(&["tags".to_string()].into()));
        assert_eq!(
            binding.first_key_dropped_on_removal(&BTreeSet::new()),
            Some("tags")
        );
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
                kind: DocumentReferenceKind::Permanent,
                key_sources: vec![
                    PreallocatedKeySource::ReferencedDocumentProperty("hashtag"),
                    PreallocatedKeySource::ReferencedDocumentId,
                ],
            }]
        );
    }

    /// An agreement on the referenced document's `$ownerId` (or
    /// `$creatorId`) determines the index path exactly like one on a
    /// schema property: the key source carries the system name for the
    /// insert path to resolve against the created document.
    #[test]
    fn binds_agreement_on_referenced_system_identifier() {
        let own_contract_id = Identifier::from([1u8; 32]);
        for referenced in ["$ownerId", "$creatorId"] {
            let mut properties = IndexMap::new();
            properties.insert("authorId".to_string(), identifier_property());
            properties.insert(
                "postId".to_string(),
                identifier_reference_property("post", None, &[("authorId", referenced)]),
            );

            let index = index_on(&["authorId", "postId"]);
            let bindings = index.preallocation_bindings(&properties, own_contract_id);
            assert_eq!(
                bindings,
                vec![PreallocationBinding {
                    referring_property: "postId",
                    target_document_type_name: "post",
                    kind: DocumentReferenceKind::Permanent,
                    key_sources: vec![
                        PreallocatedKeySource::ReferencedDocumentProperty(referenced),
                        PreallocatedKeySource::ReferencedDocumentId,
                    ],
                }]
            );
        }
    }

    /// A writer gate (`{ "$ownerId": "$ownerId" }`) does not make an
    /// owner-prefixed index preallocatable: the referring document's own
    /// system properties never come from the referenced document.
    #[test]
    fn no_binding_through_a_writer_owner_agreement() {
        let own_contract_id = Identifier::from([1u8; 32]);
        let mut properties = IndexMap::new();
        properties.insert(
            "postId".to_string(),
            identifier_reference_property("post", None, &[("$ownerId", "$ownerId")]),
        );

        let index = index_on(&["$ownerId", "postId"]);
        assert!(index
            .preallocation_bindings(&properties, own_contract_id)
            .is_empty());
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
    #[test]
    fn should_not_bind_through_a_lookup_reference() {
        let own_contract_id = Identifier::from([1u8; 32]);
        let mut property = identifier_reference_property("post", None, &[]);
        property.property_type = DocumentPropertyType::IdentifierWithReference(
            DocumentPropertyReferenceTarget::PermanentDocumentLookup {
                contract_id: None,
                document_type_name: "post".to_string(),
                where_conditions: Default::default(),
                property_agreement: BTreeMap::new(),
                lookup: DocumentReferenceLookup {
                    keys: [("$ownerId".to_string(), LookupKeySource::ReferenceValue)].into(),
                    minimum_age_blocks: None,
                    consume: false,
                    choose_first: None,
                },
            },
        );
        let mut properties = IndexMap::new();
        properties.insert("postId".to_string(), property);

        // The value names the post's owner, not the post, so no path follows
        // from the post being created
        assert!(index_on(&["postId"])
            .preallocation_bindings(&properties, own_contract_id)
            .is_empty());
    }

    #[test]
    fn should_not_bind_through_a_reference_expression() {
        let own_contract_id = Identifier::from([1u8; 32]);
        let post = |document_type_name: &str| DocumentPropertyReferenceTarget::PermanentDocument {
            contract_id: None,
            document_type_name: document_type_name.to_string(),
            where_conditions: Default::default(),
            property_agreement: BTreeMap::new(),
        };
        let index = index_on(&["postId"]);
        for expression in [
            DocumentPropertyReferenceTarget::AnyOf(ReferenceOperands::new(vec![
                post("post"),
                post("repost"),
            ])),
            DocumentPropertyReferenceTarget::AllOf(ReferenceOperands::new(vec![
                post("post"),
                DocumentPropertyReferenceTarget::Identity,
            ])),
        ] {
            let mut property = identifier_reference_property("post", None, &[]);
            property.property_type = DocumentPropertyType::IdentifierWithReference(expression);
            let mut properties = IndexMap::new();
            properties.insert("postId".to_string(), property);

            // An anyOf value may name a repost as well as a post, and an
            // allOf is refused alike: no single leaf determines the path
            assert!(index
                .preallocation_bindings(&properties, own_contract_id)
                .is_empty());
            assert!(index
                .preallocation_bindings_impl(&properties, own_contract_id, Some("post"))
                .is_empty());
        }
    }

    fn moderated_reference_property(agreement: &[(&str, &str)]) -> DocumentProperty {
        let mut property = identifier_reference_property("post", None, agreement);
        let DocumentPropertyType::IdentifierWithReference(
            DocumentPropertyReferenceTarget::PermanentDocument {
                contract_id,
                document_type_name,
                property_agreement,
                ..
            },
        ) = property.property_type
        else {
            unreachable!("identifier_reference_property builds a permanentDocument reference")
        };
        property.property_type = DocumentPropertyType::IdentifierWithReference(
            DocumentPropertyReferenceTarget::ModeratedDocument {
                contract_id,
                document_type_name,
                where_conditions: Default::default(),
                property_agreement,
            },
        );
        property
    }

    #[test]
    fn should_bind_through_a_moderated_document_reference() {
        let own_contract_id = Identifier::from([1u8; 32]);
        let mut properties = IndexMap::new();
        properties.insert("hashtag".to_string(), string_property());
        properties.insert(
            "postId".to_string(),
            moderated_reference_property(&[("hashtag", "hashtag")]),
        );

        let index = index_on(&["hashtag", "postId"]);
        assert_eq!(
            index.preallocation_bindings(&properties, own_contract_id),
            vec![PreallocationBinding {
                referring_property: "postId",
                target_document_type_name: "post",
                kind: DocumentReferenceKind::Moderated,
                key_sources: vec![
                    PreallocatedKeySource::ReferencedDocumentProperty("hashtag"),
                    PreallocatedKeySource::ReferencedDocumentId,
                ],
            }]
        );
    }

    #[test]
    fn should_not_bind_through_a_deletable_document_reference() {
        let own_contract_id = Identifier::from([1u8; 32]);
        let mut property = identifier_reference_property("post", None, &[]);
        property.property_type = DocumentPropertyType::IdentifierWithReference(
            DocumentPropertyReferenceTarget::DeletableDocument {
                contract_id: None,
                document_type_name: "post".to_string(),
                where_conditions: Default::default(),
                property_agreement: BTreeMap::new(),
            },
        );
        let mut properties = IndexMap::new();
        properties.insert("postId".to_string(), property);

        // The post can leave state without a record: nothing would be left to
        // say what its trees were keyed by
        assert!(index_on(&["postId"])
            .preallocation_bindings(&properties, own_contract_id)
            .is_empty());
    }

    #[test]
    fn should_keep_a_moderated_binding_only_when_its_record_keeps_every_key() {
        let binding = |kind, referenced| PreallocationBinding {
            referring_property: "postId",
            target_document_type_name: "post",
            kind,
            key_sources: vec![
                PreallocatedKeySource::ReferencedDocumentProperty(referenced),
                PreallocatedKeySource::ReferencedDocumentId,
            ],
        };
        let kept = |paths: &[&str]| -> BTreeSet<String> {
            paths.iter().map(|path| path.to_string()).collect()
        };
        let moderated = DocumentReferenceKind::Moderated;

        // A permanent document never leaves state: every binding holds
        assert!(binding(DocumentReferenceKind::Permanent, "hashtag").is_kept_on_removal(&kept(&[])));
        // The record always keeps the document's owner, and its id
        assert!(binding(moderated, "$ownerId").is_kept_on_removal(&kept(&[])));
        assert!(binding(moderated, "$id").is_kept_on_removal(&kept(&[])));
        // A schema property only when its type lists it, or an object around it
        assert!(!binding(moderated, "hashtag").is_kept_on_removal(&kept(&[])));
        assert!(!binding(moderated, "hashtag").is_kept_on_removal(&kept(&["text"])));
        assert!(binding(moderated, "hashtag").is_kept_on_removal(&kept(&["hashtag"])));
        assert!(binding(moderated, "meta.topic").is_kept_on_removal(&kept(&["meta"])));
        assert!(!binding(moderated, "meta").is_kept_on_removal(&kept(&["meta.topic"])));
        // Never its creator, which no record keeps
        assert!(!binding(moderated, "$creatorId").is_kept_on_removal(&kept(&[])));
    }
}
