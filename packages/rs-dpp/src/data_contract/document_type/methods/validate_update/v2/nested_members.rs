//! What an update keeps of every object of a document type, at every depth.
//!
//! A stored document is read with the type as it is now. Inside an object the members are
//! stored one after another in the order the type parses them (the schema's map order, not
//! `position`), a member required at every contract version without the presence byte an
//! optional one (or one required from a contract version) carries, so an object's members must
//! keep their place and their requiredness, and a member may only be added after them. Its requiredness is what the type's parse makes of it, whatever made it so: the
//! object's own `required`, a `required` beside its `$ref`, a dotted name in an enclosing
//! `required`, or the parser reading a required name of an enclosing property as naming it (a
//! name made of the object's name, any one character and the member's name). These rules compare
//! the parsed types, the trees the encoder reads, and the `required` lists the document
//! validator reads.
//!
//! Every walk follows the types' member order (`IndexMap`) and collects names in `BTreeSet`s,
//! and every schema read is infallible, so a schema's shape never becomes an error.

use crate::consensus::basic::data_contract::DataContractInvalidRequiredFieldsUpdateError;
use crate::consensus::state::data_contract::document_type_update_error::DocumentTypeUpdateError;
use crate::data_contract::document_type::accessors::DocumentTypeV0Getters;
use crate::data_contract::document_type::property::{DocumentProperty, DocumentPropertyType};
use crate::data_contract::document_type::property_names::{PROPERTIES, REF, REQUIRED};
use crate::data_contract::document_type::DocumentTypeRef;
use crate::validation::SimpleConsensusValidationResult;
use indexmap::IndexMap;
use platform_value::{Value, ValueMapHelper};
use std::collections::BTreeSet;

impl DocumentTypeRef<'_> {
    /// Every object of the type keeps its members in their order, and the members an update adds
    /// come after them: a document stored before reads the added members as absent. The type's
    /// own properties are held by their frozen, gapless positions, and are checked here too. A
    /// member removed after the others is left to the schema compatibility check, which refuses
    /// every removal.
    pub(super) fn validate_member_order_kept(
        &self,
        new_document_type: DocumentTypeRef,
    ) -> SimpleConsensusValidationResult {
        let Some(reordered) =
            first_reordered_member_list(self.properties(), new_document_type.properties(), None)
        else {
            return SimpleConsensusValidationResult::new();
        };
        SimpleConsensusValidationResult::new_with_error(
            DocumentTypeUpdateError::new(
                self.data_contract_id(),
                self.name(),
                format!(
                    "a document type can only add properties after its existing ones: {} would \
                     no longer start with its existing properties in their order ({}), so the \
                     documents stored before would read wrong; inside an object the stored order \
                     is the schema's map order, so an added member goes after the others there",
                    reordered.place, reordered.stored_order
                ),
            )
            .into(),
        )
    }

    /// The `required` lists the document validator reads keep what they hold, as the
    /// type's own `required` does: in the list of every object the old type has (its own list,
    /// or the one beside its `$ref`) no name is removed, and a name is added only for a member
    /// the same update adds to that object. A dotted name is never one of an object's own
    /// members, so none is added to any list, the type's own included, nor written in the list
    /// of an object the update adds: the validator reads it as a property named with a dot,
    /// which no document can hold, while the parser reads it as a nested requirement. One may leave a list when another list still requires the member it
    /// names, which leaves the parse as it was and lets the validator accept documents again.
    /// The type's own undotted names are judged by generation 1's required-fields check.
    pub(super) fn validate_required_lists_kept(
        &self,
        new_document_type: DocumentTypeRef,
    ) -> SimpleConsensusValidationResult {
        match first_required_list_change(
            self.properties(),
            new_document_type.properties(),
            self.schema(),
            new_document_type.schema(),
            None,
            new_document_type.required_fields(),
        ) {
            Some(details) => SimpleConsensusValidationResult::new_with_error(
                DataContractInvalidRequiredFieldsUpdateError::new(self.name().clone(), details)
                    .into(),
            ),
            None => SimpleConsensusValidationResult::new(),
        }
    }

    /// Every member an object keeps stays required or optional as it was, and every member an
    /// update adds is required exactly when a `required` list names it by its path, so the
    /// document validator holds every document to it. A member added to an object the type
    /// already has may be required only from the contract version the update creates
    /// (`requiredSince`): documents stored before do not hold it. Inside an object the update
    /// adds, no stored document holds the object, so its members may be required outright, or
    /// from any version the contract has reached (as a member of a shared `$defs` definition
    /// carries it): a member with `requiredSince` is stored with a presence byte at every version,
    /// so the version it names never decides how a document is read.
    pub(super) fn validate_member_requiredness_kept(
        &self,
        new_document_type: DocumentTypeRef,
        new_contract_version: u32,
    ) -> SimpleConsensusValidationResult {
        let added = AddedMembers {
            required_fields: new_document_type.required_fields(),
            new_contract_version,
        };
        match first_requiredness_change(
            self.properties(),
            new_document_type.properties(),
            None,
            &added,
        ) {
            Some(details) => SimpleConsensusValidationResult::new_with_error(
                DataContractInvalidRequiredFieldsUpdateError::new(self.name().clone(), details)
                    .into(),
            ),
            None => SimpleConsensusValidationResult::new(),
        }
    }
}

