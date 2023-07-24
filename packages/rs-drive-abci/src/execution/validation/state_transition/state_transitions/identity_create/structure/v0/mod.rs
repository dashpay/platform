use crate::error::Error;
use crate::execution::validation::state_transition::common::validate_identity_public_keys_structure::v0::validate_identity_public_keys_structure_v0;
use crate::execution::validation::state_transition::common::validate_protocol_version::v0::validate_protocol_version_v0;
use crate::execution::validation::state_transition::common::validate_schema::v0::validate_schema_v0;
use dpp::identity::state_transition::identity_create_transition::validation::basic::IDENTITY_CREATE_TRANSITION_SCHEMA_VALIDATOR;
use dpp::identity::state_transition::identity_create_transition::IdentityCreateTransition;
use dpp::state_transition::identity_create_transition::IdentityCreateTransition;
use dpp::validation::SimpleConsensusValidationResult;
use dpp::version::PlatformVersion;

pub(crate) trait StateTransitionStructureValidationV0 {
    fn validate_structure_v0(&self, platform_version: &PlatformVersion) -> Result<SimpleConsensusValidationResult, Error>;
}

impl StateTransitionStructureValidationV0 for IdentityCreateTransition {
    fn validate_structure_v0(&self, platform_version: &PlatformVersion) -> Result<SimpleConsensusValidationResult, Error> {
        let result = validate_schema_v0(&IDENTITY_CREATE_TRANSITION_SCHEMA_VALIDATOR, self);
        if !result.is_valid() {
            return Ok(result);
        }

        let result = validate_protocol_version_v0(self.protocol_version);
        if !result.is_valid() {
            return Ok(result);
        }

        validate_identity_public_keys_structure_v0(self.public_keys.as_slice(), platform_version)
    }
}
