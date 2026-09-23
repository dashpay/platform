//! The `listElement` target of a `refersTo` declaration: the value must be an
//! element of a typed array of identifiers held by the document another
//! property of the same document refers to.
//!
//! Declared on an identifier property, or on the `items` of a typed array of
//! identifiers, where every element must be one (meta-schema v3, protocol
//! version 14):
//!
//! ```json
//! "refersTo": {
//!   "type": "listElement",
//!   "documentType": "electedCharter",
//!   "documentProperty": "electedCharterId",
//!   "list": "members"
//! }
//! ```
//!
//! reads: the value must be one of the `members` of the `electedCharter`
//! document this document's `electedCharterId` refers to. `electedCharterId`
//! carries a `permanentDocument` reference (by id or through a `lookup`) to
//! `electedCharter`, so consensus already fetches that document to validate
//! it, and the list check reads the document in hand: no further read. The
//! referenced document can never be deleted and its list never changes (the
//! type is immutable or lists the property under `immutable`), so a value
//! validated once stays an element for good. The rules live here so the two
//! places that check a declaration against its referenced document type (the
//! contract parse for a type of the same contract, the registration state
//! validation for a type of another contract) cannot drift.

use crate::data_contract::document_type::accessors::DocumentTypeV0Getters;
use crate::data_contract::document_type::accessors::DocumentTypeV2Getters;
use crate::data_contract::document_type::property::DocumentPropertyType;
use crate::data_contract::document_type::DocumentTypeRef;
use bincode::{Decode, DecodeUntrusted, Encode};
use platform_value::btreemap_extensions::BTreeValueMapPathHelper;
use platform_value::{Identifier, Value};
use serde::Serialize;
use std::collections::BTreeMap;

/// The longest `documentProperty` or `list` path a declaration may name, the
/// bound meta-schema v3 puts on property paths.
pub const MAX_LIST_ELEMENT_PATH_LENGTH: usize = 256;

/// A `refersTo: listElement` declaration: the value (or each element of a
/// typed array) must be an element of the typed array `list` of the
/// `document_type_name` document that `document_property` refers to.
#[derive(Debug, PartialEq, Eq, Clone, Serialize, Encode, Decode, DecodeUntrusted)]
pub struct ListElementReference {
    /// The document type holding the list, the one `document_property`'s
    /// `permanentDocument` reference names (in whichever contract that
    /// reference names: the declaration takes no contract of its own).
    pub document_type_name: String,
    /// The identifier property of the referring document type (a dotted path
    /// when nested) whose `permanentDocument` reference finds the document
    /// holding the list.
    pub document_property: String,
    /// The typed array of identifiers of `document_type_name` (a dotted path
    /// when nested) the value must be an element of.
    pub list: String,
}

impl ListElementReference {
    /// The contract holding the list's document type: the one
    /// `document_property`'s `permanentDocument` reference in `declaring`
    /// names, `declaring_contract_id` when it names none. `None` when that
    /// property carries no such reference, which registration refuses (see
    /// [`Self::referring_side_error`]).
    pub fn list_contract_id(
        &self,
        declaring: DocumentTypeRef,
        declaring_contract_id: Identifier,
    ) -> Option<Identifier> {
        let property = declaring
            .flattened_properties()
            .get(&self.document_property)?;
        let DocumentPropertyType::IdentifierWithReference(target) = &property.property_type else {
            return None;
        };
        let declaration = target.as_any_document_reference()?;
        (declaration.permanent && declaration.document_type_name == self.document_type_name)
            .then(|| declaration.contract_id.unwrap_or(declaring_contract_id))
    }

    /// Why the referring side of this declaration, on a property of
    /// `declaring`, cannot name the document holding the list; `None` when
    /// it can. `document_property` must be an identifier property of the
    /// declaring type (not a typed array: one document holds the list)
    /// carrying a `permanentDocument` reference, by id or through a
    /// `lookup`, to `document_type_name`, and it must be stored (not
    /// transient), so a reader can tell from the stored document which list
    /// the value was checked against. It may be optional: a value set while
    /// it is not is refused when the document is written.
    pub fn referring_side_error(&self, declaring: DocumentTypeRef) -> Option<String> {
        let document_property = &self.document_property;
        let Some(property) = declaring.flattened_properties().get(document_property) else {
            return Some(format!(
                "documentProperty \"{document_property}\" is not a property of the referring \
                 document type"
            ));
        };
        let declaration = match &property.property_type {
            DocumentPropertyType::IdentifierWithReference(target) => {
                target.as_any_document_reference()
            }
            _ => None,
        };
        let Some(declaration) = declaration.filter(|declaration| declaration.permanent) else {
            return Some(format!(
                "documentProperty \"{document_property}\" must be an identifier property \
                 carrying a permanentDocument refersTo: the list is read from the document it \
                 refers to, which must never be deleted"
            ));
        };
        if declaration.document_type_name != self.document_type_name {
            return Some(format!(
                "documentProperty \"{document_property}\" refers to document type \"{}\", not \
                 \"{}\"",
                declaration.document_type_name, self.document_type_name
            ));
        }
        if declaring.transient_fields().contains(document_property) {
            return Some(format!(
                "documentProperty \"{document_property}\" is transient: the stored document must \
                 name the document whose list the value was checked against"
            ));
        }
        None
    }

    /// Why `referenced`, the document type `document_property` refers to,
    /// cannot hold the list; `None` when it can. Its documents must never be
    /// deleted, `list` must be a stored typed array of identifiers of it,
    /// and the list must be fixed once a document is written: the type is
    /// immutable (`documentsMutable: false`) or lists the list's top-level
    /// property under `immutable` (an `immutableAllowSetting` entry can only
    /// be set on a document that has no value for it, against which no
    /// value was ever accepted). Every flag read here is immutable on
    /// contract update and the `immutable` list may only grow, so the answer
    /// holds for good, and a value accepted once stays an element.
    pub fn referenced_side_error(&self, referenced: DocumentTypeRef) -> Option<String> {
        let referenced_name = referenced.name();
        let list = &self.list;
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
        if referenced.transient_fields().contains(list) {
            return Some(format!(
                "\"{list}\" of \"{referenced_name}\" is transient, so no stored document holds it"
            ));
        }
        let top_level = list.split('.').next().unwrap_or(list);
        if referenced.documents_mutable() && !referenced.immutable_fields().contains(top_level) {
            return Some(format!(
                "\"{list}\" of \"{referenced_name}\" can be changed by a replace: the list must \
                 be fixed once the document is written, so a value accepted as an element stays \
                 one (make the type immutable or list \"{top_level}\" under `immutable`)"
            ));
        }
        None
    }

    /// Whether `value` is an element of the list held by the referenced
    /// document whose properties are `referenced_properties`. An absent
    /// list holds nothing. A scan of at most the list's `maxItems` elements.
    pub fn is_listed_in(
        &self,
        referenced_properties: &BTreeMap<String, Value>,
        value: &[u8; 32],
    ) -> bool {
        match referenced_properties.get_optional_at_path(&self.list) {
            Ok(Some(Value::Array(elements))) => elements
                .iter()
                .any(|element| element.to_hash256().is_ok_and(|element| &element == value)),
            _ => false,
        }
    }
}

impl std::fmt::Display for ListElementReference {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "list element ({} of the {} document {} refers to)",
            self.list, self.document_type_name, self.document_property
        )
    }
}
