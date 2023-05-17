use itertools::Itertools;
use std::collections::{BTreeMap, BTreeSet};
use std::convert::TryInto;

use super::{
    document_field::{DocumentField, DocumentFieldType},
    index::{Index, IndexProperty},
};
use crate::data_contract::document_type::{property_names, ArrayFieldType};
use crate::data_contract::errors::{DataContractError, StructureError};

use crate::document::document_transition::INITIAL_REVISION;
use crate::document::Document;
use crate::prelude::Revision;
use crate::ProtocolError;
use platform_value::btreemap_extensions::{BTreeValueMapHelper, BTreeValueRemoveFromMapHelper};
use platform_value::{Identifier, ReplacementType, Value};
use serde::{Deserialize, Serialize};

pub const PROTOCOL_VERSION: u32 = 1;
pub const CONTRACT_DOCUMENTS_PATH_HEIGHT: u16 = 4;
pub const BASE_CONTRACT_ROOT_PATH_SIZE: usize = 33; // 1 + 32
pub const BASE_CONTRACT_KEEPING_HISTORY_STORAGE_PATH_SIZE: usize = 34; // 1 + 32 + 1
pub const BASE_CONTRACT_DOCUMENTS_KEEPING_HISTORY_STORAGE_TIME_REFERENCE_PATH: usize = 75;
pub const BASE_CONTRACT_DOCUMENTS_KEEPING_HISTORY_PRIMARY_KEY_PATH_FOR_DOCUMENT_ID_SIZE: usize = 67; // 1 + 32 + 1 + 1 + 32, then we need to add document_type_name.len()
pub const BASE_CONTRACT_DOCUMENTS_PATH: usize = 34;
pub const BASE_CONTRACT_DOCUMENTS_PRIMARY_KEY_PATH: usize = 35;
pub const DEFAULT_HASH_SIZE: usize = 32;
pub const DEFAULT_FLOAT_SIZE: usize = 8;
pub const EMPTY_TREE_STORAGE_SIZE: usize = 33;
pub const MAX_INDEX_SIZE: usize = 255;
pub const STORAGE_FLAGS_SIZE: usize = 2;

#[derive(Serialize, Deserialize, Debug, PartialEq, Default, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DocumentType {
    pub name: String,
    pub indices: Vec<Index>,
    #[serde(skip)]
    pub index_structure: IndexLevel,
    /// Flattened properties flatten all objects for quick lookups for indexes
    /// Document field should not contain sub objects.
    pub flattened_properties: BTreeMap<String, DocumentField>,
    /// Document field can contain sub objects.
    pub properties: BTreeMap<String, DocumentField>,
    #[serde(skip)]
    pub identifier_paths: BTreeSet<String>,
    #[serde(skip)]
    pub binary_paths: BTreeSet<String>,
    pub required_fields: BTreeSet<String>,
    pub documents_keep_history: bool,
    pub documents_mutable: bool,
    #[serde(skip)]
    pub data_contract_id: Identifier,
}

#[derive(Debug, PartialEq, Default, Clone)]
pub struct IndexLevel {
    /// the lower index levels from this level
    pub sub_index_levels: BTreeMap<String, IndexLevel>,
    /// did an index terminate at this level
    pub has_index_with_uniqueness: Option<bool>,
    /// unique level identifier
    pub level_identifier: u64,
}

impl From<&[Index]> for IndexLevel {
    fn from(indices: &[Index]) -> Self {
        let mut index_level = IndexLevel::default();
        let mut counter: u64 = 0;
        for index in indices {
            let mut current_level = &mut index_level;
            let mut properties_iter = index.properties.iter().peekable();
            while let Some(index_part) = properties_iter.next() {
                current_level = current_level
                    .sub_index_levels
                    .entry(index_part.name.clone())
                    .or_insert_with(|| {
                        counter += 1;
                        IndexLevel {
                            level_identifier: counter,
                            ..Default::default()
                        }
                    });
                if properties_iter.peek().is_none() {
                    current_level.has_index_with_uniqueness = Some(index.unique);
                }
            }
        }

        index_level
    }
}

