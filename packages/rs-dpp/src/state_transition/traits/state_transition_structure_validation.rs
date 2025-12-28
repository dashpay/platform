use crate::validation::SimpleConsensusValidationResult;
use platform_version::version::PlatformVersion;

/// Trait for validating the structure of a state transition
pub trait StateTransitionStructureValidation {
    /// Validates the structure of the state transition
    fn validate_structure(
        &self,
        platform_version: &PlatformVersion,
    ) -> SimpleConsensusValidationResult;
}
