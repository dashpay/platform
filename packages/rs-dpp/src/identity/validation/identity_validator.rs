use crate::validation::{JsonSchemaValidator, ValidationResult};
use crate::version::ProtocolVersionValidator;
use crate::{DashPlatformProtocolInitError, NonConsensusError, SerdeParsingError};
use std::sync::Arc;

pub struct IdentityValidator {
    protocol_version_validator: Arc<ProtocolVersionValidator>,
    json_schema_validator: JsonSchemaValidator,
}

impl IdentityValidator {
    pub fn new(
        protocol_version_validator: Arc<ProtocolVersionValidator>,
    ) -> Result<Self, DashPlatformProtocolInitError> {
        let json_schema_validator =
            JsonSchemaValidator::new(crate::schema::identity::identity_json()?)?;

        let identity_validator = Self {
            protocol_version_validator,
            json_schema_validator,
        };

        Ok(identity_validator)
    }

    pub fn validate_identity(
        &self,
        identity_json: &serde_json::Value,
    ) -> Result<ValidationResult, NonConsensusError> {
        let mut validation_result = self.json_schema_validator.validate(&identity_json)?;

        if !validation_result.is_valid() {
            return Ok(validation_result);
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
}
