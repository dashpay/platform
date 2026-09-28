//! A computed key of a `refersTo` lookup: `{ "sha256d": [part, ...] }` in
//! place of a key source names the hash of a preimage the referring document
//! reveals. The document such a key finds is a commitment made earlier, so a
//! lookup holding one is a commit and reveal: a document may be created only
//! when a commitment to values it reveals exists.
//!
//! ```json
//! "creatorRefersTo": {
//!   "type": "deletableDocument",
//!   "documentType": "preorder",
//!   "lookup": {
//!     "index": "saltedHash",
//!     "keys": { "saltedDomainHash": { "sha256d": [
//!       { "property": "preorderSalt" },
//!       { "ifEmpty": "parentDomainName",
//!         "then": [{ "property": "label" }],
//!         "else": [
//!           { "property": "normalizedLabel" },
//!           { "text": "." },
//!           { "property": "parentDomainName" }
//!         ] }
//!     ] } },
//!     "minimumAgeSeconds": 60,
//!     "consume": true
//!   },
//!   "propertyAgreement": { "$ownerId": "$ownerId" }
//! }
//! ```
//!
//! The preimage is the bytes of its parts, concatenated with nothing between
//! them: a string property's UTF-8, a byte array property's bytes, an
//! identifier's 32 bytes and a text part's UTF-8. An `ifEmpty` part stands for
//! its `then` parts when the named string or byte array property is empty and
//! for its `else` parts otherwise. So that a preimage splits back into its
//! parts one way only, every variable-length part (a string, or a byte array
//! whose length is not fixed) that another variable-length part follows is
//! followed directly by a one-byte text part, its separator, and a value
//! holding that byte is refused when the document is created. The rules live
//! here so the parse, the registration checks and the write-time check read
//! one grammar.

use crate::data_contract::document_type::accessors::DocumentTypeV0Getters;
use crate::data_contract::document_type::property::reference_lookup::schema_property_is_fixed_once_written;
use crate::data_contract::document_type::property::{
    is_transient, DocumentPropertyType, ReferenceHolder,
};
use crate::data_contract::document_type::DocumentTypeRef;
use crate::data_contract::errors::DataContractError;
use crate::util::hash::hash_double;
use bincode::{Decode, DecodeUntrusted, Encode};
use platform_value::btreemap_extensions::BTreeValueMapPathHelper;
use platform_value::Value;
use serde::ser::SerializeMap;
use serde::{Serialize, Serializer};
use std::collections::BTreeMap;

/// The wire name of a preimage part read from a property.
pub const PREIMAGE_PROPERTY: &str = "property";
/// The wire name of a preimage part holding fixed text.
pub const PREIMAGE_TEXT: &str = "text";
/// The wire name of a conditional preimage part, naming the property it tests.
pub const PREIMAGE_IF_EMPTY: &str = "ifEmpty";
/// The parts a conditional preimage part stands for when its property is empty.
pub const PREIMAGE_THEN: &str = "then";
/// The parts a conditional preimage part stands for when its property is not empty.
pub const PREIMAGE_ELSE: &str = "else";

/// The most parts one preimage may list, counting the parts of both branches
/// of its `ifEmpty`, the bound meta-schema v3 puts on it.
pub const MAX_PREIMAGE_PARTS: usize = 16;

/// The most UTF-8 bytes a text part may hold, the bound meta-schema v3 puts on
/// it.
pub const MAX_PREIMAGE_TEXT_BYTES: usize = 64;

/// The longest property path a preimage part may name, the bound meta-schema
/// v3 puts on it.
pub const MAX_PREIMAGE_PATH_LENGTH: usize = 256;

/// The hash a computed lookup key applies to its preimage: a closed list,
/// named by the key of the computed key's object.
// @append_only
#[derive(Debug, PartialEq, Eq, Clone, Copy, Encode, Decode, DecodeUntrusted)]
pub enum LookupKeyHash {
    /// `sha256d`: SHA-256 of SHA-256, 32 bytes (`dpp::util::hash::hash_double`),
    /// the hash of a DPNS preorder.
    Sha256d,
}

