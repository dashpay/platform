use crate::consensus::basic::identity::{
    InvalidCreditWithdrawalTransitionCoreFeeError,
    InvalidCreditWithdrawalTransitionOutputScriptError,
    NotImplementedCreditWithdrawalTransitionPoolingError,
};
use crate::consensus::basic::state_transition::ShieldedInvalidValueBalanceError;
use crate::consensus::basic::BasicError;
use crate::state_transition::identity_credit_withdrawal_transition::MIN_CORE_FEE_PER_BYTE;
use crate::state_transition::shielded_withdrawal_transition::v0::ShieldedWithdrawalTransitionV0;
use crate::state_transition::state_transitions::shielded::common_validation::{
    validate_actions_count, validate_anchor_not_zero, validate_encrypted_note_sizes,
    validate_proof_not_empty,
};
use crate::state_transition::StateTransitionStructureValidation;
use crate::util::is_non_zero_fibonacci_number::is_non_zero_fibonacci_number;
use crate::validation::SimpleConsensusValidationResult;
use crate::withdrawal::Pooling;
use platform_version::version::PlatformVersion;

impl StateTransitionStructureValidation for ShieldedWithdrawalTransitionV0 {
    fn validate_structure(
        &self,
        platform_version: &PlatformVersion,
    ) -> SimpleConsensusValidationResult {
        // Actions count must be in [1, max]
        let result = validate_actions_count(
            &self.actions,
            platform_version
                .system_limits
                .max_shielded_transition_actions,
        );
        if !result.is_valid() {
            return result;
        }

        // Each action's encrypted_note must be exactly ENCRYPTED_NOTE_SIZE bytes
        let result = validate_encrypted_note_sizes(&self.actions);
        if !result.is_valid() {
            return result;
        }

        // unshielding_amount must be positive and within i64::MAX
        if self.unshielding_amount == 0 {
            return SimpleConsensusValidationResult::new_with_error(
                BasicError::ShieldedInvalidValueBalanceError(
                    ShieldedInvalidValueBalanceError::new(
                        "shielded withdrawal unshielding_amount must be positive".to_string(),
                    ),
                )
                .into(),
            );
        }

        if self.unshielding_amount > i64::MAX as u64 {
            return SimpleConsensusValidationResult::new_with_error(
                BasicError::ShieldedInvalidValueBalanceError(
                    ShieldedInvalidValueBalanceError::new(
                        "shielded withdrawal unshielding_amount exceeds maximum allowed value"
                            .to_string(),
                    ),
                )
                .into(),
            );
        }

        // Proof must not be empty
        let result = validate_proof_not_empty(&self.proof);
        if !result.is_valid() {
            return result;
        }

        // Anchor must not be all zeros
        let result = validate_anchor_not_zero(&self.anchor);
        if !result.is_valid() {
            return result;
        }

        // The shielded withdrawal carries the same transparent, Core-facing fields as
        // IdentityCreditWithdrawal (output_script, pooling, core_fee_per_byte), and the
        // transformer writes them straight into the queued withdrawal document that drives
        // the Core asset-unlock TxOut. Mirror the transparent path's invariants so a valid
        // Orchard proof cannot enqueue a non-standard/oversized output script (storage
        // griefing at the flat shielded fee), an unsupported pooling discriminant, or an
        // unrelayable/zero core_fee_per_byte.

        // Pooling is not yet supported: must be Never.
        if self.pooling != Pooling::Never {
            return SimpleConsensusValidationResult::new_with_error(
                BasicError::NotImplementedCreditWithdrawalTransitionPoolingError(
                    NotImplementedCreditWithdrawalTransitionPoolingError::new(self.pooling as u8),
                )
                .into(),
            );
        }

        // core_fee_per_byte must be a non-zero Fibonacci number.
        if !is_non_zero_fibonacci_number(self.core_fee_per_byte as u64) {
            return SimpleConsensusValidationResult::new_with_error(
                BasicError::InvalidCreditWithdrawalTransitionCoreFeeError(
                    InvalidCreditWithdrawalTransitionCoreFeeError::new(
                        self.core_fee_per_byte,
                        MIN_CORE_FEE_PER_BYTE,
                    ),
                )
                .into(),
            );
        }

        // output_script must be a canonical P2PKH or P2SH script.
        if !self.output_script.is_p2pkh() && !self.output_script.is_p2sh() {
            return SimpleConsensusValidationResult::new_with_error(
                BasicError::InvalidCreditWithdrawalTransitionOutputScriptError(
                    InvalidCreditWithdrawalTransitionOutputScriptError::new(
                        self.output_script.clone(),
                    ),
                )
                .into(),
            );
        }

        SimpleConsensusValidationResult::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::consensus::ConsensusError;
    use crate::identity::core_script::CoreScript;
    use crate::withdrawal::Pooling;
    use assert_matches::assert_matches;

    fn dummy_action() -> crate::shielded::SerializedAction {
        crate::shielded::SerializedAction {
            nullifier: [1u8; 32],
            rk: [2u8; 32],
            cmx: [3u8; 32],
            encrypted_note: vec![4u8; 216],
            cv_net: [5u8; 32],
            spend_auth_sig: [6u8; 64],
        }
    }

    fn valid_shielded_withdrawal_transition() -> ShieldedWithdrawalTransitionV0 {
        ShieldedWithdrawalTransitionV0 {
            actions: vec![dummy_action()],
            unshielding_amount: 1_000_000u64,
            anchor: [7u8; 32],
            proof: vec![8u8; 100],
            binding_signature: [9u8; 64],
            core_fee_per_byte: 1u32,
            pooling: Pooling::Never,
            output_script: CoreScript::new_p2pkh([11u8; 20]),
        }
    }

    #[test]
    fn should_validate_a_valid_transition() {
        let platform_version = PlatformVersion::latest();
        let transition = valid_shielded_withdrawal_transition();
        let result = transition.validate_structure(platform_version);
        assert!(
            result.is_valid(),
            "Expected valid result, got errors: {:?}",
            result.errors
        );
    }

    #[test]
    fn should_reject_invalid_encrypted_note_size() {
        let platform_version = PlatformVersion::latest();
        let mut transition = valid_shielded_withdrawal_transition();
        transition.actions[0].encrypted_note = vec![4u8; 100]; // Wrong size

        let result = transition.validate_structure(platform_version);
        assert_matches!(
            result.errors.as_slice(),
            [ConsensusError::BasicError(
                BasicError::ShieldedEncryptedNoteSizeMismatchError(_)
            )]
        );
    }

    #[test]
    fn should_reject_empty_actions() {
        let platform_version = PlatformVersion::latest();
        let mut transition = valid_shielded_withdrawal_transition();
        transition.actions.clear();

        let result = transition.validate_structure(platform_version);
        assert_matches!(
            result.errors.as_slice(),
            [ConsensusError::BasicError(
                BasicError::ShieldedNoActionsError(_)
            )]
        );
    }

    #[test]
    fn should_reject_too_many_actions() {
        let platform_version = PlatformVersion::latest();
        let max = platform_version
            .system_limits
            .max_shielded_transition_actions;
        let mut transition = valid_shielded_withdrawal_transition();
        transition.actions = vec![dummy_action(); max as usize + 1];

        let result = transition.validate_structure(platform_version);
        assert_matches!(
            result.errors.as_slice(),
            [ConsensusError::BasicError(
                BasicError::ShieldedTooManyActionsError(_)
            )]
        );
    }

    #[test]
    fn should_reject_zero_unshielding_amount() {
        let platform_version = PlatformVersion::latest();
        let mut transition = valid_shielded_withdrawal_transition();
        transition.unshielding_amount = 0;

        let result = transition.validate_structure(platform_version);
        assert_matches!(
            result.errors.as_slice(),
            [ConsensusError::BasicError(
                BasicError::ShieldedInvalidValueBalanceError(_)
            )]
        );
    }

    #[test]
    fn should_reject_unshielding_amount_exceeding_i64_max() {
        let platform_version = PlatformVersion::latest();
        let mut transition = valid_shielded_withdrawal_transition();
        transition.unshielding_amount = i64::MAX as u64 + 1;

        let result = transition.validate_structure(platform_version);
        assert_matches!(
            result.errors.as_slice(),
            [ConsensusError::BasicError(
                BasicError::ShieldedInvalidValueBalanceError(_)
            )]
        );
    }

    #[test]
    fn should_accept_unshielding_amount_at_i64_max() {
        let platform_version = PlatformVersion::latest();
        let mut transition = valid_shielded_withdrawal_transition();
        transition.unshielding_amount = i64::MAX as u64;

        let result = transition.validate_structure(platform_version);
        assert!(
            result.is_valid(),
            "Expected valid result, got errors: {:?}",
            result.errors
        );
    }

    #[test]
    fn should_reject_empty_proof() {
        let platform_version = PlatformVersion::latest();
        let mut transition = valid_shielded_withdrawal_transition();
        transition.proof.clear();

        let result = transition.validate_structure(platform_version);
        assert_matches!(
            result.errors.as_slice(),
            [ConsensusError::BasicError(
                BasicError::ShieldedEmptyProofError(_)
            )]
        );
    }

    #[test]
    fn should_reject_zero_anchor() {
        let platform_version = PlatformVersion::latest();
        let mut transition = valid_shielded_withdrawal_transition();
        transition.anchor = [0u8; 32];

        let result = transition.validate_structure(platform_version);
        assert_matches!(
            result.errors.as_slice(),
            [ConsensusError::BasicError(
                BasicError::ShieldedZeroAnchorError(_)
            )]
        );
    }

    #[test]
    fn should_reject_non_never_pooling() {
        let platform_version = PlatformVersion::latest();
        let mut transition = valid_shielded_withdrawal_transition();
        transition.pooling = Pooling::IfAvailable;

        let result = transition.validate_structure(platform_version);
        assert_matches!(
            result.errors.as_slice(),
            [ConsensusError::BasicError(
                BasicError::NotImplementedCreditWithdrawalTransitionPoolingError(_)
            )]
        );
    }

    #[test]
    fn should_reject_zero_core_fee_per_byte() {
        let platform_version = PlatformVersion::latest();
        let mut transition = valid_shielded_withdrawal_transition();
        transition.core_fee_per_byte = 0;

        let result = transition.validate_structure(platform_version);
        assert_matches!(
            result.errors.as_slice(),
            [ConsensusError::BasicError(
                BasicError::InvalidCreditWithdrawalTransitionCoreFeeError(_)
            )]
        );
    }

    #[test]
    fn should_reject_non_fibonacci_core_fee_per_byte() {
        let platform_version = PlatformVersion::latest();
        let mut transition = valid_shielded_withdrawal_transition();
        transition.core_fee_per_byte = 4; // 4 is not a Fibonacci number

        let result = transition.validate_structure(platform_version);
        assert_matches!(
            result.errors.as_slice(),
            [ConsensusError::BasicError(
                BasicError::InvalidCreditWithdrawalTransitionCoreFeeError(_)
            )]
        );
    }

    #[test]
    fn should_reject_non_standard_output_script() {
        let platform_version = PlatformVersion::latest();
        let mut transition = valid_shielded_withdrawal_transition();
        // OP_RETURN-style script: neither P2PKH nor P2SH.
        transition.output_script = CoreScript::from_bytes(vec![0x6a, 0x01, 0x02]);

        let result = transition.validate_structure(platform_version);
        assert_matches!(
            result.errors.as_slice(),
            [ConsensusError::BasicError(
                BasicError::InvalidCreditWithdrawalTransitionOutputScriptError(_)
            )]
        );
    }

    #[test]
    fn should_accept_p2sh_output_script() {
        let platform_version = PlatformVersion::latest();
        let mut transition = valid_shielded_withdrawal_transition();
        transition.output_script = CoreScript::new_p2sh([12u8; 20]);

        let result = transition.validate_structure(platform_version);
        assert!(
            result.is_valid(),
            "Expected valid result, got errors: {:?}",
            result.errors
        );
    }
}
