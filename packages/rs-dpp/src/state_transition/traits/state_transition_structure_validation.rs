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
/// When multiple consensus errors are present, only the first is returned; any additional
/// errors are emitted at `debug` level via `tracing` so they remain visible during
/// debugging without changing the public single-error return contract.
pub(crate) fn first_consensus_error_as_protocol_error(
    result: SimpleConsensusValidationResult,
) -> Option<ProtocolError> {
    let mut errors = result.errors.into_iter();
    let first_error = errors.next()?;
    let discarded: Vec<_> = errors.collect();
    if !discarded.is_empty() {
        tracing::debug!(
            discarded_count = discarded.len(),
            ?discarded,
            "first_consensus_error_as_protocol_error: discarding {} additional consensus error(s)",
            discarded.len(),
        );
    }
    Some(ProtocolError::ConsensusError(Box::new(first_error)))
}
