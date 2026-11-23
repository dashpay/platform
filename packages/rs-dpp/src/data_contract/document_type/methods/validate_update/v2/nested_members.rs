//! What an update keeps of every object of a document type, at every depth.
//!
//! A stored document is read with the type as it is now. Inside an object the members are
//! stored one after another in the order the type parses them (the schema's map order, at every
//! protocol version), a member required at every contract version without the presence byte an
//! optional one (or one required from a contract version) carries, so an object's members must
//! keep their place and their requiredness, and a member may only be added after them. Contract
//! `validate_update` 2 also holds an added member to the object's next free position, and an
//! object the update adds to consecutive positions in map order (`MemberPositionOrder`), and
//! every object the contract gains to `required` lists a document can meet
//! ([`DocumentTypeRef::first_unmeetable_gained_requirement`], which registration runs too). Its
//! requiredness is what the type's parse makes of it, whatever made it so: the
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
use crate::data_contract::document_type::accessors::{
    DocumentTypeV0Getters, DocumentTypeV2Getters,
};
use crate::data_contract::document_type::property::{DocumentProperty, DocumentPropertyType};
use crate::data_contract::document_type::property_names::{
    ADDITIONAL_PROPERTIES, CONST, ENUM, MAX_PROPERTIES, PROPERTIES, REF, REQUIRED, TYPE,
};
use crate::data_contract::document_type::schema::DefinitionsRoot;
use crate::data_contract::document_type::{DocumentType, DocumentTypeRef};
use crate::validation::SimpleConsensusValidationResult;
use indexmap::IndexMap;
use platform_value::{Value, ValueMapHelper};
use platform_version::version::PlatformVersion;
use std::collections::{BTreeMap, BTreeSet};