impl LookupKeyHash {
    /// Every hash, in wire name order.
    pub const ALL: [LookupKeyHash; 1] = [LookupKeyHash::Sha256d];

    /// The name the schema spells the hash by.
    pub fn wire_name(self) -> &'static str {
        match self {
            LookupKeyHash::Sha256d => "sha256d",
        }
    }

    /// The hash a wire name names, `None` for any other name.
    pub fn from_wire_name(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|hash| hash.wire_name() == name)
    }

    /// How many bytes the hash produces, the size of the byte array index
    /// property it fills.
    pub fn output_length(self) -> u16 {
        match self {
            LookupKeyHash::Sha256d => 32,
        }
    }

    /// The hash of `preimage`, as the value of the index property it fills.
    pub fn digest(self, preimage: &[u8]) -> Value {
        match self {
            LookupKeyHash::Sha256d => Value::Bytes32(hash_double(preimage)),
        }
    }
}

/// One part of the preimage of a computed lookup key.
// @append_only
#[derive(Debug, PartialEq, Eq, Clone, Encode, Decode, DecodeUntrusted)]
pub enum LookupPreimagePart {
    /// `{ "property": path }`: the value of a string, byte array or
    /// identifier property of the referring document type.
    Property(String),
    /// `{ "text": text }`: fixed text, as UTF-8.
    Text(String),
    /// `{ "ifEmpty": path, "then": [...], "else": [...] }`: the `then` parts
    /// when the string or byte array property at `property` is empty, the
    /// `otherwise` parts when it is not. Neither branch holds another
    /// `ifEmpty`.
    IfEmpty {
        /// The property tested.
        property: String,
        /// The parts when the property is empty.
        then: Vec<LookupPreimagePart>,
        /// The parts when the property is not empty (`else` on the wire).
        otherwise: Vec<LookupPreimagePart>,
    },
}

impl Serialize for LookupPreimagePart {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            LookupPreimagePart::Property(path) => {
                let mut map = serializer.serialize_map(Some(1))?;
                map.serialize_entry(PREIMAGE_PROPERTY, path)?;
                map.end()
            }
            LookupPreimagePart::Text(text) => {
                let mut map = serializer.serialize_map(Some(1))?;
                map.serialize_entry(PREIMAGE_TEXT, text)?;
                map.end()
            }
            LookupPreimagePart::IfEmpty {
                property,
                then,
                otherwise,
            } => {
                let mut map = serializer.serialize_map(Some(3))?;
                map.serialize_entry(PREIMAGE_IF_EMPTY, property)?;
                map.serialize_entry(PREIMAGE_THEN, then)?;
                map.serialize_entry(PREIMAGE_ELSE, otherwise)?;
                map.end()
            }
        }
    }
}

/// A computed lookup key, `{ "<hash>": [part, ...] }`: the hash of the
/// preimage the parts assemble from the referring document.
#[derive(Debug, PartialEq, Eq, Clone, Encode, Decode, DecodeUntrusted)]
pub struct LookupHashKey {
    /// The hash applied to the preimage.
    pub hash: LookupKeyHash,
    /// The parts of the preimage, in order.
    pub preimage: Vec<LookupPreimagePart>,
}

impl Serialize for LookupHashKey {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(Some(1))?;
        map.serialize_entry(self.hash.wire_name(), &self.preimage)?;
        map.end()
    }
}

/// Why a document's values cannot be assembled into the preimage of a
/// computed key: a part it reads is absent or holds a value the part cannot
/// take, or a variable-length value holds the separator that follows it.
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct LookupPreimageError {
    /// The property whose value is at fault.
    pub property: String,
    /// What is wrong with it.
    pub reason: String,
}

