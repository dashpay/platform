use crate::data_contract::errors::DataContractError;
use crate::data_contract::serialized_version::v0::property_names as contract_property_names;
use crate::ProtocolError;
use platform_value::{Value, ValueMapHelper};

pub const DATA_CONTRACT_SCHEMA_URI_V0: &str =
    "https://github.com/dashpay/platform/blob/master/packages/rs-dpp/schema/meta_schemas/document/v0/document-meta.json";

pub const PROPERTY_SCHEMA: &str = "$schema";

pub fn enrich_with_base_schema_v0(
    mut schema: Value,
    schema_defs: Option<Value>,
) -> Result<Value, ProtocolError> {
    let schema_map = schema.to_map_mut().map_err(|err| {
        ProtocolError::DataContractError(DataContractError::InvalidContractStructure(format!(
            "document schema must be an object: {err}"
        )))
    })?;

    // Add $schema
    if schema_map.get_optional_key(PROPERTY_SCHEMA).is_some() {
        return Err(ProtocolError::DataContractError(
            DataContractError::InvalidContractStructure(
                "document schema shouldn't contain '$schema' property".to_string(),
            ),
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
        return Err(ProtocolError::DataContractError(
            DataContractError::InvalidContractStructure(
                "document schema shouldn't contain '$schema' property".to_string(),
            ),
        ));
    }

    if let Some(schema_defs) = schema_defs {
        schema_map.insert_string_key_value(
            contract_property_names::DEFINITIONS.to_string(),
            schema_defs,
        )
    }

    Ok(schema)
}
