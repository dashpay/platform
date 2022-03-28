use crate::DashPlatformProtocolInitError;
use crate::identity::validation::IdentityValidator;
use crate::validation::ValidationResult;

pub struct IdentityFacade {
    identity_validator: IdentityValidator
}

impl IdentityFacade {
    pub fn new() -> Result<Self, DashPlatformProtocolInitError> {
        Ok(Self {
            identity_validator: IdentityValidator::new()?
        })
    }

    pub fn validate(&self, identity_json: serde_json::Value) -> ValidationResult {
        self.identity_validator.validate_identity(&identity_json)
    }
}