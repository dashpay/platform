//! A computed key of a `refersTo` `findBy`, declared as a function entry:
//! `"<referenced property>": { "function": "sys.hash.sha256d", "params": [...] }`
//! says the referenced document is found by that property holding a system
//! hash (`sys.hash`, see [`HashFunction`]) of values the referring document
//! reveals, the same `function` / `params` shape a `generatedFrom` property
//! declares. The property belongs to the unique index `findBy` names, so the
//! platform finds the referenced document by the hash. The document it finds is a
//! commitment made earlier, so the reference is a commit and reveal: a
//! document may be created only when a commitment to values it carries
//! exists.
//!
//! ```json
//! "preorderSalt": {
//!   "type": "array", "byteArray": true, "minItems": 32, "maxItems": 32,
//!   "refersTo": {
//!     "type": "deletableDocument",
//!     "documentType": "preorder",
//!     "findBy": {
//!       "saltedDomainHash": {
//!         "function": "sys.hash.sha256d",
//!         "params": ["preorderSalt", "normalizedLabel", { "const": "." }, "parentDomainName"]
//!       }
//!     },
//!     "where": { "$ownerId": "$ownerId" },
//!     "minimumAgeBlocks": 1,
//!     "consume": true
//!   }
//! }
//! ```
//!
//! The hash is taken over the bytes of the params joined in order with nothing
//! between them: a dotted path a property of the referring document (a
//! string's UTF-8, a byte array's bytes, an identifier's 32 bytes), the
//! property carrying the reference included, named by its path;
//! `{ "const": text }` fixed UTF-8 text; and `"."` the value carrying the
//! reference where it has no path (each element of a typed array, the writer
//! or the creator). So that the joined bytes split back into the params one
//! way only, every variable-length param (a string, or a byte array whose size
//! is not fixed) that another variable-length param follows is followed
//! directly by a one-byte `const`, its separator, and a value holding that
//! byte is refused when the document is created. The rules live here so the
//! parse, the registration checks and the write-time check read one grammar.

use crate::data_contract::document_type::accessors::DocumentTypeV0Getters;
use crate::data_contract::document_type::methods::{read_data_at_path, RepeatedKey};
use crate::data_contract::document_type::property::generated_from::{HashFunction, SystemFunction};
use crate::data_contract::document_type::property::reference_lookup::{
    clearable_once_its_document_is_deleted, schema_property_is_fixed_once_written,
};
use crate::data_contract::document_type::property::{
    is_transient, DocumentPropertyType, PropertyReference, ReferenceHolder,
};
use crate::data_contract::document_type::DocumentTypeRef;
use crate::data_contract::errors::DataContractError;
use bincode::{Decode, DecodeUntrusted, Encode};
use platform_value::{Identifier, Value};
use serde::ser::SerializeMap;
use serde::{Serialize, Serializer};
use std::borrow::Cow;
use std::collections::BTreeMap;

/// The key of a computed lookup key naming its function.
pub const LOOKUP_KEY_FUNCTION: &str = "function";
/// The key of a computed lookup key listing its params.
pub const LOOKUP_KEY_PARAMS: &str = "params";
/// The key of a param holding fixed text.
pub const LOOKUP_KEY_CONST: &str = "const";
/// The param naming the value carrying the reference.
pub const LOOKUP_KEY_REFERENCE_VALUE: &str = ".";

/// The most params a computed key may list, the bound meta-schema v3 puts on
/// it.
pub const MAX_LOOKUP_KEY_PARAMS: usize = 16;

/// The most UTF-8 bytes a `const` param may hold.
pub const MAX_LOOKUP_KEY_CONST_BYTES: usize = 64;

/// The longest property path a param may name, the bound meta-schema v3 puts
/// on it.
pub const MAX_LOOKUP_KEY_PATH_LENGTH: usize = 256;

/// One param of a computed lookup key.
// @append_only
#[derive(Debug, PartialEq, Eq, Clone, Encode, Decode, DecodeUntrusted)]
pub enum LookupKeyParam {
    /// `"."`: the value carrying the reference where it has no path of its
    /// own: each element of a typed array, or for an `ownerRefersTo` or
    /// `creatorRefersTo` the writer's or creator's id. A single property
    /// carrying the reference is named by its path instead.
    ReferenceValue,
    /// A dotted path of a string, byte array or identifier property of the
    /// referring document type.
    Property(String),
    /// `{ "const": text }`: fixed text, as UTF-8.
    Const(String),
}

