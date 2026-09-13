use crate::consensus::basic::state_transition::ShieldedInvalidValueBalanceError;
use crate::consensus::basic::BasicError;
use crate::state_transition::identity_top_up_from_shielded_pool_transition::v0::IdentityTopUpFromShieldedPoolTransitionV0;
use crate::state_transition::state_transitions::shielded::common_validation::{
    validate_actions_count, validate_anchor_not_zero, validate_encrypted_note_sizes,
    validate_proof_not_empty,
};
use crate::state_transition::StateTransitionStructureValidation;
use crate::validation::SimpleConsensusValidationResult;
use platform_version::version::PlatformVersion;

impl StateTransitionStructureValidation for IdentityTopUpFromShieldedPoolTransitionV0 {
    fn validate_structure(
        &self,
        platform_version: &PlatformVersion,
    ) -> SimpleConsensusValidationResult {
        let result = validate_actions_count(
            &self.actions,
            platform_version
                .system_limits
                .max_shielded_transition_actions,
        );
        if !result.is_valid() {
            return result;
        }

        let result = validate_encrypted_note_sizes(&self.actions);
        if !result.is_valid() {
            return result;
        }

        if self.top_up_amount == 0 {
            return SimpleConsensusValidationResult::new_with_error(
                BasicError::ShieldedInvalidValueBalanceError(
                    ShieldedInvalidValueBalanceError::new(
                        "identity top up amount must be greater than zero".to_string(),
                    ),
                )
                .into(),
            );
        }

        if self.top_up_amount > i64::MAX as u64 {
            return SimpleConsensusValidationResult::new_with_error(
                BasicError::ShieldedInvalidValueBalanceError(
                    ShieldedInvalidValueBalanceError::new(
                        "identity top up amount exceeds maximum allowed value".to_string(),
                    ),
                )
                .into(),
            );
        }

        let result = validate_proof_not_empty(&self.proof);
        if !result.is_valid() {
            return result;
        }

        validate_anchor_not_zero(&self.anchor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::consensus::ConsensusError;
    use crate::shielded::SerializedAction;
    use assert_matches::assert_matches;
    use platform_value::Identifier;

    fn action() -> SerializedAction {
        SerializedAction {
            nullifier: [1u8; 32],
            rk: [2u8; 32],
            cmx: [3u8; 32],
            encrypted_note: vec![4u8; 216],
            cv_net: [5u8; 32],
            spend_auth_sig: [6u8; 64],
        }
    }

    fn valid() -> IdentityTopUpFromShieldedPoolTransitionV0 {
        IdentityTopUpFromShieldedPoolTransitionV0 {
            identity_id: Identifier::from([1u8; 32]),
            actions: vec![action()],
            top_up_amount: 1_000,
            anchor: [7u8; 32],
            proof: vec![8u8; 100],
            binding_signature: [9u8; 64],
        }
    }

    #[test]
    fn should_accept_a_well_formed_transition() {
        let result = valid().validate_structure(PlatformVersion::latest());
        assert!(result.is_valid(), "{:?}", result.errors);
    }

    #[test]
    fn should_reject_no_actions() {
        let mut t = valid();
        t.actions = vec![];
        let result = t.validate_structure(PlatformVersion::latest());
        assert_matches!(
            result.errors.as_slice(),
            [ConsensusError::BasicError(
                BasicError::ShieldedNoActionsError(_)
            )]
        );
    }

    #[test]
    fn should_reject_zero_amount() {
        let mut t = valid();
        t.top_up_amount = 0;
        let result = t.validate_structure(PlatformVersion::latest());
        assert_matches!(
            result.errors.as_slice(),
            [ConsensusError::BasicError(
                BasicError::ShieldedInvalidValueBalanceError(_)
            )]
        );
    }

    #[test]
    fn should_reject_amount_above_i64_max() {
        let mut t = valid();
        t.top_up_amount = i64::MAX as u64 + 1;
        let result = t.validate_structure(PlatformVersion::latest());
        assert_matches!(
            result.errors.as_slice(),
            [ConsensusError::BasicError(
                BasicError::ShieldedInvalidValueBalanceError(_)
            )]
        );
    }

    #[test]
    fn should_reject_empty_proof_and_zero_anchor() {
        let mut t = valid();
        t.proof = vec![];
        let result = t.validate_structure(PlatformVersion::latest());
        assert_matches!(
            result.errors.as_slice(),
            [ConsensusError::BasicError(
                BasicError::ShieldedEmptyProofError(_)
            )]
        );
        let mut t = valid();
        t.anchor = [0u8; 32];
        let result = t.validate_structure(PlatformVersion::latest());
        assert_matches!(
            result.errors.as_slice(),
            [ConsensusError::BasicError(
                BasicError::ShieldedZeroAnchorError(_)
            )]
        );
    }
}
