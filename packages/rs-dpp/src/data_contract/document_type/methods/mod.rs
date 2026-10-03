#[cfg(feature = "validation")]
mod validate_update;
mod versioned_methods;

use std::borrow::Cow;
use std::collections::BTreeMap;

use crate::data_contract::document_type::index::{Index, IndexProperty};
use crate::data_contract::document_type::index_level::IndexLevel;
use crate::document::Document;
use crate::document::INITIAL_REVISION;
use crate::prelude::{BlockHeight, CoreBlockHeight, Revision};
use crate::validation::SimpleConsensusValidationResult;
use crate::version::PlatformVersion;
use crate::ProtocolError;

#[cfg(feature = "validation")]
use crate::consensus::basic::document::{
    DocumentPropertyMaxBytesExceededError, DocumentPropertyNotGeneratedError,
    InvalidEncryptedPropertyShapeError,
};
use crate::data_contract::document_type::accessors::{
    DocumentTypeV0Getters, DocumentTypeV2Getters,
};
use crate::data_contract::document_type::methods::versioned_methods::DocumentTypeV0MethodsVersioned;
use crate::data_contract::document_type::property_constraints::{
    DocumentSystemValues, SystemChange,
};
use crate::data_contract::document_type::DocumentPropertyType;
#[cfg(feature = "validation")]
use crate::data_contract::document_type::StringPropertySizes;
use crate::fee::Credits;
use crate::voting::vote_polls::VotePoll;
use platform_value::btreemap_extensions::{
    BTreeValueMapInsertionPathHelper, BTreeValueMapPathHelper,
};
use platform_value::{Identifier, Value};

pub trait DocumentTypeBasicMethods: DocumentTypeV0Getters {
    fn unique_id_for_storage(&self) -> [u8; 32] {
        rand::random::<[u8; 32]>()
    }

    fn unique_id_for_document_field(
        &self,
        index_level: &IndexLevel,
        base_event: [u8; 32],
    ) -> Vec<u8> {
        let mut bytes = index_level.identifier().to_be_bytes().to_vec();
        bytes.extend_from_slice(&base_event);
        bytes
    }

    fn initial_revision(&self) -> Option<Revision> {
        if self.requires_revision() {
            Some(INITIAL_REVISION)
        } else {
            None
        }
    }

    /// Whether the type lists fields only its contract's moderators write
    /// (`moderatorAbilities.changeFields`, protocol version 14): a moderator's
    /// change is stored as an update, so such a type keeps a `$revision` on its
    /// documents even when their owners can not replace them. False on every
    /// generation before the keyword.
    fn has_moderator_changeable_fields(&self) -> bool {
        false
    }

    /// The type of the value a derived index property of the type holds (an
    /// index property `"<reference property>.<field>"` read from the referenced
    /// document, protocol version 14), `None` for any other name and on every
    /// generation before them. Encodes the values of such a property into keys,
    /// as [`DocumentTypeV0Getters::flattened_properties`] gives the types of the
    /// properties the documents hold.
    fn derived_index_property_type(&self, _name: &str) -> Option<&DocumentPropertyType> {
        None
    }

    fn requires_revision(&self) -> bool {
        self.documents_mutable()
            || self.documents_transferable().is_transferable()
            || self.trade_mode().seller_sets_price()
            || self.has_moderator_changeable_fields()
    }

