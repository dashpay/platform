use crate::identity::validation::{IdentityValidator, PublicKeysValidator};
use crate::validation::ValidationResult;
use crate::version::ProtocolVersionValidator;
use crate::{DashPlatformProtocolInitError, NonConsensusError};
use std::sync::Arc;

pub struct IdentityFacade {
    identity_validator: IdentityValidator<PublicKeysValidator>,
}

impl IdentityFacade {
    pub fn new(
        protocol_version_validator: Arc<ProtocolVersionValidator>,
        public_keys_validator: Arc<PublicKeysValidator>,
    ) -> Result<Self, DashPlatformProtocolInitError> {
        Ok(Self {
            identity_validator: IdentityValidator::new(protocol_version_validator, public_keys_validator)?,
        })
    }

    pub fn validate(
        &self,
        identity_json: serde_json::Value,
    ) -> Result<ValidationResult, NonConsensusError> {
        self.identity_validator.validate_identity(&identity_json)
    }
}
