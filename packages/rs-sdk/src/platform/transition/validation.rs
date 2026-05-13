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

/// Convert a structure-validation result into [`Error`], with one special
/// case for [`UnsupportedFeatureError`].
///
/// `UnsupportedFeatureError` has *two* meanings in DPP:
///
/// 1. **"Structure validation is not implemented for this state transition
///    kind"** — e.g. identity-based STs return a result that is *entirely*
///    `UnsupportedFeatureError` entries. In this case we treat the result
///    as a no-op pass so the prepare APIs can sign and broadcast these STs
///    even though their structure check is a stub.
/// 2. **"A specific feature inside an otherwise-validated ST is not
///    supported on this platform version"** — in this case the result
///    mixes `UnsupportedFeatureError` entries with real validation
///    failures. Here the unsupported entries are *not* placeholders: they
///    are legitimate rejections that explain why a particular sub-feature
///    is unavailable, and silently dropping them would discard
///    user-visible diagnostic information.
///
/// To honor both meanings we only treat the "all errors are unsupported"
/// case as `Ok`. Once *any* non-unsupported error is present we surface
/// the result via the existing `From<SimpleConsensusValidationResult> for
/// Error` conversion — which keeps the first error as a *typed*
/// `ConsensusError` so callers can pattern-match on it. To avoid the
/// conversion picking an `UnsupportedFeatureError` placeholder when a
/// real failure is also present, we first reorder the error list so the
/// first non-`UnsupportedFeatureError` entry is primary.
fn map_validation_result(mut result: SimpleConsensusValidationResult) -> Result<(), Error> {
    if result.is_valid() {
        return Ok(());
    }

    if result.errors.iter().all(is_unsupported_feature_error) {
        return Ok(());
    }

    // Mixed `UnsupportedFeatureError` + real-error case. The default
    // `From<SimpleConsensusValidationResult> for Error` conversion keeps
    // the *first* error as a typed `ConsensusError`. Stable-partition the
    // list so real failures come first, ensuring the typed error returned
    // is the most actionable one and not an `UnsupportedFeatureError`
    // placeholder. We deliberately use the existing `From` conversion so
    // the returned `Error` preserves the typed `ConsensusError` variant
    // for downstream pattern-matching, instead of being flattened into a
    // `ProtocolError::Generic` string.
    result.errors.sort_by_key(|e| {
        if is_unsupported_feature_error(e) {
            1
        } else {
            0
        }
    });
    Err(Error::from(result))
}

/// Ensures a state transition passes structure validation before broadcasting.
///
/// `UnsupportedFeatureError` has two meanings in DPP — see
/// [`map_validation_result`] for the full discussion. In short:
///
/// * an all-unsupported result is treated as `Ok` because DPP uses that
///   shape as a "structure validation is not implemented for this state
///   transition kind" sentinel (e.g. identity-based STs). The platform
///   will still perform validation during execution.
/// * a result that mixes `UnsupportedFeatureError` with real errors is
///   surfaced as an `Err` via the existing
///   `From<SimpleConsensusValidationResult> for Error` conversion, with
///   the real failures reordered first so the returned typed
///   `ConsensusError` is the actionable one (not an
///   `UnsupportedFeatureError` placeholder).
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

    /// When errors mix unsupported with real ones we return an `Err`
    /// via the existing `From<SimpleConsensusValidationResult> for Error`
    /// conversion, with real failures reordered first so the typed
    /// `ConsensusError` returned is the actionable one — not an
    /// `UnsupportedFeatureError` placeholder.
    #[test]
    fn mixed_errors_promote_real_error_to_primary_typed_error() {
        let result = SimpleConsensusValidationResult::new_with_errors(vec![
            unsupported_error(),
            value_error("real failure"),
        ]);
        let err = map_validation_result(result).expect_err("expected error");
        let rendered = format!("{err}");
        assert!(
            rendered.contains("real failure"),
            "expected real-failure message, got: {rendered}"
        );

        // The conversion must preserve the typed `ConsensusError` variant
        // (a `ValueError` here) rather than wrapping the rendered text in
        // a `ProtocolError::Generic`. Pattern-matching on the typed
        // variant is what downstream callers rely on.
        match err {
            Error::Protocol(dpp::ProtocolError::ConsensusError(boxed)) => match *boxed {
                ConsensusError::BasicError(BasicError::ValueError(_)) => {}
                other => panic!("expected ValueError typed variant, got: {other:?}"),
            },
            other => panic!("expected typed ConsensusError, got: {other:?}"),
        }
    }

    /// Real errors are reordered to the front even when they appear after
    /// `UnsupportedFeatureError` entries in the input list, so the typed
    /// `ConsensusError` returned by the `From` conversion is always the
    /// actionable failure, never an unsupported-feature placeholder.
    #[test]
    fn mixed_errors_reorder_real_failure_before_unsupported() {
        let result = SimpleConsensusValidationResult::new_with_errors(vec![
            unsupported_error(),
            unsupported_error(),
            value_error("primary failure"),
            unsupported_error(),
        ]);
        let err = map_validation_result(result).expect_err("expected error");
        match err {
            Error::Protocol(dpp::ProtocolError::ConsensusError(boxed)) => match *boxed {
                ConsensusError::BasicError(BasicError::ValueError(ref ve)) => {
                    assert!(
                        ve.to_string().contains("primary failure"),
                        "expected primary failure, got: {ve}"
                    );
                }
                other => panic!("expected ValueError typed variant, got: {other:?}"),
            },
            other => panic!("expected typed ConsensusError, got: {other:?}"),
        }
    }
}
