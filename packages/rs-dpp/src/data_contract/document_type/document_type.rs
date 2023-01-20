use std::collections::{BTreeMap, BTreeSet};
use std::convert::TryInto;

use ciborium::value::Value;
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use crate::data_contract::document_type::{ArrayFieldType, property_names};
use crate::data_contract::errors::DataContractError;
use crate::data_contract::extra::common::{btree_map_inner_bool_value, btree_map_inner_btree_map, btree_map_inner_map_value, btree_map_inner_text_value, btree_map_inner_u16_value, bytes_for_system_value, cbor_inner_array_of_strings, cbor_inner_array_value, cbor_inner_btree_map, cbor_inner_text_value, cbor_map_to_btree_map};
use crate::ProtocolError;
use crate::util::cbor_value::CborBTreeMapHelper;
use super::{
    document_field::{DocumentField, DocumentFieldType},
    index::{Index, IndexProperty},
};

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
pub struct DocumentType {
    pub name: String,
    pub indices: Vec<Index>,
    #[serde(skip)]
    pub index_structure: IndexLevel,
    pub properties: BTreeMap<String, DocumentField>,
    pub required_fields: BTreeSet<String>,
    pub documents_keep_history: bool,
    pub documents_mutable: bool,
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

impl DocumentType {
    pub fn new(
        name: String,
        indices: Vec<Index>,
        properties: BTreeMap<String, DocumentField>,
        required_fields: BTreeSet<String>,
        documents_keep_history: bool,
        documents_mutable: bool,
    ) -> Self {
        let index_structure = Self::build_index_structure(indices.as_slice());
        DocumentType {
            name,
            indices,
            index_structure,
            properties,
            required_fields,
            documents_keep_history,
            documents_mutable,
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

    pub fn build_index_structure(indices: &[Index]) -> IndexLevel {
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

    pub fn serialize_value_for_key<'a>(
        &'a self,
        key: &str,
        value: &Value,
    ) -> Result<Vec<u8>, DataContractError> {
        match key {
            "$ownerId" | "$id" => {
                let bytes = bytes_for_system_value(value)?.ok_or({
                    DataContractError::FieldRequirementUnmet("expected system value to be deserialized")
                })?;
                if bytes.len() != DEFAULT_HASH_SIZE {
                    Err(DataContractError::FieldRequirementUnmet(
                        "expected system value to be 32 bytes long",
                    ))
                } else {
                    Ok(bytes)
                }
            }
            _ => {
                let field_type = self.properties.get(key).ok_or({
                    DataContractError::DocumentTypeFieldNotFound("expected contract to have field")
                })?;
                let bytes = field_type.document_type.encode_value_for_tree_keys(value)?;
                if bytes.len() > MAX_INDEX_SIZE {
                    Err(DataContractError::FieldRequirementUnmet(
                        "value must be less than 256 bytes long",
                    ))
                } else {
                    Ok(bytes)
                }
            }
        }
    }

    pub fn from_cbor_value(
        name: &str,
        document_type_value_map: BTreeMap<String, &Value>,
        definition_references: &BTreeMap<String, &Value>,
        default_keeps_history: bool,
        default_mutability: bool,
    ) -> Result<Self, ProtocolError> {
        let mut document_properties: BTreeMap<String, DocumentField> = BTreeMap::new();

        // Do documents of this type keep history? (Overrides contract value)
        let documents_keep_history = document_type_value_map.get_optional_bool(property_names::DOCUMENTS_KEEP_HISTORY)?.unwrap_or(default_keeps_history);


        // Are documents of this type mutable? (Overrides contract value)
        let documents_mutable = document_type_value_map.get_optional_bool(property_names::DOCUMENTS_MUTABLE)?.unwrap_or(default_mutability);

        let index_values = document_type_value_map.get_optional_inner_value_array(property_names::INDICES)?;
        let indices = index_values.map(|index_values| {
            index_values.iter().map(|index_value| {
                index_value.as_map().try_into().ok_or(ProtocolError::DataContractError(DataContractError::InvalidContractStructure(
                    "table document is not a map as expected",
                )))
            }).collect()
        }
        ).unwrap_or_default();

        let property_values = document_type_value_map.get_inner_string_value_map(property_names::PROPERTIES)?;

        let mut required_fields = document_type_value_map.get_inner_string_array(property_names::REQUIRED)?;

        fn insert_values(
            document_properties: &mut BTreeMap<String, DocumentField>,
            known_required: &mut BTreeSet<String>,
            prefix: Option<&str>,
            property_key: String,
            property_value: &Value,
            definition_references: &BTreeMap<String, &Value>,
        ) -> Result<(), DataContractError> {
            let prefixed_property_key = match prefix {
                None => property_key,
                Some(prefix) => [prefix, property_key.as_str()].join("."),
            };

            if !property_value.is_map() {
                return Err(DataContractError::InvalidContractStructure(
                    "document property is not a map as expected",
                ));
            }

            let inner_property_values = property_value.as_map().expect("confirmed as map");
            let base_inner_properties = cbor_map_to_btree_map(inner_property_values);

            let type_value = cbor_inner_text_value(inner_property_values, "type");
            let result: Result<(&str, BTreeMap<String, &Value>), DataContractError> = match type_value {
                None => {
                    let ref_value = btree_map_inner_text_value(&base_inner_properties, "$ref")
                        .ok_or({
                            DataContractError::InvalidContractStructure("cannot find type property")
                        })?;
                    if !ref_value.starts_with("#/$defs/") {
                        return Err(DataContractError::InvalidContractStructure(
                            "malformed reference",
                        ));
                    }
                    let ref_value = ref_value.split_at(8).1;
                    let inner_properties_map =
                        btree_map_inner_map_value(definition_references, ref_value).ok_or({
                            DataContractError::ReferenceDefinitionNotFound(
                                "document reference not found",
                            )
                        })?;
                    let type_value =
                        cbor_inner_text_value(inner_properties_map, "type").ok_or({
                            DataContractError::InvalidContractStructure(
                                "cannot find type property on reference",
                            )
                        })?;
                    let inner_properties = cbor_map_to_btree_map(inner_properties_map);
                    Ok((type_value, inner_properties))
                }
                Some(type_value) => Ok((type_value, base_inner_properties)),
            };

            let (type_value, inner_properties) = result?;

            let required = known_required.contains(&type_value.to_string());

            let field_type: DocumentFieldType;

            match type_value {
                "array" => {
                    // Only handling bytearrays for v1
                    // Return an error if it is not a byte array
                    field_type = match btree_map_inner_bool_value(&inner_properties, "byteArray") {
                        Some(inner_bool) => {
                            if inner_bool {
                                DocumentFieldType::ByteArray(
                                    btree_map_inner_u16_value(&inner_properties, "minItems"),
                                    btree_map_inner_u16_value(&inner_properties, "maxItems"),
                                )
                            } else {
                                return Err(DataContractError::InvalidContractStructure(
                                    "byteArray should always be true if defined",
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
                            required,
                        },
                    );
                }
                "object" => {
                    let properties = btree_map_inner_btree_map(&inner_properties, "properties")
                        .ok_or({
                            DataContractError::InvalidContractStructure("object must have properties")
                        })?;
                    for (object_property_key, object_property_value) in properties.into_iter() {
                        insert_values(
                            document_properties,
                            known_required,
                            Some(&prefixed_property_key),
                            object_property_key,
                            object_property_value,
                            definition_references,
                        )?
                    }
                }
                "string" => {
                    field_type = DocumentFieldType::String(
                        btree_map_inner_u16_value(&inner_properties, "minLength"),
                        btree_map_inner_u16_value(&inner_properties, "maxLength"),
                    );
                    document_properties.insert(
                        prefixed_property_key,
                        DocumentField {
                            document_type: field_type,
                            required,
                        },
                    );
                }
                _ => {
                    field_type = string_to_field_type(type_value)
                        .ok_or(DataContractError::ValueWrongType("invalid type"))?;
                    document_properties.insert(
                        prefixed_property_key,
                        DocumentField {
                            document_type: field_type,
                            required,
                        },
                    );
                }
            }
            Ok(())
        }

        // Based on the property name, determine the type
        for (property_key, property_value) in property_values {
            insert_values(
                &mut document_properties,
                &mut required_fields,
                None,
                property_key,
                property_value,
                definition_references,
            )?;
        }

        // Add system properties
        if required_fields.contains("$createdAt") {
            document_properties.insert(
                String::from("$createdAt"),
                DocumentField {
                    document_type: DocumentFieldType::Date,
                    required: true,
                },
            );
        }

        if required_fields.contains("$updatedAt") {
            document_properties.insert(
                String::from("$updatedAt"),
                DocumentField {
                    document_type: DocumentFieldType::Date,
                    required: true,
                },
            );
        }

        let index_structure = Self::build_index_structure(indices.as_slice());

        Ok(DocumentType {
            name: String::from(name),
            indices,
            index_structure,
            properties: document_properties,
            required_fields,
            documents_keep_history,
            documents_mutable,
        })
    }

    pub fn max_size(&self) -> u16 {
        let mut iter = self
            .properties
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
            .properties
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
        self.properties.get(property).cloned()
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
