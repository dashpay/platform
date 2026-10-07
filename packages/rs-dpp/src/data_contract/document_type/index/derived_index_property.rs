//! Derived index properties (protocol version 14).
//!
//! An index of a stored document type may name a value of the document one of
//! its references points at, as `"<reference property>.<field>"`: a reply
//! indexed by `postId.$ownerId` is filed under the owner of the post it
//! replies to, and the reply never stores that owner. Drive reads the value
//! from the referenced document whenever it writes or removes the reply's
//! index entries.
//!
//! That is sound only while the value Drive reads is the one the entry was
//! written under, so the parser admits a derived property only where nothing
//! can change or lose it:
//!
//! - the reference is a same-contract `permanentDocument` or
//!   `moderatedDocument` one, by id, on a top-level identifier the referring
//!   type fixes once written;
//! - the field is fixed once written on the referenced type: `$creatorId`,
//!   `$ownerId` of a type whose documents never change hands, or a schema
//!   property no replace or moderator can change;
//! - through a `moderatedDocument` reference, only what a moderator's removal
//!   record keeps of the removed document: its `$ownerId`, and a schema
//!   property its type lists under `moderatorAbilities.deleteKeepsFields` (or
//!   one inside an object listed there).

use crate::data_contract::document_type::property::{
    DocumentPropertyReferenceTarget, DocumentPropertyType, DocumentReferenceKind,
};
use crate::data_contract::document_type::DocumentProperty;
use crate::document::{Document, DocumentV0Getters};
use crate::ProtocolError;
use indexmap::IndexMap;
use platform_value::btreemap_extensions::BTreeValueMapPathHelper;
use platform_value::{Identifier, Value};

/// The field of the referenced document a derived index property reads.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DerivedIndexField {
    /// `$ownerId`: the referenced document's owner.
    OwnerId,
    /// `$creatorId`: the identity that created the referenced document, on a
    /// type that records it.
    CreatorId,
    /// A schema property of the referenced document type, by its dotted path.
    Property(String),
}

impl DerivedIndexField {
    /// The value this field holds on `referenced`, the document a reference points at:
    /// `Value::Null` when it holds none. Shared by the reference validation of a create, which
    /// has the document fetched already, and Drive, which reads it to key a document.
    pub fn value_in(&self, referenced: &Document) -> Result<Value, ProtocolError> {
        Ok(match self {
            DerivedIndexField::OwnerId => Value::Identifier(referenced.owner_id().to_buffer()),
            DerivedIndexField::CreatorId => referenced
                .creator_id()
                .map(|creator_id| Value::Identifier(creator_id.to_buffer()))
                .unwrap_or(Value::Null),
            DerivedIndexField::Property(path) => referenced
                .properties()
                .get_optional_at_path(path)?
                .cloned()
                .unwrap_or(Value::Null),
        })
    }
}

/// An index property whose value is read from the document a reference of the
/// document type points at, declared as `"<reference property>.<field>"`.
#[derive(Debug, Clone, PartialEq)]
pub struct DerivedIndexProperty {
    /// The top-level identifier property holding the referenced document's id.
    pub reference_property: String,
    /// The document type of the declaring contract the reference points into.
    pub referenced_document_type_name: String,
    /// `permanentDocument` or `moderatedDocument`: whether a moderator's
    /// removal can take the referenced document out of state, leaving its
    /// removal record to read the owner and the kept values from.
    pub kind: DocumentReferenceKind,
    /// The field of the referenced document the index holds.
    pub field: DerivedIndexField,
    /// The field's type: an identifier for `$ownerId` and `$creatorId`; for a
    /// schema property, its type on the referenced document type, which only
    /// the parse of the whole contract can see (`None` until then).
    pub property_type: Option<DocumentPropertyType>,
}

/// Why an index property name reading through a reference can not be a
/// derived index property, or the parts of one it declares.
pub(crate) enum DerivedIndexPropertyName {
    /// The name does not read through a reference: its first segment is not a
    /// top-level identifier property carrying a `refersTo`.
    NotDerived,
    /// The name reads through a reference that can not carry a derived
    /// property.
    Refused(String),
    /// The derived property the name declares.
    Derived(Box<DerivedIndexProperty>),
}