impl DocumentType {
    pub fn new(
        data_contract_id: Identifier,
        name: String,
        indices: Vec<Index>,
        properties: BTreeMap<String, DocumentField>,
        required_fields: BTreeSet<String>,
        documents_keep_history: bool,
        documents_mutable: bool,
    ) -> Self {
        let index_structure = IndexLevel::from(indices.as_slice());
        let (identifier_paths, binary_paths) = Self::find_identifier_and_binary_paths(&properties);
        DocumentType {
            name,
            indices,
            index_structure,
            flattened_properties: properties.clone(),
            properties,
            identifier_paths,
            binary_paths,
            required_fields,
            documents_keep_history,
            documents_mutable,
            data_contract_id,
        }
    }
    // index_names can be in any order
    // in field name must be in the last two indexes.
    pub fn index_for_types(
        &self,
        index_names: &[&str],
        in_field_name: Option<&str>,
        order_by: &[&str],
    ) -> Option<(&Index, u16)> {
        let mut best_index: Option<(&Index, u16)> = None;
        let mut best_difference = u16::MAX;
        for index in self.indices.iter() {
            let difference_option = index.matches(index_names, in_field_name, order_by);
            if let Some(difference) = difference_option {
                if difference == 0 {
                    return Some((index, 0));
                } else if difference < best_difference {
                    best_difference = difference;
                    best_index = Some((index, best_difference));
                }
            }
        }
        best_index
    }

    pub fn unique_id_for_storage(&self) -> [u8; 32] {
        rand::random::<[u8; 32]>()
    }

    /// Unique id that combines the index_level and the base event id
    pub fn unique_id_for_document_field(
        &self,
        index_level: &IndexLevel,
        base_event: [u8; 32],
    ) -> Vec<u8> {
        let mut bytes = index_level.level_identifier.to_be_bytes().to_vec();
        bytes.extend_from_slice(&base_event);
        bytes
    }

    pub fn serialize_value_for_key(
        &self,
        key: &str,
        value: &Value,
    ) -> Result<Vec<u8>, ProtocolError> {
        match key {
            "$ownerId" | "$id" => {
                let bytes = value
                    .to_identifier_bytes()
                    .map_err(ProtocolError::ValueError)?;
                if bytes.len() != DEFAULT_HASH_SIZE {
                    Err(ProtocolError::DataContractError(
                        DataContractError::FieldRequirementUnmet(
                            "expected system value to be 32 bytes long",
                        ),
                    ))
                } else {
                    Ok(bytes)
                }
            }
            _ => {
                let field_type = self.flattened_properties.get(key).ok_or_else(|| {
                    DataContractError::DocumentTypeFieldNotFound(format!("expected contract to have field: {key}, contract fields are {} on document type {}", self.flattened_properties.keys().join(" | "), self.name))
                })?;
                let bytes = field_type.document_type.encode_value_for_tree_keys(value)?;
                if bytes.len() > MAX_INDEX_SIZE {
                    Err(ProtocolError::DataContractError(
                        DataContractError::FieldRequirementUnmet(
                            "value must be less than 256 bytes long",
                        ),
                    ))
                } else {
                    Ok(bytes)
                }
            }
        }
    }

    pub fn convert_value_to_document(&self, mut data: Value) -> Result<Document, ProtocolError> {
        let mut document = Document {
            id: data.remove_identifier("$id")?,
            owner_id: data.remove_identifier("$ownerId")?,
            properties: Default::default(),
            revision: data.remove_optional_integer("$revision")?,
            created_at: data.remove_optional_integer("$createdAt")?,
            updated_at: data.remove_optional_integer("$updatedAt")?,
        };

        data.replace_at_paths(
            self.identifier_paths.iter().map(|s| s.as_str()),
            ReplacementType::Identifier,
        )?;

        data.replace_at_paths(
            self.binary_paths.iter().map(|s| s.as_str()),
            ReplacementType::BinaryBytes,
        )?;

        document.properties = data
            .into_btree_string_map()
            .map_err(ProtocolError::ValueError)?;

        Ok(document)
    }

