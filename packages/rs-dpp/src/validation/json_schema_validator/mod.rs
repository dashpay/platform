pub mod methods;

use crate::data_contract::JsonValue;
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
    type Item = JsonValue;
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