impl LookupHashKey {
    /// Reads a computed key from the value of a lookup key: an object whose
    /// one key names the hash and whose value lists the preimage's parts.
    /// What the parts' properties resolve to is checked once the document type
    /// is parsed, see [`Self::referring_side_error`].
    pub fn from_value(value: &Value) -> Result<Self, DataContractError> {
        let map = value.to_btree_ref_string_map()?;
        let mut entries = map.iter();
        let (Some((hash_name, parts_value)), None) = (entries.next(), entries.next()) else {
            return Err(DataContractError::InvalidContractStructure(
                "a computed refersTo lookup key holds exactly one key, the hash, listing the \
                 preimage's parts"
                    .to_string(),
            ));
        };
        let hash = LookupKeyHash::from_wire_name(hash_name).ok_or_else(|| {
            DataContractError::InvalidContractStructure(format!(
                "refersTo lookup key hash {hash_name:?} is unknown, expected one of {:?}",
                LookupKeyHash::ALL.map(LookupKeyHash::wire_name)
            ))
        })?;
        let preimage = parse_parts(parts_value, true)?;
        let conditionals = preimage
            .iter()
            .filter(|part| matches!(part, LookupPreimagePart::IfEmpty { .. }))
            .count();
        if conditionals > 1 {
            return Err(DataContractError::InvalidContractStructure(format!(
                "a refersTo lookup key preimage holds at most one {PREIMAGE_IF_EMPTY} part, found \
                 {conditionals}"
            )));
        }
        let key = LookupHashKey { hash, preimage };
        let part_count = key.part_count();
        if part_count > MAX_PREIMAGE_PARTS {
            return Err(DataContractError::InvalidContractStructure(format!(
                "a refersTo lookup key preimage lists at most {MAX_PREIMAGE_PARTS} parts, those \
                 of its {PREIMAGE_IF_EMPTY} branches included, found {part_count}"
            )));
        }
        Ok(key)
    }

    /// How many parts the preimage lists, the parts of an `ifEmpty` counted
    /// with its branches' parts.
    fn part_count(&self) -> usize {
        self.preimage
            .iter()
            .map(|part| match part {
                LookupPreimagePart::IfEmpty {
                    then, otherwise, ..
                } => 1 + then.len() + otherwise.len(),
                _ => 1,
            })
            .sum()
    }

    /// Every property path the preimage reads: its property parts, those of
    /// both branches of its `ifEmpty` and the property that one tests.
    pub fn properties_read(&self) -> Vec<&str> {
        let mut paths = Vec::new();
        for part in &self.preimage {
            match part {
                LookupPreimagePart::Property(path) => paths.push(path.as_str()),
                LookupPreimagePart::Text(_) => {}
                LookupPreimagePart::IfEmpty {
                    property,
                    then,
                    otherwise,
                } => {
                    paths.push(property.as_str());
                    paths.extend(then.iter().chain(otherwise).filter_map(|part| match part {
                        LookupPreimagePart::Property(path) => Some(path.as_str()),
                        _ => None,
                    }));
                }
            }
        }
        paths
    }

    /// The sequences of property and text parts the preimage can stand for:
    /// one, or two when it holds an `ifEmpty`, its `then` branch first.
    fn branches(&self) -> Vec<Vec<&LookupPreimagePart>> {
        let mut branches: Vec<Vec<&LookupPreimagePart>> = vec![Vec::new()];
        for part in &self.preimage {
            match part {
                LookupPreimagePart::IfEmpty {
                    then, otherwise, ..
                } => {
                    let mut with_otherwise = branches.clone();
                    for branch in &mut branches {
                        branch.extend(then);
                    }
                    for branch in &mut with_otherwise {
                        branch.extend(otherwise);
                    }
                    branches.extend(with_otherwise);
                }
                part => {
                    for branch in &mut branches {
                        branch.push(part);
                    }
                }
            }
        }
        branches
    }

