use crate::consensus::basic::state_transition::{
    ShieldedEmptyProofError, ShieldedEncryptedNoteSizeMismatchError, ShieldedNoActionsError,
    ShieldedTooManyActionsError, ShieldedZeroAnchorError,
};
use crate::consensus::basic::BasicError;
use crate::shielded::SerializedAction;
use crate::validation::SimpleConsensusValidationResult;

/// Expected size of the encrypted_note field in each SerializedAction.
/// This is epk (32) + enc_ciphertext (104) + out_ciphertext (80) = 216 bytes.
/// Matches the ENCRYPTED_NOTE_SIZE constant in drive-abci's shielded_common module.
pub const ENCRYPTED_NOTE_SIZE: usize = 216;

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

/// Defense-in-depth: validate that every action's `encrypted_note` field is exactly
/// `ENCRYPTED_NOTE_SIZE` (216) bytes. This rejects malformed data early at the DPP
/// layer before it reaches the ABCI bundle reconstruction, saving network bandwidth.
pub fn validate_encrypted_note_sizes(
    actions: &[SerializedAction],
) -> SimpleConsensusValidationResult {
    for action in actions {
        if action.encrypted_note.len() != ENCRYPTED_NOTE_SIZE {
            return SimpleConsensusValidationResult::new_with_error(
                BasicError::ShieldedEncryptedNoteSizeMismatchError(
                    ShieldedEncryptedNoteSizeMismatchError::new(
                        ENCRYPTED_NOTE_SIZE as u32,
                        action.encrypted_note.len() as u32,
                    ),
                )
                .into(),
            );
        }
    }
    SimpleConsensusValidationResult::new()
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

    // --- validate_encrypted_note_sizes ---

    #[test]
    fn validate_encrypted_note_sizes_should_accept_correct_size() {
        let actions = vec![dummy_action()];
        let result = validate_encrypted_note_sizes(&actions);
        assert!(
            result.is_valid(),
            "Expected valid, got: {:?}",
            result.errors
        );
    }

    #[test]
    fn validate_encrypted_note_sizes_should_accept_multiple_correct_actions() {
        let actions = vec![dummy_action(); 3];
        let result = validate_encrypted_note_sizes(&actions);
        assert!(
            result.is_valid(),
            "Expected valid, got: {:?}",
            result.errors
        );
    }

    #[test]
    fn validate_encrypted_note_sizes_should_reject_too_short() {
        let mut action = dummy_action();
        action.encrypted_note = vec![4u8; 100]; // Too short
        let actions = vec![action];
        let result = validate_encrypted_note_sizes(&actions);
        assert_matches!(
            result.errors.as_slice(),
            [ConsensusError::BasicError(
                BasicError::ShieldedEncryptedNoteSizeMismatchError(_)
            )]
        );
    }

    #[test]
    fn validate_encrypted_note_sizes_should_reject_too_long() {
        let mut action = dummy_action();
        action.encrypted_note = vec![4u8; 300]; // Too long
        let actions = vec![action];
        let result = validate_encrypted_note_sizes(&actions);
        assert_matches!(
            result.errors.as_slice(),
            [ConsensusError::BasicError(
                BasicError::ShieldedEncryptedNoteSizeMismatchError(_)
            )]
        );
    }

    #[test]
    fn validate_encrypted_note_sizes_should_reject_empty() {
        let mut action = dummy_action();
        action.encrypted_note = vec![]; // Empty
        let actions = vec![action];
        let result = validate_encrypted_note_sizes(&actions);
        assert_matches!(
            result.errors.as_slice(),
            [ConsensusError::BasicError(
                BasicError::ShieldedEncryptedNoteSizeMismatchError(_)
            )]
        );
    }

    #[test]
    fn validate_encrypted_note_sizes_should_reject_second_invalid_action() {
        let good_action = dummy_action();
        let mut bad_action = dummy_action();
        bad_action.encrypted_note = vec![4u8; 100];
        let actions = vec![good_action, bad_action];
        let result = validate_encrypted_note_sizes(&actions);
        assert_matches!(
            result.errors.as_slice(),
            [ConsensusError::BasicError(
                BasicError::ShieldedEncryptedNoteSizeMismatchError(_)
            )]
        );
    }

    #[test]
    fn validate_encrypted_note_sizes_should_accept_empty_actions_list() {
        let result = validate_encrypted_note_sizes(&[]);
        assert!(
            result.is_valid(),
            "Expected valid for empty actions list, got: {:?}",
            result.errors
        );
    }
}
