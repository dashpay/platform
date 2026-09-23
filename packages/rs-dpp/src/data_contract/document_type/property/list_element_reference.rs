//! The `listElement` target of a `refersTo` declaration: the value must be an
//! element of a typed array of identifiers held by a document that agrees
//! with the referring document, found by the `propertyAgreement` pair whose
//! referenced side is `$id`.
//!
//! Declared on an identifier property, or on the `items` of a typed array of
//! identifiers, where every element must be one, alone or as a leaf of a
//! reference expression (meta-schema v3, protocol version 14):
//!
//! ```json
//! "refersTo": {
//!   "type": "listElement",
//!   "documentType": "electedCharter",
//!   "propertyAgreement": { "electedCharterId": "$id" },
//!   "inList": "members"
//! }
//! ```
//!
//! reads: the value must be one of the `members` of the `electedCharter`
//! document whose `$id` this document's `electedCharterId` holds. It is a
//! document reference like `permanentDocument`, with the same `contractId`,
//! `documentType` and `propertyAgreement` and the same checks on them, except
//! that the value is not the document's id: the document is the one the `$id`
//! pair names, any other pair is an ordinary agreement checked against it, and
//! the value must be in its list. The referenced document can never be deleted
//! and its list never changes (the type is immutable or lists the property
//! under `immutable`), so a value accepted once stays an element for good. The
//! rules live here so the two places that check a declaration against its
//! referenced document type (the contract parse for a type of the same
//! contract, the registration state validation for a type of another
//! contract) cannot drift.

use crate::data_contract::document_type::accessors::DocumentTypeV0Getters;
use crate::data_contract::document_type::accessors::DocumentTypeV2Getters;
use crate::data_contract::document_type::property::reference_lookup::schema_property_is_fixed_once_written;
use crate::data_contract::document_type::property::{is_transient, DocumentPropertyType};
use crate::data_contract::document_type::DocumentTypeRef;
use crate::document::property_names::ID;
use bincode::{Decode, DecodeUntrusted, Encode};
use platform_value::btreemap_extensions::BTreeValueMapPathHelper;
use platform_value::{Identifier, Value};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};

/// A `refersTo: listElement` declaration: the value (or each element of a
/// typed array) must be an element of the typed array `in_list` of the
/// `document_type_name` document the `$id` pair of `property_agreement`
/// names.
#[derive(Debug, PartialEq, Eq, Clone, Serialize, Encode, Decode, DecodeUntrusted)]
pub struct ListElementReference {
    /// The contract the document type holding the list lives in; `None`
    /// means the declaring contract itself.
    pub contract_id: Option<Identifier>,
    /// The document type holding the list.
    pub document_type_name: String,
    /// The `{referring property: referenced property}` equalities, exactly
    /// one of which has `$id` on the referenced side: its referring side, an
    /// identifier property of the declaring type, holds the id of the
    /// document the list is read from. The others are checked against that
    /// document as any agreement is.
    pub property_agreement: BTreeMap<String, String>,
    /// The typed array of identifiers of `document_type_name` (a dotted path
    /// when nested) the value must be an element of.
    pub in_list: String,
}

impl ListElementReference {
    /// The referring side of the `$id` pair: the identifier property of the
    /// declaring type whose value is the id of the document the list is read
    /// from. `None` only for a declaration the parser never produces.
    pub fn document_id_property(&self) -> Option<&str> {
        self.property_agreement
            .iter()
            .find(|(_, referenced)| referenced.as_str() == ID)
            .map(|(referring, _)| referring.as_str())
    }

