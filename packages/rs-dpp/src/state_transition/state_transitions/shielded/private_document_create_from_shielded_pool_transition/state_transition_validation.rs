use crate::state_transition::private_document_create_from_shielded_pool_transition::PrivateDocumentCreateFromShieldedPoolTransition;
use crate::state_transition::StateTransitionStructureValidation;
use crate::validation::SimpleConsensusValidationResult;
use platform_version::version::PlatformVersion;

impl StateTransitionStructureValidation for PrivateDocumentCreateFromShieldedPoolTransition {
    fn validate_structure(
        &self,
        platform_version: &PlatformVersion,
    ) -> SimpleConsensusValidationResult {
        match self {
            PrivateDocumentCreateFromShieldedPoolTransition::V0(v0) => {
                v0.validate_structure(platform_version)
            }
        }
    }
}