impl DocumentTypeRef<'_> {
    /// Every object of the type keeps its members in their order (the schema's map order, the
    /// positions for the type's own properties), and the members an update adds come after them:
    /// a document stored before reads the added members as absent. A list that loses a member is
    /// left to the schema compatibility check, which refuses every removal.
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
    /// type's own `required` does (whose list may not grow past the type's `maxProperties`
    /// either): in the list of every object the old type has (its own list,
    /// or the one beside its `$ref`) no name is removed, and a name is added only for a member
    /// the same update adds to that object, or for one the type already parses as required
    /// through a dotted name, which repairs that requirement. A dotted name is never one of an
    /// object's own members, so none is added to any list, the type's own included: the
    /// validator reads it as a property named with a dot, which no document can hold, while the
    /// parser reads it as a nested requirement. One may leave a list when another list still
    /// requires the member it names, which leaves the parse as it was and lets the validator
    /// accept documents again. An entry written beside a `$ref` the type keeps may not be added
    /// for a member documents stored before hold. The type's own undotted names are judged by
    /// generation 1's required-fields check; the lists of an object the update adds, by
    /// [`Self::first_unmeetable_gained_requirement`].
    pub(super) fn validate_required_lists_kept(
        &self,
        new_document_type: DocumentTypeRef,
        platform_version: &PlatformVersion,
    ) -> SimpleConsensusValidationResult {
        match first_required_list_change(
            self.properties(),
            new_document_type.properties(),
            self.schema(),
            new_document_type.schema(),
            None,
            new_document_type.required_fields(),
        )
        .or_else(|| {
            type_list_past_max_properties(
                Some(self.schema()),
                new_document_type.schema(),
                platform_version,
            )
        }) {
            Some(details) => SimpleConsensusValidationResult::new_with_error(
                DataContractInvalidRequiredFieldsUpdateError::new(self.name().clone(), details)
                    .into(),
            ),
            None => SimpleConsensusValidationResult::new(),
        }
    }

    /// The first `required` list no document could meet, or member requiredness the lists do
    /// not give, in what the contract gains in this type, the type as a registration or an
    /// update writes it: `old` is the type as the contract stores it, `None` for a type the
    /// update adds and every type of a contract being registered. `definitions` are the
    /// contract's `$defs` as the registration or update writes them.
    ///
    /// An object the contract gains (in a type it has, at any depth, a definition it reaches
    /// through a `$ref` the type keeps included) holds no stored document, so its lists are its
    /// own to choose, except where no document could meet them. Every list the document
    /// validator reads for it, in each schema it applies to the object (the object as written,
    /// the schema its `$ref` points at, and the entry written for it beside an enclosing `$ref`),
    /// names only members of the object as the type parses it (a `$ref` site need not restate
    /// them), none a schema written for it with `additionalProperties: false` does not declare,
    /// and no dotted name; together they name no more members than the least `maxProperties` of
    /// those schemas lets the object hold. A member the type parses as anything but an object
    /// holds no members, and the lists written for it are left alone. A type the contract gains
    /// is held to that at every depth, and its own list names only its properties, the system
    /// timestamps the validator leaves out of the list aside, no more of them than its
    /// `maxProperties`; and each of its members is required exactly when a list names it by its
    /// path, as a member an update adds to a type is (`validate_member_requiredness_kept`).
    pub(crate) fn first_unmeetable_gained_requirement(
        &self,
        old: Option<DocumentTypeRef>,
        definitions: &DefinitionsRoot,
        new_contract_version: u32,
        platform_version: &PlatformVersion,
    ) -> Option<String> {
        let gained = GainedObjects {
            type_schema: self.schema(),
            definitions,
        };
        let Some(old) = old else {
            return first_unmeetable_type_list(*self, platform_version)
                .or_else(|| {
                    first_requiredness_change(
                        &IndexMap::new(),
                        self.properties(),
                        None,
                        &AddedMembers::of(self, new_contract_version),
                    )
                })
                .or_else(|| {
                    gained.first_unmeetable_list_below(
                        &IndexMap::new(),
                        self.properties(),
                        vec![self.schema()],
                        None,
                    )
                });
        };
        gained.first_unmeetable_list_below(
            old.properties(),
            self.properties(),
            vec![self.schema()],
            None,
        )
    }

    /// Every member an object keeps stays required or optional as it was, and keeps whether it
    /// is required at every contract version or from one (`always_required`), which decides
    /// whether it is stored with a presence byte; every member an
    /// update adds is required exactly when a `required` list names it by its path, so the
    /// document validator holds every document to it. A member added to an object the type
    /// already has may be required only from the contract version the update creates
    /// (`requiredSince`): documents stored before do not hold it. Inside an object the update
    /// adds, no stored document holds the object, so its members may be required outright, or
    /// from any version the contract has reached (as a member of a shared `$defs` definition
    /// carries it): a member with `requiredSince` is stored with a presence byte at every version,
    /// so the version it names never decides how a document is read. Under a property the type
    /// lists as `immutable`, with a condition or without, a member added to an existing object is
    /// optional: a replace sends the whole document again, so a document stored before would
    /// have to add the member, which changes the property it may not change. So it is in a type
    /// whose moderators change fields (`moderatorAbilities.changeFields`): a moderator's change
    /// checks the whole document again, and can not add the member to one stored before.
    pub(super) fn validate_member_requiredness_kept(
        &self,
        new_document_type: DocumentTypeRef,
        new_contract_version: u32,
    ) -> SimpleConsensusValidationResult {
        match first_requiredness_change(
            self.properties(),
            new_document_type.properties(),
            None,
            &AddedMembers::of(&new_document_type, new_contract_version),
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
/// both hold at each place, a longer `new` adding members after `old`'s, and when `new` lacks a
/// member of `old`: the schema compatibility check refuses that removal (10246).
fn first_reordered_member_list(
    old: &IndexMap<String, DocumentProperty>,
    new: &IndexMap<String, DocumentProperty>,
    path: Option<&str>,
) -> Option<ReorderedMemberList> {
    let removes_a_member = old.keys().any(|name| !new.contains_key(name));
    let keeps_order = removes_a_member || old.keys().zip(new.keys()).all(|(old, new)| old == new);
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
    /// The type's own properties the new type lists as `immutable`, with a condition or without.
    immutable: BTreeSet<&'a str>,
    /// Whether the type's moderators change fields of its documents
    /// (`moderatorAbilities.changeFields`).
    moderators_change_fields: bool,
    /// The new type's schema, as written.
    new_schema: &'a Value,
    new_contract_version: u32,
}

impl<'a> AddedMembers<'a> {
    /// What a member `new_document_type` adds is judged against, in the contract version
    /// `new_contract_version`.
    fn of(new_document_type: &'a DocumentTypeRef, new_contract_version: u32) -> Self {
        Self {
            required_fields: new_document_type.required_fields(),
            immutable: new_document_type
                .immutable_fields()
                .iter()
                .chain(new_document_type.immutable_field_conditions().keys())
                .map(String::as_str)
                .collect(),
            moderators_change_fields: !new_document_type.moderator_changeable_fields().is_empty(),
            new_schema: new_document_type.schema(),
            new_contract_version,
        }
    }
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
                // An object's member required at every contract version is stored without the
                // presence byte one required from a contract version carries
                if path.is_some()
                    && old_property.always_required() != new_property.always_required()
                {
                    return Some(format!(
                        "existing property '{member_path}' can not {} requiredSince: a member \
                         required at every contract version is stored without the presence byte \
                         one required from a contract version carries, so the documents stored \
                         before would read wrong",
                        if new_property.always_required() {
                            "lose"
                        } else {
                            "gain"
                        }
                    ));
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
    if property.required && in_kept_object {
        if property.required_since != Some(new_contract_version) {
            return Some(format!(
                "new required field '{path}' must carry requiredSince {new_contract_version}, the \
                 contract version this update creates"
            ));
        }
        // A document stored before must add the member on replace, so everything holding it
        // must let it do so
        if let Some(refusal) = holding_object_refusal(added.new_schema, path) {
            return Some(format!(
                "new required field '{path}' can not be added: {refusal}; add it as optional"
            ));
        }
        let top_level = path.split('.').next().unwrap_or(path);
        if added.immutable.contains(top_level) {
            return Some(format!(
                "new required field '{path}' can not be added under the immutable property \
                 '{top_level}': a document stored before would have to add it on replace, which \
                 changes '{top_level}'; add it as optional"
            ));
        }
        if added.moderators_change_fields {
            return Some(format!(
                "new required field '{path}' can not be added to a type whose moderators change \
                 fields of its documents: a moderator's change checks the whole document \
                 again, and one stored before does not hold the member, which the moderator \
                 can not add; add it as optional"
            ));
        }
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
/// `path` (`None`: the type itself, whose dotted names only are judged here; the lists of an
/// object the update adds are judged with the contract's `$defs` at hand,
/// [`DocumentTypeRef::first_unmeetable_gained_requirement`]). `old_schema` and
/// `new_schema` are the schemas of that object as the document type schema writes them; the
/// walk stops at a `$ref`: the list written beside it stays as it is, since the object takes its
/// members from a definition that other types may share, and the definition's own lists are not
/// read for storage and are frozen by the contract's `$defs` compatibility. A list may also not
/// grow past the object's `maxProperties`, which no update can change. `new_required_fields`
/// holds every path the new type's lists require.
fn first_required_list_change(
    old: &IndexMap<String, DocumentProperty>,
    new: &IndexMap<String, DocumentProperty>,
    old_schema: &Value,
    new_schema: &Value,
    path: Option<&str>,
    new_required_fields: &BTreeSet<String>,
) -> Option<String> {
    let (old_names, new_names) = (required_names(old_schema), required_names(new_schema));
    if let Some(path) = path {
        if (has_ref(old_schema) || has_ref(new_schema)) && old_names != new_names {
            return Some(format!(
                "the required written beside the $ref of '{path}' can not change: the object takes \
                 its members from a definition, which it gains only as optional"
            ));
        }
    }
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
            // A member the type already parses as required (through a dotted name in an
            // enclosing list) may be named here, which repairs a requirement the validator
            // could not read without changing how a document is stored
            if old.get(*added).is_some_and(|member| !member.required) {
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
        if new_names.len() > old_names.len() {
            if let Some(max_properties) = max_properties(new_schema) {
                if new_names.len() as u64 > max_properties {
                    return Some(format!(
                        "the required of '{path}' would name {} members, more than the \
                         {max_properties} its object may hold (maxProperties), so no document \
                         could hold the object",
                        new_names.len()
                    ));
                }
            }
        }
    }
    let (old_member_schemas, new_member_schemas) = (
        inline_member_schemas(old_schema),
        inline_member_schemas(new_schema),
    );
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
            old_member_schemas.get(name.as_str()),
            new_member_schemas.get(name.as_str()),
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
    // The `properties` written beside a `$ref` are not parsed, but the document validator reads
    // them, so an entry an update writes there, at any depth, holds no dotted name either
    if has_ref(new_schema) {
        return first_dotted_name_in_entries_added(
            old_schema,
            new_schema,
            Some(old),
            Parsed::Object(new),
            path,
        );
    }
    None
}

/// The first entry of `new_schema`'s `properties`, as written beside a `$ref` the type keeps,
/// that `old_schema`'s does not have and that is refused, at any depth below the entries both
/// have: one written for a member the stored type parses (`old_members`, the members of the
/// object the entries are written for, `new` what the new type parses there), which the
/// validator would hold every document stored before to, or one holding a `required` list no
/// document could meet. A schema written inline with another type than `object`, or written
/// for a member the new type parses as anything but an object, holds no members: the lists
/// written for it, and below it, name none.
fn first_dotted_name_in_entries_added(
    old_schema: &Value,
    new_schema: &Value,
    old_members: Option<&IndexMap<String, DocumentProperty>>,
    new: Parsed,
    path: Option<&str>,
) -> Option<String> {
    if is_inline_scalar(new_schema) || matches!(new, Parsed::NotAnObject) {
        return None;
    }
    let old_entries = properties_as_written(old_schema);
    properties_as_written(new_schema)
        .into_iter()
        .find_map(|(name, entry)| {
            let entry_path = member_path(path, name);
            let old_member = old_members.and_then(|old_members| old_members.get(name));
            match old_entries.get(name) {
                Some(old_entry) => first_dotted_name_in_entries_added(
                    old_entry,
                    entry,
                    object_members(old_member),
                    new.member(name),
                    Some(&entry_path),
                ),
                None if old_member.is_some() => Some(format!(
                    "an entry can not be written beside the $ref for '{entry_path}', a member \
                     documents stored before hold: the document validator would hold them to it"
                )),
                None => first_unmeetable_list_in_entry(entry, &entry_path, new.member(name)),
            }
        })
}

/// What the new type parses at the place an entry written beside a `$ref` is for.
#[derive(Clone, Copy)]
enum Parsed<'a> {
    /// An object, with these members.
    Object(&'a IndexMap<String, DocumentProperty>),
    /// A member the type parses as anything but an object, which holds no members.
    NotAnObject,
    /// No member the type parses: the entry is for a key the object does not have.
    Nothing,
}

impl<'a> Parsed<'a> {
    fn of(member: Option<&'a DocumentProperty>) -> Self {
        match member.map(|member| &member.property_type) {
            Some(DocumentPropertyType::Object(members)) => Parsed::Object(members),
            Some(_) => Parsed::NotAnObject,
            None => Parsed::Nothing,
        }
    }

    /// What the type parses for the member `name` of what it parses here.
    fn member(self, name: &str) -> Self {
        match self {
            Parsed::Object(members) => Parsed::of(members.get(name)),
            Parsed::NotAnObject | Parsed::Nothing => Parsed::Nothing,
        }
    }
}

/// The members of `member` when the type parses it as an object.
fn object_members(
    member: Option<&DocumentProperty>,
) -> Option<&IndexMap<String, DocumentProperty>> {
    match member.map(|member| &member.property_type) {
        Some(DocumentPropertyType::Object(members)) => Some(members),
        _ => None,
    }
}

/// The entries of `schema`'s `properties` map as written, beside a `$ref` too.
fn properties_as_written(schema: &Value) -> BTreeMap<&str, &Value> {
    let mut entries = BTreeMap::new();
    if let Some(properties) = schema
        .as_map()
        .and_then(|schema| schema.get_optional_key(PROPERTIES))
        .and_then(Value::as_map)
    {
        for (name, entry) in properties {
            if let Some(name) = name.as_text() {
                entries.entry(name).or_insert(entry);
            }
        }
    }
    entries
}

/// Why an object holding the member at `path` keeps it from being added as required, read in
/// the type's `schema` as written: one takes its members from a `$ref` (through which members
/// are added only as optional), or declares a whole-object `const` or `enum`, which these rules
/// do not check a stored document holding the new member against: it may not meet it.
fn holding_object_refusal(schema: &Value, path: &str) -> Option<String> {
    let segments: Vec<&str> = path.split('.').collect();
    let mut current = schema;
    for depth in 0..segments.len().saturating_sub(1) {
        let holder = *inline_member_schemas(current).get(segments[depth])?;
        let holder_path = || segments[..=depth].join(".");
        if has_ref(holder) {
            return Some(format!(
                "'{}' takes its members from a $ref, through which members are added only as \
                 optional",
                holder_path()
            ));
        }
        if let Some(keyword) = [CONST, ENUM].into_iter().find(|keyword| {
            holder
                .as_map()
                .is_some_and(|holder| holder.get_optional_key(keyword).is_some())
        }) {
            return Some(format!(
                "'{}' declares {keyword} for the whole object, which a stored document holding \
                 the member may not meet",
                holder_path()
            ));
        }
        current = holder;
    }
    None
}

/// The first `required` list in the entry written at `path` beside a `$ref`, or in an entry
/// written inside it, that no document could meet: one naming a dotted name, which the validator
/// reads as a property named with a dot, or more names than the entry's `maxProperties` lets an
/// object hold. An entry beside a `$ref` restates members its definition supplies, so its names
/// are not checked against the entry's own `properties`. A schema written inline with another
/// type than `object`, or written for a member the type parses (`parsed`) as anything but an
/// object, holds no members: the validator reads neither the `required` nor the `properties`
/// written for it as an object's, and what the parser collects from them names no member it
/// parses, so the walk stops there.
fn first_unmeetable_list_in_entry(schema: &Value, path: &str, parsed: Parsed) -> Option<String> {
    if is_inline_scalar(schema) || matches!(parsed, Parsed::NotAnObject) {
        return None;
    }
    let names = required_names(schema);
    if let Some(dotted) = names.iter().find(|name| name.contains('.')) {
        return Some(dotted_name_refusal(path, dotted));
    }
    if let Some(refusal) = past_max_properties(path, names.len(), [schema]) {
        return Some(refusal);
    }
    properties_as_written(schema)
        .into_iter()
        .find_map(|(name, entry)| {
            first_unmeetable_list_in_entry(
                entry,
                &member_path(Some(path), name),
                parsed.member(name),
            )
        })
}

/// What the walk of the objects a contract gains in a document type reads: the type's schema
/// and the contract's `$defs`, where a `$ref` resolves as the document validator resolves it.
struct GainedObjects<'a> {
    type_schema: &'a Value,
    definitions: &'a DefinitionsRoot<'a>,
}

impl GainedObjects<'_> {
    /// The first `required` list no document could meet in an object the contract gains below
    /// the object at `path` (`None`: the type itself), whose members as the stored type parses
    /// them are `old` and as the new type parses them `new`, and which the validator reads in
    /// `schemas` as written: an object `new` holds and `old` does not is judged with
    /// [`Self::first_unmeetable_list_of_gained_object`], one both hold is searched below.
    fn first_unmeetable_list_below(
        &self,
        old: &IndexMap<String, DocumentProperty>,
        new: &IndexMap<String, DocumentProperty>,
        schemas: Vec<&Value>,
        path: Option<&str>,
    ) -> Option<String> {
        let read = self.schemas_read(schemas);
        new.iter().find_map(|(name, new_property)| {
            let DocumentPropertyType::Object(new_members) = &new_property.property_type else {
                return None;
            };
            let member_schemas = member_schemas(&read, name);
            let path = member_path(path, name);
            match old.get(name).map(|property| &property.property_type) {
                Some(DocumentPropertyType::Object(old_members)) => self
                    .first_unmeetable_list_below(
                        old_members,
                        new_members,
                        member_schemas,
                        Some(&path),
                    ),
                Some(_) => None,
                None => {
                    self.first_unmeetable_list_of_gained_object(member_schemas, new_members, &path)
                }
            }
        })
    }

    /// The first `required` list no document could meet in the object the contract gains at
    /// `path`, whose members as the type parses them are `members` and which the validator reads
    /// in `schemas` as written (and in the schemas the chain of `$ref`s from each reaches), or in
    /// an object inside it. Every list those schemas hold names only members of the object, no
    /// dotted name, and, together, no more members than the least `maxProperties` among them;
    /// a schema written with `additionalProperties: false` admits only the members its own
    /// `properties` declare, so the lists name none it does not.
    fn first_unmeetable_list_of_gained_object(
        &self,
        schemas: Vec<&Value>,
        members: &IndexMap<String, DocumentProperty>,
        path: &str,
    ) -> Option<String> {
        let read = self.schemas_read(schemas);
        let mut names = BTreeSet::new();
        for name in read.iter().copied().flat_map(required_names) {
            if name.contains('.') {
                return Some(dotted_name_refusal(path, name));
            }
            if !members.contains_key(name) {
                return Some(format!(
                    "the required of '{path}' names '{name}', which is not one of its members, \
                     so no document can hold the object"
                ));
            }
            names.insert(name);
        }
        for schema in read
            .iter()
            .copied()
            .filter(|schema| closes_its_members(schema))
        {
            let declared = properties_as_written(schema);
            if let Some(undeclared) = names.iter().find(|name| !declared.contains_key(**name)) {
                return Some(format!(
                    "the required of '{path}' names '{undeclared}', which a schema written for \
                     the object with additionalProperties false does not declare, so no document \
                     can hold the object"
                ));
            }
        }
        if let Some(refusal) = past_max_properties(path, names.len(), read.iter().copied()) {
            return Some(refusal);
        }
        members.iter().find_map(|(name, member)| {
            let DocumentPropertyType::Object(member_members) = &member.property_type else {
                return None;
            };
            self.first_unmeetable_list_of_gained_object(
                member_schemas(&read, name),
                member_members,
                &member_path(Some(path), name),
            )
        })
    }

    /// Every schema the validator applies to an object written as `schemas`: each of them, and
    /// the schemas the chain of `$ref`s from it reaches, each resolved as the validator resolves
    /// it (a `$ref` the parser reads too resolves alike for both, or the validation schema
    /// refuses the contract). The chain ends at a schema with no `$ref`, at one already read,
    /// which a cycle reaches, or at a `$ref` that points at nothing, which the validation schema
    /// refuses; the lists of the others are still read.
    fn schemas_read<'s>(&'s self, schemas: Vec<&'s Value>) -> Vec<&'s Value> {
        let mut read: Vec<&Value> = Vec::with_capacity(schemas.len() * 2);
        for schema in schemas {
            let mut current = schema;
            read.push(current);
            while let Some(target) = reference_of(current).and_then(|reference| {
                self.definitions
                    .resolve_as_validated(reference, Some(self.type_schema))
            }) {
                if read.iter().any(|seen| std::ptr::eq(*seen, target)) {
                    break;
                }
                read.push(target);
                current = target;
            }
        }
        read
    }
}

/// The schemas written for the member `name` in `read`, the schemas the validator applies to its
/// object: the entries of their `properties` under that name.
fn member_schemas<'s>(read: &[&'s Value], name: &str) -> Vec<&'s Value> {
    read.iter()
        .filter_map(|schema| properties_as_written(schema).get(name).copied())
        .collect()
}

