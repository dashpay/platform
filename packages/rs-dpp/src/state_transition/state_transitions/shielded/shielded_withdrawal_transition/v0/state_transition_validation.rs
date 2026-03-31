use crate::consensus::basic::state_transition::ShieldedInvalidValueBalanceError;
use crate::consensus::basic::BasicError;
use crate::state_transition::shielded_withdrawal_transition::v0::ShieldedWithdrawalTransitionV0;
use crate::state_transition::state_transitions::shielded::common_validation::{
    validate_actions_count, validate_anchor_not_zero, validate_proof_not_empty,
};
use crate::state_transition::StateTransitionStructureValidation;
use crate::validation::SimpleConsensusValidationResult;
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
}
