use crate::error::Error;
use crate::execution::validation::state_transition::common::validate_identity_public_keys_structure::v0::validate_identity_public_keys_structure_v0;
use dpp::state_transition::identity_create_transition::accessors::IdentityCreateTransitionAccessorsV0;
use dpp::state_transition::identity_create_transition::IdentityCreateTransition;
use dpp::validation::SimpleConsensusValidationResult;
use dpp::version::PlatformVersion;

pub(in crate::execution::validation::state_transition::state_transitions::identity_create) trait IdentityCreateStateTransitionStructureValidationV0
{
    fn validate_base_structure_v0(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error>;
}

impl IdentityCreateStateTransitionStructureValidationV0 for IdentityCreateTransition {
    fn validate_base_structure_v0(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error> {
        validate_identity_public_keys_structure_v0(self.public_keys(), platform_version)
    }
}