/// Whether `schema` writes `additionalProperties: false`, which the validator holds against the
/// `properties` written in the same schema.
fn closes_its_members(schema: &Value) -> bool {
    schema
        .as_map()
        .and_then(|schema| schema.get_optional_key(ADDITIONAL_PROPERTIES))
        .and_then(Value::as_bool)
        == Some(false)
}

/// The first reason the own `required` of a document type the contract gains is one no
/// document could meet: a dotted name, a name the validator holds a document to that is not one
/// of its properties (the system timestamps the validation schema leaves out of the list are
/// not), or more of those names than its `maxProperties` lets a document hold.
fn first_unmeetable_type_list(
    document_type: DocumentTypeRef,
    platform_version: &PlatformVersion,
) -> Option<String> {
    let schema = document_type.schema();
    if let Some(dotted) = required_names(schema)
        .into_iter()
        .find(|name| name.contains('.'))
    {
        return Some(format!(
            "the document type's required can not name the dotted name '{dotted}': a required \
             list names the members of its own object, and the document validator reads a \
             dotted name as a property named with a dot, which no document can hold"
        ));
    }
    let validated = validated_type_list(schema, platform_version)?;
    if let Some(unknown) = validated
        .iter()
        .find(|name| !document_type.properties().contains_key(*name))
    {
        return Some(format!(
            "the document type's required names '{unknown}', which is not one of its \
             properties, so no document of the type can be stored"
        ));
    }
    type_list_past_max_properties(None, schema, platform_version)
}

