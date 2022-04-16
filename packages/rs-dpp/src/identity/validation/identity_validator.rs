use crate::identity::validation::TPublicKeysValidator;
use crate::validation::{JsonSchemaValidator, ValidationResult};
use crate::version::ProtocolVersionValidator;
use crate::{DashPlatformProtocolInitError, NonConsensusError, SerdeParsingError};
use serde_json::{Map, Value};
use std::sync::Arc;

pub struct IdentityValidator<TPublicKeyValidator> {
    protocol_version_validator: Arc<ProtocolVersionValidator>,
    json_schema_validator: JsonSchemaValidator,
    public_keys_validator: Arc<TPublicKeyValidator>,
}

impl<T: TPublicKeysValidator> IdentityValidator<T> {
    pub fn new(
        protocol_version_validator: Arc<ProtocolVersionValidator>,
        public_keys_validator: Arc<T>,
    ) -> Result<Self, DashPlatformProtocolInitError> {
        let json_schema_validator =
            JsonSchemaValidator::new(crate::schema::identity::identity_json()?)?;

        let identity_validator = Self {
            protocol_version_validator,
            json_schema_validator,
            public_keys_validator,
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

        let protocol_version = get_protocol_version(identity_map)?;
        validation_result.merge(self.protocol_version_validator.validate(protocol_version)?);

        if !validation_result.is_valid() {
            return Ok(validation_result);
        }

        let raw_public_keys = get_raw_public_keys(identity_map)?;
        validation_result.merge(self.public_keys_validator.validate_keys(raw_public_keys)?);

        Ok(validation_result)
    }
}

fn get_protocol_version(identity_map: &Map<String, Value>) -> Result<u64, SerdeParsingError> {
    Ok(identity_map
        .get("protocolVersion")
        .ok_or(SerdeParsingError::new(
            "Expected identity to have protocolVersion",
        ))?
        .as_u64()
        .ok_or(SerdeParsingError::new(
            "Expected protocolVersion to be a uint",
        ))?)
}

fn get_raw_public_keys(
    identity_map: &Map<String, Value>,
) -> Result<&Vec<Value>, SerdeParsingError> {
    identity_map
        .get("publicKeys")
        .ok_or(SerdeParsingError::new(
            "Expected identity.publicKeys to exist",
        ))?
        .as_array()
        .ok_or(SerdeParsingError::new(
            "Expected identity.publicKeys to be an array",
        ))
}
