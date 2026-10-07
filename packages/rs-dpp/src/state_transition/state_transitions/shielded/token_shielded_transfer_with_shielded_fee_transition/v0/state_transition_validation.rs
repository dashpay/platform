use crate::consensus::basic::state_transition::ShieldedInvalidValueBalanceError;
use crate::consensus::basic::token::InvalidTokenIdError;
use crate::consensus::basic::BasicError;
use crate::state_transition::state_transitions::shielded::common_validation::{
    validate_actions_count, validate_anchor_not_zero, validate_encrypted_note_sizes,
    validate_proof_not_empty,
};
use crate::state_transition::token_shielded_transfer_with_shielded_fee_transition::v0::TokenShieldedTransferWithShieldedFeeTransitionV0;
use crate::state_transition::StateTransitionStructureValidation;
use crate::tokens::calculate_token_id;
use crate::validation::SimpleConsensusValidationResult;
use platform_value::Identifier;
use platform_version::version::PlatformVersion;

impl StateTransitionStructureValidation for TokenShieldedTransferWithShieldedFeeTransitionV0 {
    /// The token id must derive from the contract and position, the amounts must be within
    /// bounds and both bundles pass the stateless checks every shielded bundle passes (action
    /// count, note sizes, non-empty proof, non-zero anchor). The proofs are verified by the
    /// processor once the fee floor has been checked.
    fn validate_structure(
        &self,
        platform_version: &PlatformVersion,
    ) -> SimpleConsensusValidationResult {
        let calculated_token_id: Identifier = calculate_token_id(
            self.data_contract_id.as_bytes(),
            self.token_contract_position,
        )
        .into();
        if self.token_id != calculated_token_id {
            return SimpleConsensusValidationResult::new_with_error(
                BasicError::InvalidTokenIdError(InvalidTokenIdError::new(
                    calculated_token_id,
                    self.token_id,
                ))
                .into(),
            );
        }
        if self.credit_amount == 0 {
            return SimpleConsensusValidationResult::new_with_error(
                BasicError::ShieldedInvalidValueBalanceError(
                    ShieldedInvalidValueBalanceError::new(
                        "the credits leaving the pool must be greater than zero".to_string(),
                    ),
                )
                .into(),
            );
        }
        if self.credit_amount > i64::MAX as u64 {
            return SimpleConsensusValidationResult::new_with_error(
                BasicError::ShieldedInvalidValueBalanceError(
                    ShieldedInvalidValueBalanceError::new(
                        "the credits leaving the pool exceed the maximum allowed value".to_string(),
                    ),
                )
                .into(),
            );
        }
        let max_actions = platform_version
            .system_limits
            .max_shielded_transition_actions;
        for (actions, proof, anchor) in [
            (&self.token_actions, &self.token_proof, &self.token_anchor),
            (&self.fee_actions, &self.fee_proof, &self.fee_anchor),
        ] {
            let result = validate_actions_count(actions, max_actions);
            if !result.is_valid() {
                return result;
            }
            let result = validate_encrypted_note_sizes(actions);
            if !result.is_valid() {
                return result;
            }
            let result = validate_proof_not_empty(proof);
            if !result.is_valid() {
                return result;
            }
            let result = validate_anchor_not_zero(anchor);
            if !result.is_valid() {
                return result;
            }
        }
        SimpleConsensusValidationResult::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::consensus::ConsensusError;
    use crate::shielded::SerializedAction;
    use assert_matches::assert_matches;

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

    fn valid() -> TokenShieldedTransferWithShieldedFeeTransitionV0 {
        TokenShieldedTransferWithShieldedFeeTransitionV0 {
            data_contract_id: Identifier::from([1u8; 32]),
            token_contract_position: 0,
            token_id: Identifier::from(calculate_token_id(&[1u8; 32], 0)),
            token_actions: vec![action()],
            token_anchor: [7u8; 32],
            token_proof: vec![8u8; 100],
            token_binding_signature: [9u8; 64],
            fee_actions: vec![action()],
            fee_anchor: [7u8; 32],
            fee_proof: vec![8u8; 100],
            fee_binding_signature: [9u8; 64],
            credit_amount: 2_000,
        }
    }

    #[test]
    fn should_accept_a_well_formed_transition() {
        let result = valid().validate_structure(PlatformVersion::latest());
        assert!(result.is_valid(), "{:?}", result.errors);
    }

    #[test]
    fn should_reject_a_wrong_token_id() {
        let mut t = valid();
        t.token_id = Identifier::from([3u8; 32]);
        let result = t.validate_structure(PlatformVersion::latest());
        assert_matches!(
            result.errors.as_slice(),
            [ConsensusError::BasicError(BasicError::InvalidTokenIdError(
                _
            ))]
        );
    }

    #[test]
    fn should_reject_a_zero_credit_amount() {
        let mut t = valid();
        t.credit_amount = 0;
        let result = t.validate_structure(PlatformVersion::latest());
        assert_matches!(
            result.errors.as_slice(),
            [ConsensusError::BasicError(
                BasicError::ShieldedInvalidValueBalanceError(_)
            )]
        );
    }

    #[test]
    fn should_reject_a_fee_bundle_without_actions_or_proof() {
        let mut t = valid();
        t.fee_actions = vec![];
        let result = t.validate_structure(PlatformVersion::latest());
        assert_matches!(
            result.errors.as_slice(),
            [ConsensusError::BasicError(
                BasicError::ShieldedNoActionsError(_)
            )]
        );
        let mut t = valid();
        t.fee_proof = vec![];
        let result = t.validate_structure(PlatformVersion::latest());
        assert_matches!(
            result.errors.as_slice(),
            [ConsensusError::BasicError(
                BasicError::ShieldedEmptyProofError(_)
            )]
        );
        let mut t = valid();
        t.token_anchor = [0u8; 32];
        let result = t.validate_structure(PlatformVersion::latest());
        assert_matches!(
            result.errors.as_slice(),
            [ConsensusError::BasicError(
                BasicError::ShieldedZeroAnchorError(_)
            )]
        );
    }
}
