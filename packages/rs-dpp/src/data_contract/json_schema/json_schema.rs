use crate::consensus::ConsensusError;
use crate::data_contract::errors::{DataContractError, JSONSchemaError};
use crate::data_contract::validation::data_contract_validation::BASE_DOCUMENT_SCHEMA;
use crate::data_contract::{property_names, DefinitionName, DocumentName, JsonSchema};
use crate::util::json_schema::JsonSchemaExt;
use crate::validation::ConsensusValidationResult;
use crate::ProtocolError;
use anyhow::anyhow;
use jsonschema::JSONSchema;
use serde_json::Map;
use serde_json::Value as JsonValue;
use std::collections::BTreeMap;

pub const DATA_CONTRACT_SCHEMA_URI_V0: &str =
    "https://github.com/dashpay/platform/blob/master/packages/rs-dpp/schema/meta_schemas/data_contract/v0/dataContractMeta.json";

const PROPERTY_PROPERTIES: &str = "properties";
const PROPERTY_REQUIRED: &str = "required";

struct DataContractJsonSchema(JSONSchema);

impl DataContractJsonSchema {
    pub fn from_documents(
        documents: BTreeMap<DocumentName, JsonSchema>,
        defs: Option<BTreeMap<DefinitionName, JsonSchema>>,
    ) -> Result<ConsensusValidationResult<Self>, ProtocolError> {
        let mut map = JsonSchema::Object(Map::new());

        let mut_map = map.as_object_mut().ok_or_else(|| {
            ProtocolError::DataContractError(DataContractError::JSONSchema(
                JSONSchemaError::CreateSchemaError("can't create a mutable map from a json schema"),
            ))
        })?;

        mut_map.insert(
            property_names::SCHEMA.to_string(),
            JsonSchema::String(DATA_CONTRACT_SCHEMA_URI_V0.to_string()),
        );

        mut_map
            .insert(
                property_names::DOCUMENTS.to_string(),
                documents
                    .into_iter()
                    .map(|(k, v)| (JsonValue::String(k), v))
                    .collect(),
            )
            .ok_or_else(|| {
                ProtocolError::DataContractError(DataContractError::JSONSchema(
                    JSONSchemaError::CreateSchemaError("can't insert documents into a json schema"),
                ))
            })?;

        if let Some(defs) = defs {
            mut_map
                .insert(
                    property_names::DEFINITIONS.to_string(),
                    defs.into_iter()
                        .map((|(k, v)| (JsonValue::String(k), v)))
                        .collect(),
                )
                .ok_or_else(|| {
                    ProtocolError::DataContractError(DataContractError::JSONSchema(
                        JSONSchemaError::CreateSchemaError(
                            "can't create a mutable map from a json schema",
                        ),
                    ))
                })?;
        }

        let full_schema = Self.enrich_with_base_schema(map)?;

        let compilation_result = JSONSchema::options()
            .should_ignore_unknown_formats(false)
            .should_validate_formats(true)
            .compile(&full_schema);

        match compilation_result {
            Ok(json_schema) => Ok(ConsensusValidationResult::new_with_data(Self(json_schema))),
            Err(validation_error) => {
                let mut validation_result = ConsensusValidationResult::default();

                validation_result.add_error(ConsensusError::from(validation_error));

                Ok(validation_result)
            }
        }
    }
}
