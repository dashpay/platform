use crate::Error;
use dpp::{
    consensus::{basic::BasicError, ConsensusError},
    state_transition::{StateTransition, StateTransitionStructureValidation},
    version::PlatformVersion,
};

/// Checks if an error is an UnsupportedFeatureError
fn is_unsupported_feature_error(error: &ConsensusError) -> bool {
    matches!(
        error,
        ConsensusError::BasicError(BasicError::UnsupportedFeatureError(_))
    )
}

/// Ensures a state transition passes structure validation before broadcasting.
///
/// Note: UnsupportedFeatureError is allowed to pass through, as it indicates
/// that structure validation is not implemented for that state transition type
/// (e.g., identity-based state transitions). The platform will still perform
/// validation during execution.
pub(crate) fn ensure_valid_state_transition_structure(
    state_transition: &StateTransition,
    platform_version: &PlatformVersion,
) -> Result<(), Error> {
    let validation_result = state_transition.validate_structure(platform_version);
    if validation_result.is_valid() {
        Ok(())
    } else {
        // Allow UnsupportedFeatureError to pass through - this means structure
        // validation is not implemented for this state transition type
        let all_unsupported_feature_errors = validation_result
            .errors
            .iter()
            .all(is_unsupported_feature_error);
        if all_unsupported_feature_errors {
            Ok(())
        } else {
            Err(validation_result.into())
        }
    }
}
