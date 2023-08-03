use itertools::Itertools;
use std::collections::{BTreeMap, BTreeSet};
use std::convert::{TryFrom, TryInto};

use crate::consensus::ConsensusError;
use crate::data_contract::document_type::document_field::{DocumentField, DocumentFieldType};
use crate::data_contract::document_type::enrich_with_base_schema::enrich_with_base_schema;
use crate::data_contract::document_type::index::Index;
use crate::data_contract::document_type::index_level::IndexLevel;
use crate::data_contract::document_type::{property_names, DocumentType};
use crate::data_contract::errors::{DataContractError, JsonSchemaError};
use crate::data_contract::{DataContract, DocumentName, JsonValue, PropertyPath};
use crate::validation::meta_validators::DOCUMENT_META_SCHEMA_V0;
use crate::version::PlatformVersion;
use crate::ProtocolError;
use platform_value::btreemap_extensions::{BTreeValueMapHelper, BTreeValueRemoveFromMapHelper};
use platform_value::{Identifier, Value};
use serde::{Deserialize, Serialize};

mod accessors;
#[cfg(feature = "random-documents")]
pub mod random_document;
#[cfg(feature = "random-document-types")]
pub mod random_document_type;
pub mod v0_methods;

// TODO: Is this needed?
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

// TODO: We aren't going to serialize it so we should remove it
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DocumentTypeV0 {
    pub(crate) name: String,
    pub(crate) schema: Value,
    pub(crate) indices: Vec<Index>,
    #[serde(skip)]
    pub(crate) index_structure: IndexLevel,
    /// Flattened properties flatten all objects for quick lookups for indexes
    /// Document field should not contain sub objects.
    pub(crate) flattened_properties: BTreeMap<String, DocumentField>,
    /// Document field can contain sub objects.
    pub(crate) properties: BTreeMap<String, DocumentField>,
    pub(crate) binary_properties: BTreeMap<PropertyPath, Value>,
    #[serde(skip)]
    pub(crate) identifier_paths: BTreeSet<String>,
    #[serde(skip)]
    pub(crate) binary_paths: BTreeSet<String>,
    pub(crate) required_fields: BTreeSet<String>,
    pub(crate) documents_keep_history: bool,
    pub(crate) documents_mutable: bool,
    // TODO: why is this here? do we update it when data contract id is changed
    #[serde(skip)]
    pub(crate) data_contract_id: Identifier,
}

impl DocumentTypeV0 {
    pub(crate) fn from_platform_value(
        data_contract_id: Identifier,
        name: &str,
        schema: Value,
        schema_defs: &Option<BTreeMap<String, Value>>,
        default_keeps_history: bool,
        default_mutability: bool,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError> {
        // TODO: Do not validate if feature validation is disabled?

        // Create a root JSON Schema from shorten document schema
        let full_schema = enrich_with_base_schema(
            schema.clone(),
            schema_defs.as_ref().map(|defs| Value::from(defs.clone())),
            &[],
        )?;

        // validate_data_contract_max_depth(full_schema);

        // Validate against JSON Schema
        DOCUMENT_META_SCHEMA_V0
            .validate(
                &full_schema
                    .try_to_validating_json()
                    .map_err(ProtocolError::ValueError)?,
            )
            .map_err(|mut errs| ConsensusError::from(errs.next().unwrap()))?;

        // result.merge(multi_validator::validate(
        //     raw_data_contract,
        //     &[
        //         pattern_is_valid_regex_validator,
        //         byte_array_has_no_items_as_parent_validator,
        //     ],
        // ));

        let mut flattened_document_properties: BTreeMap<String, DocumentField> = BTreeMap::new();
        let mut document_properties: BTreeMap<String, DocumentField> = BTreeMap::new();

        let schema_map = schema.as_map().ok_or_else(|| {
            ProtocolError::DataContractError(DataContractError::InvalidContractStructure(
                "document must be an object".to_string(),
            ))
        })?;

        // Do documents of this type keep history? (Overrides contract value)
        let documents_keep_history: bool =
            Value::inner_optional_bool_value(schema_map, "documentsKeepHistory")
                .map_err(ProtocolError::ValueError)?
                .unwrap_or(default_keeps_history);

        // Are documents of this type mutable? (Overrides contract value)
        let documents_mutable: bool =
            Value::inner_optional_bool_value(schema_map, "documentsMutable")
                .map_err(ProtocolError::ValueError)?
                .unwrap_or(default_mutability);

        let index_values =
            Value::inner_optional_array_slice_value(schema_map, property_names::INDICES)?;

        let indices: Vec<Index> = index_values
            .map(|index_values| {
                index_values
                    .iter()
                    .map(|index_value| {
                        index_value
                            .as_map()
                            .ok_or(ProtocolError::DataContractError(
                                DataContractError::InvalidContractStructure(
                                    "index definition is not a map as expected".to_string(),
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
            Value::inner_optional_btree_map(schema_map, property_names::PROPERTIES)?
                .unwrap_or_default();

        let required_fields = Value::inner_recursive_optional_array_of_strings(
            schema_map,
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
                schema_defs,
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
                schema_defs,
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

        // TODO: Figure out why do we need this and how it differs from `binary_paths`
        //   and move this function to DocumentType
        let binary_properties = DataContract::create_binary_properties(&schema, platform_version)?;

        Ok(DocumentTypeV0 {
            name: String::from(name),
            schema,
            indices,
            index_structure,
            flattened_properties: flattened_document_properties,
            properties: document_properties,
            binary_properties,
            identifier_paths,
            binary_paths,
            required_fields,
            documents_keep_history,
            documents_mutable,
            data_contract_id,
        })
    }
}
