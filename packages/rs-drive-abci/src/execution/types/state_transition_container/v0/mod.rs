use dpp::consensus::ConsensusError;
use dpp::state_transition::StateTransition;
use dpp::validation::ConsensusValidationResult;
use crate::execution::types::state_transition_aware_error::v0::StateTransitionAwareErrorV0;

/// This is a container that holds state transitions

pub struct StateTransitionContainerV0<'a> {
    /// The asset lock state transitions
    state_transitions: Vec<StateTransition>,
    /// Deserialization errors
    consensus_errors: Vec<StateTransitionAwareErrorV0<'a>>,
}

impl<'a> FromIterator<Result<ConsensusValidationResult<StateTransition>, StateTransitionAwareErrorV0<'a>>> for StateTransitionContainerV0<'a> {
    fn from_iter<I: IntoIterator<Item = Result<ConsensusValidationResult<StateTransition>, StateTransitionAwareErrorV0<'a>>>>(iter: I) -> Self {
        let mut asset_lock_state_transitions = Vec::new();
        let mut signed_state_transitions = Vec::new();
        let mut consensus_errors = Vec::new();

        for item in iter {
            match item {
                Ok(result) => {
                    if result.is_success() {
                        // Assuming all successful results go into signed_state_transitions for the sake of example
                        // Adjust this logic based on your requirements
                        signed_state_transitions.push(result.get_state_transition().unwrap().clone());
                    } else {
                        // Handle consensus error
                        consensus_errors.push(result.get_consensus_error().unwrap().clone());
                    }
                }
                Err(e) => {
                    // Handle deserialization error or other types of errors
                    consensus_errors.push(ConsensusError::from(e)); // Assuming you have a way to convert errors
                }
            }
        }

        StateTransitionContainerV0 {
            asset_lock_state_transitions,
            signed_state_transitions,
            consensus_errors,
        }
    }
}