    /// Why this key, declared on `declaring`, cannot always be assembled into
    /// a preimage that splits back one way; `None` when it can. Every property
    /// it reads must exist and hold a single value of a kind a preimage takes:
    /// a string, a byte array or an identifier for a part, a string or a byte
    /// array for the property an `ifEmpty` tests. A part may be transient,
    /// since the key is read only from the create transition, where transient
    /// values are present, and it may be optional, since a create missing a
    /// part is refused; a stored one must be fixed once written, so no
    /// replace can change the values a commitment was revealed for. In each
    /// sequence of parts the preimage can stand for, every variable-length
    /// part another variable-length part follows is followed directly by a
    /// one-byte text part.
    pub fn referring_side_error(&self, declaring: DocumentTypeRef) -> Option<String> {
        let conditions: Vec<&str> = self
            .preimage
            .iter()
            .filter_map(|part| match part {
                LookupPreimagePart::IfEmpty { property, .. } => Some(property.as_str()),
                _ => None,
            })
            .collect();
        for path in self.properties_read() {
            let Some(property) = declaring.flattened_properties().get(path) else {
                return Some(format!(
                    "preimage reads \"{path}\", which is not a property of the referring document \
                     type"
                ));
            };
            let is_condition = conditions.contains(&path);
            let takes_kind = match &property.property_type {
                DocumentPropertyType::String(_) | DocumentPropertyType::ByteArray(_) => true,
                DocumentPropertyType::Identifier
                | DocumentPropertyType::IdentifierWithReference(_) => !is_condition,
                _ => false,
            };
            if !takes_kind {
                let kinds = if is_condition {
                    "a string or a byte array"
                } else {
                    "a string, a byte array or an identifier"
                };
                return Some(format!("preimage reads \"{path}\", which is not {kinds}"));
            }
            if !is_transient(declaring, path)
                && !schema_property_is_fixed_once_written(declaring, path)
            {
                return Some(format!(
                    "preimage reads \"{path}\", which a replace can change: the key is checked \
                     when the document is created only, so every stored value it reads must be \
                     fixed once written (make the type immutable or list the property under \
                     `immutable`)"
                ));
            }
        }
        for branch in self.branches() {
            if branch.is_empty() {
                return Some("a preimage branch lists no part".to_string());
            }
            for (position, part) in branch.iter().enumerate() {
                if !is_variable_length_part(part, declaring) {
                    continue;
                }
                let followed_by_variable = branch[position + 1..]
                    .iter()
                    .any(|later| is_variable_length_part(later, declaring));
                if !followed_by_variable {
                    continue;
                }
                let separated = matches!(
                    branch.get(position + 1),
                    Some(LookupPreimagePart::Text(text)) if text.len() == 1
                );
                if !separated {
                    let LookupPreimagePart::Property(path) = part else {
                        continue;
                    };
                    return Some(format!(
                        "preimage part \"{path}\" has no fixed length and another such part \
                         follows it, so it must be followed directly by a one-byte text part, \
                         the separator its value may not hold: without one the preimage could \
                         split into its parts more than one way"
                    ));
                }
            }
        }
        None
    }

    /// The preimage `document_data` assembles, a create of a document of
    /// `declaring`: the bytes of every part in order, with the `ifEmpty` part
    /// standing for the branch its property picks. An error names the first
    /// property whose value is absent or of a kind the part cannot take, or a
    /// variable-length value holding the separator that follows it.
    pub fn preimage(
        &self,
        declaring: DocumentTypeRef,
        document_data: &BTreeMap<String, Value>,
    ) -> Result<Vec<u8>, LookupPreimageError> {
        let mut parts: Vec<&LookupPreimagePart> = Vec::new();
        for part in &self.preimage {
            match part {
                LookupPreimagePart::IfEmpty {
                    property,
                    then,
                    otherwise,
                } => {
                    let tested = read_part_bytes(property, document_data)?;
                    if tested.is_empty() {
                        parts.extend(then);
                    } else {
                        parts.extend(otherwise);
                    }
                }
                part => parts.push(part),
            }
        }
        let mut preimage = Vec::new();
        for (position, part) in parts.iter().enumerate() {
            match part {
                LookupPreimagePart::Text(text) => preimage.extend_from_slice(text.as_bytes()),
                LookupPreimagePart::Property(path) => {
                    let bytes = read_part_bytes(path, document_data)?;
                    // A separator is required, and so checked, only where
                    // another variable-length part follows
                    let followed_by_variable = parts[position + 1..]
                        .iter()
                        .any(|later| is_variable_length_part(later, declaring));
                    if followed_by_variable && is_variable_length_part(part, declaring) {
                        if let Some(LookupPreimagePart::Text(separator)) = parts.get(position + 1) {
                            if let [separator_byte] = separator.as_bytes() {
                                if bytes.contains(separator_byte) {
                                    return Err(LookupPreimageError {
                                        property: path.clone(),
                                        reason: format!(
                                            "the value holds {separator:?}, the separator that \
                                             follows it in the preimage"
                                        ),
                                    });
                                }
                            }
                        }
                    }
                    preimage.extend_from_slice(&bytes);
                }
                // Resolved into its branch above, and a branch holds no
                // other `ifEmpty`
                LookupPreimagePart::IfEmpty { .. } => {}
            }
        }
        Ok(preimage)
    }

