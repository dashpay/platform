pub mod methods;

use crate::validation::{DataValidator, SimpleConsensusValidationResult};
use anyhow::Context;
use jsonschema::JSONSchema;
use platform_version::version::PlatformVersion;
use std::sync::RwLock;

#[derive(Debug)]
pub struct JsonSchemaValidator {
    validator: RwLock<Option<JSONSchema>>,
}

// TODO: Remove?
impl DataValidator for JsonSchemaValidator {
    type Item = serde_json::Value;
    fn validate(
        &self,
        data: &Self::Item,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, crate::ProtocolError> {
        let result = self
            .validate(data, platform_version)
            .context("error during validating json schema")?;
        Ok(result)
    }
}
#[allow(non_camel_case_types)]
#[repr(C)]
#[ferment_macro::register(dpp::validation::json_schema_validator::JsonSchemaValidator)]
pub struct dpp_validation_JsonSchemaValidator {
    validator: RwLock<Option<jsonschema::JSONSchema>>,
}
impl ferment::FFIConversionFrom<JsonSchemaValidator> for dpp_validation_JsonSchemaValidator {
    unsafe fn ffi_from_const(_ffi: *const Self) -> JsonSchemaValidator {
        JsonSchemaValidator::new()
    }
}
impl ferment::FFIConversionTo<JsonSchemaValidator> for dpp_validation_JsonSchemaValidator {
    unsafe fn ffi_to_const(obj: JsonSchemaValidator) -> *const Self {
        ferment::boxed(dpp_validation_JsonSchemaValidator { validator: obj.validator })
    }
}