/// A member list of the new type that does not start with the old one's members in order.
struct ReorderedMemberList {
    /// `the type`, or `the object <path>`.
    place: String,
    /// The old members, in their stored order.
    stored_order: String,
}

/// The first member list of `new` whose members do not start with `old`'s in their order, as
/// far as both go: the type's own properties when `path` is `None`, else those of the object at
/// `path`, searched at every depth. `None` when every pair of lists agrees on the members they
/// both hold at each place, a longer `new` adding members after `old`'s and a shorter one
/// removing them from the end.
fn first_reordered_member_list(
    old: &IndexMap<String, DocumentProperty>,
    new: &IndexMap<String, DocumentProperty>,
    path: Option<&str>,
) -> Option<ReorderedMemberList> {
    let keeps_order = old.keys().zip(new.keys()).all(|(old, new)| old == new);
    if !keeps_order {
        return Some(ReorderedMemberList {
            place: path.map_or_else(
                || "the type".to_string(),
                |path| format!("the object {path}"),
            ),
            stored_order: old
                .keys()
                .map(String::as_str)
                .collect::<Vec<_>>()
                .join(", "),
        });
    }
    old.iter().find_map(|(name, old_property)| {
        let (
            DocumentPropertyType::Object(old_members),
            Some(DocumentPropertyType::Object(new_members)),
        ) = (
            &old_property.property_type,
            new.get(name).map(|property| &property.property_type),
        )
        else {
            return None;
        };
        first_reordered_member_list(old_members, new_members, Some(&member_path(path, name)))
    })
}

/// What a member an update adds is judged against.
struct AddedMembers<'a> {
    /// Every name the new type's `required` lists hold, by path: what the document validator
    /// holds a document to, and what the parser reads except where it reads a required name of
    /// an enclosing property as naming a member.
    required_fields: &'a BTreeSet<String>,
    new_contract_version: u32,
}