    /// The value of the index property this key fills for `document_data`, a
    /// create of a document of `declaring`: the hash of its preimage.
    pub fn key_value(
        &self,
        declaring: DocumentTypeRef,
        document_data: &BTreeMap<String, Value>,
    ) -> Result<Value, LookupPreimageError> {
        let preimage = self.preimage(declaring, document_data)?;
        Ok(self.hash.digest(&preimage))
    }
}

/// Reads a list of preimage parts. `allow_conditional` is false inside the
/// branches of an `ifEmpty`, which hold property and text parts only.
fn parse_parts(
    value: &Value,
    allow_conditional: bool,
) -> Result<Vec<LookupPreimagePart>, DataContractError> {
    let Some(values) = value.as_array() else {
        return Err(DataContractError::InvalidContractStructure(
            "a refersTo lookup key preimage must be a list of parts".to_string(),
        ));
    };
    if values.is_empty() {
        return Err(DataContractError::InvalidContractStructure(
            "a refersTo lookup key preimage lists at least one part".to_string(),
        ));
    }
    values
        .iter()
        .map(|part_value| parse_part(part_value, allow_conditional))
        .collect()
}

/// Reads one preimage part: `{ "property": path }`, `{ "text": text }` or,
/// where `allow_conditional`, `{ "ifEmpty": path, "then": [...], "else": [...] }`.
fn parse_part(
    value: &Value,
    allow_conditional: bool,
) -> Result<LookupPreimagePart, DataContractError> {
    let map = value.to_btree_ref_string_map()?;
    if let Some(tested) = map.get(PREIMAGE_IF_EMPTY) {
        if !allow_conditional {
            return Err(DataContractError::InvalidContractStructure(format!(
                "a refersTo lookup key preimage {PREIMAGE_IF_EMPTY} branch holds property and text \
                 parts only"
            )));
        }
        if let Some(unknown) = map.keys().find(|key| {
            !matches!(
                key.as_str(),
                PREIMAGE_IF_EMPTY | PREIMAGE_THEN | PREIMAGE_ELSE
            )
        }) {
            return Err(DataContractError::InvalidContractStructure(format!(
                "a refersTo lookup key preimage {PREIMAGE_IF_EMPTY} part takes {PREIMAGE_THEN} and \
                 {PREIMAGE_ELSE}, not {unknown:?}"
            )));
        }
        let branch = |name: &str| {
            map.get(name)
                .ok_or_else(|| {
                    DataContractError::InvalidContractStructure(format!(
                        "a refersTo lookup key preimage {PREIMAGE_IF_EMPTY} part must list its \
                         {name} parts"
                    ))
                })
                .and_then(|parts| parse_parts(parts, false))
        };
        return Ok(LookupPreimagePart::IfEmpty {
            property: parse_path(tested)?,
            then: branch(PREIMAGE_THEN)?,
            otherwise: branch(PREIMAGE_ELSE)?,
        });
    }
    let mut entries = map.iter();
    let (Some((kind, part_value)), None) = (entries.next(), entries.next()) else {
        return Err(DataContractError::InvalidContractStructure(
            "a refersTo lookup key preimage part holds exactly one key".to_string(),
        ));
    };
    match kind.as_str() {
        PREIMAGE_PROPERTY => Ok(LookupPreimagePart::Property(parse_path(part_value)?)),
        PREIMAGE_TEXT => {
            let text = part_value.as_text().ok_or_else(|| {
                DataContractError::InvalidContractStructure(
                    "a refersTo lookup key preimage text part must be a string".to_string(),
                )
            })?;
            if text.is_empty() || text.len() > MAX_PREIMAGE_TEXT_BYTES {
                return Err(DataContractError::InvalidContractStructure(format!(
                    "a refersTo lookup key preimage text part holds between 1 and \
                     {MAX_PREIMAGE_TEXT_BYTES} bytes"
                )));
            }
            Ok(LookupPreimagePart::Text(text.to_string()))
        }
        other => Err(DataContractError::InvalidContractStructure(format!(
            "a refersTo lookup key preimage part {other:?} is unknown: a part is \
             {PREIMAGE_PROPERTY}, {PREIMAGE_TEXT} or {PREIMAGE_IF_EMPTY}"
        ))),
    }
}

