use crate::data_contract::errors::{DataContractError, JsonSchemaError};
use crate::data_contract::JsonValue;
use crate::util::json_schema::JsonSchemaExt;
use crate::ProtocolError;
use lazy_static::lazy_static;

lazy_static! {
    pub static ref BASE_DOCUMENT_SCHEMA: JsonValue =
        serde_json::from_str(include_str!("../../../schema/document/documentBase.json"))
            .expect("can't parse documentBase.json");
}

const PROPERTY_PROPERTIES: &str = "properties";
const PROPERTY_REQUIRED: &str = "required";

pub fn enrich_with_base_schema(
    schema: &JsonValue,
    exclude_properties: &[&str], // TODO: Do we need this?
) -> Result<JsonValue, ProtocolError> {
    let cloned_schema = schema.clone();

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

    let mut documents_value = cloned_schema.get("documents").ok_or_else(|| {
        ProtocolError::DataContractError(DataContractError::JsonSchema(
            JsonSchemaError::CreateSchemaError("can't find 'documents' in the schema"),
        ))
    })?;

    let documents = documents_value.as_object_mut().ok_or_else(|| {
        ProtocolError::DataContractError(DataContractError::JsonSchema(
            JsonSchemaError::CreateSchemaError("documents "),
        ))
    })?;

    for cloned_document in documents.values_mut() {
        if let Some(JsonValue::Object(ref mut properties)) =
            cloned_document.get_mut(PROPERTY_PROPERTIES)
        {
            properties.extend(
                base_properties
                    .iter()
                    .map(|(k, v)| ((*k).to_owned(), (*v).to_owned())),
            );
        }

        if let Some(JsonValue::Array(ref mut required)) = cloned_document.get_mut(PROPERTY_REQUIRED)
        {
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
    }

    Ok(cloned_schema)
}
