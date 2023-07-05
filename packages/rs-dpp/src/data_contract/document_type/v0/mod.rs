use itertools::Itertools;
use std::collections::{BTreeMap, BTreeSet};
use std::convert::TryInto;

use crate::data_contract::document_type::{property_names};
use crate::data_contract::errors::{DataContractError, StructureError};
use crate::document::INITIAL_REVISION;
use crate::document::{Document, DocumentV0};
use crate::prelude::Revision;
use crate::ProtocolError;
use platform_value::btreemap_extensions::{BTreeValueMapHelper, BTreeValueRemoveFromMapHelper};
use platform_value::{Identifier, ReplacementType, Value};
use serde::{Deserialize, Serialize};
use crate::data_contract::document_type::array_field::ArrayFieldType;
use crate::data_contract::document_type::document_field::{DocumentField, DocumentFieldType};
use crate::data_contract::document_type::index::Index;
use crate::data_contract::document_type::index_level::IndexLevel;
use crate::data_contract::document_type::v0::v0_methods::DocumentTypeV0Methods;

pub mod document_factory;
pub mod random_document;
pub mod random_document_type;
pub(crate) mod v0_methods;

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
pub struct DocumentTypeV0 {
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

impl DocumentTypeV0 {
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
        DocumentTypeV0 {
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


    pub(crate) fn from_platform_value(
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
            Self::insert_values(
                &mut flattened_document_properties,
                &required_fields,
                None,
                property_key.clone(),
                property_value,
                definition_references,
            )?;

            Self::insert_values_nested(
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
        Ok(DocumentTypeV0 {
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

                        Self::insert_values_nested(
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
                field_type = Self::string_to_field_type(type_value.as_str())
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
                    field_type = Self::string_to_field_type(type_value.as_str())
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

    fn string_to_field_type(field_type_name: &str) -> Option<DocumentFieldType> {
        match field_type_name {
            "integer" => Some(DocumentFieldType::Integer),
            "number" => Some(DocumentFieldType::Number),
            "boolean" => Some(DocumentFieldType::Boolean),
            "date" => Some(DocumentFieldType::Date),
            _ => None,
        }
    }

    fn find_identifier_and_binary_paths(
        properties: &BTreeMap<String, DocumentField>,
    ) -> (BTreeSet<String>, BTreeSet<String>) {
        Self::find_identifier_and_binary_paths_inner(properties, "")
    }
}


