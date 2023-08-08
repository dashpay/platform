mod v0;

use crate::data_contract::document_type::validation::enrich_with_base_schema::v0::enrich_with_base_schema_v0;
use crate::ProtocolError;
use platform_value::Value;
use platform_version::version::PlatformVersion;

pub fn enrich_with_base_schema(
    schema: Value,
    schema_defs: Option<Value>,
    exclude_properties: &[&str], // TODO: Do we need this?
    platform_version: &PlatformVersion,
) -> Result<Value, ProtocolError> {
    match platform_version
        .dpp
        .contract_versions
        .document_type_versions
        .validation_versions
        .enrich_with_base_schema
    {
        0 => enrich_with_base_schema_v0(schema, schema_defs, exclude_properties),
        _ => Err(ProtocolError::UnknownVersionMismatch {
            method: "enrich_with_base_schema".to_string(),
            known_versions: vec![0],
            received: platform_version
                .dpp
                .contract_versions
                .document_type_versions
                .validation_versions
                .enrich_with_base_schema,
        }),
    }
}
