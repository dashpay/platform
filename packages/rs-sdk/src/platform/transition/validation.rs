use crate::Error;
use dpp::{
    consensus::{basic::BasicError, ConsensusError},
    state_transition::{StateTransition, StateTransitionStructureValidation},
    validation::SimpleConsensusValidationResult,
    version::PlatformVersion,
};

/// Prefix that DPP's root `validate_structure` uses on the
/// `UnsupportedFeatureError::feature_name` when it returns the
/// "structure validation is not implemented for this state transition
/// kind" sentinel (see rs-dpp `state_transition/mod.rs`
/// `StateTransitionStructureValidation` impl). We match on the prefix
/// rather than the exact string so a future DPP refinement of the
/// sentinel message — e.g. broadening it past identity-based STs to
/// cover Batch as well, which already shares that arm — does not
/// silently break the pass-through.
const STRUCTURE_VALIDATION_SENTINEL_PREFIX: &str = "structure validation";

/// Checks if an error is the DPP "structure validation is not
/// implemented for this state-transition kind" sentinel: an
/// `UnsupportedFeatureError` whose `feature_name` starts with
/// `"structure validation"`. This is what DPP's root
/// `validate_structure` returns for identity-based STs and Batch (see
/// rs-dpp `state_transition/mod.rs`). Other `UnsupportedFeatureError`
/// instances — e.g. token-config-update-transition rejecting a
/// sub-feature it does not yet support — use different
/// `feature_name`s and are treated as real failures, never as
/// placeholders to be passed through.
fn is_structure_validation_sentinel(error: &ConsensusError) -> bool {
    matches!(
        error,
        ConsensusError::BasicError(BasicError::UnsupportedFeatureError(e))
            if e.feature().starts_with(STRUCTURE_VALIDATION_SENTINEL_PREFIX)
    )
}

