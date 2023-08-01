use crate::consensus::ConsensusError;
use crate::data_contract::errors::{DataContractError, JsonSchemaError};
use crate::data_contract::property_names;
use crate::data_contract::schema::enrich_with_base_schema::enrich_with_base_schema;
use crate::validation::{ConsensusValidationResult, JsonSchemaValidator};
use crate::ProtocolError;
use platform_version::version::PlatformVersion;
use serde_json::Value as JsonValue;

pub const DATA_CONTRACT_SCHEMA_URI_V0: &str =
    "https://github.com/dashpay/platform/blob/master/packages/rs-dpp/schema/meta_schemas/data_contract/v0/dataContractMeta.json";

#[derive(Debug, Clone)]
pub struct DataContractSchema {
    validator: JsonSchemaValidator,
}

impl DataContractSchema {
    pub fn from_json_schema(
        mut json_schema: JsonValue,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<Self>, ProtocolError> {
        let mut_map = json_schema.as_object_mut().ok_or_else(|| {
            ProtocolError::DataContractError(DataContractError::JsonSchema(
                JsonSchemaError::CreateSchemaError("can't create a mutable map from a json schema"),
            ))
        })?;

        mut_map
            .insert(
                property_names::SCHEMA.to_string(),
                JsonValue::String(DATA_CONTRACT_SCHEMA_URI_V0.to_string()),
            )
            .ok_or_else(|| {
                ProtocolError::DataContractError(DataContractError::JsonSchema(
                    JsonSchemaError::CreateSchemaError("$schema shouldn't be set"),
                ))
            })?;

        let full_schema = enrich_with_base_schema(&json_schema, &[])?;

        // TODO: Validate contract schema against meta schema and other stuff

        // result.merge(multi_validator::validate(
        //     &schema,
        //     &[
        //         pattern_is_valid_regex_validator,
        //         byte_array_has_no_items_as_parent_validator,
        //     ],
        // ));
        //
        // if !result.is_valid() {
        //     return Ok(result);
        // }

        match JsonSchemaValidator::new(full_schema, platform_version) {
            Ok(validator) => {
                let data_contract_schema = Self { validator };

                Ok(ConsensusValidationResult::new_with_data(
                    data_contract_schema,
                ))
            }
            Err(validation_error) => {
                let mut validation_result = ConsensusValidationResult::default();

                validation_result.add_error(ConsensusError::from(validation_error));

                Ok(validation_result)
            }
        }
    }
}
