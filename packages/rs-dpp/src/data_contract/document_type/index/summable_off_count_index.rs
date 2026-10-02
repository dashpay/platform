//! `summableOffCountIndex` index derivations.
//!
//! A summableOffCountIndex index (see [`super::SUMMABLE_OFF_COUNT_INDEX`]) keeps, per group of its own
//! properties, the number of its source index's entries in that group. The
//! count is lossless only when a group of the summableOffCountIndex index holds exactly
//! the documents of one group of the source: every source property is a
//! property of the summableOffCountIndex index, and every other property of the
//! summableOffCountIndex index is fixed by the source's values. This module derives,
//! for each of those other properties, how the source fixes it — the
//! *derivation* — so the document type parser (which requires one for every
//! such property) and the contract-level validation (which requires its
//! referenced value never to change) read the same function.
//!
//! A property is fixed by the source when it is a referring value of a
//! `where` on a same-contract `permanentDocument` or `moderatedDocument`
//! reference held by a source property: consensus makes it equal the
//! referenced document's value when the document is written, and the
//! referenced document is the one the source property's value names.

use crate::data_contract::document_type::property::{
    DocumentProperty, DocumentPropertyReferenceTarget, DocumentPropertyType, DocumentReferenceKind,
};
use crate::data_contract::document_type::Index;
use indexmap::IndexMap;
use platform_value::Identifier;

/// How one property of a summableOffCountIndex index that its source index lacks is
/// fixed by the source.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CountIndexDerivation<'a> {
    /// The summableOffCountIndex index's property the source lacks.
    pub property: &'a str,
    /// The source property holding the `refersTo` declaration whose `where`
    /// binds [`Self::property`].
    pub referring_property: &'a str,
    /// The referenced document type, in the declaring contract.
    pub target_document_type_name: &'a str,
    /// The kind of the reference: [`DocumentReferenceKind::Permanent`] or
    /// [`DocumentReferenceKind::Moderated`].
    pub kind: DocumentReferenceKind,
    /// The referenced document's value [`Self::property`] equals: one of its
    /// schema properties, or its `$id`, `$ownerId` or `$creatorId`.
    pub referenced: &'a str,
}

impl Index {
    /// For a summableOffCountIndex index, the derivation of every property `source`
    /// lacks, in index order; `Err` names the first property no source
    /// reference fixes. A property of the referring document's own system
    /// (`$ownerId`, `$createdAt`) is never fixed by a referenced document.
    ///
    /// `flattened_properties` are the declaring document type's flattened
    /// properties; `own_contract_id` is the declaring contract's id (a
    /// reference naming it explicitly counts as same-contract).
    pub fn count_index_derivations<'a>(
        &'a self,
        source: &'a Index,
        flattened_properties: &'a IndexMap<String, DocumentProperty>,
        own_contract_id: Identifier,
    ) -> Result<Vec<CountIndexDerivation<'a>>, &'a str> {
        let mut derivations = Vec::new();
        for index_property in &self.properties {
            let property = index_property.name.as_str();
            if source
                .properties
                .iter()
                .any(|source_property| source_property.name == property)
            {
                continue;
            }
            if property.starts_with('$') {
                return Err(property);
            }
            let derivation = source.properties.iter().find_map(|source_property| {
                let declaration = flattened_properties.get(&source_property.name)?;
                let (kind, contract_id, document_type_name, property_agreement) =
                    match &declaration.property_type {
                        DocumentPropertyType::IdentifierWithReference(
                            DocumentPropertyReferenceTarget::PermanentDocument {
                                contract_id,
                                document_type_name,
                                property_agreement,
                            },
                        ) => (
                            DocumentReferenceKind::Permanent,
                            contract_id,
                            document_type_name,
                            property_agreement,
                        ),
                        DocumentPropertyType::IdentifierWithReference(
                            DocumentPropertyReferenceTarget::ModeratedDocument {
                                contract_id,
                                document_type_name,
                                property_agreement,
                            },
                        ) => (
                            DocumentReferenceKind::Moderated,
                            contract_id,
                            document_type_name,
                            property_agreement,
                        ),
                        _ => return None,
                    };
                if contract_id.is_some_and(|id| id != own_contract_id) {
                    return None;
                }
                let referenced = property_agreement.get(property)?;
                Some(CountIndexDerivation {
                    property,
                    referring_property: source_property.name.as_str(),
                    target_document_type_name: document_type_name.as_str(),
                    kind,
                    referenced: referenced.as_str(),
                })
            });
            match derivation {
                Some(derivation) => derivations.push(derivation),
                None => return Err(property),
            }
        }
        Ok(derivations)
    }
}
