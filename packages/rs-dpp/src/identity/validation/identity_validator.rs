use jsonschema::{ErrorIterator, JSONSchema, ValidationError};
use thiserror::Error;
use crate::dash_platform_protocol::JsonSchemas;
use crate::DashPlatformProtocolInitError;
use crate::errors::consensus::basic::JsonSchemaError;
use crate::errors::consensus::ConsensusError;
use crate::schema::IdentitySchemaJsons;
use crate::validation::ValidationResult;

pub struct IdentityValidator {
    identity_schema_json: serde_json::Value,
    identity_schema: Option<JSONSchema>,
}

impl IdentityValidator {
    pub fn new() -> Result<Self, DashPlatformProtocolInitError> {
        let identity_schema_json = crate::schema::identity::identity_json()?;

        let mut identity_validator = Self {
            identity_schema_json,
            identity_schema: None
        };

        let identity_schema = JSONSchema::compile(&identity_validator.identity_schema_json)?;
        identity_validator.identity_schema = Some(identity_schema);

        Ok(identity_validator)
    }

    pub fn validate_identity(&self, identity_json: &serde_json::Value) -> ValidationResult {
        // Validator.validate(schema, identity_json, None) should be here
        let res = self.identity_schema.as_ref().unwrap().validate(&identity_json);
        let mut validation_result = ValidationResult::new(None);
        match res {
            Ok(_) => {}
            Err(validation_errors) => {
                let errors: Vec<ConsensusError> = validation_errors
                    .map(|e| ConsensusError::from(e))
                    .collect();
                validation_result.add_errors(errors);
            }
        }
        validation_result
    }

    pub fn validate_public_keys() {}
}