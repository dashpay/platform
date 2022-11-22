use std::sync::Arc;

use crate::identity::validation::{IdentityValidator, PublicKeysValidator};
use crate::validation::ValidationResult;
use crate::version::ProtocolVersionValidator;
use crate::{BlsModule, DashPlatformProtocolInitError, NonConsensusError};

pub struct IdentityFacade<T: BlsModule> {
    identity_validator: IdentityValidator<PublicKeysValidator<T>>,
}

impl<T: BlsModule> IdentityFacade<T> {
    pub fn new(
        protocol_version_validator: Arc<ProtocolVersionValidator>,
        public_keys_validator: Arc<PublicKeysValidator<T>>,
    ) -> Result<Self, DashPlatformProtocolInitError> {
        Ok(Self {
            identity_validator: IdentityValidator::new(
                protocol_version_validator,
                public_keys_validator,
            )?,
        })
    }

    pub fn validate(
        &self,
        identity_json: &serde_json::Value,
    ) -> Result<ValidationResult<()>, NonConsensusError> {
        self.identity_validator.validate_identity(identity_json)
    }
}