impl Serialize for LookupKeyParam {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            LookupKeyParam::ReferenceValue => serializer.serialize_str(LOOKUP_KEY_REFERENCE_VALUE),
            LookupKeyParam::Property(path) => serializer.serialize_str(path),
            LookupKeyParam::Const(text) => {
                let mut map = serializer.serialize_map(Some(1))?;
                map.serialize_entry(LOOKUP_KEY_CONST, text)?;
                map.end()
            }
        }
    }
}

/// A computed lookup key: the hash `function` of the bytes of `params`,
/// joined in order.
#[derive(Debug, PartialEq, Eq, Clone, Encode, Decode, DecodeUntrusted)]
pub struct LookupHashKey {
    /// The hash applied to the joined params.
    pub function: HashFunction,
    /// The params, in order.
    pub params: Vec<LookupKeyParam>,
}

impl Serialize for LookupHashKey {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(Some(2))?;
        map.serialize_entry(LOOKUP_KEY_FUNCTION, self.function.as_str())?;
        map.serialize_entry(LOOKUP_KEY_PARAMS, &self.params)?;
        map.end()
    }
}

/// Why a document's values cannot be joined into the preimage of a computed
/// key: a param it reads is absent, holds a value of a kind a param cannot
/// take, sits behind a repeated key, or is a variable-length value holding the
/// separator that follows it.
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct LookupPreimageError {
    /// The param at fault: a property path, or `"."` for the value carrying
    /// the reference.
    pub param: String,
    /// What is wrong with it.
    pub reason: String,
}

impl LookupHashKey {
    /// Reads a computed key from the value of a `findBy` entry:
    /// `{ "function": <a sys.hash function>, "params": [...] }`. What the
    /// params' properties resolve to is checked once the document type is
    /// parsed, see [`Self::referring_side_error`].
    pub fn from_value(value: &Value) -> Result<Self, DataContractError> {
        let map = value.to_btree_ref_string_map()?;
        if let Some(unknown) = map
            .keys()
            .find(|key| !matches!(key.as_str(), LOOKUP_KEY_FUNCTION | LOOKUP_KEY_PARAMS))
        {
            return Err(DataContractError::InvalidContractStructure(format!(
                "a refersTo findBy function takes function and params, not {unknown:?}"
            )));
        }
        let function_name = map
            .get(LOOKUP_KEY_FUNCTION)
            .and_then(|value| value.as_text())
            .ok_or_else(|| {
                DataContractError::InvalidContractStructure(
                    "a refersTo findBy function names its function, a string".to_string(),
                )
            })?;
        let function = match SystemFunction::from_wire_name(function_name) {
            Some(SystemFunction::Hash(function)) => function,
            _ => {
                return Err(DataContractError::InvalidContractStructure(format!(
                    "a refersTo findBy function {function_name:?} is not a hash, \
                     expected one of {:?}",
                    HashFunction::ALL.map(|function| function.as_str())
                )))
            }
        };
        let values = map
            .get(LOOKUP_KEY_PARAMS)
            .and_then(|value| value.as_array())
            .ok_or_else(|| {
                DataContractError::InvalidContractStructure(
                    "a refersTo findBy function lists its params".to_string(),
                )
            })?;
        if values.is_empty() || values.len() > MAX_LOOKUP_KEY_PARAMS {
            return Err(DataContractError::InvalidContractStructure(format!(
                "a refersTo findBy function lists between 1 and {MAX_LOOKUP_KEY_PARAMS} \
                 params, found {}",
                values.len()
            )));
        }
        let params = values
            .iter()
            .map(parse_param)
            .collect::<Result<Vec<_>, _>>()?;
        Ok(LookupHashKey { function, params })
    }

    /// How many params are `"."`, the value carrying the reference.
    pub fn reference_value_uses(&self) -> usize {
        self.params
            .iter()
            .filter(|param| matches!(param, LookupKeyParam::ReferenceValue))
            .count()
    }

