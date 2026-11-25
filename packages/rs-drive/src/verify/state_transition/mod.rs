/// Provides functions to verify that a state transition was executed using a cryptographic proof.
pub mod verify_state_transition_was_executed_with_proof;

/// Defines path queries needed to verify state transitions based on their type and expected result.
pub mod state_transition_execution_path_queries;

/// Provides functions to verify the state a state transition's keys held before it executed.
pub mod verify_state_transition_prior_state_with_proof;