    /// Why the referring side of this declaration, on a property of
    /// `declaring`, cannot name the document holding the list; `None` when
    /// it can. The `$id` pair's referring side must be an identifier property
    /// of the declaring type (a schema property, not the writer: no document
    /// has the writer's id; not a typed array: one document holds the list),
    /// and it must be stored (it and every object around it not transient),
    /// so a reader can tell from the stored document which list the value was
    /// checked against. It may be optional: a value set while it is not is
    /// refused when the document is written. It needs no `refersTo` of its
    /// own, but one it carries must be a reference by id to
    /// `document_type_name` in the list's contract, or the pair could never
    /// hold. The other pairs are checked as every agreement is, at
    /// registration.
    pub fn referring_side_error(&self, declaring: DocumentTypeRef) -> Option<String> {
        let Some(document_id_property) = self.document_id_property() else {
            return Some(
                "propertyAgreement must hold exactly one pair with $id on the referenced side, \
                 naming the property whose value is the id of the document holding the list"
                    .to_string(),
            );
        };
        if document_id_property.starts_with('$') {
            return Some(format!(
                "the $id pair reads \"{document_id_property}\": it must read an identifier \
                 property of the referring document type, which holds the id of the document \
                 holding the list"
            ));
        }
        let Some(property) = declaring.flattened_properties().get(document_id_property) else {
            return Some(format!(
                "the $id pair reads \"{document_id_property}\", which is not a property of the \
                 referring document type"
            ));
        };
        if !matches!(
            property.property_type,
            DocumentPropertyType::Identifier | DocumentPropertyType::IdentifierWithReference(_)
        ) {
            return Some(format!(
                "the $id pair reads \"{document_id_property}\", which is not an identifier \
                 property"
            ));
        }
        if is_transient(declaring, document_id_property) {
            return Some(format!(
                "the $id pair reads \"{document_id_property}\", which is transient: the stored \
                 document must name the document whose list the value was checked against"
            ));
        }
        // A reference of its own must agree that the value is the id of a
        // document of the list's type in the list's contract: anything else
        // (an identity, a lookup key part, a list element, another type or
        // contract, an expression) holds a value no such document has, and
        // every write setting the list element would be refused
        if let DocumentPropertyType::IdentifierWithReference(target) = &property.property_type {
            let declaring_contract_id = declaring.data_contract_id();
            let names_the_list_document = target.as_document_reference().is_some_and(|reference| {
                reference.document_type_name == self.document_type_name
                    && reference.contract_id.unwrap_or(declaring_contract_id)
                        == self.contract_id.unwrap_or(declaring_contract_id)
            });
            if !names_the_list_document {
                return Some(format!(
                    "the $id pair reads \"{document_id_property}\", whose refersTo is not a \
                     reference by id to \"{}\" in the list's contract: its value could never be \
                     the id of the document holding the list",
                    self.document_type_name
                ));
            }
        }
        None
    }

    /// Why `referenced`, the document type holding the list, cannot hold it;
    /// `None` when it can. Its documents must never be deleted, `in_list`
    /// must be a stored typed array of identifiers of it (it and every object
    /// around it not transient), and the list must be fixed once a document
    /// is written (see `schema_property_is_fixed_once_written`, the rule a
    /// lookup's key parts are judged by), so a value accepted once stays an
    /// element.
    pub fn referenced_side_error(&self, referenced: DocumentTypeRef) -> Option<String> {
        let referenced_name = referenced.name();
        let list = &self.in_list;
        if referenced.documents_can_be_deleted()
            || referenced.documents_can_be_deleted_by_moderators()
        {
            return Some(format!(
                "documents of \"{referenced_name}\" can be deleted: the list must be held by a \
                 document that never is"
            ));
        }
        let Some(property) = referenced.flattened_properties().get(list) else {
            return Some(format!("\"{referenced_name}\" has no property \"{list}\""));
        };
        let holds_identifiers = match &property.property_type {
            DocumentPropertyType::TypedArray(typed_array) => matches!(
                typed_array.item_type.as_ref(),
                DocumentPropertyType::Identifier | DocumentPropertyType::IdentifierWithReference(_)
            ),
            _ => false,
        };
        if !holds_identifiers {
            return Some(format!(
                "\"{list}\" of \"{referenced_name}\" is not a typed array of identifiers"
            ));
        }
        if is_transient(referenced, list) {
            return Some(format!(
                "\"{list}\" of \"{referenced_name}\" is transient, so no stored document holds it"
            ));
        }
        if !schema_property_is_fixed_once_written(referenced, list) {
            let top_level = list.split('.').next().unwrap_or(list);
            return Some(format!(
                "\"{list}\" of \"{referenced_name}\" can be changed by a replace: the list must \
                 be fixed once the document is written, so a value accepted as an element stays \
                 one (make the type immutable or list \"{top_level}\" under `immutable`)"
            ));
        }
        None
    }

    /// The identifiers the list holds on the referenced document whose
    /// properties are `referenced_properties`, collected once so each value
    /// checked against them is a set lookup rather than a scan of the list.
    /// An absent list holds nothing.
    pub fn listed_values(
        &self,
        referenced_properties: &BTreeMap<String, Value>,
    ) -> BTreeSet<[u8; 32]> {
        match referenced_properties.get_optional_at_path(&self.in_list) {
            Ok(Some(Value::Array(elements))) => elements
                .iter()
                .filter_map(|element| element.to_hash256().ok())
                .collect(),
            _ => BTreeSet::new(),
        }
    }
}

impl std::fmt::Display for ListElementReference {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "list element ({} of the ", self.in_list)?;
        if let Some(contract_id) = self.contract_id {
            write!(f, "contract {contract_id} ")?;
        }
        write!(f, "{} document", self.document_type_name)?;
        if let Some(document_id_property) = self.document_id_property() {
            write!(f, " {document_id_property} names")?;
        }
        write!(f, ")")
    }
}