/// A property path a preimage part names: a schema property, never a system
/// one, which no create transition carries in its data.
fn parse_path(value: &Value) -> Result<String, DataContractError> {
    let path = value.as_text().ok_or_else(|| {
        DataContractError::InvalidContractStructure(
            "a refersTo lookup key preimage names a property by its path, a string".to_string(),
        )
    })?;
    if path.is_empty() || path.len() > MAX_PREIMAGE_PATH_LENGTH || path.starts_with('$') {
        return Err(DataContractError::InvalidContractStructure(format!(
            "a refersTo lookup key preimage names a schema property by a path of 1 to \
             {MAX_PREIMAGE_PATH_LENGTH} characters, not {path:?}"
        )));
    }
    Ok(path.to_string())
}

/// Whether `part` is a property part whose value has no fixed length: a
/// string, or a byte array whose minimum and maximum sizes differ. Text parts
/// and identifiers are fixed; a part naming no property of `declaring` is
/// judged by [`LookupHashKey::referring_side_error`] instead.
fn is_variable_length_part(part: &LookupPreimagePart, declaring: DocumentTypeRef) -> bool {
    let LookupPreimagePart::Property(path) = part else {
        return false;
    };
    match declaring
        .flattened_properties()
        .get(path)
        .map(|property| &property.property_type)
    {
        Some(DocumentPropertyType::String(_)) => true,
        Some(DocumentPropertyType::ByteArray(sizes)) => {
            sizes.min_size.is_none() || sizes.min_size != sizes.max_size
        }
        _ => false,
    }
}

/// The bytes the value at `path` of `document_data` contributes to a
/// preimage: a string's UTF-8, or the bytes of a byte array or identifier.
/// Schema validation has already refused a value of the wrong kind for its
/// property, so the value is judged by its own shape.
fn read_part_bytes(
    path: &str,
    document_data: &BTreeMap<String, Value>,
) -> Result<Vec<u8>, LookupPreimageError> {
    let error = |reason: &str| LookupPreimageError {
        property: path.to_string(),
        reason: reason.to_string(),
    };
    let value = match document_data.get_optional_at_path(path) {
        Ok(Some(value)) => value,
        Ok(None) => return Err(error("the value is absent")),
        Err(_) => return Err(error("the path does not resolve to a value")),
    };
    match value {
        Value::Text(text) => Ok(text.as_bytes().to_vec()),
        Value::Bytes(_)
        | Value::Bytes20(_)
        | Value::Bytes32(_)
        | Value::Bytes36(_)
        | Value::Identifier(_)
        | Value::Array(_) => value
            .to_binary_bytes()
            .map_err(|_| error("the value is not a string or bytes")),
        _ => Err(error("the value is not a string or bytes")),
    }
}

