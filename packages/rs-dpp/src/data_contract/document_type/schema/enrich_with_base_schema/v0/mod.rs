use crate::data_contract::document_type::property_names;
use crate::data_contract::errors::DataContractError;
use crate::data_contract::serialized_version::v0::property_names as contract_property_names;
use crate::data_contract::JsonValue;
use crate::util::json_schema::JsonSchemaExt;
use crate::ProtocolError;
use lazy_static::lazy_static;
use platform_value::{Value, ValueMapHelper};

lazy_static! {
    pub static ref BASE_DOCUMENT_SCHEMA: JsonValue = serde_json::from_str(include_str!(
        "../../../../../../schema/document/document-base.json"
    ))
    .expect("can't parse documentBase.json");
}

pub const DATA_CONTRACT_SCHEMA_URI_V0: &str =
    "https://github.com/dashpay/platform/blob/master/packages/rs-dpp/schema/meta_schemas/document/v0/document-meta.json";

// TODO: Duplicates packages/rs-dpp/src/data_contract/document_type/mod.rs
const PROPERTY_PROPERTIES: &str = "properties";
const PROPERTY_REQUIRED: &str = "required";

pub const PROPERTY_SCHEMA: &str = "$schema";

pub fn enrich_with_base_schema_v0(
    mut schema: Value,
    schema_defs: Option<Value>,
    exclude_properties: &[&str], // TODO: Do we need this?
) -> Result<Value, ProtocolError> {
    let mut schema_map = schema.to_map_mut().map_err(|err| {
        ProtocolError::DataContractError(DataContractError::InvalidContractStructure(format!(
            "document schema must be an object: {err}"
        )))
    })?;

    let base_properties = BASE_DOCUMENT_SCHEMA
        .get_schema_properties()?
        .as_object()
        .ok_or_else(|| {
            ProtocolError::DataContractError(DataContractError::InvalidContractStructure(
                "base document schema's 'properties' is not a map".to_string(),
            ))
        })?;

    let base_required = BASE_DOCUMENT_SCHEMA.get_schema_required_fields()?;

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
            Value::from(schema_defs),
        )
    }

    if let Some(Value::Map(ref mut properties)) =
        schema_map.get_optional_key_mut(PROPERTY_PROPERTIES)
    {
        properties.extend(
            base_properties
                .iter()
                .map(|(k, v)| (k.clone().into(), v.into())),
        );
    }

    if let Some(Value::Array(ref mut required)) = schema_map.get_optional_key_mut(PROPERTY_REQUIRED)
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
