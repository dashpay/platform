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
///
/// When multiple consensus errors are present, only the first is returned. To keep this
/// helper quiet on the hot validation path, any additional errors are not collected or
/// formatted; only the count of discarded errors is recorded at `debug` level via
/// `tracing`, preserving the single-error return contract without emitting noisy payloads.
pub(crate) fn first_consensus_error_as_protocol_error(
    result: SimpleConsensusValidationResult,
) -> Option<ProtocolError> {
    let mut errors = result.errors.into_iter();
    let first_error = errors.next()?;
    let discarded_count = errors.count();
    if discarded_count > 0 {
        tracing::debug!(
            discarded_count,
            "first_consensus_error_as_protocol_error: discarding {} additional consensus error(s)",
            discarded_count,
        );
    }
    Some(ProtocolError::ConsensusError(Box::new(first_error)))
}