    /// Every property path the params read, in order.
    pub fn properties_read(&self) -> impl Iterator<Item = &str> {
        self.params.iter().filter_map(|param| match param {
            LookupKeyParam::Property(path) => Some(path.as_str()),
            _ => None,
        })
    }

    /// Why this key, declared on `declaring`, cannot always be joined into a
    /// preimage that splits back one way; `None` when it can. Every property a param reads
    /// must exist and hold a single string, byte array or identifier. A param
    /// may be transient, since the key is read only from the create
    /// transition, where transient values are present, and optional, since a
    /// create missing a param is refused; a stored one must be fixed once
    /// written, so no replace can change the values a commitment was revealed
    /// for. Every variable-length param another variable-length param follows
    /// is followed directly by a one-byte `const`.
    pub fn referring_side_error(&self, declaring: DocumentTypeRef) -> Option<String> {
        for path in self.properties_read() {
            let Some(property) = declaring.flattened_properties().get(path) else {
                return Some(format!(
                    "param \"{path}\" is not a property of the referring document type"
                ));
            };
            if !matches!(
                property.property_type,
                DocumentPropertyType::String(_)
                    | DocumentPropertyType::ByteArray(_)
                    | DocumentPropertyType::Identifier
                    | DocumentPropertyType::IdentifierWithReference(_)
            ) {
                return Some(format!(
                    "param \"{path}\" is not a string, a byte array or an identifier"
                ));
            }
            if !is_transient(declaring, path)
                && !schema_property_is_fixed_once_written(declaring, path)
            {
                return Some(format!(
                    "param \"{path}\" is a property a replace can change: the key is checked \
                     when the document is created only, so every stored value it reads must be \
                     fixed once written (make the type immutable or list the property under \
                     `immutable`)"
                ));
            }
            if !is_transient(declaring, path)
                && clearable_once_its_document_is_deleted(declaring, path)
            {
                return Some(format!(
                    "param \"{path}\" is an optional `deletableDocument` reference a replace can \
                     clear once its document is deleted: the key is checked when the document is \
                     created only, so every stored value it reads must be fixed once written \
                     (make the reference required)"
                ));
            }
        }
        for (position, param) in self.params.iter().enumerate() {
            if !is_variable_length(param, declaring) {
                continue;
            }
            let followed_by_variable = self.params[position + 1..]
                .iter()
                .any(|later| is_variable_length(later, declaring));
            let separated = matches!(
                self.params.get(position + 1),
                Some(LookupKeyParam::Const(text)) if text.len() == 1
            );
            if followed_by_variable && !separated {
                let name = param_name(param);
                return Some(format!(
                    "param \"{name}\" has no fixed length and another such param follows it, so \
                     it must be followed directly by a one-byte const, the separator its value \
                     may not hold: without one the joined bytes could split into the params more \
                     than one way"
                ));
            }
        }
        None
    }

