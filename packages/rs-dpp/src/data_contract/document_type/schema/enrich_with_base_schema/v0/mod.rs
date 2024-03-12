use crate::data_contract::document_type::property_names;
use crate::data_contract::errors::DataContractError;
use crate::data_contract::serialized_version::v0::property_names as contract_property_names;
use crate::ProtocolError;
use platform_value::{Value, ValueMapHelper};
use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;

pub const DATA_CONTRACT_SCHEMA_URI_V0: &str =
    "https://github.com/dashpay/platform/blob/master/packages/rs-dpp/schema/meta_schemas/document/v0/document-meta.json";

pub const PROPERTY_SCHEMA: &str = "$schema";

const TIMESTAMPS: [&str; 2] = ["$createdAt", "$updatedAt"];

pub fn enrich_with_base_schema_v0(
    mut schema: Value,
    schema_defs: Option<Value>,
) -> Result<Value, ProtocolError> {
    let schema_map = schema.to_map_mut().map_err(|err| {
        ProtocolError::ConsensusError(ConsensusError::BasicError(BasicError::ContractError(DataContractError::InvalidContractStructure(format!(
            "document schema must be an object: {err}"
        )))).into())
    })?;

    // Add $schema
    if schema_map.get_optional_key(PROPERTY_SCHEMA).is_some() {
        return Err(ProtocolError::ConsensusError(
            ConsensusError::BasicError(BasicError::ContractError(DataContractError::InvalidContractStructure(
                "document schema shouldn't contain '$schema' property".to_string(),
            ))).into()
        ));
    }

    schema_map.insert_string_key_value(
        PROPERTY_SCHEMA.to_string(),
        DATA_CONTRACT_SCHEMA_URI_V0.into(),
    );

    // Add $defs
    if schema_map
        .get_optional_key(contract_property_names::DEFINITIONS)
        .is_some()
    {
        return Err(ProtocolError::ConsensusError(
            ConsensusError::BasicError(BasicError::ContractError(DataContractError::InvalidContractStructure(
                "document schema shouldn't contain '$schema' property".to_string(),
            ))).into()
        ));
    }

    // Remove $createdAt and $updatedAt from JSON Schema since they aren't part of
    // dynamic (user defined) document data which is validating against the schema
    if let Some(required) = schema_map.get_optional_key_mut(property_names::REQUIRED) {
        if let Some(required_array) = required.as_array_mut() {
            required_array.retain(|field_value| {
                if let Some(field) = field_value.as_text() {
                    !TIMESTAMPS.contains(&field)
                } else {
                    true
                }
            });
        }
    }

    if let Some(schema_defs) = schema_defs {
        schema_map.insert_string_key_value(
            contract_property_names::DEFINITIONS.to_string(),
            schema_defs,
        )
    }

    Ok(schema)
}
