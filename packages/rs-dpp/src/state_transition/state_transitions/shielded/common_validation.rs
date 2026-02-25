use crate::consensus::basic::state_transition::{
    ShieldedEmptyProofError, ShieldedNoActionsError, ShieldedTooManyActionsError,
    ShieldedZeroAnchorError,
};
use crate::consensus::basic::BasicError;
use crate::shielded::SerializedAction;
use crate::validation::SimpleConsensusValidationResult;

/// Validate that the actions list is not empty and does not exceed the maximum.
pub fn validate_actions_count(
    actions: &[SerializedAction],
    max_actions: u16,
) -> Option<SimpleConsensusValidationResult> {
    if actions.is_empty() {
        Some(SimpleConsensusValidationResult::new_with_error(
            BasicError::ShieldedNoActionsError(ShieldedNoActionsError::new()).into(),
        ))
    } else if actions.len() > max_actions as usize {
        Some(SimpleConsensusValidationResult::new_with_error(
            BasicError::ShieldedTooManyActionsError(ShieldedTooManyActionsError::new(
                actions.len().min(u16::MAX as usize) as u16,
                max_actions,
            ))
            .into(),
        ))
    } else {
        None
    }
}

/// Validate that the proof is not empty.
pub fn validate_proof_not_empty(proof: &[u8]) -> Option<SimpleConsensusValidationResult> {
    if proof.is_empty() {
        Some(SimpleConsensusValidationResult::new_with_error(
            BasicError::ShieldedEmptyProofError(ShieldedEmptyProofError::new()).into(),
        ))
    } else {
        None
    }
}

/// Validate that the anchor is not all zeros (for transitions that consume notes).
pub fn validate_anchor_not_zero(anchor: &[u8; 32]) -> Option<SimpleConsensusValidationResult> {
    if *anchor == [0u8; 32] {
        Some(SimpleConsensusValidationResult::new_with_error(
            BasicError::ShieldedZeroAnchorError(ShieldedZeroAnchorError::new()).into(),
        ))
    } else {
        None
    }
}
