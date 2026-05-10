use crate::Error;
use dpp::{
    consensus::{basic::BasicError, ConsensusError},
    state_transition::{StateTransition, StateTransitionStructureValidation},
    validation::SimpleConsensusValidationResult,
    version::PlatformVersion,
};

/// Checks if an error is an UnsupportedFeatureError
fn is_unsupported_feature_error(error: &ConsensusError) -> bool {
    matches!(
        error,
        ConsensusError::BasicError(BasicError::UnsupportedFeatureError(_))
    )
}

/// Filter `UnsupportedFeatureError` entries out of a structure-validation
/// result and convert any remaining errors into [`Error`].
///
/// `UnsupportedFeatureError` only signals that structure validation is not yet
/// implemented for that state transition kind, so it must never mask a real
/// validation failure. If the result becomes empty after filtering we treat it
/// as a no-op pass.
fn map_validation_result(result: SimpleConsensusValidationResult) -> Result<(), Error> {
    if result.is_valid() {
        return Ok(());
    }

    let real_errors: Vec<ConsensusError> = result
        .errors
        .into_iter()
        .filter(|e| !is_unsupported_feature_error(e))
        .collect();

    if real_errors.is_empty() {
        Ok(())
    } else {
        Err(SimpleConsensusValidationResult::new_with_errors(real_errors).into())
    }
}

/// Ensures a state transition passes structure validation before broadcasting.
///
/// Note: `UnsupportedFeatureError` is allowed to pass through, as it indicates
/// that structure validation is not implemented for that state transition type
/// (e.g., identity-based state transitions). The platform will still perform
/// validation during execution.
///
/// When the result mixes `UnsupportedFeatureError`s with other errors, the
/// unsupported variants are filtered out so the surfaced error is always a
/// real validation problem rather than a "not implemented" placeholder.
pub fn ensure_valid_state_transition_structure(
    state_transition: &StateTransition,
    platform_version: &PlatformVersion,
) -> Result<(), Error> {
    map_validation_result(state_transition.validate_structure(platform_version))
}

#[cfg(test)]
mod tests {
    use super::*;
    use dpp::consensus::basic::unsupported_feature_error::UnsupportedFeatureError;
    use dpp::consensus::basic::value_error::ValueError;

    fn unsupported_error() -> ConsensusError {
        ConsensusError::BasicError(BasicError::UnsupportedFeatureError(
            UnsupportedFeatureError::new("feature".to_string(), 0),
        ))
    }

    fn value_error(msg: &str) -> ConsensusError {
        ConsensusError::BasicError(BasicError::ValueError(ValueError::new_from_string(
            msg.to_string(),
        )))
    }

    /// When every error is an UnsupportedFeatureError we treat the validation
    /// result as a no-op and return Ok.
    #[test]
    fn all_unsupported_errors_are_treated_as_ok() {
        let result = SimpleConsensusValidationResult::new_with_errors(vec![
            unsupported_error(),
            unsupported_error(),
        ]);
        assert!(map_validation_result(result).is_ok());
    }

    /// A single non-UnsupportedFeature error is surfaced as a real failure.
    #[test]
    fn single_real_error_is_surfaced() {
        let result =
            SimpleConsensusValidationResult::new_with_errors(vec![value_error("bad value")]);
        let err = map_validation_result(result).expect_err("expected real error");
        assert!(format!("{err}").contains("bad value"));
    }

    /// When errors mix unsupported with real ones, the real one wins —
    /// without this filtering, the first-error conversion could surface the
    /// UnsupportedFeatureError and incorrectly let the transition through.
    #[test]
    fn mixed_errors_skip_unsupported_and_return_real_error() {
        let result = SimpleConsensusValidationResult::new_with_errors(vec![
            unsupported_error(),
            value_error("real failure"),
        ]);
        let err = map_validation_result(result).expect_err("expected real error");
        assert!(
            format!("{err}").contains("real failure"),
            "expected real-failure message, got: {err}"
        );
    }
}