/// Checks if an error is *any* `UnsupportedFeatureError` (including
/// non-sentinel uses that flag a specific in-ST sub-feature as
/// unsupported on this platform version).
fn is_any_unsupported_feature_error(error: &ConsensusError) -> bool {
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
///    kind"** — DPP's root `validate_structure` returns a result whose
///    sole error is an `UnsupportedFeatureError` with `feature_name`
///    starting with `"structure validation"` (e.g. `"structure
///    validation for identity-based state transitions"`). The Batch ST
///    currently routes through the same sentinel arm. In this case we
///    treat the result as a no-op pass so the prepare APIs can sign
///    and broadcast these STs even though their structure check is a
///    stub. Platform itself still validates the transition during
///    execution.
/// 2. **"A specific feature inside an otherwise-validated ST is not
///    supported on this platform version"** — e.g. the
///    `token_config_update_transition` v0 structure check emits its own
///    `UnsupportedFeatureError` with a different `feature_name`. Here
///    the unsupported entries are *not* placeholders: they are
///    legitimate rejections that explain why a particular sub-feature
///    is unavailable, and silently dropping them would discard
///    user-visible diagnostic information.
///
/// To honor both meanings we only treat the result as `Ok` when *every*
/// error is the structure-validation sentinel. Once any non-sentinel
/// error is present (including a non-sentinel
/// `UnsupportedFeatureError` from case 2) we surface the result via the
/// existing `From<SimpleConsensusValidationResult> for Error`
/// conversion — which keeps the first error as a *typed*
/// `ConsensusError` so callers can pattern-match on it. To avoid the
/// conversion picking an `UnsupportedFeatureError` entry when a real
/// failure is also present, we first **reorder** (stable-sort) the error
/// list so the first non-`UnsupportedFeatureError` entry is primary;
/// every error — sentinel and non-sentinel `UnsupportedFeatureError`
/// alike — is preserved in the result.
fn map_validation_result(mut result: SimpleConsensusValidationResult) -> Result<(), Error> {
    if result.is_valid() {
        return Ok(());
    }

    // Pass-through only when *every* error is the DPP sentinel. A
    // non-sentinel `UnsupportedFeatureError` (case 2 above) is a real
    // rejection and must surface as an `Err`.
    if result.errors.iter().all(is_structure_validation_sentinel) {
        return Ok(());
    }

    // Mixed real-error / `UnsupportedFeatureError` case. The default
    // `From<SimpleConsensusValidationResult> for Error` conversion keeps
    // the *first* error as a typed `ConsensusError`. Stable-sort so
    // non-`UnsupportedFeatureError` failures come first, ensuring the
    // typed error returned is the most actionable one and not an
    // `UnsupportedFeatureError` entry. We deliberately use the existing
    // `From` conversion so the returned `Error` preserves the typed
    // `ConsensusError` variant for downstream pattern-matching, instead
    // of being flattened into a `ProtocolError::Generic` string. Note
    // this is a **reorder**, not a filter: every original error
    // (sentinel and non-sentinel `UnsupportedFeatureError` alike)
    // remains in the result.
    result.errors.sort_by_key(|e| {
        if is_any_unsupported_feature_error(e) {
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
/// * a result whose every error is the DPP structure-validation
///   sentinel (`UnsupportedFeatureError` with `feature_name` starting
///   with `"structure validation"`) is treated as `Ok` because DPP uses
///   that shape as a "structure validation is not implemented for this
///   state transition kind" placeholder (e.g. identity-based STs and
///   Batch). The platform will still perform validation during
///   execution.
/// * a result that mixes `UnsupportedFeatureError` with real errors —
///   or that contains a non-sentinel `UnsupportedFeatureError` flagging
///   a specific in-ST sub-feature as unsupported on this platform
///   version — is surfaced as an `Err` via the existing
///   `From<SimpleConsensusValidationResult> for Error` conversion, with
///   real failures reordered first so the returned typed
///   `ConsensusError` is the actionable one. Every original error is
///   preserved in the result; nothing is dropped.
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

    /// Non-sentinel `UnsupportedFeatureError` (case 2 in the
    /// [`map_validation_result`] docs): the `feature_name` does not
    /// match the DPP structure-validation sentinel prefix, so this
    /// represents a real rejection of a specific in-ST sub-feature on
    /// the current platform version and must surface as `Err`.
    fn unsupported_error() -> ConsensusError {
        ConsensusError::BasicError(BasicError::UnsupportedFeatureError(
            UnsupportedFeatureError::new("token-config-update sub-feature X".to_string(), 0),
        ))
    }

    /// DPP root `validate_structure` sentinel — an
    /// `UnsupportedFeatureError` whose `feature_name` starts with
    /// `"structure validation"` (e.g. identity-based / Batch STs). Pass
    /// through as a no-op so prepare APIs can sign and broadcast these
    /// STs even though their structure check is a stub.
    fn sentinel_unsupported_error() -> ConsensusError {
        ConsensusError::BasicError(BasicError::UnsupportedFeatureError(
            UnsupportedFeatureError::new(
                "structure validation for identity-based state transitions".to_string(),
                0,
            ),
        ))
    }

    fn value_error(msg: &str) -> ConsensusError {
        ConsensusError::BasicError(BasicError::ValueError(ValueError::new_from_string(
            msg.to_string(),
        )))
    }

    /// When every error is the DPP structure-validation sentinel we
    /// treat the validation result as a no-op and return Ok. This is
    /// the pass-through that lets identity-based STs and Batch sign /
    /// broadcast.
    #[test]
    fn all_sentinel_unsupported_errors_are_treated_as_ok() {
        let result = SimpleConsensusValidationResult::new_with_errors(vec![
            sentinel_unsupported_error(),
            sentinel_unsupported_error(),
        ]);
        assert!(map_validation_result(result).is_ok());
    }

    /// A non-sentinel `UnsupportedFeatureError` (e.g. an in-ST
    /// sub-feature unsupported on this platform version) must surface
    /// as `Err` and never as the sentinel pass-through. Silently
    /// dropping it would discard a real user-visible rejection.
    #[test]
    fn non_sentinel_unsupported_errors_surface_as_err() {
        let result = SimpleConsensusValidationResult::new_with_errors(vec![unsupported_error()]);
        let err = map_validation_result(result)
            .expect_err("non-sentinel UnsupportedFeatureError must not be passed through");
        match err {
            Error::Protocol(dpp::ProtocolError::ConsensusError(boxed)) => match *boxed {
                ConsensusError::BasicError(BasicError::UnsupportedFeatureError(_)) => {}
                other => panic!("expected UnsupportedFeatureError typed variant, got: {other:?}"),
            },
            other => panic!("expected typed ConsensusError, got: {other:?}"),
        }
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

    /// A result that mixes the DPP structure-validation sentinel with
    /// a real failure must surface as `Err` — the sentinel pass-through
    /// only applies when *every* error is the sentinel. The reordering
    /// also ensures the typed `ConsensusError` returned is the real
    /// failure, not the sentinel placeholder.
    #[test]
    fn sentinel_plus_real_error_is_surfaced_as_err() {
        let result = SimpleConsensusValidationResult::new_with_errors(vec![
            sentinel_unsupported_error(),
            value_error("real failure"),
        ]);
        let err = map_validation_result(result)
            .expect_err("sentinel + real failure must not be passed through");
        match err {
            Error::Protocol(dpp::ProtocolError::ConsensusError(boxed)) => match *boxed {
                ConsensusError::BasicError(BasicError::ValueError(ref ve)) => {
                    assert!(
                        ve.to_string().contains("real failure"),
                        "expected real-failure primary typed error, got: {ve}"
                    );
                }
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