    /// The bytes of every param of a create of a document of `declaring`, in
    /// order, borrowed from the document where they can be: `"."` reads
    /// `reference_id`, the identifier carrying the reference where it has no
    /// path (an element's, the writer's or the creator's id; a property carrying
    /// the reference is named by its path, so it passes `None`). An error names
    /// the first param whose value is absent, of a kind a param cannot take,
    /// behind a repeated key, or a variable-length value holding the separator
    /// that follows it.
    fn parts<'a>(
        &'a self,
        declaring: DocumentTypeRef,
        reference_id: Option<Identifier>,
        document_data: &'a BTreeMap<String, Value>,
    ) -> Result<Vec<Cow<'a, [u8]>>, LookupPreimageError> {
        let mut parts = Vec::with_capacity(self.params.len());
        for (position, param) in self.params.iter().enumerate() {
            let bytes = match param {
                LookupKeyParam::Const(text) => {
                    parts.push(Cow::Borrowed(text.as_bytes()));
                    continue;
                }
                LookupKeyParam::ReferenceValue => {
                    let reference_id = reference_id.ok_or_else(|| LookupPreimageError {
                        param: LOOKUP_KEY_REFERENCE_VALUE.to_string(),
                        reason: "the value is absent".to_string(),
                    })?;
                    Cow::Owned(reference_id.to_buffer().to_vec())
                }
                LookupKeyParam::Property(path) => {
                    value_bytes(path, read_param(document_data, path)?)?
                }
            };
            // A separator is required, and so checked, only after a variable-length
            // param another variable-length param follows
            if is_variable_length(param, declaring)
                && self.params[position + 1..]
                    .iter()
                    .any(|later| is_variable_length(later, declaring))
            {
                if let Some(LookupKeyParam::Const(separator)) = self.params.get(position + 1) {
                    if let [separator_byte] = separator.as_bytes() {
                        if bytes.contains(separator_byte) {
                            return Err(LookupPreimageError {
                                param: param_name(param).to_string(),
                                reason: format!(
                                    "the value holds {separator:?}, the separator that follows \
                                     it in the preimage"
                                ),
                            });
                        }
                    }
                }
            }
            parts.push(bytes);
        }
        Ok(parts)
    }

    /// Whether a create of a document of `declaring` can reveal this key, the
    /// checks of [`Self::preimage`] without joining the bytes.
    pub fn check_preimage(
        &self,
        declaring: DocumentTypeRef,
        reference_id: Option<Identifier>,
        document_data: &BTreeMap<String, Value>,
    ) -> Result<(), LookupPreimageError> {
        self.parts(declaring, reference_id, document_data)
            .map(|_| ())
    }

    /// The preimage a create of a document of `declaring` reveals: the bytes of
    /// every param in order, as [`Self::parts`] reads them.
    pub fn preimage(
        &self,
        declaring: DocumentTypeRef,
        reference_id: Option<Identifier>,
        document_data: &BTreeMap<String, Value>,
    ) -> Result<Vec<u8>, LookupPreimageError> {
        Ok(self.parts(declaring, reference_id, document_data)?.concat())
    }

    /// The value of the index property this key fills for a create of a
    /// document of `declaring`, the hash of its preimage, with how many
    /// SHA-256 blocks computing it took ([`HashFunction::block_count`]), what
    /// the write is billed for.
    pub fn key_value(
        &self,
        declaring: DocumentTypeRef,
        reference_id: Option<Identifier>,
        document_data: &BTreeMap<String, Value>,
    ) -> Result<([u8; 32], u16), LookupPreimageError> {
        let preimage = self.preimage(declaring, reference_id, document_data)?;
        Ok((
            self.function.digest(&preimage),
            self.function.block_count(preimage.len()),
        ))
    }
}

/// Reads one param: `"."`, a dotted property path, or `{ "const": text }`.
fn parse_param(value: &Value) -> Result<LookupKeyParam, DataContractError> {
    if let Some(path) = value.as_text() {
        if path == LOOKUP_KEY_REFERENCE_VALUE {
            return Ok(LookupKeyParam::ReferenceValue);
        }
        if path.is_empty() || path.len() > MAX_LOOKUP_KEY_PATH_LENGTH || path.starts_with('$') {
            return Err(DataContractError::InvalidContractStructure(format!(
                "a refersTo findBy function param is \".\", {{ \"const\": text }} or the path \
                 of a schema property of 1 to {MAX_LOOKUP_KEY_PATH_LENGTH} characters, not \
                 {path:?}"
            )));
        }
        return Ok(LookupKeyParam::Property(path.to_string()));
    }
    let map = value.to_btree_ref_string_map().map_err(|_| {
        DataContractError::InvalidContractStructure(
            "a refersTo findBy function param is \".\", a property path or { \"const\": text }"
                .to_string(),
        )
    })?;
    let mut entries = map.iter();
    let (Some((key, text)), None) = (entries.next(), entries.next()) else {
        return Err(DataContractError::InvalidContractStructure(
            "a refersTo findBy function param object holds exactly one key, const".to_string(),
        ));
    };
    if key.as_str() != LOOKUP_KEY_CONST {
        return Err(DataContractError::InvalidContractStructure(format!(
            "a refersTo findBy function param object takes const, not {key:?}"
        )));
    }
    let text = text.as_text().ok_or_else(|| {
        DataContractError::InvalidContractStructure(
            "a refersTo findBy function const must be a string".to_string(),
        )
    })?;
    if text.is_empty() || text.len() > MAX_LOOKUP_KEY_CONST_BYTES {
        return Err(DataContractError::InvalidContractStructure(format!(
            "a refersTo findBy function const holds between 1 and \
             {MAX_LOOKUP_KEY_CONST_BYTES} bytes"
        )));
    }
    Ok(LookupKeyParam::Const(text.to_string()))
}