    pub fn from_platform_value(
        data_contract_id: Identifier,
        name: &str,
        document_type_value_map: &[(Value, Value)],
        definition_references: &BTreeMap<String, &Value>,
        default_keeps_history: bool,
        default_mutability: bool,
    ) -> Result<Self, ProtocolError> {
        let mut flattened_document_properties: BTreeMap<String, DocumentField> = BTreeMap::new();
        let mut document_properties: BTreeMap<String, DocumentField> = BTreeMap::new();

        // Do documents of this type keep history? (Overrides contract value)
        let documents_keep_history: bool =
            Value::inner_optional_bool_value(document_type_value_map, "documentsKeepHistory")
                .map_err(ProtocolError::ValueError)?
                .unwrap_or(default_keeps_history);

        // Are documents of this type mutable? (Overrides contract value)
        let documents_mutable: bool =
            Value::inner_optional_bool_value(document_type_value_map, "documentsMutable")
                .map_err(ProtocolError::ValueError)?
                .unwrap_or(default_mutability);

        let index_values = Value::inner_optional_array_slice_value(
            document_type_value_map,
            property_names::INDICES,
        )?;
        let indices: Vec<Index> = index_values
            .map(|index_values| {
                index_values
                    .iter()
                    .map(|index_value| {
                        index_value
                            .as_map()
                            .ok_or(ProtocolError::DataContractError(
                                DataContractError::InvalidContractStructure(
                                    "table document is not a map as expected",
                                ),
                            ))?
                            .as_slice()
                            .try_into()
                    })
                    .collect::<Result<Vec<Index>, ProtocolError>>()
            })
            .transpose()?
            .unwrap_or_default();

        // Extract the properties
        let property_values =
            Value::inner_optional_btree_map(document_type_value_map, property_names::PROPERTIES)?
                .unwrap_or_default();

        let required_fields = Value::inner_recursive_optional_array_of_strings(
            document_type_value_map,
            "".to_string(),
            property_names::PROPERTIES,
            property_names::REQUIRED,
        );
        // Based on the property name, determine the type
        for (property_key, property_value) in property_values {
            insert_values(
                &mut flattened_document_properties,
                &required_fields,
                None,
                property_key.clone(),
                property_value,
                definition_references,
            )?;

            insert_values_nested(
                &mut document_properties,
                &required_fields,
                property_key,
                property_value,
                definition_references,
            )?;
        }
        // Add system properties
        if required_fields.contains(property_names::CREATED_AT) {
            flattened_document_properties.insert(
                String::from(property_names::CREATED_AT),
                DocumentField {
                    document_type: DocumentFieldType::Date,
                    required: true,
                },
            );
            document_properties.insert(
                String::from(property_names::CREATED_AT),
                DocumentField {
                    document_type: DocumentFieldType::Date,
                    required: true,
                },
            );
        }

        if required_fields.contains(property_names::UPDATED_AT) {
            flattened_document_properties.insert(
                String::from(property_names::UPDATED_AT),
                DocumentField {
                    document_type: DocumentFieldType::Date,
                    required: true,
                },
            );
            document_properties.insert(
                String::from(property_names::UPDATED_AT),
                DocumentField {
                    document_type: DocumentFieldType::Date,
                    required: true,
                },
            );
        }

        let index_structure = IndexLevel::from(indices.as_slice());

        let (identifier_paths, binary_paths) =
            Self::find_identifier_and_binary_paths(&document_properties);
        Ok(DocumentType {
            name: String::from(name),
            indices,
            index_structure,
            flattened_properties: flattened_document_properties,
            properties: document_properties,
            identifier_paths,
            binary_paths,
            required_fields,
            documents_keep_history,
            documents_mutable,
            data_contract_id,
        })
    }

    pub fn max_size(&self) -> u16 {
        let mut iter = self
            .flattened_properties
            .iter()
            .filter_map(|(_, document_field_type)| {
                document_field_type.document_type.max_byte_size()
            });
        let first = Some(iter.next().unwrap_or_default());

        iter.fold(first, |acc, item| acc.and_then(|acc| acc.checked_add(item)))
            .unwrap_or(u16::MAX)
    }

    /// The estimated size uses the middle ceil size of all attributes
    pub fn estimated_size(&self) -> u16 {
        let mut iter = self
            .flattened_properties
            .iter()
            .filter_map(|(_, document_field_type)| {
                document_field_type.document_type.middle_byte_size_ceil()
            });
        let first = Some(iter.next().unwrap_or_default());

        iter.fold(first, |acc, item| acc.and_then(|acc| acc.checked_add(item)))
            .unwrap_or(u16::MAX)
    }

    pub fn top_level_indices(&self) -> Vec<&IndexProperty> {
        let mut index_properties: Vec<&IndexProperty> = Vec::with_capacity(self.indices.len());
        for index in &self.indices {
            if let Some(property) = index.properties.get(0) {
                index_properties.push(property);
            }
        }
        index_properties
    }

    pub fn document_field_for_property(&self, property: &str) -> Option<DocumentField> {
        self.flattened_properties.get(property).cloned()
    }

