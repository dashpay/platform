use crate::data_contract::errors::{DataContractError, JsonSchemaError};
use crate::data_contract::property_names;
use crate::data_contract::JsonValue;
use crate::util::json_schema::JsonSchemaExt;
use crate::ProtocolError;
use lazy_static::lazy_static;
use platform_value::Value;

lazy_static! {
    pub static ref BASE_DOCUMENT_SCHEMA: JsonValue =
        serde_json::from_str(include_str!("../../../schema/document/documentBase.json"))
            .expect("can't parse documentBase.json");
}

pub const DATA_CONTRACT_SCHEMA_URI_V0: &str =
    "https://github.com/dashpay/platform/blob/master/packages/rs-dpp/schema/meta_schemas/data_contract/v0/dataContractMeta.json";

// TODO: Duplicates packages/rs-dpp/src/data_contract/document_type/mod.rs
const PROPERTY_PROPERTIES: &str = "properties";
const PROPERTY_REQUIRED: &str = "required";

pub fn enrich_with_base_schema(
    mut schema: Value,
    schema_defs: Option<Value>,
    exclude_properties: &[&str], // TODO: Do we need this?
) -> Result<Value, ProtocolError> {
    let base_properties = BASE_DOCUMENT_SCHEMA
        .get_schema_properties()?
        .as_object()
        .ok_or_else(|| {
            ProtocolError::DataContractError(DataContractError::JsonSchema(
                JsonSchemaError::CreateSchemaError(
                    "base document schema's 'properties' is not a map",
                ),
            ))
        })?;

    let base_required = BASE_DOCUMENT_SCHEMA.get_schema_required_fields()?;

    // TODO: $schema and $defs shouldn't be present

    schema
        .insert(
            property_names::SCHEMA.to_string(),
            DATA_CONTRACT_SCHEMA_URI_V0.into(),
        )
        .map_err(ProtocolError::ValueError)?;

    // Add $defs
    if let Some(schema_defs) = schema_defs {
        schema
            .insert(
                property_names::DEFINITIONS.to_string(),
                Value::from(schema_defs),
            )
            .map_err(ProtocolError::ValueError)?;
    }

    if let Some(Value::Map(ref mut properties)) = schema
        .get_mut(PROPERTY_PROPERTIES)
        .map_err(ProtocolError::ValueError)?
    {
        properties.extend(
            base_properties
                .iter()
                .map(|(k, v)| (k.clone().into(), v.into())),
        );
    }

    if let Some(Value::Array(ref mut required)) = schema
        .get_mut(PROPERTY_REQUIRED)
        .map_err(ProtocolError::ValueError)?
    {
        required.extend(base_required.iter().map(|v| v.to_string().into()));
        required.retain(|p| {
            if let Value::Text(v) = p {
                return !exclude_properties.contains(&v.as_str());
            }
            true
        });
    }

    Ok(schema)
}