/// The names of the own `required` of the document type written as `schema` that the document
/// validator holds a document to: those of its validation schema, which leaves out the system
/// timestamps. `None` when the schema has no validation schema, which the parse refuses.
fn validated_type_list(
    schema: &Value,
    platform_version: &PlatformVersion,
) -> Option<BTreeSet<String>> {
    let validation_schema =
        DocumentType::enrich_with_base_schema(schema.clone(), None, platform_version).ok()?;
    Some(
        required_names(&validation_schema)
            .into_iter()
            .map(str::to_string)
            .collect(),
    )
}

/// Why the own `required` of the document type written as `new_schema` names more properties
/// than its `maxProperties` lets a document hold, if it does and has grown since `old_schema`
/// (`None` for a type the contract gains). The names counted are those the document validator
/// holds a document to ([`validated_type_list`]).
fn type_list_past_max_properties(
    old_schema: Option<&Value>,
    new_schema: &Value,
    platform_version: &PlatformVersion,
) -> Option<String> {
    let max_properties = max_properties(new_schema)?;
    let count = validated_type_list(new_schema, platform_version)?.len();
    if count as u64 <= max_properties {
        return None;
    }
    if let Some(old_schema) = old_schema {
        if validated_type_list(old_schema, platform_version)
            .is_some_and(|old_names| old_names.len() >= count)
        {
            return None;
        }
    }
    Some(format!(
        "the document type's required names {count} properties, more than the \
         {max_properties} a document may hold (maxProperties), so no document of the type could \
         be stored"
    ))
}

