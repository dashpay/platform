use crate::data_contract::JsonValue;
use crate::ProtocolError;
use platform_version::version::PlatformVersion;

mod v0;

use crate::data_contract::document_type::schema::validate_schema_compatibility::v0::IncompatibleOperations;
pub use v0::EMPTY_JSON;

pub fn validate_schema_compatibility(
    original_schema: &JsonValue,
    new_schema: &JsonValue,
    platform_version: &PlatformVersion,
) -> Result<IncompatibleOperations, ProtocolError> {
    match platform_version
        .dpp
        .contract_versions
        .document_type_versions
        .schema
        .validate_schema_compatibility
    {
        0 => v0::validate_schema_compatibility(original_schema, new_schema),
        version => Err(ProtocolError::UnknownVersionMismatch {
            method: "validate_schema_compatibility".to_string(),
            known_versions: vec![0],
            received: version,
        }),
    }
}

pub fn get_operation_and_property_name_json(
    p: &json_patch::PatchOperation,
) -> (&'static str, &str) {
    match &p {
        json_patch::PatchOperation::Add(ref o) => ("add json", o.path.as_str()),
        json_patch::PatchOperation::Copy(ref o) => ("copy json", o.path.as_str()),
        json_patch::PatchOperation::Remove(ref o) => ("remove json", o.path.as_str()),
        json_patch::PatchOperation::Replace(ref o) => ("replace json", o.path.as_str()),
        json_patch::PatchOperation::Move(ref o) => ("move json", o.path.as_str()),
        json_patch::PatchOperation::Test(ref o) => ("test json", o.path.as_str()),
    }
}
