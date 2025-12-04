use crate::Error;
use dpp::{
    state_transition::{StateTransition, StateTransitionStructureValidation},
    version::PlatformVersion,
};

/// Ensures a state transition passes structure validation before broadcasting.
pub(crate) fn ensure_valid_state_transition_structure(
    state_transition: &StateTransition,
    platform_version: &PlatformVersion,
) -> Result<(), Error> {
    let validation_result = state_transition.validate_structure(platform_version);
    if validation_result.is_valid() {
        Ok(())
    } else {
        Err(validation_result.into())
    }
}