    pub fn document_field_type_for_property(&self, property: &str) -> Option<DocumentFieldType> {
        match property {
            "$id" => Some(DocumentFieldType::ByteArray(
                Some(DEFAULT_HASH_SIZE as u16),
                Some(DEFAULT_HASH_SIZE as u16),
            )),
            "$ownerId" => Some(DocumentFieldType::ByteArray(
                Some(DEFAULT_HASH_SIZE as u16),
                Some(DEFAULT_HASH_SIZE as u16),
            )),
            "$createdAt" => Some(DocumentFieldType::Date),
            "$updatedAt" => Some(DocumentFieldType::Date),
            &_ => self
                .document_field_for_property(property)
                .map(|document_field| document_field.document_type),
        }
    }

    pub fn field_can_be_null(&self, name: &str) -> bool {
        !self.required_fields.contains(name)
    }

    pub fn initial_revision(&self) -> Option<Revision> {
        if self.documents_mutable {
            Some(INITIAL_REVISION)
        } else {
            None
        }
    }

    pub fn requires_revision(&self) -> bool {
        self.documents_mutable
    }

    pub(crate) fn find_identifier_and_binary_paths(
        properties: &BTreeMap<String, DocumentField>,
    ) -> (BTreeSet<String>, BTreeSet<String>) {
        Self::find_identifier_and_binary_paths_inner(properties, "")
    }
    fn find_identifier_and_binary_paths_inner(
        properties: &BTreeMap<String, DocumentField>,
        current_path: &str,
    ) -> (BTreeSet<String>, BTreeSet<String>) {
        let mut identifier_paths = BTreeSet::new();
        let mut binary_paths = BTreeSet::new();

        for (key, value) in properties.iter() {
            let new_path = if current_path.is_empty() {
                key.clone()
            } else {
                format!("{}.{}", current_path, key)
            };

            match &value.document_type {
                DocumentFieldType::Identifier => {
                    identifier_paths.insert(new_path);
                }
                DocumentFieldType::ByteArray(_, _) => {
                    binary_paths.insert(new_path);
                }
                DocumentFieldType::Object(inner_properties) => {
                    let (inner_identifier_paths, inner_binary_paths) =
                        Self::find_identifier_and_binary_paths_inner(inner_properties, &new_path);

                    identifier_paths.extend(inner_identifier_paths);
                    binary_paths.extend(inner_binary_paths);
                }
                DocumentFieldType::Array(array_field_type) => {
                    let new_path = format!("{}[]", new_path);
                    match array_field_type {
                        ArrayFieldType::Identifier => {
                            identifier_paths.insert(new_path.clone());
                        }
                        ArrayFieldType::ByteArray(_, _) => {
                            binary_paths.insert(new_path.clone());
                        }
                        _ => {}
                    }
                }
                DocumentFieldType::VariableTypeArray(array_field_types) => {
                    for (i, array_field_type) in array_field_types.iter().enumerate() {
                        let new_path = format!("{}[{}]", new_path, i);
                        match array_field_type {
                            ArrayFieldType::Identifier => {
                                identifier_paths.insert(new_path.clone());
                            }
                            ArrayFieldType::ByteArray(_, _) => {
                                binary_paths.insert(new_path.clone());
                            }
                            _ => {}
                        }
                    }
                }
                _ => {}
            }
        }

        (identifier_paths, binary_paths)
    }
}

pub fn string_to_field_type(field_type_name: &str) -> Option<DocumentFieldType> {
    match field_type_name {
        "integer" => Some(DocumentFieldType::Integer),
        "number" => Some(DocumentFieldType::Number),
        "boolean" => Some(DocumentFieldType::Boolean),
        "date" => Some(DocumentFieldType::Date),
        _ => None,
    }
}