/// How a param is named in the errors.
fn param_name(param: &LookupKeyParam) -> &str {
    match param {
        LookupKeyParam::ReferenceValue => LOOKUP_KEY_REFERENCE_VALUE,
        LookupKeyParam::Property(path) => path,
        LookupKeyParam::Const(text) => text,
    }
}

/// Whether `param` has no fixed length: a string, or a byte array whose
/// minimum and maximum sizes differ, read from a property of `declaring`. A
/// `const` is fixed, and so is `"."`, always an identifier (an element's, the
/// writer's or the creator's id); a path naming no property is judged by
/// [`LookupHashKey::referring_side_error`].
fn is_variable_length(param: &LookupKeyParam, declaring: DocumentTypeRef) -> bool {
    let property_type = match param {
        LookupKeyParam::Const(_) | LookupKeyParam::ReferenceValue => return false,
        LookupKeyParam::Property(path) => match declaring.flattened_properties().get(path) {
            Some(property) => &property.property_type,
            None => return false,
        },
    };
    match property_type {
        DocumentPropertyType::String(_) => true,
        DocumentPropertyType::ByteArray(sizes) => {
            sizes.min_size.is_none() || sizes.min_size != sizes.max_size
        }
        _ => false,
    }
}

/// The value at a dotted `path` of a document's data, read through
/// [`read_data_at_path`]: a map on the way that holds the path's key more than
/// once is refused, as the `generatedFrom` check refuses it, since the schema
/// validation and the stored document keep the last of repeated keys, so a
/// commitment could otherwise be revealed for one value while another is
/// stored.
fn read_param<'a>(
    document_data: &'a BTreeMap<String, Value>,
    path: &str,
) -> Result<&'a Value, LookupPreimageError> {
    let error = |reason: &str| LookupPreimageError {
        param: path.to_string(),
        reason: reason.to_string(),
    };
    match read_data_at_path(document_data, path) {
        Ok(Some(value)) => Ok(value),
        Ok(None) => Err(error("the value is absent")),
        Err(RepeatedKey) => Err(error(REPEATED_KEY_REASON)),
    }
}

/// Why a value behind a repeated key is refused.
const REPEATED_KEY_REASON: &str = "a map on the way holds the key more than once";

/// The bytes a value contributes to a preimage: a string's UTF-8, or the bytes
/// of a byte array or identifier. Schema validation has already refused a value
/// of the wrong kind for its property, so the value is judged by its own shape.
fn value_bytes<'a>(param: &str, value: &'a Value) -> Result<Cow<'a, [u8]>, LookupPreimageError> {
    let error = || LookupPreimageError {
        param: param.to_string(),
        reason: "the value is not a string or bytes".to_string(),
    };
    match value {
        Value::Text(text) => Ok(Cow::Borrowed(text.as_bytes())),
        Value::Bytes(bytes) => Ok(Cow::Borrowed(bytes)),
        Value::Bytes20(bytes) => Ok(Cow::Borrowed(bytes)),
        Value::Bytes32(bytes) | Value::Identifier(bytes) => Ok(Cow::Borrowed(bytes)),
        Value::Bytes36(bytes) => Ok(Cow::Borrowed(bytes)),
        // A byte array a JSON transition carries as an array of numbers
        Value::Array(_) => value.to_binary_bytes().map(Cow::Owned).map_err(|_| error()),
        _ => Err(error()),
    }
}