/// The first requiredness change below the object at `path` (`None`: the type's own
/// properties, whose requiredness generation 1 judges), as the details of the refusal.
fn first_requiredness_change(
    old: &IndexMap<String, DocumentProperty>,
    new: &IndexMap<String, DocumentProperty>,
    path: Option<&str>,
    added: &AddedMembers,
) -> Option<String> {
    for (name, new_property) in new {
        let member_path = member_path(path, name);
        match old.get(name) {
            Some(old_property) => {
                if path.is_some() && old_property.required != new_property.required {
                    return Some(if new_property.required {
                        format!("existing property '{member_path}' cannot become required")
                    } else {
                        format!("removed required field '{member_path}'")
                    });
                }
                if let (
                    DocumentPropertyType::Object(old_members),
                    DocumentPropertyType::Object(new_members),
                ) = (&old_property.property_type, &new_property.property_type)
                {
                    if let Some(change) = first_requiredness_change(
                        old_members,
                        new_members,
                        Some(&member_path),
                        added,
                    ) {
                        return Some(change);
                    }
                }
            }
            None if path.is_some() => {
                if let Some(change) = added_member_error(new_property, &member_path, true, added) {
                    return Some(change);
                }
            }
            None => {
                // A property the type itself adds: its own requiredness is generation 1's to
                // judge, the members of an object it holds are judged here
                if let DocumentPropertyType::Object(members) = &new_property.property_type {
                    if let Some(change) = first_added_members_error(members, &member_path, added) {
                        return Some(change);
                    }
                }
            }
        }
    }
    None
}

/// Why the member `property` the update adds at `path` is refused, if it is: `in_kept_object`
/// when its object holds stored documents.
fn added_member_error(
    property: &DocumentProperty,
    path: &str,
    in_kept_object: bool,
    added: &AddedMembers,
) -> Option<String> {
    let new_contract_version = added.new_contract_version;
    if property.required != added.required_fields.contains(path) {
        return Some(format!(
            "added property '{path}' would be stored as required without a required list naming \
             it, so documents could leave it out and then not be stored: the parser reads a \
             required name made of an enclosing property's name, any one character and the \
             member's name as naming the member; rename one of them"
        ));
    }
    if property.required && in_kept_object && property.required_since != Some(new_contract_version)
    {
        return Some(format!(
            "new required field '{path}' must carry requiredSince {new_contract_version}, the \
             contract version this update creates"
        ));
    }
    match &property.property_type {
        DocumentPropertyType::Object(members) => first_added_members_error(members, path, added),
        _ => None,
    }
}

/// The first member of an object the update adds at `path` that is refused.
fn first_added_members_error(
    members: &IndexMap<String, DocumentProperty>,
    path: &str,
    added: &AddedMembers,
) -> Option<String> {
    members.iter().find_map(|(name, member)| {
        added_member_error(member, &member_path(Some(path), name), false, added)
    })
}

