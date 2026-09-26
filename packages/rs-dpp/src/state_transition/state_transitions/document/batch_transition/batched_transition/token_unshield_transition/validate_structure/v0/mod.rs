use crate::consensus::basic::token::InvalidTokenAmountError;
use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::data_contract::associated_token::token_perpetual_distribution::distribution_function::MAX_DISTRIBUTION_PARAM;
use crate::state_transition::batch_transition::token_unshield_transition::v0::v0_methods::TokenUnshieldTransitionV0Methods;
use crate::state_transition::batch_transition::TokenUnshieldTransition;
use crate::state_transition::state_transitions::shielded::common_validation::{
    validate_actions_count, validate_anchor_not_zero, validate_encrypted_note_sizes,
    validate_proof_not_empty,
};
use crate::validation::SimpleConsensusValidationResult;
use crate::ProtocolError;
use platform_version::version::PlatformVersion;

pub(super) trait TokenUnshieldTransitionActionStructureValidationV0 {
    fn validate_structure_v0(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, ProtocolError>;
}

impl TokenUnshieldTransitionActionStructureValidationV0 for TokenUnshieldTransition {
    /// The amount bounds match the other token transitions; the bundle checks are the same
    /// stateless checks the credit-pool transitions run (action count, note sizes, non-empty
    /// proof, non-zero anchor). The proof itself is verified against state, where its cost
    /// can be charged.
    fn validate_structure_v0(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, ProtocolError> {
        if self.amount() > MAX_DISTRIBUTION_PARAM || self.amount() == 0 {
            return Ok(SimpleConsensusValidationResult::new_with_error(
                ConsensusError::BasicError(BasicError::InvalidTokenAmountError(
                    InvalidTokenAmountError::new(MAX_DISTRIBUTION_PARAM, self.amount()),
                )),
            ));
        }

        let result = validate_actions_count(
            self.actions(),
            platform_version
                .system_limits
                .max_shielded_transition_actions,
        );
        if !result.is_valid() {
            return Ok(result);
        }

        let result = validate_encrypted_note_sizes(self.actions());
        if !result.is_valid() {
            return Ok(result);
        }

        let result = validate_proof_not_empty(self.proof());
        if !result.is_valid() {
            return Ok(result);
        }

        Ok(validate_anchor_not_zero(self.anchor()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shielded::SerializedAction;
    use crate::state_transition::batch_transition::token_base_transition::v0::TokenBaseTransitionV0;
    use crate::state_transition::batch_transition::token_base_transition::TokenBaseTransition;
    use crate::state_transition::batch_transition::token_unshield_transition::v0::TokenUnshieldTransitionV0;
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

    fn make_transition(amount: u64) -> TokenUnshieldTransition {
        TokenUnshieldTransition::V0(TokenUnshieldTransitionV0 {
            base: TokenBaseTransition::V0(TokenBaseTransitionV0 {
                identity_contract_nonce: 1,
                token_contract_position: 0,
                data_contract_id: Identifier::default(),
                token_id: Identifier::default(),
                using_group_info: None,
            }),
            amount,
            recipient_id: Identifier::new([2u8; 32]),
            actions: vec![action()],
            anchor: [9u8; 32],
            proof: vec![1u8; 10],
            binding_signature: [7u8; 64],
        })
    }

    #[test]
    fn valid_unshield_passes() {
        let result = make_transition(100)
            .validate_structure_v0(PlatformVersion::latest())
            .unwrap();
        assert!(result.is_valid(), "{:?}", result.errors);
    }

    #[test]
    fn zero_amount_is_rejected() {
        let result = make_transition(0)
            .validate_structure_v0(PlatformVersion::latest())
            .unwrap();
        assert!(matches!(
            result.errors.first(),
            Some(ConsensusError::BasicError(
                BasicError::InvalidTokenAmountError(_)
            ))
        ));
    }

    #[test]
    fn amount_above_max_is_rejected() {
        let result = make_transition(MAX_DISTRIBUTION_PARAM + 1)
            .validate_structure_v0(PlatformVersion::latest())
            .unwrap();
        assert!(matches!(
            result.errors.first(),
            Some(ConsensusError::BasicError(
                BasicError::InvalidTokenAmountError(_)
            ))
        ));
    }

    #[test]
    fn empty_actions_are_rejected() {
        let mut transition = make_transition(100);
        let TokenUnshieldTransition::V0(v0) = &mut transition;
        v0.actions.clear();
        let result = transition
            .validate_structure_v0(PlatformVersion::latest())
            .unwrap();
        assert!(matches!(
            result.errors.first(),
            Some(ConsensusError::BasicError(
                BasicError::ShieldedNoActionsError(_)
            ))
        ));
    }

    #[test]
    fn wrong_note_size_is_rejected() {
        let mut transition = make_transition(100);
        let TokenUnshieldTransition::V0(v0) = &mut transition;
        v0.actions[0].encrypted_note = vec![0u8; 10];
        let result = transition
            .validate_structure_v0(PlatformVersion::latest())
            .unwrap();
        assert!(matches!(
            result.errors.first(),
            Some(ConsensusError::BasicError(
                BasicError::ShieldedEncryptedNoteSizeMismatchError(_)
            ))
        ));
    }

    #[test]
    fn empty_proof_is_rejected() {
        let mut transition = make_transition(100);
        let TokenUnshieldTransition::V0(v0) = &mut transition;
        v0.proof.clear();
        let result = transition
            .validate_structure_v0(PlatformVersion::latest())
            .unwrap();
        assert!(matches!(
            result.errors.first(),
            Some(ConsensusError::BasicError(
                BasicError::ShieldedEmptyProofError(_)
            ))
        ));
    }

    #[test]
    fn zero_anchor_is_rejected() {
        let mut transition = make_transition(100);
        let TokenUnshieldTransition::V0(v0) = &mut transition;
        v0.anchor = [0u8; 32];
        let result = transition
            .validate_structure_v0(PlatformVersion::latest())
            .unwrap();
        assert!(matches!(
            result.errors.first(),
            Some(ConsensusError::BasicError(
                BasicError::ShieldedZeroAnchorError(_)
            ))
        ));
    }
}
