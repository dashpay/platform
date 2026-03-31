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
) -> SimpleConsensusValidationResult {
    if actions.is_empty() {
        SimpleConsensusValidationResult::new_with_error(
            BasicError::ShieldedNoActionsError(ShieldedNoActionsError::new()).into(),
        )
    } else if actions.len() > max_actions as usize {
        SimpleConsensusValidationResult::new_with_error(
            BasicError::ShieldedTooManyActionsError(ShieldedTooManyActionsError::new(
                actions.len().min(u16::MAX as usize) as u16,
                max_actions,
            ))
            .into(),
        )
    } else {
        SimpleConsensusValidationResult::new()
    }
}

/// Validate that the proof is not empty.
pub fn validate_proof_not_empty(proof: &[u8]) -> SimpleConsensusValidationResult {
    if proof.is_empty() {
        SimpleConsensusValidationResult::new_with_error(
            BasicError::ShieldedEmptyProofError(ShieldedEmptyProofError::new()).into(),
        )
    } else {
        SimpleConsensusValidationResult::new()
    }
}

/// Validate that the anchor is not all zeros (for transitions that consume notes).
pub fn validate_anchor_not_zero(anchor: &[u8; 32]) -> SimpleConsensusValidationResult {
    if *anchor == [0u8; 32] {
        SimpleConsensusValidationResult::new_with_error(
            BasicError::ShieldedZeroAnchorError(ShieldedZeroAnchorError::new()).into(),
        )
    } else {
        SimpleConsensusValidationResult::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::consensus::ConsensusError;
    use assert_matches::assert_matches;

    fn dummy_action() -> SerializedAction {
        SerializedAction {
            nullifier: [1u8; 32],
            rk: [2u8; 32],
            cmx: [3u8; 32],
            encrypted_note: vec![4u8; 216],
            cv_net: [5u8; 32],
            spend_auth_sig: [6u8; 64],
        }
    }

    // --- validate_actions_count ---

    #[test]
    fn validate_actions_count_should_reject_empty_actions() {
        let result = validate_actions_count(&[], 100);
        assert_matches!(
            result.errors.as_slice(),
            [ConsensusError::BasicError(
                BasicError::ShieldedNoActionsError(_)
            )]
        );
    }

    #[test]
    fn validate_actions_count_should_accept_single_action() {
        let actions = vec![dummy_action()];
        let result = validate_actions_count(&actions, 100);
        assert!(
            result.is_valid(),
            "Expected valid, got: {:?}",
            result.errors
        );
    }

    #[test]
    fn validate_actions_count_should_accept_exactly_max_actions() {
        let actions = vec![dummy_action(); 5];
        let result = validate_actions_count(&actions, 5);
        assert!(
            result.is_valid(),
            "Expected valid, got: {:?}",
            result.errors
        );
    }

    #[test]
    fn validate_actions_count_should_reject_more_than_max_actions() {
        let actions = vec![dummy_action(); 6];
        let result = validate_actions_count(&actions, 5);
        assert_matches!(
            result.errors.as_slice(),
            [ConsensusError::BasicError(
                BasicError::ShieldedTooManyActionsError(_)
            )]
        );
    }

    // --- validate_proof_not_empty ---

    #[test]
    fn validate_proof_not_empty_should_reject_empty_proof() {
        let result = validate_proof_not_empty(&[]);
        assert_matches!(
            result.errors.as_slice(),
            [ConsensusError::BasicError(
                BasicError::ShieldedEmptyProofError(_)
            )]
        );
    }

    #[test]
    fn validate_proof_not_empty_should_accept_non_empty_proof() {
        let result = validate_proof_not_empty(&[1u8; 100]);
        assert!(
            result.is_valid(),
            "Expected valid, got: {:?}",
            result.errors
        );
    }

    // --- validate_anchor_not_zero ---

    #[test]
    fn validate_anchor_not_zero_should_reject_all_zero_anchor() {
        let result = validate_anchor_not_zero(&[0u8; 32]);
        assert_matches!(
            result.errors.as_slice(),
            [ConsensusError::BasicError(
                BasicError::ShieldedZeroAnchorError(_)
            )]
        );
    }

    #[test]
    fn validate_anchor_not_zero_should_accept_non_zero_anchor() {
        let result = validate_anchor_not_zero(&[7u8; 32]);
        assert!(
            result.is_valid(),
            "Expected valid, got: {:?}",
            result.errors
        );
    }

    #[test]
    fn validate_anchor_not_zero_should_accept_single_bit_set() {
        let mut anchor = [0u8; 32];
        anchor[31] = 1;
        let result = validate_anchor_not_zero(&anchor);
        assert!(
            result.is_valid(),
            "Expected valid, got: {:?}",
            result.errors
        );
    }
}