/// The first reference of `document_type` whose computed lookup key a create
/// carrying `document_data` cannot reveal, with the path the reference is
/// declared at (a property path, `$ownerId` or `$creatorId`): a property a
/// part reads is absent or of the wrong kind, or a variable-length value holds
/// its separator. `None` when every such key can be assembled.
///
/// Only a single target is judged here, on the writer, on the creator, or on
/// a property (or the elements of a typed array) that holds a value, since an
/// unset reference property is not validated. A computed key that is a leaf of
/// a reference expression is left to the write-time check, where a key it
/// cannot assemble finds no document: that operand fails and the others still
/// decide, as for any other leaf.
pub fn first_unrevealable_lookup_key(
    document_type: DocumentTypeRef,
    document_data: &BTreeMap<String, Value>,
) -> Option<(String, LookupPreimageError)> {
    for (holder, reference) in document_type.reference_declarations() {
        let Some(target) = reference.target() else {
            continue;
        };
        let Some((_, key)) = target
            .as_any_document_reference()
            .and_then(|declaration| declaration.lookup)
            .and_then(|lookup| lookup.hash_key())
        else {
            continue;
        };
        if let ReferenceHolder::Property(path) = holder {
            if !matches!(document_data.get_optional_at_path(path), Ok(Some(_))) {
                continue;
            }
        }
        if let Err(error) = key.preimage(document_type, document_data) {
            return Some((holder.path().to_string(), error));
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data_contract::config::DataContractConfig;
    use crate::data_contract::document_type::property::LookupKeySource;
    use crate::data_contract::document_type::DocumentType;
    use platform_value::{platform_value, Identifier};
    use platform_version::version::PlatformVersion;

    /// A DPNS-shaped `domain`: strings `label`, `normalizedLabel` and an
    /// optional `parentDomainName`, and a transient 32-byte `preorderSalt`.
    fn domain() -> DocumentType {
        let platform_version = PlatformVersion::latest();
        let config =
            DataContractConfig::default_for_version(platform_version).expect("config should build");
        DocumentType::try_from_schema(
            Identifier::from([1; 32]),
            1,
            config.version(),
            "domain",
            platform_value!({
                "type": "object",
                "documentsMutable": false,
                "properties": {
                    "label": { "type": "string", "maxLength": 63u32, "position": 0u32 },
                    "normalizedLabel": { "type": "string", "maxLength": 63u32, "position": 1u32 },
                    "parentDomainName": { "type": "string", "maxLength": 63u32, "position": 2u32 },
                    "preorderSalt": {
                        "type": "array",
                        "byteArray": true,
                        "minItems": 32u32,
                        "maxItems": 32u32,
                        "position": 3u32
                    }
                },
                "required": ["label", "normalizedLabel", "preorderSalt"],
                "transient": ["preorderSalt"],
                "additionalProperties": false
            }),
            None,
            &BTreeMap::new(),
            &config,
            false,
            &mut vec![],
            platform_version,
        )
        .expect("the document type should parse")
    }

    /// The DPNS preorder key: `preorderSalt ++ label` for a top-level name,
    /// `preorderSalt ++ normalizedLabel ++ "." ++ parentDomainName` otherwise.
    fn dpns_key() -> LookupHashKey {
        LookupHashKey::from_value(&platform_value!({
            "sha256d": [
                { "property": "preorderSalt" },
                {
                    "ifEmpty": "parentDomainName",
                    "then": [{ "property": "label" }],
                    "else": [
                        { "property": "normalizedLabel" },
                        { "text": "." },
                        { "property": "parentDomainName" }
                    ]
                }
            ]
        }))
        .expect("the DPNS key should parse")
    }

    fn data(entries: &[(&str, Value)]) -> BTreeMap<String, Value> {
        entries
            .iter()
            .map(|(property, value)| (property.to_string(), value.clone()))
            .collect()
    }

    /// The hash the DPNS create trigger (`create_domain_data_trigger_v1`)
    /// computes, written out as it does.
    fn trigger_hash(salt: [u8; 32], label: &str, normalized_label: &str, parent: &str) -> Value {
        let full_domain_name = if parent.is_empty() {
            label.to_string()
        } else {
            format!("{normalized_label}.{parent}")
        };
        let mut salted_domain_buffer: Vec<u8> = vec![];
        salted_domain_buffer.extend(salt);
        salted_domain_buffer.extend(full_domain_name.as_bytes());
        Value::Bytes32(hash_double(salted_domain_buffer))
    }

    #[test]
    fn should_hash_both_dpns_branches_byte_for_byte_as_the_create_trigger() {
        let domain = domain();
        let key = dpns_key();
        let salt = [0x42; 32];
        for (label, normalized_label, parent) in [
            ("Alice", "al1ce", "dash"),
            ("Dash", "dash", ""),
            ("bob-2", "b0b-2", "Some-Parent"),
        ] {
            // A byte array as bytes, or as the array of numbers a JSON
            // transition carries, hashes the same
            for salt_value in [
                Value::Bytes32(salt),
                Value::Array(salt.iter().map(|byte| Value::U8(*byte)).collect()),
            ] {
                let document = data(&[
                    ("label", label.into()),
                    ("normalizedLabel", normalized_label.into()),
                    ("parentDomainName", parent.into()),
                    ("preorderSalt", salt_value),
                ]);
                assert_eq!(
                    key.key_value(domain.as_ref(), &document),
                    Ok(trigger_hash(salt, label, normalized_label, parent)),
                    "{label} {normalized_label} {parent}"
                );
            }
        }
    }

    #[test]
    fn should_refuse_a_preimage_missing_a_part_or_holding_a_separator() {
        let domain = domain();
        let key = dpns_key();
        let salt = Value::Bytes32([0x42; 32]);

        // Without the property the conditional tests, there is no branch
        let missing_parent = data(&[
            ("label", "Alice".into()),
            ("normalizedLabel", "al1ce".into()),
            ("preorderSalt", salt.clone()),
        ]);
        assert_matches::assert_matches!(
            key.preimage(domain.as_ref(), &missing_parent),
            Err(LookupPreimageError { property, .. }) if property == "parentDomainName"
        );

        // "a.b" + "." + "c" and "a" + "." + "b.c" are the same bytes
        let dotted_label = data(&[
            ("label", "A.b".into()),
            ("normalizedLabel", "a.b".into()),
            ("parentDomainName", "c".into()),
            ("preorderSalt", salt.clone()),
        ]);
        assert_matches::assert_matches!(
            key.preimage(domain.as_ref(), &dotted_label),
            Err(LookupPreimageError { property, .. }) if property == "normalizedLabel"
        );

        // The last variable-length part may hold anything: nothing follows it
        let dotted_parent = data(&[
            ("label", "Alice".into()),
            ("normalizedLabel", "al1ce".into()),
            ("parentDomainName", "b.c".into()),
            ("preorderSalt", salt),
        ]);
        assert!(key.preimage(domain.as_ref(), &dotted_parent).is_ok());
    }

    #[test]
    fn should_serialize_a_computed_key_as_the_schema_spells_it() {
        let source = LookupKeySource::Hash(dpns_key());
        assert_eq!(
            serde_json::to_value(&source).expect("the key serializes"),
            serde_json::json!({
                "sha256d": [
                    { "property": "preorderSalt" },
                    {
                        "ifEmpty": "parentDomainName",
                        "then": [{ "property": "label" }],
                        "else": [
                            { "property": "normalizedLabel" },
                            { "text": "." },
                            { "property": "parentDomainName" }
                        ]
                    }
                ]
            })
        );
        assert_eq!(
            serde_json::to_value(LookupKeySource::ReferenceValue).expect("serializes"),
            serde_json::json!(".")
        );
    }
}
