use crate::error::Error;
use dpp::state_transition::identity_update_transition::IdentityUpdateTransition;
use dpp::validation::SimpleConsensusValidationResult;
use dpp::version::PlatformVersion;

pub(in crate::execution::validation::state_transition::state_transitions::identity_update) trait IdentityUpdateStateTransitionStructureValidationV0
{
    fn validate_basic_structure_v0(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error>;
}

impl IdentityUpdateStateTransitionStructureValidationV0 for IdentityUpdateTransition {
    fn validate_basic_structure_v0(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error> {
        // Delegate to the DPP-owned implementation so both client-side
        // construction and server-side validation apply identical checks
        // (including the shared `MAX_IDENTITY_PUBLIC_KEYS_TO_DISABLE` limit
        // and check ordering).
        match self {
            IdentityUpdateTransition::V0(v0) => v0
                .validate_basic_structure_v0(platform_version)
                .map_err(Error::Protocol),
        }
    }
}