fn insert_values_nested(
    document_properties: &mut BTreeMap<String, DocumentField>,
    known_required: &BTreeSet<String>,
    property_key: String,
    property_value: &Value,
    definition_references: &BTreeMap<String, &Value>,
) -> Result<(), ProtocolError> {
    let mut inner_properties = property_value.to_btree_ref_string_map()?;

    let type_value = inner_properties
        .remove_optional_string(property_names::TYPE)
        .map_err(ProtocolError::ValueError)?;
    let type_value = match type_value {
        None => {
            let ref_value = inner_properties
                .get_str(property_names::REF)
                .map_err(ProtocolError::ValueError)?;
            let Some(ref_value) = ref_value.strip_prefix("#/$defs/") else {
                return Err(ProtocolError::DataContractError(
                    DataContractError::InvalidContractStructure("malformed reference"),
                ));
            };
            inner_properties = definition_references
                .get_inner_borrowed_str_value_map(ref_value)
                .map_err(ProtocolError::ValueError)?;

            inner_properties.get_string(property_names::TYPE)?
        }
        Some(type_value) => type_value,
    };
    let is_required = known_required.contains(&property_key);
    let field_type: DocumentFieldType;

    match type_value.as_str() {
        "integer" => {
            field_type = DocumentFieldType::Integer;
        }
        "number" => {
            field_type = DocumentFieldType::Number;
        }
        "string" => {
            field_type = DocumentFieldType::String(
                inner_properties.get_optional_integer(property_names::MIN_LENGTH)?,
                inner_properties.get_optional_integer(property_names::MAX_LENGTH)?,
            );
        }
        "array" => {
            // Only handling bytearrays for v1
            // Return an error if it is not a byte array
            field_type = match inner_properties.get_optional_bool(property_names::BYTE_ARRAY)? {
                Some(inner_bool) => {
                    if inner_bool {
                        match inner_properties
                            .get_optional_str(property_names::CONTENT_MEDIA_TYPE)?
                        {
                            Some(content_media_type)
                                if content_media_type == "application/x.dash.dpp.identifier" =>
                            {
                                DocumentFieldType::Identifier
                            }
                            Some(_) | None => DocumentFieldType::ByteArray(
                                inner_properties.get_optional_integer(property_names::MIN_ITEMS)?,
                                inner_properties.get_optional_integer(property_names::MAX_ITEMS)?,
                            ),
                        }
                    } else {
                        return Err(ProtocolError::DataContractError(
                            DataContractError::InvalidContractStructure(
                                "byteArray should always be true if defined",
                            ),
                        ));
                    }
                }
                // TODO: Contract indices and new encoding format don't support arrays
                //   but we still can use them as document fields with current cbor encoding
                //   This is a temporary workaround to bring back v0.22 behavior and should be
                //   replaced with a proper array support in future versions
                None => DocumentFieldType::Array(ArrayFieldType::Boolean),
            };
        }
        "object" => {
            let mut nested_properties = BTreeMap::new();
            if let Some(properties_as_value) = inner_properties.get(property_names::PROPERTIES) {
                let properties =
                    properties_as_value
                        .as_map()
                        .ok_or(ProtocolError::StructureError(
                            StructureError::ValueWrongType("properties must be a map"),
                        ))?;

                // Create a new set with the prefix removed from the keys
                let stripped_required: BTreeSet<String> = known_required
                    .iter()
                    .filter_map(|key| {
                        if key.starts_with(&property_key) && key.len() > property_key.len() {
                            Some(key[property_key.len() + 1..].to_string())
                        } else {
                            None
                        }
                    })
                    .collect();

                // Create a new set with the prefix removed from the keys
                let inner_definition_references: BTreeMap<String, &Value> = definition_references
                    .iter()
                    .filter_map(|(key, value)| {
                        if key.starts_with(&property_key) && key.len() > property_key.len() {
                            Some((key[property_key.len() + 1..].to_string(), *value))
                        } else {
                            None
                        }
                    })
                    .collect();

                for (object_property_key, object_property_value) in properties.iter() {
                    let object_property_string = object_property_key
                        .as_text()
                        .ok_or(ProtocolError::StructureError(StructureError::KeyWrongType(
                            "property key must be a string",
                        )))?
                        .to_string();

                    insert_values_nested(
                        &mut nested_properties,
                        &stripped_required,
                        object_property_string,
                        object_property_value,
                        &inner_definition_references,
                    )?;
                }
            }
            field_type = DocumentFieldType::Object(nested_properties);
            document_properties.insert(
                property_key,
                DocumentField {
                    document_type: field_type,
                    required: is_required,
                },
            );
            return Ok(());
        }
        _ => {
            field_type = string_to_field_type(type_value.as_str())
                .ok_or(DataContractError::ValueWrongType("invalid type"))?;
        }
    }

    document_properties.insert(
        property_key,
        DocumentField {
            document_type: field_type,
            required: is_required,
        },
    );

    Ok(())
}