/// The first change to a `required` list of an object the old type has, below the object at
/// `path` (`None`: the type itself, whose dotted names only are judged here). `old_schema` and
/// `new_schema` are the schemas of that object as the document type schema writes them; the
/// walk stops at a `$ref`, whose definition's own lists the parser does not read and the
/// contract's `$defs` compatibility freezes. `new_required_fields` holds every path the new
/// type's lists require.
fn first_required_list_change(
    old: &IndexMap<String, DocumentProperty>,
    new: &IndexMap<String, DocumentProperty>,
    old_schema: &Value,
    new_schema: &Value,
    path: Option<&str>,
    new_required_fields: &BTreeSet<String>,
) -> Option<String> {
    let (old_names, new_names) = (required_names(old_schema), required_names(new_schema));
    let list = path.map_or_else(
        || "the document type's required".to_string(),
        |path| format!("the required of '{path}'"),
    );
    if let Some(dotted) = new_names
        .difference(&old_names)
        .find(|name| name.contains('.'))
    {
        return Some(format!(
            "{list} can not gain the dotted name '{dotted}': a required list names the members \
             of its own object, and the document validator reads a dotted name as a property \
             named with a dot, which no document can hold"
        ));
    }
    if let Some(dotted) = old_names
        .difference(&new_names)
        .find(|name| name.contains('.') && !new_required_fields.contains(&member_path(path, name)))
    {
        return Some(format!(
            "{list} can not lose the dotted name '{dotted}' while no other required list names \
             the member: the type would stop requiring a member it stores as required"
        ));
    }
    if let Some(path) = path {
        if let Some(removed) = old_names
            .difference(&new_names)
            .find(|name| !name.contains('.'))
        {
            return Some(format!("removed required field '{path}.{removed}'"));
        }
        for added in new_names.difference(&old_names) {
            if old.contains_key(*added) {
                return Some(format!(
                    "existing property '{path}.{added}' cannot become required"
                ));
            }
            if !new.contains_key(*added) {
                return Some(format!(
                    "added required field '{path}.{added}' references an unknown property"
                ));
            }
        }
    }
    for (name, old_property) in old {
        let (
            DocumentPropertyType::Object(old_members),
            Some(DocumentPropertyType::Object(new_members)),
        ) = (
            &old_property.property_type,
            new.get(name).map(|property| &property.property_type),
        )
        else {
            continue;
        };
        let (Some(old_member_schema), Some(new_member_schema)) = (
            member_schema(old_schema, name),
            member_schema(new_schema, name),
        ) else {
            continue;
        };
        if let Some(change) = first_required_list_change(
            old_members,
            new_members,
            old_member_schema,
            new_member_schema,
            Some(&member_path(path, name)),
            new_required_fields,
        ) {
            return Some(change);
        }
    }
    // An object the update adds holds no stored document, so its lists are its own to choose,
    // except a dotted name, which would leave the object unable to hold any document
    for (name, new_property) in new {
        if old.contains_key(name)
            || !matches!(new_property.property_type, DocumentPropertyType::Object(_))
        {
            continue;
        }
        if let Some(added_schema) = member_schema(new_schema, name) {
            if let Some(change) =
                first_dotted_name_in_added_object(added_schema, &member_path(path, name))
            {
                return Some(change);
            }
        }
    }
    None
}

/// The first dotted name in a `required` list of the object the update adds at `path`, whose
/// schema is `schema`, or of an object inside it written inline.
fn first_dotted_name_in_added_object(schema: &Value, path: &str) -> Option<String> {
    if let Some(dotted) = required_names(schema)
        .into_iter()
        .find(|name| name.contains('.'))
    {
        return Some(format!(
            "the required of '{path}' can not name the dotted name '{dotted}': a required list \
             names the members of its own object, and the document validator reads a dotted \
             name as a property named with a dot, which no document can hold"
        ));
    }
    let members = schema
        .as_map()
        .filter(|schema| schema.get_optional_key(REF).is_none())
        .and_then(|schema| schema.get_optional_key(PROPERTIES))
        .and_then(Value::as_map)?;
    members.iter().find_map(|(name, member)| {
        first_dotted_name_in_added_object(member, &member_path(Some(path), name.as_text()?))
    })
}

fn member_path(path: Option<&str>, name: &str) -> String {
    path.map_or_else(|| name.to_string(), |path| format!("{path}.{name}"))
}

/// The schema of the member `name` of the object `schema` describes inline, `None` when it
/// describes none or takes its members from a `$ref`. A key named twice reads its first entry:
/// a contract updated from protocol version 15 names no key twice in either schema (contract
/// update generation 2 and the registration check refuse it).
fn member_schema<'a>(schema: &'a Value, name: &str) -> Option<&'a Value> {
    let schema = schema.as_map()?;
    if schema.get_optional_key(REF).is_some() {
        return None;
    }
    schema
        .get_optional_key(PROPERTIES)?
        .as_map()?
        .get_optional_key(name)
}

/// The names of the `required` list of `schema`, entries that are not text left out as the
/// parser leaves them out.
fn required_names(schema: &Value) -> BTreeSet<&str> {
    schema
        .as_map()
        .and_then(|schema| schema.get_optional_key(REQUIRED))
        .and_then(Value::as_array)
        .map(|names| names.iter().filter_map(Value::as_text).collect())
        .unwrap_or_default()
}
