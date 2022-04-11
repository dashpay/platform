use std::sync::Arc;
use crate::{DashPlatformProtocolInitError, NonConsensusError};
use crate::identity::validation::IdentityValidator;
use crate::validation::ValidationResult;
use crate::version::ProtocolVersionValidator;

pub struct IdentityFacade {
    identity_validator: IdentityValidator
}

impl IdentityFacade {
    pub fn new(protocol_version_validator: Arc<ProtocolVersionValidator>) -> Result<Self, DashPlatformProtocolInitError> {
        Ok(Self {
            identity_validator: IdentityValidator::new(protocol_version_validator)?
        })
    }

    pub fn validate(&self, identity_json: serde_json::Value) -> Result<ValidationResult, NonConsensusError> {
        self.identity_validator.validate_identity(&identity_json)
    }
}