/// Whether the index property `name` reads through a reference: its first
/// segment names a top-level identifier property carrying a `refersTo`, so it
/// declares a derived index property, or one refused as such
/// ([`parse_derived_index_property_name`]), and never a property of the type.
pub(crate) fn reads_through_reference(
    name: &str,
    flattened_properties: &IndexMap<String, DocumentProperty>,
) -> bool {
    name.split_once('.').is_some_and(|(reference_property, _)| {
        matches!(
            flattened_properties.get(reference_property),
            Some(DocumentProperty {
                property_type: DocumentPropertyType::IdentifierWithReference(_),
                ..
            })
        )
    })
}

/// What an index property name declares when its first segment names a
/// top-level identifier property carrying a `refersTo`: a derived index
/// property, or a refusal saying why that reference can not carry one. Every
/// other name is [`DerivedIndexPropertyName::NotDerived`], to be resolved as a
/// property of the type.
///
/// A property name is word characters only, so a name with a `.` after an
/// identifier can never be a property path: an identifier holds no nested
/// property.
pub(crate) fn parse_derived_index_property_name(
    name: &str,
    flattened_properties: &IndexMap<String, DocumentProperty>,
    data_contract_id: Identifier,
) -> DerivedIndexPropertyName {
    let Some((reference_property, field)) = name.split_once('.') else {
        return DerivedIndexPropertyName::NotDerived;
    };
    let Some(DocumentProperty {
        property_type: DocumentPropertyType::IdentifierWithReference(target),
        ..
    }) = flattened_properties.get(reference_property)
    else {
        return DerivedIndexPropertyName::NotDerived;
    };
    let Some(declaration) = target.as_any_document_reference() else {
        let form = match target {
            DocumentPropertyReferenceTarget::AnyOf(_)
            | DocumentPropertyReferenceTarget::AllOf(_) => "a reference expression",
            _ => "a reference to something other than a document",
        };
        return DerivedIndexPropertyName::Refused(format!(
            "\"{reference_property}\" is {form}: a derived index property reads through a \
             permanentDocument or moderatedDocument reference by id"
        ));
    };
    if declaration.lookup.is_some() || declaration.in_list.is_some() {
        return DerivedIndexPropertyName::Refused(format!(
            "\"{reference_property}\" finds its document with findBy: a derived index property \
             reads through a reference whose value is the document's id"
        ));
    }
    if declaration
        .contract_id
        .is_some_and(|contract_id| contract_id != data_contract_id)
    {
        return DerivedIndexPropertyName::Refused(format!(
            "\"{reference_property}\" refers into another contract: a derived index property \
             reads a document of its own contract"
        ));
    }
    let kind = declaration.kind;
    if kind == DocumentReferenceKind::Deletable {
        return DerivedIndexPropertyName::Refused(format!(
            "\"{reference_property}\" is a deletableDocument reference: its document can leave \
             state without a record, and the index entry could then no longer be found; a \
             derived index property reads through a permanentDocument or moderatedDocument \
             reference"
        ));
    }
    let (field, property_type) = match field {
        "$ownerId" => (
            DerivedIndexField::OwnerId,
            Some(DocumentPropertyType::Identifier),
        ),
        "$creatorId" if kind == DocumentReferenceKind::Moderated => {
            return DerivedIndexPropertyName::Refused(format!(
                "\"{reference_property}\" is a moderatedDocument reference, whose document a \
                 moderator's removal replaces with a record keeping its owner and the fields \
                 its type lists under `moderatorAbilities.deleteKeepsFields`, never its \
                 creator: through it a derived index property reads $ownerId or a kept field"
            ))
        }
        "$creatorId" => (
            DerivedIndexField::CreatorId,
            Some(DocumentPropertyType::Identifier),
        ),
        "$id" => {
            return DerivedIndexPropertyName::Refused(format!(
                "the referenced document's $id is \"{reference_property}\" itself: index \
                 \"{reference_property}\""
            ))
        }
        system if system.starts_with('$') => {
            return DerivedIndexPropertyName::Refused(format!(
                "a derived index property reads $ownerId, $creatorId or a schema property of \
                 the referenced document, not {system}"
            ))
        }
        // Through a moderatedDocument reference, whether the removal record keeps it is
        // judged where the referenced type is in hand (`resolve_derived_index_properties`).
        path => (DerivedIndexField::Property(path.to_string()), None),
    };
    DerivedIndexPropertyName::Derived(Box::new(DerivedIndexProperty {
        reference_property: reference_property.to_string(),
        referenced_document_type_name: declaration.document_type_name.to_string(),
        kind,
        field,
        property_type,
    }))
}
