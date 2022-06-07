use crate::{errors::ProtocolError, util::json_schema::JsonSchemaExt};

use super::DataContract;
use anyhow::anyhow;
use serde_json::Value as JsonValue;

const PROPERTY_PROPERTIES: &str = "properties";
const PROPERTY_REQUIRED: &str = "required";

pub const PREFIX_BYTE_0: u8 = 0;
pub const PREFIX_BYTE_1: u8 = 1;
pub const PREFIX_BYTE_2: u8 = 2;
pub const PREFIX_BYTE_3: u8 = 3;

pub fn enrich_data_contract_with_base_schema(
    data_contract: &DataContract,
    base_schema: &JsonValue,
    schema_id_byte_prefix: u8,
    exclude_properties: &[&str],
) -> Result<DataContract, ProtocolError> {
    let mut cloned_data_contract = data_contract.clone();
    cloned_data_contract.schema = String::from("");

    let base_properties = base_schema
        .get_schema_properties()?
        .as_object()
        .ok_or_else(|| anyhow!("'properties' is not a map"))?;

    let base_required = base_schema.get_schema_required_fields()?;

    for (_, cloned_document) in cloned_data_contract.documents.iter_mut() {
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

    // TODO - decide if this is necessary
    // Ajv caches schemas using $id internally
    // so we can't pass two different schemas with the same $id.
    // Hacky solution for that is to replace first four bytes
    // in $id with passed prefix byte
    cloned_data_contract.id.buffer[0] = schema_id_byte_prefix;
    cloned_data_contract.id.buffer[1] = schema_id_byte_prefix;
    cloned_data_contract.id.buffer[2] = schema_id_byte_prefix;
    cloned_data_contract.id.buffer[3] = schema_id_byte_prefix;
    Ok(cloned_data_contract)
}