fn insert_values(
    document_properties: &mut BTreeMap<String, DocumentField>,
    known_required: &BTreeSet<String>,
    prefix: Option<String>,
    property_key: String,
    property_value: &Value,
    definition_references: &BTreeMap<String, &Value>,
) -> Result<(), ProtocolError> {
    let mut to_visit: Vec<(Option<String>, String, &Value)> =
        vec![(prefix, property_key, property_value)];

    while let Some((prefix, property_key, property_value)) = to_visit.pop() {
        let prefixed_property_key = match prefix {
            None => property_key,
            Some(prefix) => [prefix, property_key].join(".").to_owned(),
        };
        let mut inner_properties = property_value.to_btree_ref_string_map()?;
        let type_value = inner_properties
            .remove_optional_string(property_names::TYPE)
            .map_err(ProtocolError::ValueError)?;
        let type_value = match type_value {
            None => {
                let ref_value = inner_properties
                    .get_str(property_names::REF)
                    .map_err(ProtocolError::ValueError)?;
                let Some(ref_value) = ref_value.strip_prefix("#/$defs/") else {
                    return Err(ProtocolError::DataContractError(
                        DataContractError::InvalidContractStructure("malformed reference"),
                    ));
                };
                inner_properties = definition_references
                    .get_inner_borrowed_str_value_map(ref_value)
                    .map_err(ProtocolError::ValueError)?;

                inner_properties.get_string(property_names::TYPE)?
            }
            Some(type_value) => type_value,
        };
        let is_required = known_required.contains(&prefixed_property_key);
        let field_type: DocumentFieldType;

        match type_value.as_str() {
            "array" => {
                // Only handling bytearrays for v1
                // Return an error if it is not a byte array
                field_type = match inner_properties.get_optional_bool(property_names::BYTE_ARRAY)? {
                    Some(inner_bool) => {
                        if inner_bool {
                            match inner_properties
                                .get_optional_str(property_names::CONTENT_MEDIA_TYPE)?
                            {
                                Some(content_media_type)
                                    if content_media_type
                                        == "application/x.dash.dpp.identifier" =>
                                {
                                    DocumentFieldType::Identifier
                                }
                                Some(_) | None => DocumentFieldType::ByteArray(
                                    inner_properties
                                        .get_optional_integer(property_names::MIN_ITEMS)?,
                                    inner_properties
                                        .get_optional_integer(property_names::MAX_ITEMS)?,
                                ),
                            }
                        } else {
                            return Err(ProtocolError::DataContractError(
                                DataContractError::InvalidContractStructure(
                                    "byteArray should always be true if defined",
                                ),
                            ));
                        }
                    }
                    // TODO: Contract indices and new encoding format don't support arrays
                    //   but we still can use them as document fields with current cbor encoding
                    //   This is a temporary workaround to bring back v0.22 behavior and should be
                    //   replaced with a proper array support in future versions
                    None => DocumentFieldType::Array(ArrayFieldType::Boolean),
                };

                document_properties.insert(
                    prefixed_property_key,
                    DocumentField {
                        document_type: field_type,
                        required: is_required,
                    },
                );
            }
            "object" => {
                if let Some(properties_as_value) = inner_properties.get(property_names::PROPERTIES)
                {
                    let properties =
                        properties_as_value
                            .as_map()
                            .ok_or(ProtocolError::StructureError(
                                StructureError::ValueWrongType("properties must be a map"),
                            ))?;

                    for (object_property_key, object_property_value) in properties.iter() {
                        let object_property_string = object_property_key
                            .as_text()
                            .ok_or(ProtocolError::StructureError(StructureError::KeyWrongType(
                                "property key must be a string",
                            )))?
                            .to_string();
                        to_visit.push((
                            Some(prefixed_property_key.clone()),
                            object_property_string,
                            object_property_value,
                        ));
                    }
                }
            }

            "string" => {
                field_type = DocumentFieldType::String(
                    inner_properties.get_optional_integer(property_names::MIN_LENGTH)?,
                    inner_properties.get_optional_integer(property_names::MAX_LENGTH)?,
                );
                document_properties.insert(
                    prefixed_property_key,
                    DocumentField {
                        document_type: field_type,
                        required: is_required,
                    },
                );
            }

            _ => {
                field_type = string_to_field_type(type_value.as_str())
                    .ok_or(DataContractError::ValueWrongType("invalid type"))?;
                document_properties.insert(
                    prefixed_property_key,
                    DocumentField {
                        document_type: field_type,
                        required: is_required,
                    },
                );
            }
        }
    }

    Ok(())
}