/// Why a `required` list naming `count` members of the object at `path` is one no document
/// could meet: more than the least `maxProperties` of `schemas`, the schemas the validator reads
/// for the object.
fn past_max_properties<'s>(
    path: &str,
    count: usize,
    schemas: impl IntoIterator<Item = &'s Value>,
) -> Option<String> {
    let max_properties = schemas.into_iter().filter_map(max_properties).min()?;
    (count as u64 > max_properties).then(|| {
        format!(
            "the required of '{path}' names {count} members, more than the {max_properties} its \
             object may hold (maxProperties), so no document could hold the object"
        )
    })
}

fn dotted_name_refusal(path: &str, dotted: &str) -> String {
    format!(
        "the required of '{path}' can not name the dotted name '{dotted}': a required list names \
         the members of its own object, and the document validator reads a dotted name as a \
         property named with a dot, which no document can hold"
    )
}

/// The `$ref` written in `schema`, if it writes one as text.
fn reference_of(schema: &Value) -> Option<&str> {
    schema.as_map()?.get_optional_key(REF)?.as_text()
}

fn member_path(path: Option<&str>, name: &str) -> String {
    path.map_or_else(|| name.to_string(), |path| format!("{path}.{name}"))
}

/// The schemas of the members of the object `schema` describes inline, by name, read once per
/// object; none when it describes none or takes its members from a `$ref`. A key named twice
/// keeps its first entry: a contract updated from protocol version 15 names no key twice in
/// either schema (contract update generation 2 and the registration check refuse it).
fn inline_member_schemas(schema: &Value) -> BTreeMap<&str, &Value> {
    let mut members = BTreeMap::new();
    if has_ref(schema) {
        return members;
    }
    if let Some(properties) = schema
        .as_map()
        .and_then(|schema| schema.get_optional_key(PROPERTIES))
        .and_then(Value::as_map)
    {
        for (name, member) in properties {
            if let Some(name) = name.as_text() {
                members.entry(name).or_insert(member);
            }
        }
    }
    members
}

/// Whether `schema` declares a type other than `object` without a `$ref`: the parser reads the
/// schema a `$ref` points at, whatever `type` is written beside it.
fn is_inline_scalar(schema: &Value) -> bool {
    !has_ref(schema)
        && schema
            .as_map()
            .and_then(|schema| schema.get_optional_key(TYPE))
            .and_then(Value::as_text)
            .is_some_and(|declared_type| declared_type != "object")
}

fn has_ref(schema: &Value) -> bool {
    schema
        .as_map()
        .is_some_and(|schema| schema.get_optional_key(REF).is_some())
}

fn max_properties(schema: &Value) -> Option<u64> {
    schema
        .as_map()?
        .get_optional_key(MAX_PROPERTIES)?
        .as_integer::<u64>()
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