/// The first reference of `document_type` whose computed lookup key a create
/// by `owner_id` carrying `document_data` cannot reveal, with the path the
/// reference is declared at (a property path, `$ownerId` or `$creatorId`): a
/// param it reads is absent, of the wrong kind or behind a repeated key, or a
/// variable-length value holds its separator. `None` when every such key can
/// be joined.
///
/// A property carrying a computed key anywhere in its declaration (a single
/// target, a leaf of a reference expression, on its elements) is first read
/// through [`read_data_at_path`]: a map on the way holding the property's key
/// more than once is refused, since the check would read the first of the
/// repeated keys while the schema validation and the stored document keep the
/// last, and could otherwise find the property unset and skip the reference.
///
/// Only a single target's preimage is judged here, on the writer, on the
/// creator (the writer, on a create), or on a property that holds a value,
/// since an unset reference property is not validated. A computed key on the
/// elements of a typed array, or that is a leaf of a reference expression, is
/// left to the write-time check, where a key it cannot join finds no
/// document: that value or that operand fails, and an expression's other
/// operands still decide.
pub fn first_unrevealable_lookup_key(
    document_type: DocumentTypeRef,
    document_data: &BTreeMap<String, Value>,
    owner_id: Identifier,
) -> Option<(String, LookupPreimageError)> {
    for (holder, reference) in document_type.reference_declarations() {
        let Some(target) = reference.target() else {
            continue;
        };
        let carried = match holder {
            ReferenceHolder::Property(path) => {
                let computed = target.leaves().into_iter().any(|leaf| {
                    leaf.as_any_document_reference()
                        .and_then(|declaration| declaration.lookup)
                        .is_some_and(|lookup| lookup.is_checked_on_create_only())
                });
                match read_data_at_path(document_data, path) {
                    Err(RepeatedKey) if computed => {
                        return Some((
                            path.to_string(),
                            LookupPreimageError {
                                param: path.to_string(),
                                reason: REPEATED_KEY_REASON.to_string(),
                            },
                        ))
                    }
                    Ok(Some(_)) => true,
                    _ => false,
                }
            }
            ReferenceHolder::Owner | ReferenceHolder::Creator => true,
        };
        let Some(key) = target
            .as_any_document_reference()
            .and_then(|declaration| declaration.lookup)
            .and_then(|lookup| lookup.hash_key())
            .map(|(_, key)| key)
        else {
            continue;
        };
        // An unset reference property is not validated
        if !carried {
            continue;
        }
        let reference_id = match (holder, reference) {
            (_, PropertyReference::Elements { .. } | PropertyReference::KeyId(_)) => continue,
            (ReferenceHolder::Owner | ReferenceHolder::Creator, _) => Some(owner_id),
            // A property carrying the reference is named by its path
            (ReferenceHolder::Property(_), _) => None,
        };
        if let Err(error) = key.check_preimage(document_type, reference_id, document_data) {
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
    use crate::data_contract::document_type::DocumentReferenceLookup;
    use crate::data_contract::document_type::DocumentType;
    use crate::util::hash::hash_double;
    use platform_value::platform_value;
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

    /// The DPNS preorder key, declared on the salt: `salt ++ normalizedLabel
    /// ++ "." ++ parentDomainName`.
    fn dpns_key() -> LookupHashKey {
        LookupHashKey::from_value(&platform_value!({
            "function": "sys.hash.sha256d",
            "params": ["preorderSalt", "normalizedLabel", { "const": "." }, "parentDomainName"]
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
    /// computes for a subdomain, written out as it does.
    fn trigger_hash(salt: [u8; 32], normalized_label: &str, parent: &str) -> Value {
        let full_domain_name = format!("{normalized_label}.{parent}");
        let mut salted_domain_buffer: Vec<u8> = vec![];
        salted_domain_buffer.extend(salt);
        salted_domain_buffer.extend(full_domain_name.as_bytes());
        Value::Bytes32(hash_double(salted_domain_buffer))
    }

    #[test]
    fn should_hash_a_subdomain_byte_for_byte_as_the_create_trigger() {
        let domain = domain();
        let key = dpns_key();
        let salt = [0x42; 32];
        for (normalized_label, parent) in [("al1ce", "dash"), ("b0b-2", "Some-Parent")] {
            // A byte array as bytes, or as the array of numbers a JSON
            // transition carries, hashes the same
            for salt_value in [
                Value::Bytes32(salt),
                Value::Array(salt.iter().map(|byte| Value::U8(*byte)).collect()),
            ] {
                let document = data(&[
                    ("normalizedLabel", normalized_label.into()),
                    ("parentDomainName", parent.into()),
                    ("preorderSalt", salt_value.clone()),
                ]);
                assert_eq!(
                    key.key_value(domain.as_ref(), None, &document)
                        .map(|(digest, _)| Value::Bytes32(digest)),
                    Ok(trigger_hash(salt, normalized_label, parent)),
                    "{normalized_label} {parent}"
                );
            }
        }
    }

    #[test]
    fn should_report_the_blocks_a_key_hashes_and_take_the_hash_the_caller_billed() {
        let domain = domain();
        let document = data(&[
            ("normalizedLabel", "al1ce".into()),
            ("parentDomainName", "dash".into()),
            ("preorderSalt", Value::Bytes32([0x42; 32])),
        ]);
        let hash = trigger_hash([0x42; 32], "al1ce", "dash");
        // 32 + 5 + 1 + 4 bytes pad to one block, plus the second pass
        assert_eq!(
            dpns_key()
                .key_value(domain.as_ref(), None, &document)
                .map(|(digest, blocks)| (Value::Bytes32(digest), blocks)),
            Ok((hash.clone(), 2))
        );
        // The lookup never hashes: its computed key is the hash the caller took
        // and billed, and without one the key has a missing part
        let lookup = DocumentReferenceLookup::new(BTreeMap::from([(
            "saltedDomainHash".to_string(),
            LookupKeySource::Hash(dpns_key()),
        )]));
        let owner = Identifier::from([1; 32]);
        assert_eq!(
            lookup.key_values(owner, &document, owner, Some(&hash)),
            Some(BTreeMap::from([("saltedDomainHash".to_string(), hash)]))
        );
        assert_eq!(lookup.key_values(owner, &document, owner, None), None);
    }

    #[test]
    fn should_refuse_a_carrier_behind_a_repeated_key_before_it_is_taken_for_unset() {
        // `a.b.salt` two objects deep: the first `b` holds no salt, the last does,
        // and the stored document keeps the last
        let platform_version = PlatformVersion::latest();
        let config =
            DataContractConfig::default_for_version(platform_version).expect("config should build");
        let deep_claim = DocumentType::try_from_schema(
            Identifier::from([1; 32]),
            1,
            config.version(),
            "deepClaim",
            platform_value!({
                "type": "object",
                "documentsMutable": false,
                "properties": {
                    "label": { "type": "string", "maxLength": 63u32, "position": 0u32 },
                    "a": {
                        "type": "object",
                        "position": 1u32,
                        "properties": {
                            "b": {
                                "type": "object",
                                "position": 0u32,
                                "properties": {
                                    "salt": {
                                        "type": "array",
                                        "byteArray": true,
                                        "minItems": 32u32,
                                        "maxItems": 32u32,
                                        "position": 0u32,
                                        "refersTo": {
                                            "type": "deletableDocument",
                                            "documentType": "preorder",
                                            "findBy": {
                                                "saltedDomainHash": {
                                                    "function": "sys.hash.sha256d",
                                                    "params": ["a.b.salt", "label"]
                                                }
                                            }
                                        }
                                    }
                                },
                                "required": ["salt"],
                                "additionalProperties": false
                            }
                        },
                        "required": ["b"],
                        "additionalProperties": false
                    }
                },
                "required": ["label", "a"],
                "additionalProperties": false
            }),
            None,
            &BTreeMap::new(),
            &config,
            false,
            &mut vec![],
            platform_version,
        )
        .expect("the document type should parse");
        let salted = |bs: Vec<Value>| {
            data(&[
                ("label", "deep".into()),
                (
                    "a",
                    Value::Map(
                        bs.into_iter()
                            .map(|b| (Value::Text("b".to_string()), b))
                            .collect(),
                    ),
                ),
            ])
        };
        let salt = Value::Map(vec![(
            Value::Text("salt".to_string()),
            Value::Bytes32([0x44; 32]),
        )]);
        let owner = Identifier::from([2; 32]);
        assert!(matches!(
            first_unrevealable_lookup_key(
                deep_claim.as_ref(),
                &salted(vec![Value::Map(vec![]), salt.clone()]),
                owner
            ),
            Some((path, LookupPreimageError { reason, .. }))
                if path == "a.b.salt" && reason == REPEATED_KEY_REASON
        ));
        assert_eq!(
            first_unrevealable_lookup_key(deep_claim.as_ref(), &salted(vec![salt]), owner),
            None
        );
    }

    #[test]
    fn should_refuse_a_preimage_missing_a_param_or_holding_a_separator() {
        let domain = domain();
        let key = dpns_key();
        let salt = Value::Bytes32([0x42; 32]);

        let missing_parent = data(&[
            ("normalizedLabel", "al1ce".into()),
            ("preorderSalt", salt.clone()),
        ]);
        assert!(matches!(
            key.preimage(domain.as_ref(), None, &missing_parent),
            Err(LookupPreimageError { param, .. }) if param == "parentDomainName"
        ));

        // "a.b" + "." + "c" and "a" + "." + "b.c" are the same bytes
        let dotted_label = data(&[
            ("normalizedLabel", "a.b".into()),
            ("parentDomainName", "c".into()),
            ("preorderSalt", salt.clone()),
        ]);
        assert!(matches!(
            key.preimage(domain.as_ref(), None, &dotted_label),
            Err(LookupPreimageError { param, .. }) if param == "normalizedLabel"
        ));

        // The last variable-length param may hold anything: nothing follows it
        let dotted_parent = data(&[
            ("normalizedLabel", "al1ce".into()),
            ("parentDomainName", "b.c".into()),
            ("preorderSalt", salt.clone()),
        ]);
        assert!(key.preimage(domain.as_ref(), None, &dotted_parent).is_ok());
    }

    #[test]
    fn should_refuse_a_param_behind_a_repeated_key() {
        let key = LookupHashKey::from_value(&platform_value!({
            "function": "sys.hash.sha256d",
            "params": [".", "meta.name"]
        }))
        .expect("the key should parse");
        let repeated = BTreeMap::from([(
            "meta".to_string(),
            Value::Map(vec![
                (Value::Text("name".into()), Value::Text("first".into())),
                (Value::Text("name".into()), Value::Text("second".into())),
            ]),
        )]);
        assert!(matches!(
            read_param(&repeated, "meta.name"),
            Err(LookupPreimageError { param, .. }) if param == "meta.name"
        ));
        assert_eq!(key.properties_read().collect::<Vec<_>>(), vec!["meta.name"]);
    }

    #[test]
    fn should_serialize_a_computed_key_as_its_function_and_params() {
        // The shape of a `findByFunction`, under the index property it fills,
        // as `findBy` declares it (see `LookupKeySource`)
        let source = LookupKeySource::Hash(dpns_key());
        assert_eq!(
            serde_json::to_value(&source).expect("the key serializes"),
            serde_json::json!({
                "function": "sys.hash.sha256d",
                "params": ["preorderSalt", "normalizedLabel", { "const": "." }, "parentDomainName"]
            })
        );
        assert_eq!(
            serde_json::to_value(LookupKeySource::ReferenceValue).expect("serializes"),
            serde_json::json!(".")
        );
    }

    #[test]
    fn should_refuse_malformed_computed_keys() {
        for key in [
            platform_value!({ "function": "sys.hash.sha256", "params": ["."] }),
            platform_value!({ "function": "sys.stringTransformations.lowercase", "params": ["."] }),
            platform_value!({ "function": "sys.hash.sha256d", "params": [] }),
            platform_value!({ "function": "sys.hash.sha256d" }),
            platform_value!({ "function": "sys.hash.sha256d", "params": ["."], "extra": 1 }),
            platform_value!({ "function": "sys.hash.sha256d", "params": [{ "const": "" }] }),
            platform_value!({ "function": "sys.hash.sha256d", "params": ["$ownerId"] }),
            platform_value!({ "function": "sys.hash.sha256d", "params": [{ "text": "." }] }),
            platform_value!({ "sha256d": ["."] }),
        ] {
            assert!(LookupHashKey::from_value(&key).is_err(), "{key:?}");
        }
    }
}
