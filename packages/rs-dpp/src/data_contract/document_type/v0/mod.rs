use itertools::Itertools;
use std::collections::{BTreeMap, BTreeSet};
use std::convert::{TryFrom, TryInto};

use crate::data_contract::document_type::document_field::{DocumentField, DocumentFieldType};
use crate::data_contract::document_type::index::Index;
use crate::data_contract::document_type::index_level::IndexLevel;
use crate::data_contract::document_type::{property_names, DocumentType};
use crate::data_contract::errors::{DataContractError, StructureError};
use crate::document::{Document, DocumentV0};
use crate::prelude::Revision;
use crate::version::dpp_versions::DocumentTypeVersions;
use crate::version::PlatformVersion;
use crate::ProtocolError;
use platform_value::btreemap_extensions::{BTreeValueMapHelper, BTreeValueRemoveFromMapHelper};
use platform_value::{Identifier, ReplacementType, Value};
use serde::{Deserialize, Serialize};

mod accessors;
pub mod document_factory;
#[cfg(feature = "random-documents")]
pub mod random_document;
#[cfg(feature = "random-document-types")]
pub mod random_document_type;
pub mod v0_methods;

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
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError> {
        let index_structure =
            IndexLevel::try_from_indices(indices.as_slice(), name.as_str(), platform_version)?;
        let (identifier_paths, binary_paths) = DocumentType::find_identifier_and_binary_paths(
            &properties,
            &platform_version
                .dpp
                .contract_versions
                .document_type_versions,
        )?;
        Ok(DocumentTypeV0 {
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
        })
    }

    pub(crate) fn from_platform_value(
        data_contract_id: Identifier,
        name: &str,
        document_type_value_map: &[(Value, Value)],
        definition_references: &BTreeMap<String, &Value>,
        default_keeps_history: bool,
        default_mutability: bool,
        platform_version: &PlatformVersion,
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
            DocumentType::insert_values(
                &mut flattened_document_properties,
                &required_fields,
                None,
                property_key.clone(),
                property_value,
                definition_references,
                &platform_version
                    .dpp
                    .contract_versions
                    .document_type_versions,
            )?;

            DocumentType::insert_values_nested(
                &mut document_properties,
                &required_fields,
                property_key,
                property_value,
                definition_references,
                &platform_version
                    .dpp
                    .contract_versions
                    .document_type_versions,
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

        let index_structure =
            IndexLevel::try_from_indices(indices.as_slice(), name, platform_version)?;

        let (identifier_paths, binary_paths) = DocumentType::find_identifier_and_binary_paths(
            &document_properties,
            &platform_version
                .dpp
                .contract_versions
                .document_type_versions,
        )?;
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
}
