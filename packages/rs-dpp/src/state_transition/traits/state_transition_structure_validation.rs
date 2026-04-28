use crate::validation::SimpleConsensusValidationResult;
use crate::ProtocolError;
use platform_version::version::PlatformVersion;

/// Trait for validating the structure of a state transition
pub trait StateTransitionStructureValidation {
    /// Validates the structure of the state transition
    fn validate_structure(
        &self,
        platform_version: &PlatformVersion,
    ) -> SimpleConsensusValidationResult;
}

/// Converts a `SimpleConsensusValidationResult` into `Some(ProtocolError::ConsensusError)`
/// containing the first error if the result is invalid, or `None` otherwise.
///
/// This avoids `unwrap` while still surfacing only the first consensus error, which is the
/// pattern used by client-side state transition constructors before/after signing.
pub(crate) fn first_consensus_error_as_protocol_error(
    result: SimpleConsensusValidationResult,
) -> Option<ProtocolError> {
    result
        .errors
        .into_iter()
        .next()
        .map(|error| ProtocolError::ConsensusError(Box::new(error)))
}