    /// Checks the shape of every `encryptedFor` property `properties` supplies
    /// against the scheme its declaration names: at least the IV plus one
    /// block, and a multiple of the block length. Nothing else about a
    /// ciphertext is verifiable on chain. A declared property the document
    /// leaves out is not checked; whether it may be left out is the schema's
    /// `required` list's business.
    ///
    /// Meant to run after the JSON schema validation of `properties`, which
    /// already established that every supplied value is a byte array. A value
    /// that still is not one is reported as a zero-length ciphertext rather
    /// than skipped, so the two nodes can never disagree on it.
    ///
    /// Versioned on `validate_encrypted_property_shapes` in the document type
    /// method versions: `None` before protocol version 14 returns an empty
    /// result, which keeps the shipped structure validations that call it inert.
    #[cfg(feature = "validation")]
    fn validate_encrypted_property_shapes(
        &self,
        properties: &BTreeMap<String, Value>,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, ProtocolError> {
        match platform_version
            .dpp
            .contract_versions
            .document_type_versions
            .methods
            .validate_encrypted_property_shapes
        {
            None => Ok(SimpleConsensusValidationResult::default()),
            Some(0) => Ok(self.validate_encrypted_property_shapes_v0(properties)),
            Some(version) => Err(ProtocolError::UnknownVersionMismatch {
                method: "validate_encrypted_property_shapes".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }

    #[cfg(feature = "validation")]
    fn validate_encrypted_property_shapes_v0(
        &self,
        properties: &BTreeMap<String, Value>,
    ) -> SimpleConsensusValidationResult {
        let declared = self
            .flattened_properties()
            .iter()
            .filter_map(|(path, property)| Some((path, property.encrypted_for.as_ref()?)));
        for (path, encrypted_for) in declared {
            let Ok(Some(value)) = properties.get_optional_at_path(path) else {
                continue;
            };
            let length = match value {
                Value::Bytes(bytes) => bytes.len(),
                Value::Bytes20(_) => 20,
                Value::Bytes32(_) | Value::Identifier(_) => 32,
                Value::Bytes36(_) => 36,
                Value::Array(items) => items.len(),
                other => other
                    .to_binary_bytes()
                    .map(|bytes| bytes.len())
                    .unwrap_or(0),
            };
            let scheme = encrypted_for.scheme;
            if !scheme.is_valid_ciphertext_length(length) {
                return SimpleConsensusValidationResult::new_with_error(
                    InvalidEncryptedPropertyShapeError::new(
                        path.clone(),
                        scheme.as_str().to_string(),
                        u32::try_from(length).unwrap_or(u32::MAX),
                        scheme.minimum_ciphertext_length() as u32,
                        scheme.block_length() as u32,
                    )
                    .into(),
                );
            }
        }
        SimpleConsensusValidationResult::new()
    }

    /// Checks every string `properties` (the document's properties map) supplies for a
    /// string declaring `maxBytes` against it, counting UTF-8 bytes: the property's value,
    /// or every element of a typed array of strings, whose error names the element
    /// (`tags[2]`). A declared property the document leaves out is not checked, and a value
    /// that is not a string holds no bytes to count: the JSON schema validation that
    /// `DataContract::validate_document_properties` runs alongside refuses it.
    ///
    /// Versioned on `validate_max_bytes` in the document type method versions: `None`
    /// before protocol version 14 returns an empty result, which keeps the shipped document
    /// validation that calls it inert.
    #[cfg(feature = "validation")]
    fn validate_max_bytes_properties(
        &self,
        properties: &Value,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, ProtocolError> {
        match platform_version
            .dpp
            .contract_versions
            .document_type_versions
            .methods
            .validate_max_bytes
        {
            None => Ok(SimpleConsensusValidationResult::default()),
            Some(0) => Ok(self.validate_max_bytes_properties_v0(properties)),
            Some(version) => Err(ProtocolError::UnknownVersionMismatch {
                method: "validate_max_bytes_properties".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }

    #[cfg(feature = "validation")]
    fn validate_max_bytes_properties_v0(
        &self,
        properties: &Value,
    ) -> SimpleConsensusValidationResult {
        let over = |path: String, value: &Value, max_bytes: u16| {
            let length = value.as_text()?.len();
            (length > max_bytes as usize).then(|| {
                DocumentPropertyMaxBytesExceededError::new(
                    path,
                    u32::try_from(length).unwrap_or(u32::MAX),
                    max_bytes,
                )
            })
        };
        for (path, property) in self.flattened_properties() {
            // A lookup error (an intermediate that is not a map) reads as absent: the schema
            // validation refuses that shape on its own
            let error = match &property.property_type {
                DocumentPropertyType::String(StringPropertySizes {
                    max_bytes: Some(max_bytes),
                    ..
                }) => {
                    let Ok(Some(value)) = properties.get_optional_value_at_path(path) else {
                        continue;
                    };
                    over(path.clone(), value, *max_bytes)
                }
                // A typed array declares on its items: every element is bounded on its own
                DocumentPropertyType::TypedArray(typed_array) => {
                    let DocumentPropertyType::String(StringPropertySizes {
                        max_bytes: Some(max_bytes),
                        ..
                    }) = typed_array.item_type.as_ref()
                    else {
                        continue;
                    };
                    let Ok(Some(Value::Array(elements))) =
                        properties.get_optional_value_at_path(path)
                    else {
                        continue;
                    };
                    elements.iter().enumerate().find_map(|(index, element)| {
                        over(format!("{path}[{index}]"), element, *max_bytes)
                    })
                }
                _ => continue,
            };
            if let Some(error) = error {
                return SimpleConsensusValidationResult::new_with_error(error.into());
            }
        }
        SimpleConsensusValidationResult::new()
    }

    /// Writes every `generatedFrom` property `data` (a created or replaced document's
    /// properties, as they arrive) leaves out, generated from its parameters: the platform
    /// generates a property a client does not send, and checks one it does send
    /// (`validate_generated_from_properties`). A property the document supplies is left as
    /// it is, whatever it holds, and nothing is written when a parameter is absent or is not
    /// a string (the schema validation refuses a parameter that is not a string on its own).
    ///
    /// Runs wherever a document arrives, before anything reads its data: the action
    /// transformers of document create, replace and indexOnly delete, and the proof
    /// verification that rebuilds the document a transition wrote.
    ///
    /// Versioned on `fill_generated_properties` in the document type method versions:
    /// `None` before protocol version 14 leaves `data` untouched, which keeps the shipped
    /// transformers and proof verification that call it inert.
    fn fill_generated_properties(
        &self,
        data: &mut BTreeMap<String, Value>,
        platform_version: &PlatformVersion,
    ) -> Result<(), ProtocolError>
    where
        Self: DocumentTypeV2Getters,
    {
        self.generate_properties(data, false, platform_version)
    }

    /// Sets every `generatedFrom` property of `data` (a document a client is about to
    /// send) to what the platform generates from its current parameters, replacing a value
    /// it holds and removing it when a parameter is absent. A document fetched, edited and
    /// sent back keeps the generated values of its old parameters, which the platform
    /// refuses; the transition builders call this instead of `fill_generated_properties`
    /// so the transition carries the values the platform would generate.
    ///
    /// Versioned on `fill_generated_properties` like it: `None` before protocol version 14
    /// leaves `data` untouched.
    fn regenerate_generated_properties(
        &self,
        data: &mut BTreeMap<String, Value>,
        platform_version: &PlatformVersion,
    ) -> Result<(), ProtocolError>
    where
        Self: DocumentTypeV2Getters,
    {
        self.generate_properties(data, true, platform_version)
    }

    /// `data` (a created or replaced document's properties, or an indexOnly delete's
    /// values, as a transition carries them) as the platform reads it on arrival: with
    /// every `generatedFrom` property it leaves out generated (`fill_generated_properties`).
    /// Borrowed as it is when the document type declares none, which covers every type
    /// parsed before protocol version 14; copied otherwise. For a reader of a transition
    /// outside its execution, such as a subscription filter, that must see what the
    /// platform stores.
    fn data_as_stored<'a>(
        &self,
        data: &'a BTreeMap<String, Value>,
        platform_version: &PlatformVersion,
    ) -> Result<Cow<'a, BTreeMap<String, Value>>, ProtocolError>
    where
        Self: DocumentTypeV2Getters,
    {
        if self.generated_from_fields().is_empty() {
            return Ok(Cow::Borrowed(data));
        }
        let mut stored = data.clone();
        self.fill_generated_properties(&mut stored, platform_version)?;
        Ok(Cow::Owned(stored))
    }

    /// `fill_generated_properties` (`replace_present` false) and
    /// `regenerate_generated_properties` (`replace_present` true).
    fn generate_properties(
        &self,
        data: &mut BTreeMap<String, Value>,
        replace_present: bool,
        platform_version: &PlatformVersion,
    ) -> Result<(), ProtocolError>
    where
        Self: DocumentTypeV2Getters,
    {
        match platform_version
            .dpp
            .contract_versions
            .document_type_versions
            .methods
            .fill_generated_properties
        {
            None => Ok(()),
            Some(0) => {
                self.generate_properties_v0(data, replace_present);
                Ok(())
            }
            Some(version) => Err(ProtocolError::UnknownVersionMismatch {
                method: "fill_generated_properties".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }

    fn generate_properties_v0(&self, data: &mut BTreeMap<String, Value>, replace_present: bool)
    where
        Self: DocumentTypeV2Getters,
    {
        for (path, generated_from) in self.generated_from_fields() {
            if replace_present {
                remove_at_path(data, path);
            } else if !matches!(data.get_optional_at_path(path), Ok(None)) {
                // Supplied, or unreadable (an intermediate that is not a map, which the
                // schema validation refuses on its own): either way not the platform's to
                // write
                continue;
            }
            let arguments: Option<Vec<&str>> = generated_from
                .property_params()
                .map(|param| match data.get_optional_at_path(param) {
                    Ok(Some(Value::Text(value))) => Some(value.as_str()),
                    _ => None,
                })
                .collect();
            let Some(generated) =
                arguments.and_then(|arguments| generated_from.function.apply(&arguments))
            else {
                continue;
            };
            // Registration puts every parameter inside every object that holds the
            // property, so the objects on the way are present and are maps: the parameters
            // were read through them. The insert cannot fail; if it ever did, the property
            // would stay absent and the generatedFrom check would refuse the document.
            let _ = data.insert_at_path(path, Value::Text(generated));
        }
    }

    /// Checks every `generatedFrom` property of `properties` (the document's properties
    /// map) against its parameters: when every parameter is present the property must hold
    /// what the function generates from them, and when a parameter is absent the property
    /// must be absent too. A document that repeats a key on the way to the property or to
    /// a parameter is refused as well: the schema validation and the stored document keep
    /// the last of repeated keys, where the platform generated from the first. The first
    /// property that does not pass is refused with `DocumentPropertyNotGeneratedError`. A
    /// value that is not a string is not compared: the JSON schema validation that
    /// `DataContract::validate_document_properties` runs alongside refuses it, and its
    /// result is reported first.
    ///
    /// A document that arrived at the platform has been through
    /// `fill_generated_properties`, so a left-out property whose parameters are present is
    /// already written; one that has not (a client validating a document before sending
    /// it) is refused for the missing property.
    ///
    /// Versioned on `validate_generated_from` in the document type method versions: `None`
    /// before protocol version 14 returns an empty result, which keeps the shipped document
    /// validation that calls it inert.
    #[cfg(feature = "validation")]
    fn validate_generated_from_properties(
        &self,
        properties: &Value,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, ProtocolError>
    where
        Self: DocumentTypeV2Getters,
    {
        match platform_version
            .dpp
            .contract_versions
            .document_type_versions
            .methods
            .validate_generated_from
        {
            None => Ok(SimpleConsensusValidationResult::default()),
            Some(0) => Ok(self.validate_generated_from_properties_v0(properties)),
            Some(version) => Err(ProtocolError::UnknownVersionMismatch {
                method: "validate_generated_from_properties".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }

    #[cfg(feature = "validation")]
    fn validate_generated_from_properties_v0(
        &self,
        properties: &Value,
    ) -> SimpleConsensusValidationResult
    where
        Self: DocumentTypeV2Getters,
    {
        for (path, generated_from) in self.generated_from_fields() {
            let generated = match (
                read_at_path(properties, path),
                generated_from
                    .property_params()
                    .map(|param| read_at_path(properties, param))
                    .collect::<Result<Vec<_>, RepeatedKey>>(),
            ) {
                (Err(RepeatedKey), _) | (_, Err(RepeatedKey)) => false,
                (Ok(value), Ok(arguments)) => {
                    if arguments.iter().any(Option::is_none) {
                        // Nothing to generate from: the property must be left out too
                        value.is_none()
                    } else {
                        let texts: Option<Vec<&str>> = arguments
                            .iter()
                            .map(|argument| argument.and_then(|argument| argument.as_text()))
                            .collect();
                        match (value.map(|value| value.as_text()), texts) {
                            (None, _) => false,
                            (Some(Some(value)), Some(arguments)) => {
                                generated_from.function.apply(&arguments).as_deref() == Some(value)
                            }
                            // A value that is not a string: the schema validation refuses it
                            _ => true,
                        }
                    }
                }
            };
            if !generated {
                return SimpleConsensusValidationResult::new_with_error(
                    DocumentPropertyNotGeneratedError::new(
                        self.name().clone(),
                        path.clone(),
                        generated_from.function.as_str().to_string(),
                        generated_from
                            .property_params()
                            .map(str::to_string)
                            .collect(),
                    )
                    .into(),
                );
            }
        }
        SimpleConsensusValidationResult::new()
    }

    fn top_level_indices(&self) -> Vec<&IndexProperty> {
        self.indexes()
            .values()
            .filter_map(|index| index.properties.first())
            .collect()
    }

    // This should normally just be 1 item, however we keep a vec in case we want to change things
    //  in the future.
    fn top_level_indices_of_contested_unique_indexes(&self) -> Vec<&IndexProperty> {
        self.indexes()
            .values()
            .filter_map(|index| {
                if index.contested_index.is_some() {
                    index.properties.first()
                } else {
                    None
                }
            })
            .collect()
    }
}

/// A map on the way to a path holds the path's key more than once.
pub struct RepeatedKey;

/// The value at a dotted `path` of a document's properties: `Ok(None)` when it is absent or
/// the path runs through a value that is not a map (the schema validation refuses that
/// shape on its own), `Err` when a map on the way holds the path's key more than once. The
/// schema validation and the stored document keep the last of repeated keys, where a plain
/// path read finds the first, so a value checked through the first could differ from the
/// one stored: every check that must read what is stored reads through here.
#[cfg(feature = "validation")]
fn read_at_path<'a>(properties: &'a Value, path: &str) -> Result<Option<&'a Value>, RepeatedKey> {
    read_path_segments(properties, path.split('.'))
}

/// The value at a dotted `path` of a document's data, read as [`read_at_path`] reads it.
pub fn read_data_at_path<'a>(
    data: &'a BTreeMap<String, Value>,
    path: &str,
) -> Result<Option<&'a Value>, RepeatedKey> {
    let mut segments = path.split('.');
    let Some(value) = segments.next().and_then(|first| data.get(first)) else {
        return Ok(None);
    };
    read_path_segments(value, segments)
}

/// The value `segments` lead to from `current`, refusing a map that holds a segment's key
/// more than once.
fn read_path_segments<'a, 'p>(
    mut current: &'a Value,
    segments: impl Iterator<Item = &'p str>,
) -> Result<Option<&'a Value>, RepeatedKey> {
    for segment in segments {
        let Value::Map(map) = current else {
            return Ok(None);
        };
        let mut matches = map
            .iter()
            .filter(|(key, _)| key.as_text() == Some(segment))
            .map(|(_, value)| value);
        let Some(value) = matches.next() else {
            return Ok(None);
        };
        if matches.next().is_some() {
            return Err(RepeatedKey);
        }
        current = value;
    }
    Ok(Some(current))
}

/// Removes the value at a dotted `path` of a document's properties, if there is one.
fn remove_at_path(data: &mut BTreeMap<String, Value>, path: &str) {
    match path.split_once('.') {
        None => {
            data.remove(path);
        }
        Some((head, rest)) => {
            if let Some(value) = data.get_mut(head) {
                // A value on the way that is not a map holds nothing to remove
                let _ = value.remove_optional_value_at_path(rest);
            }
        }
    }
}

/// The flat field list the v0 (protocol versions <= 13, frozen on chain)
/// matchers run over, assembled the way `select_best_index` always has:
/// equality fields first, then the range and `in` fields, then any
/// order-by fields not already present. The v0 selection loops must keep
/// receiving exactly this list for on-chain replay.
fn flatten_query_fields<'a>(
    equality_fields: &[&'a str],
    range_field: Option<&'a str>,
    in_field_name: Option<&'a str>,
    order_by: &[&'a str],
) -> Vec<&'a str> {
    let mut fields = equality_fields.to_vec();
    if let Some(range_field) = range_field {
        fields.push(range_field);
    }
    if let Some(in_field_name) = in_field_name {
        fields.push(in_field_name);
    }
    for field in order_by {
        if !fields.contains(field) {
            fields.push(field);
        }
    }
    fields
}

// TODO: Some of those methods are only for tests. Hide under feature
pub trait DocumentTypeV0Methods: DocumentTypeV0Getters + DocumentTypeV0MethodsVersioned {
    /// Best index for a query's bound fields, given by role. Version 0
    /// (protocol versions <= 13, frozen on chain) flattens the roles into
    /// one positionless field set; version 1 (protocol version 14+)
    /// additionally requires the fields to cover a contiguous prefix of
    /// the index — the shape the positional path lowering depends on.
    fn index_for_types(
        &self,
        equality_fields: &[&str],
        range_field: Option<&str>,
        in_field_name: Option<&str>,
        order_by: &[&str],
        platform_version: &PlatformVersion,
    ) -> Result<Option<(&Index, u16)>, ProtocolError> {
        match platform_version
            .dpp
            .contract_versions
            .document_type_versions
            .methods
            .index_for_types
        {
            0 => Ok(self.index_for_types_v0(
                &flatten_query_fields(equality_fields, range_field, in_field_name, order_by),
                in_field_name,
                order_by,
            )),
            1 => Ok(self.index_for_types_v1(equality_fields, range_field, in_field_name, order_by)),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "index_for_types".to_string(),
                known_versions: vec![0, 1],
                received: version,
            }),
        }
    }

    /// [`Self::index_for_types`] restricted to the indexes `filter` admits.
    ///
    /// Candidates rejected by `filter` are skipped before they are scored, so
    /// they can never be returned. This is the only correct way to require a
    /// property of the selected index: because several indexes can cover the
    /// same fields and ties are broken by the index map's name ordering,
    /// checking the property after an unrestricted search can reject the
    /// winner but cannot surface the index the caller actually needed.
    ///
    /// Shares the `index_for_types` feature-version gate — the filter narrows
    /// the candidate set, it does not change how a candidate is scored.
    fn index_for_types_matching(
        &self,
        equality_fields: &[&str],
        range_field: Option<&str>,
        in_field_name: Option<&str>,
        order_by: &[&str],
        filter: impl Fn(&Index) -> bool,
        platform_version: &PlatformVersion,
    ) -> Result<Option<(&Index, u16)>, ProtocolError> {
        match platform_version
            .dpp
            .contract_versions
            .document_type_versions
            .methods
            .index_for_types
        {
            0 => Ok(self.index_for_types_matching_v0(
                &flatten_query_fields(equality_fields, range_field, in_field_name, order_by),
                in_field_name,
                order_by,
                filter,
            )),
            1 => Ok(self.index_for_types_matching_v1(
                equality_fields,
                range_field,
                in_field_name,
                order_by,
                filter,
            )),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "index_for_types_matching".to_string(),
                known_versions: vec![0, 1],
                received: version,
            }),
        }
    }

    /// [`Self::index_for_types_matching`] with terminals (an indexOnly
    /// index's member-key property) as matchable deepest components. The
    /// returned bool says whether the winning index consumed its terminal;
    /// generic (non-terminal) matches always take precedence, so on any
    /// document type without terminals this is exactly
    /// [`Self::index_for_types_matching`]. Shares the `index_for_types`
    /// feature-version gate: terminal participation only changes outcomes
    /// on indexOnly document types, which cannot exist below protocol
    /// version 14.
    fn index_for_types_matching_including_terminal(
        &self,
        equality_fields: &[&str],
        range_field: Option<&str>,
        in_field_name: Option<&str>,
        order_by: &[&str],
        filter: impl Fn(&Index) -> bool,
        platform_version: &PlatformVersion,
    ) -> Result<Option<(&Index, u16, bool)>, ProtocolError> {
        match platform_version
            .dpp
            .contract_versions
            .document_type_versions
            .methods
            .index_for_types
        {
            0 => Ok(self.index_for_types_matching_including_terminal_v0(
                &flatten_query_fields(equality_fields, range_field, in_field_name, order_by),
                in_field_name,
                order_by,
                filter,
            )),
            1 => Ok(self.index_for_types_matching_including_terminal_v1(
                equality_fields,
                range_field,
                in_field_name,
                order_by,
                filter,
            )),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "index_for_types_matching_including_terminal".to_string(),
                known_versions: vec![0, 1],
                received: version,
            }),
        }
    }

    fn serialize_value_for_key(
        &self,
        key: &str,
        value: &Value,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, ProtocolError> {
        match platform_version
            .dpp
            .contract_versions
            .document_type_versions
            .methods
            .serialize_value_for_key
        {
            0 => self.serialize_value_for_key_v0(key, value),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "serialize_value_for_key".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }
    fn deserialize_value_for_key(
        &self,
        key: &str,
        serialized_value: &[u8],
        platform_version: &PlatformVersion,
    ) -> Result<Value, ProtocolError> {
        match platform_version
            .dpp
            .contract_versions
            .document_type_versions
            .methods
            .deserialize_value_for_key
        {
            0 => self.deserialize_value_for_key_v0(key, serialized_value),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "deserialize_value_for_key".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }

    fn max_size(&self, platform_version: &PlatformVersion) -> Result<u16, ProtocolError> {
        match platform_version
            .dpp
            .contract_versions
            .document_type_versions
            .methods
            .max_size
        {
            0 => self.max_size_v0(platform_version),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "max_size".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }

    fn estimated_size(&self, platform_version: &PlatformVersion) -> Result<u16, ProtocolError> {
        match platform_version
            .dpp
            .contract_versions
            .document_type_versions
            .methods
            .estimated_size
        {
            0 => self.estimated_size_v0(platform_version),
            1 => self.estimated_size_v1(platform_version),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "estimated_size".to_string(),
                known_versions: vec![0, 1],
                received: version,
            }),
        }
    }

    fn create_document_from_data(
        &self,
        data: Value,
        owner_id: Identifier,
        block_height: BlockHeight,
        core_block_height: CoreBlockHeight,
        document_entropy: [u8; 32],
        platform_version: &PlatformVersion,
    ) -> Result<Document, ProtocolError> {
        match platform_version
            .dpp
            .contract_versions
            .document_type_versions
            .methods
            .create_document_from_data
        {
            0 => self.create_document_from_data_v0(
                data,
                owner_id,
                block_height,
                core_block_height,
                document_entropy,
                platform_version,
            ),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "create_document_from_data".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }

    /// Creates a document at the current time based on specified document type information.
    /// This function requires that all properties provided are pre-validated according to
    /// the document's schema requirements.
    ///
    /// # Parameters:
    /// - `id`: An identifier for the document. Unique within the context of the document's type.
    /// - `owner_id`: The identifier of the entity that will own this document.
    /// - `block_height`: The block height at which this document is considered to have been created.
    ///   While this value is recorded in the document, it is ignored when the document is broadcasted
    ///   to the network. This is because the actual block height at the time of broadcast may differ.
    ///   This parameter is included to fulfill schema requirements that specify a block height; you may
    ///   use the current block height, a placeholder value of 0, or any other value as necessary.
    /// - `core_block_height`: Similar to `block_height`, this represents the core network's block height
    ///   at the document's creation time. It is handled the same way as `block_height` regarding broadcast
    ///   and schema requirements.
    /// - `properties`: A collection of properties for the document, structured as a `BTreeMap<String, Value>`.
    ///   These must be pre-validated to match the document's schema definitions.
    /// - `platform_version`: A reference to the current version of the platform for which the document is created.
    ///
    /// # Returns:
    /// A `Result<Document, ProtocolError>`, which is `Ok` if the document was successfully created, or an error
    /// indicating what went wrong during the creation process.
    ///
    /// # Note:
    /// The `block_height` and `core_block_height` are primarily included for schema compliance and local record-keeping.
    /// These values are not used when the document is broadcasted to the network, as the network assigns its own block
    /// heights upon receipt and processing of the document. After broadcasting, it is recommended to update these fields
    /// in their created_at/updated_at variants as well as the base created_at/updated_at in the client-side
    /// representation of the document to reflect the values returned by the network. The base created_at/updated_at
    /// uses current time when creating the local document and is also ignored as it is also set network side.
    fn create_document_with_prevalidated_properties(
        &self,
        id: Identifier,
        owner_id: Identifier,
        block_height: BlockHeight,
        core_block_height: CoreBlockHeight,
        properties: BTreeMap<String, Value>,
        platform_version: &PlatformVersion,
    ) -> Result<Document, ProtocolError> {
        match platform_version
            .dpp
            .contract_versions
            .document_type_versions
            .methods
            .create_document_with_prevalidated_properties
        {
            0 => self.create_document_with_prevalidated_properties_v0(
                id,
                owner_id,
                block_height,
                core_block_height,
                properties,
                platform_version,
            ),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "create_document_with_prevalidated_properties".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }

    /// Figures out the minimum prefunded voting balance needed for a document
    fn prefunded_voting_balance_for_document(
        &self,
        document: &Document,
        platform_version: &PlatformVersion,
    ) -> Result<Option<(String, Credits)>, ProtocolError> {
        match platform_version
            .dpp
            .contract_versions
            .document_type_versions
            .methods
            .prefunded_voting_balance_for_document
        {
            0 => Ok(self.prefunded_voting_balance_for_document_v0(document, platform_version)),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "prefunded_voting_balances_for_document".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }

    /// Gets the vote poll associated with a document
    fn contested_vote_poll_for_document(
        &self,
        document: &Document,
        platform_version: &PlatformVersion,
    ) -> Result<Option<VotePoll>, ProtocolError> {
        match platform_version
            .dpp
            .contract_versions
            .document_type_versions
            .methods
            .contested_vote_poll_for_document
        {
            0 => self.contested_vote_poll_for_document_v0(document, platform_version),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "contested_vote_poll_for_document".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }
    /// Gets the vote poll associated with a document
    fn contested_vote_poll_for_document_properties(
        &self,
        document_properties: &BTreeMap<String, Value>,
        platform_version: &PlatformVersion,
    ) -> Result<Option<VotePoll>, ProtocolError> {
        match platform_version
            .dpp
            .contract_versions
            .document_type_versions
            .methods
            .contested_vote_poll_for_document
        {
            0 => self.contested_vote_poll_for_document_properties_v0(
                document_properties,
                platform_version,
            ),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "contested_vote_poll_for_document_properties".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }

    /// Judges the `distinctFrom` declarations of the document type against a document's
    /// `data` and the id of the identity writing it: an identifier property whose value
    /// equals the named property of the same document, or the writer's `$ownerId`, fails
    /// with `DocumentPropertyNotDistinctError` (10419). A declaration whose named property
    /// is absent from `data` passes. Reads the transition alone, so it runs in the structure
    /// stage of document create and replace, and of transfer and purchase against the
    /// stored document and its new owner.
    ///
    /// `None` in the version table (protocol versions before 14) selects the behavior of
    /// the versions that predate the keyword: nothing is checked, as no parsed property
    /// carries a declaration there.
    fn validate_distinct_from_properties(
        &self,
        data: &BTreeMap<String, Value>,
        owner_id: Identifier,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, ProtocolError>
    where
        Self: DocumentTypeV2Getters,
    {
        match platform_version
            .dpp
            .contract_versions
            .document_type_versions
            .methods
            .validate_distinct_from
        {
            None => Ok(SimpleConsensusValidationResult::default()),
            Some(0) => Ok(self.validate_distinct_from_properties_v0(data, owner_id)),
            Some(version) => Err(ProtocolError::UnknownVersionMismatch {
                method: "validate_distinct_from_properties".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }

    /// Judges a document's properties, `data` (a map), against every rule of the document
    /// type's `propertyConstraints`, in name order: the first rule it breaks fails with
    /// `DocumentPropertyConstraintViolatedError` (10422), naming the rule and why (the rule
    /// does not hold, or evaluating it overflowed, divided by zero, raised to a negative
    /// power or read a value that is not an integer). A property the document
    /// leaves out counts as 0, or as its `ifAbsent` value; `$ownerId` and the system
    /// times and heights read `system`, the values of the document version being written
    /// (an owner the caller does not know equals no identifier, and a rule reading a time
    /// or height it does not know is not judged). Reads the properties and `system` alone:
    /// `DataContract::validate_document_properties` runs it after the schema validation,
    /// so document create and replace, and every client validating a document, apply it.
    ///
    /// `None` in the version table (protocol versions before 14) selects the behavior of
    /// the versions that predate the keyword: nothing is checked, as no parsed document
    /// type carries a rule there.
    fn validate_property_constraints(
        &self,
        data: &Value,
        system: &DocumentSystemValues,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, ProtocolError>
    where
        Self: DocumentTypeV2Getters,
    {
        match platform_version
            .dpp
            .contract_versions
            .document_type_versions
            .methods
            .validate_property_constraints
        {
            None => Ok(SimpleConsensusValidationResult::default()),
            Some(0) => self.validate_property_constraints_v0(data, system),
            Some(version) => Err(ProtocolError::UnknownVersionMismatch {
                method: "validate_property_constraints".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }

    /// Judges a stored document's properties, `data`, against the rules of the document
    /// type's `propertyConstraints` that `change` can break, with `system` the document's
    /// system values after it, in name order. A transfer or a purchase gives the document a
    /// new owner and a new transfer time and heights, and a price update a new update time
    /// and heights; neither changes a property, so the rules reading what it changes are
    /// the only ones it can break ([`PropertyConstraint::reads_change`]), and the first
    /// broken fails with `DocumentPropertyConstraintViolatedError` (10422) as it would on a
    /// write. The document type's other rules held when the document was written and still
    /// do. A type with no such rule costs nothing, and its data is not copied.
    ///
    /// Versioned with [`Self::validate_property_constraints`]: `None` before protocol
    /// version 14, where no parsed document type carries a rule.
    ///
    /// [`PropertyConstraint::reads_change`]: crate::data_contract::document_type::property_constraints::PropertyConstraint::reads_change
    fn validate_property_constraints_for_system_change(
        &self,
        data: &BTreeMap<String, Value>,
        system: &DocumentSystemValues,
        change: SystemChange,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, ProtocolError>
    where
        Self: DocumentTypeV2Getters,
    {
        match platform_version
            .dpp
            .contract_versions
            .document_type_versions
            .methods
            .validate_property_constraints
        {
            None => Ok(SimpleConsensusValidationResult::default()),
            Some(0) => {
                self.validate_property_constraints_for_system_change_v0(data, system, change)
            }
            Some(version) => Err(ProtocolError::UnknownVersionMismatch {
                method: "validate_property_constraints_for_system_change".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }

    fn sanitize_document_properties(&self, properties: &mut BTreeMap<String, Value>) {
        // Iterate through each property in the document
        for (field_name, field_value) in properties.iter_mut() {
            // Get the property definition from the document type schema
            if let Some(property_def) = self.properties().get(field_name) {
                // Sanitize the value based on its property type
                property_def.property_type.sanitize_value_mut(field_value);
            }
            // If the property is not in the schema, leave it as is
            // (validation will catch unknown properties later)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data_contract::config::DataContractConfig;
    use crate::data_contract::document_type::{DocumentType, CONTRACT_VERSION_STAMP_MAX_SIZE};
    use platform_value::{platform_value, Identifier};

    /// Build a document type from a schema using latest platform version.
    fn build_doc_type(name: &str, schema: Value) -> DocumentType {
        build_doc_type_at(name, schema, PlatformVersion::latest())
    }

    fn build_doc_type_at(
        name: &str,
        schema: Value,
        platform_version: &PlatformVersion,
    ) -> DocumentType {
        let config = DataContractConfig::default_for_version(platform_version)
            .expect("should create default config");
        DocumentType::try_from_schema(
            Identifier::new([1; 32]),
            1,
            config.version(),
            name,
            schema,
            None,
            &BTreeMap::new(),
            &config,
            false,
            &mut Vec::new(),
            platform_version,
        )
        .expect("should build doc type")
    }

    // --------------------------------------------------------------
    // DocumentTypeV0Methods::estimated_size
    // --------------------------------------------------------------

    /// A note type with one `text` property.
    fn note_schema(text: Value) -> Value {
        platform_value!({
            "type": "object",
            "properties": {"text": text},
            "additionalProperties": false,
        })
    }

    /// A string of up to 20000 characters without `maxBytes`: up to 80000
    /// bytes, past `u16::MAX`.
    fn long_text() -> Value {
        platform_value!({"type": "string", "maxLength": 20000, "position": 0})
    }

    #[test]
    fn should_estimate_a_string_past_16383_characters_as_a_string_without_max_length() {
        let platform_version = PlatformVersion::latest();
        let long = build_doc_type("note", note_schema(long_text()));
        let unbounded = build_doc_type(
            "note",
            note_schema(platform_value!({"type": "string", "position": 0})),
        );

        let estimated_size = long
            .as_ref()
            .estimated_size(platform_version)
            .expect("the long string is estimated");
        assert_eq!(
            estimated_size,
            unbounded
                .as_ref()
                .estimated_size(platform_version)
                .expect("the unbounded string is estimated")
        );
        // Half of u16::MAX, rounded up, and the contract-version stamp
        assert_eq!(estimated_size, 32768 + CONTRACT_VERSION_STAMP_MAX_SIZE);
    }

    #[test]
    fn should_estimate_a_typed_array_of_strings_past_16383_characters() {
        let platform_version = PlatformVersion::latest();
        let list = build_doc_type(
            "list",
            platform_value!({
                "type": "object",
                "properties": {
                    "items": {
                        "type": "array",
                        "items": {"type": "string", "maxLength": 20000},
                        "maxItems": 2,
                        "position": 0
                    }
                },
                "additionalProperties": false,
            }),
        );

        // Between the one-byte count of an empty list and u16::MAX
        assert_eq!(
            list.as_ref()
                .estimated_size(platform_version)
                .expect("the typed array is estimated"),
            32768 + CONTRACT_VERSION_STAMP_MAX_SIZE
        );
    }

    /// Generation 0, which protocol version 13 selects, still fails on the
    /// overflow.
    #[test]
    fn should_fail_the_estimate_of_a_string_past_16383_characters_at_protocol_version_13() {
        let platform_version = PlatformVersion::get(13).expect("expected version 13");
        let long = build_doc_type_at("note", note_schema(long_text()), platform_version);

        assert!(matches!(
            long.as_ref().estimated_size(platform_version),
            Err(ProtocolError::Overflow(_))
        ));
    }

    /// Below 16384 characters both generations size every property alike;
    /// generation 1 adds only the contract-version stamp.
    #[test]
    fn should_estimate_bounded_properties_as_protocol_version_13_does_plus_the_stamp() {
        let platform_version = PlatformVersion::latest();
        let platform_version_13 = PlatformVersion::get(13).expect("expected version 13");
        let schema = platform_value!({
            "type": "object",
            "properties": {
                "title": {"type": "string", "minLength": 3, "maxLength": 16383, "position": 0},
                "count": {"type": "integer", "minimum": 0, "maximum": 1000, "position": 1},
                "data": {"type": "array", "byteArray": true, "maxItems": 64, "position": 2},
                "owner": {
                    "type": "array",
                    "byteArray": true,
                    "minItems": 32,
                    "maxItems": 32,
                    "contentMediaType": "application/x.dash.dpp.identifier",
                    "position": 3
                }
            },
            "additionalProperties": false,
        });
        let at_latest = build_doc_type("doc", schema.clone());
        let at_13 = build_doc_type_at("doc", schema, platform_version_13);

        assert_eq!(
            at_latest
                .as_ref()
                .estimated_size(platform_version)
                .expect("estimated"),
            at_13
                .as_ref()
                .estimated_size(platform_version_13)
                .expect("estimated")
                + CONTRACT_VERSION_STAMP_MAX_SIZE
        );
    }

    // --------------------------------------------------------------
    // DocumentTypeBasicMethods::requires_revision / initial_revision
    // --------------------------------------------------------------
    #[test]
    fn requires_revision_false_when_immutable_non_transferable_and_no_trade_mode() {
        let schema = platform_value!({
            "type": "object",
            "documentsMutable": false,
            "transferable": 0_u64,
            "tradeMode": 0_u64,
            "properties": {
                "field_a": {"type": "string", "position": 0, "maxLength": 20_u32}
            },
            "additionalProperties": false,
        });
        let dt = build_doc_type("immutable_doc", schema);
        // Requires revision false => initial_revision must be None
        assert!(!dt.as_ref().requires_revision_ref());
        assert_eq!(dt.as_ref().initial_revision_ref(), None);
    }

    #[test]
    fn requires_revision_true_when_mutable() {
        let schema = platform_value!({
            "type": "object",
            "documentsMutable": true,
            "properties": {
                "field_a": {"type": "string", "position": 0, "maxLength": 20_u32}
            },
            "additionalProperties": false,
        });
        let dt = build_doc_type("mutable_doc", schema);
        assert!(dt.as_ref().requires_revision_ref());
        assert_eq!(dt.as_ref().initial_revision_ref(), Some(INITIAL_REVISION));
    }

    #[test]
    fn requires_revision_true_when_transferable_even_if_immutable() {
        let schema = platform_value!({
            "type": "object",
            "documentsMutable": false,
            "transferable": 1_u64,
            "properties": {
                "field_a": {"type": "string", "position": 0, "maxLength": 20_u32}
            },
            "additionalProperties": false,
        });
        let dt = build_doc_type("transferable_doc", schema);
        assert!(dt.as_ref().requires_revision_ref());
        assert_eq!(dt.as_ref().initial_revision_ref(), Some(INITIAL_REVISION));
    }

    #[test]
    fn requires_revision_true_when_trade_mode_seller_sets_price() {
        let schema = platform_value!({
            "type": "object",
            "documentsMutable": false,
            "tradeMode": 1_u64, // DirectPurchase -> seller_sets_price = true
            "properties": {
                "field_a": {"type": "string", "position": 0, "maxLength": 20_u32}
            },
            "additionalProperties": false,
        });
        let dt = build_doc_type("nft_doc", schema);
        assert!(dt.as_ref().requires_revision_ref());
    }

    // --------------------------------------------------------------
    // DocumentTypeBasicMethods::top_level_indices and
    // top_level_indices_of_contested_unique_indexes
    // --------------------------------------------------------------
    #[test]
    fn top_level_indices_returns_first_property_of_each_index() {
        let schema = platform_value!({
            "type": "object",
            "properties": {
                "first_name": {"type": "string", "position": 0, "maxLength": 60_u32},
                "last_name": {"type": "string", "position": 1, "maxLength": 60_u32}
            },
            "indices": [
                {
                    "name": "byFirst",
                    "properties": [{"first_name": "asc"}],
                },
                {
                    "name": "byLast",
                    "properties": [{"last_name": "asc"}],
                },
            ],
            "additionalProperties": false,
        });
        let dt = build_doc_type("person", schema);
        let dt_ref = dt.as_ref();
        let top: Vec<&IndexProperty> = dt_ref.top_level_indices_ref();
        // Two indices each contribute their first property
        assert_eq!(top.len(), 2);
        let names: Vec<&str> = top.iter().map(|p| p.name.as_str()).collect();
        assert!(names.contains(&"first_name"));
        assert!(names.contains(&"last_name"));
    }

    #[test]
    fn top_level_indices_of_contested_unique_indexes_excludes_non_contested() {
        let schema = platform_value!({
            "type": "object",
            "documentsMutable": false,
            "properties": {
                "first_name": {"type": "string", "position": 0, "maxLength": 60_u32},
                "last_name": {"type": "string", "position": 1, "maxLength": 60_u32}
            },
            "indices": [
                {
                    "name": "byFirst",
                    "properties": [{"first_name": "asc"}],
                },
                {
                    "name": "byLast",
                    "properties": [{"last_name": "asc"}],
                },
            ],
            "additionalProperties": false,
        });
        let dt = build_doc_type("person_no_contested", schema);
        let dt_ref = dt.as_ref();
        let contested = dt_ref.top_level_indices_of_contested_unique_indexes_ref();
        // Neither index is contested
        assert!(contested.is_empty());
    }

    // --------------------------------------------------------------
    // DocumentTypeBasicMethods::unique_id_for_document_field
    // --------------------------------------------------------------
    #[test]
    fn unique_id_for_document_field_concatenates_identifier_and_base_event() {
        let schema = platform_value!({
            "type": "object",
            "properties": {
                "field_a": {"type": "string", "position": 0, "maxLength": 20_u32}
            },
            "additionalProperties": false,
        });
        let dt = build_doc_type("events", schema);
        let dt_ref = dt.as_ref();
        let index_level = dt_ref.index_structure_ref();
        let base_event: [u8; 32] = [7; 32];
        let out = dt_ref.unique_id_for_document_field_ref(index_level, base_event);
        // Output must be 8 bytes (u64 identifier) + 32 bytes (base_event) = 40
        assert_eq!(out.len(), 8 + 32);
        // Last 32 bytes must match base_event exactly
        assert_eq!(&out[8..], &base_event);
        // First 8 bytes must be identifier BE bytes
        let id_bytes = index_level.identifier().to_be_bytes();
        assert_eq!(&out[..8], &id_bytes);
    }

    // --------------------------------------------------------------
    // DocumentTypeV0Methods::sanitize_document_properties
    // --------------------------------------------------------------
    #[test]
    fn sanitize_document_properties_converts_hex_bytearray_to_bytes() {
        let schema = platform_value!({
            "type": "object",
            "properties": {
                "payload": {
                    "type": "array",
                    "byteArray": true,
                    "minItems": 1_u32,
                    "maxItems": 64_u32,
                    "position": 0
                }
            },
            "additionalProperties": false,
        });
        let dt = build_doc_type("blob_doc", schema);
        // 4-byte hex string
        let mut props: BTreeMap<String, Value> = BTreeMap::new();
        props.insert("payload".to_string(), Value::Text("deadbeef".to_string()));

        dt.as_ref().sanitize_document_properties_ref(&mut props);

        let got = props.get("payload").unwrap();
        match got {
            Value::Bytes(bytes) => assert_eq!(bytes.as_slice(), &[0xde, 0xad, 0xbe, 0xef]),
            other => panic!("expected sanitized Bytes, got {:?}", other),
        }
    }

    #[test]
    fn sanitize_document_properties_converts_integer_array_bytearray_to_bytes() {
        // A binary property re-hydrated through a schemaless JSON layer (an edited
        // and replaced cached document) arrives as a plain array of numbers that
        // decode to wider Value integer variants (U64), not Value::U8. Sanitize must
        // normalize it to Value::Bytes so the strict binary serializer accepts it;
        // otherwise the replace fails with "not an array of bytes".
        let schema = platform_value!({
            "type": "object",
            "properties": {
                "payload": {
                    "type": "array",
                    "byteArray": true,
                    "minItems": 1_u32,
                    "maxItems": 64_u32,
                    "position": 0
                }
            },
            "additionalProperties": false,
        });
        let dt = build_doc_type("blob_doc", schema);
        let mut props: BTreeMap<String, Value> = BTreeMap::new();
        props.insert(
            "payload".to_string(),
            Value::Array(vec![
                Value::U64(0xde),
                Value::U64(0xad),
                Value::U64(0xbe),
                Value::U64(0xef),
            ]),
        );

        dt.as_ref().sanitize_document_properties_ref(&mut props);

        match props.get("payload").unwrap() {
            Value::Bytes(bytes) => assert_eq!(bytes.as_slice(), &[0xde, 0xad, 0xbe, 0xef]),
            other => panic!("expected sanitized Bytes, got {:?}", other),
        }
    }

    #[test]
    fn sanitize_document_properties_leaves_unknown_fields_untouched() {
        let schema = platform_value!({
            "type": "object",
            "properties": {
                "known": {"type": "string", "position": 0, "maxLength": 20_u32}
            },
            "additionalProperties": false,
        });
        let dt = build_doc_type("known_only", schema);
        let mut props: BTreeMap<String, Value> = BTreeMap::new();
        props.insert(
            "unknown_field".to_string(),
            Value::Text("abcdef".to_string()),
        );
        props.insert("known".to_string(), Value::Text("hello".to_string()));

        dt.as_ref().sanitize_document_properties_ref(&mut props);

        // unknown_field should be unchanged
        assert_eq!(
            props.get("unknown_field").unwrap(),
            &Value::Text("abcdef".to_string())
        );
        // known stays a string (no sanitization applies for String type)
        assert_eq!(
            props.get("known").unwrap(),
            &Value::Text("hello".to_string())
        );
    }

    // --------------------------------------------------------------
    // Helper extensions so we can dispatch to the underlying
    // DocumentTypeV0/V1 via the enum without exposing new API.
    // (Implemented inline for the tests.)
    // --------------------------------------------------------------
    trait DocumentTypeTestHelpers {
        fn requires_revision_ref(&self) -> bool;
        fn initial_revision_ref(&self) -> Option<Revision>;
        fn top_level_indices_ref(&self) -> Vec<&IndexProperty>;
        fn top_level_indices_of_contested_unique_indexes_ref(&self) -> Vec<&IndexProperty>;
        fn index_structure_ref(&self) -> &IndexLevel;
        fn unique_id_for_document_field_ref(
            &self,
            index_level: &IndexLevel,
            base_event: [u8; 32],
        ) -> Vec<u8>;
        fn sanitize_document_properties_ref(&self, properties: &mut BTreeMap<String, Value>);
    }

    impl<'a> DocumentTypeTestHelpers for crate::data_contract::document_type::DocumentTypeRef<'a> {
        fn requires_revision_ref(&self) -> bool {
            match self {
                crate::data_contract::document_type::DocumentTypeRef::V0(v0) => {
                    v0.requires_revision()
                }
                crate::data_contract::document_type::DocumentTypeRef::V1(v1) => {
                    v1.requires_revision()
                }
                crate::data_contract::document_type::DocumentTypeRef::V2(v2) => {
                    v2.requires_revision()
                }
            }
        }

        fn initial_revision_ref(&self) -> Option<Revision> {
            match self {
                crate::data_contract::document_type::DocumentTypeRef::V0(v0) => {
                    v0.initial_revision()
                }
                crate::data_contract::document_type::DocumentTypeRef::V1(v1) => {
                    v1.initial_revision()
                }
                crate::data_contract::document_type::DocumentTypeRef::V2(v2) => {
                    v2.initial_revision()
                }
            }
        }

        fn top_level_indices_ref(&self) -> Vec<&IndexProperty> {
            match self {
                crate::data_contract::document_type::DocumentTypeRef::V0(v0) => {
                    v0.top_level_indices()
                }
                crate::data_contract::document_type::DocumentTypeRef::V1(v1) => {
                    v1.top_level_indices()
                }
                crate::data_contract::document_type::DocumentTypeRef::V2(v2) => {
                    v2.top_level_indices()
                }
            }
        }

        fn top_level_indices_of_contested_unique_indexes_ref(&self) -> Vec<&IndexProperty> {
            match self {
                crate::data_contract::document_type::DocumentTypeRef::V0(v0) => {
                    v0.top_level_indices_of_contested_unique_indexes()
                }
                crate::data_contract::document_type::DocumentTypeRef::V1(v1) => {
                    v1.top_level_indices_of_contested_unique_indexes()
                }
                crate::data_contract::document_type::DocumentTypeRef::V2(v2) => {
                    v2.top_level_indices_of_contested_unique_indexes()
                }
            }
        }

        fn index_structure_ref(&self) -> &IndexLevel {
            use crate::data_contract::document_type::accessors::DocumentTypeV0Getters;
            match self {
                crate::data_contract::document_type::DocumentTypeRef::V0(v0) => {
                    v0.index_structure()
                }
                crate::data_contract::document_type::DocumentTypeRef::V1(v1) => {
                    v1.index_structure()
                }
                crate::data_contract::document_type::DocumentTypeRef::V2(v2) => {
                    v2.index_structure()
                }
            }
        }

        fn unique_id_for_document_field_ref(
            &self,
            index_level: &IndexLevel,
            base_event: [u8; 32],
        ) -> Vec<u8> {
            match self {
                crate::data_contract::document_type::DocumentTypeRef::V0(v0) => {
                    v0.unique_id_for_document_field(index_level, base_event)
                }
                crate::data_contract::document_type::DocumentTypeRef::V1(v1) => {
                    v1.unique_id_for_document_field(index_level, base_event)
                }
                crate::data_contract::document_type::DocumentTypeRef::V2(v2) => {
                    v2.unique_id_for_document_field(index_level, base_event)
                }
            }
        }

        fn sanitize_document_properties_ref(&self, properties: &mut BTreeMap<String, Value>) {
            match self {
                crate::data_contract::document_type::DocumentTypeRef::V0(v0) => {
                    v0.sanitize_document_properties(properties)
                }
                crate::data_contract::document_type::DocumentTypeRef::V1(v1) => {
                    v1.sanitize_document_properties(properties)
                }
                crate::data_contract::document_type::DocumentTypeRef::V2(v2) => {
                    v2.sanitize_document_properties(properties)
                }
            }
        }
    }
}
