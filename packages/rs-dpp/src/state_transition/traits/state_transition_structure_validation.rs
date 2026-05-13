#[cfg(any(feature = "state-transition-signing", test))]
use crate::consensus::basic::state_transition::StateTransitionNotActiveError;
#[cfg(any(feature = "state-transition-signing", test))]
use crate::consensus::ConsensusError;
#[cfg(feature = "state-transition-signing")]
use crate::state_transition::StateTransitionType;
use crate::validation::SimpleConsensusValidationResult;
use crate::ProtocolError;
#[cfg(feature = "state-transition-signing")]
use platform_version::version::feature_initial_protocol_versions::ADDRESS_FUNDS_INITIAL_PROTOCOL_VERSION;
use platform_version::version::PlatformVersion;

/// Trait for validating the structure of a state transition
pub trait StateTransitionStructureValidation {
    /// Validates the structure of the state transition
    fn validate_structure(
        &self,
        platform_version: &PlatformVersion,
    ) -> SimpleConsensusValidationResult;
}

/// Converts a `SimpleConsensusValidationResult` into a `ProtocolError` when it
/// contains at least one consensus error.
///
/// The historical helper name is kept to minimize churn at call sites. When
/// there is exactly one error it returns `ProtocolError::ConsensusError`; when
/// there are multiple it returns `ProtocolError::ConsensusErrors`, preserving
/// the full payload instead of silently discarding the remainder.
pub(crate) fn first_consensus_error_as_protocol_error(
    result: SimpleConsensusValidationResult,
) -> Option<ProtocolError> {
    if result.errors.is_empty() {
        None
    } else {
        Some(result.errors.into())
    }
}

#[cfg(feature = "state-transition-signing")]
pub(crate) fn address_funds_constructor_activation_error(
    state_transition_type: StateTransitionType,
    platform_version: &PlatformVersion,
) -> Option<ProtocolError> {
    (platform_version.protocol_version < ADDRESS_FUNDS_INITIAL_PROTOCOL_VERSION).then(|| {
        ProtocolError::from(ConsensusError::from(StateTransitionNotActiveError::new(
            state_transition_type.to_string(),
            platform_version.protocol_version,
            ADDRESS_FUNDS_INITIAL_PROTOCOL_VERSION,
        )))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn helper_returns_single_consensus_error_for_one_error() {
        let error = ConsensusError::from(StateTransitionNotActiveError::new("test", 1, 11));
        let result = SimpleConsensusValidationResult::new_with_error(error.clone());

        let protocol_error = first_consensus_error_as_protocol_error(result);

        assert!(matches!(
            protocol_error,
            Some(ProtocolError::ConsensusError(boxed)) if *boxed == error
        ));
    }

    #[test]
    fn helper_preserves_all_consensus_errors_for_multiple_errors() {
        let first = ConsensusError::from(StateTransitionNotActiveError::new("first", 1, 11));
        let second = ConsensusError::from(StateTransitionNotActiveError::new("second", 1, 11));
        let result =
            SimpleConsensusValidationResult::new_with_errors(vec![first.clone(), second.clone()]);

        let protocol_error = first_consensus_error_as_protocol_error(result);

        assert!(matches!(
            protocol_error,
            Some(ProtocolError::ConsensusErrors(errors))
                if errors == vec![first, second]
        ));
    }
}
