use crate::errors::consensus::ConsensusError;
use crate::validation::{byte_array_meta, ValidationResult};
use crate::version::ProtocolVersionValidator;
use crate::{DashPlatformProtocolInitError, NonConsensusError, SerdeParsingError};
use jsonschema::{JSONSchema, KeywordDefinition};
use serde_json::json;
use serde_json::Value as JsonValue;
use std::sync::Arc;

pub struct IdentityValidator {
    identity_schema_json: JsonValue,
    identity_schema: Option<JSONSchema>,
    protocol_version_validator: Arc<ProtocolVersionValidator>,
}

impl IdentityValidator {
    pub fn new(
        protocol_version_validator: Arc<ProtocolVersionValidator>,
    ) -> Result<Self, DashPlatformProtocolInitError> {
        let identity_schema_json = crate::schema::identity::identity_json()?;

        let mut identity_validator = Self {
            identity_schema_json,
            identity_schema: None,
            protocol_version_validator,
        };

        // BYTE_ARRAY META SCHEMA
        let identity_schema = &identity_validator.identity_schema_json.clone();
        let res = byte_array_meta::validate(&identity_schema);

        match res {
            Ok(_) => {}
            Err(mut errors) => {
                return Err(DashPlatformProtocolInitError::from(errors.remove(0)));
            }
        }
        // BYTE_ARRAY META SCHEMA END

        let identity_schema = JSONSchema::options()
            .add_keyword(
                "byteArray",
                KeywordDefinition::Schema(json!({
                    "items": {
                        "type": "integer",
                        "minimum": 0,
                        "maximum": 255,
                    },
                })),
            )
            .compile(&identity_validator.identity_schema_json)?;
        identity_validator.identity_schema = Some(identity_schema);

        Ok(identity_validator)
    }

    pub fn validate_identity(
        &self,
        identity_json: &serde_json::Value,
    ) -> Result<ValidationResult, NonConsensusError> {
        // TODO: create better error messages
        let res = self
            .identity_schema
            .as_ref()
            .ok_or(SerdeParsingError::new(
                "Expected identity schema to be initialized",
            ))?
            .validate(&identity_json);

        let mut validation_result = ValidationResult::new(None);

        match res {
            Ok(_) => {}
            Err(validation_errors) => {
                let errors: Vec<ConsensusError> =
                    validation_errors.map(|e| ConsensusError::from(e)).collect();
                validation_result.add_errors(errors);
                return Ok(validation_result);
            }
        }

        let identity_map = identity_json.as_object().ok_or(SerdeParsingError::new(
            "Expected identity to be a json object",
        ))?;

        let protocol_version = identity_map
            .get("protocolVersion")
            .ok_or(SerdeParsingError::new(
                "Expected identity to have protocolVersion",
            ))?
            .as_u64()
            .ok_or(SerdeParsingError::new(
                "Expected protocolVersion to be a uint",
            ))?;

        let version_validation_result =
            self.protocol_version_validator.validate(protocol_version)?;

        validation_result.merge(version_validation_result);

        Ok(validation_result)
    }

    pub fn validate_public_keys() {}

    pub fn validate_public_keys_in_identity_create_transition() {}
}
