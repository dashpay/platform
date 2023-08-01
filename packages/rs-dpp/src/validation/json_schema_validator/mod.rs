pub mod methods;

use crate::validation::{DataValidator, SimpleConsensusValidationResult};
use crate::version::PlatformVersion;
use anyhow::Context;
use jsonschema::JSONSchema;
use serde_json::Value as JsonValue;

pub struct JsonSchemaValidator {
    raw_schema_json: JsonValue,
    schema: Option<JSONSchema>,
}

impl DataValidator for JsonSchemaValidator {
    type Item = JsonValue;
    fn validate(
        &self,
        data: &Self::Item,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, crate::ProtocolError> {
        let result = self
            .validate(data, platform_version)
            .context("error during validating json schema")?; // TODO: Remove?
        Ok(result)
    }
}
