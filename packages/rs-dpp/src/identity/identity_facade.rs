use std::sync::Arc;

use crate::identity::factory::IdentityFactory;
use crate::identity::validation::{IdentityValidator, PublicKeysValidator};
use crate::state_repository::StateRepositoryLike;
use crate::validation::ValidationResult;
use crate::version::ProtocolVersionValidator;
use crate::{BlsModule, DashPlatformProtocol, DashPlatformProtocolInitError, NonConsensusError};

pub struct IdentityFacade<T: BlsModule> {
    identity_validator: Arc<IdentityValidator<PublicKeysValidator<T>>>,
    factory: IdentityFactory<T>,
}

impl<T> IdentityFacade<T>
where
    T: BlsModule,
{
    pub fn new(
        protocol_version: u32,
        protocol_version_validator: Arc<ProtocolVersionValidator>,
        public_keys_validator: Arc<PublicKeysValidator<T>>,
    ) -> Result<Self, DashPlatformProtocolInitError> {
        let identity_validator = Arc::new(IdentityValidator::new(
            protocol_version_validator,
            public_keys_validator,
        )?);

        Ok(Self {
            identity_validator: identity_validator.clone(),
            factory: IdentityFactory::new(protocol_version, identity_validator),
        })
    }

    pub fn validate(
        &self,
        identity_json: &serde_json::Value,
    ) -> Result<ValidationResult<()>, NonConsensusError> {
        self.identity_validator.validate_identity(identity_json)
    }
}
