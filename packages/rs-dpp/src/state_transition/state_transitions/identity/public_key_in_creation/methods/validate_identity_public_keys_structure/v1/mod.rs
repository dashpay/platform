use crate::consensus::basic::identity::InvalidAuthenticationScopeError;
use crate::identity::{contract_bounds::ContractBounds, Purpose, SecurityLevel};
use crate::state_transition::public_key_in_creation::{
    accessors::IdentityPublicKeyInCreationV0Getters, IdentityPublicKeyInCreation,
};
use crate::{validation::SimpleConsensusValidationResult, version::PlatformVersion, ProtocolError};

impl IdentityPublicKeyInCreation {
    pub(super) fn validate_identity_public_keys_structure_v1(
        keys: &[Self],
        in_create_identity: bool,
        version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, ProtocolError> {
        for key in keys {
            if let Some(ContractBounds::Scoped(scope)) = key.contract_bounds() {
                let reason = if key.purpose() != Purpose::AUTHENTICATION
                    || key.security_level() == SecurityLevel::MASTER
                {
                    Some("scoped keys must be non-MASTER authentication keys".to_owned())
                } else {
                    scope.validate().err().map(|error| error.to_string())
                };
                if let Some(reason) = reason {
                    return Ok(SimpleConsensusValidationResult::new_with_error(
                        InvalidAuthenticationScopeError::new(reason).into(),
                    ));
                }
            }
        }
        Self::validate_identity_public_keys_structure_common(keys, in_create_identity, version)
    }
}
