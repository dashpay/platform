use crate::data_contract::errors::{DataContractError, JsonSchemaError};
use crate::data_contract::property_names;
use crate::data_contract::schema::json_schema::DATA_CONTRACT_SCHEMA_URI_V0;
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

const PROPERTY_PROPERTIES: &str = "properties";
const PROPERTY_REQUIRED: &str = "required";

pub fn enrich_with_base_schema(
    mut schema: Value,
    schema_defs: Value,
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

    if let Some(JsonValue::Object(ref mut properties)) = schema.get_mut(PROPERTY_PROPERTIES) {
        properties.extend(
            base_properties
                .iter()
                .map(|(k, v)| ((*k).to_owned(), (*v).to_owned())),
        );
    }

    if let Some(JsonValue::Array(ref mut required)) = schema.get_mut(PROPERTY_REQUIRED) {
        required.extend(
            base_required
                .iter()
                .map(|v| JsonValue::String(v.to_string())),
        );
        required.retain(|p| {
            if let JsonValue::String(v) = p {
                return !exclude_properties.contains(&v.as_str());
            }
            true
        });
    }

    Ok(schema)
}
