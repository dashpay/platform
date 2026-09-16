use crate::consensus::basic::token::{InvalidTokenAmountError, InvalidTokenNoteTooBigError, TokenNoteOnlyAllowedWhenProposerError};
use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::data_contract::associated_token::token_perpetual_distribution::distribution_function::MAX_DISTRIBUTION_PARAM;
use crate::state_transition::batch_transition::token_base_transition::token_base_transition_accessors::TokenBaseTransitionAccessors;
use crate::state_transition::batch_transition::token_base_transition::v0::v0_methods::TokenBaseTransitionV0Methods;
use crate::state_transition::batch_transition::token_burn_from_pool_transition::v0::v0_methods::TokenBurnFromPoolTransitionV0Methods;
use crate::state_transition::batch_transition::TokenBurnFromPoolTransition;
use crate::state_transition::state_transitions::shielded::common_validation::{
    validate_actions_count, validate_anchor_not_zero, validate_encrypted_note_sizes,
    validate_proof_not_empty,
};
use crate::tokens::MAX_TOKEN_NOTE_LEN;
use crate::validation::SimpleConsensusValidationResult;
use crate::ProtocolError;
use platform_version::version::PlatformVersion;

pub(super) trait TokenBurnFromPoolTransitionActionStructureValidationV0 {
    fn validate_structure_v0(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, ProtocolError>;
}

impl TokenBurnFromPoolTransitionActionStructureValidationV0 for TokenBurnFromPoolTransition {
    /// The bounds match the equivalent transparent token transition; the bundle checks are the
    /// same stateless checks the other pool transitions run (action count, note sizes,
    /// non-empty proof, non-zero anchor). The proof itself is verified against state, where its
    /// cost can be charged.
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

        if let Some(public_note) = self.public_note() {
            if public_note.len() > MAX_TOKEN_NOTE_LEN {
                return Ok(SimpleConsensusValidationResult::new_with_error(
                    ConsensusError::BasicError(BasicError::InvalidTokenNoteTooBigError(
                        InvalidTokenNoteTooBigError::new(
                            MAX_TOKEN_NOTE_LEN as u32,
                            "public_note",
                            public_note.len() as u32,
                        ),
                    )),
                ));
            }
            if let Some(group_state_transition_info) = self.base().using_group_info() {
                if !group_state_transition_info.action_is_proposer {
                    return Ok(SimpleConsensusValidationResult::new_with_error(
                        ConsensusError::BasicError(
                            BasicError::TokenNoteOnlyAllowedWhenProposerError(
                                TokenNoteOnlyAllowedWhenProposerError::new(),
                            ),
                        ),
                    ));
                }
            }
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
    use crate::state_transition::batch_transition::token_burn_from_pool_transition::v0::TokenBurnFromPoolTransitionV0;
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

    fn make_transition(amount: u64) -> TokenBurnFromPoolTransition {
        TokenBurnFromPoolTransition::V0(TokenBurnFromPoolTransitionV0 {
            base: TokenBaseTransition::V0(TokenBaseTransitionV0 {
                identity_contract_nonce: 1,
                token_contract_position: 0,
                data_contract_id: Identifier::default(),
                token_id: Identifier::default(),
                using_group_info: None,
            }),
            amount,
            actions: vec![action()],
            anchor: [9u8; 32],
            proof: vec![1u8; 10],
            binding_signature: [7u8; 64],
            public_note: None,
        })
    }

    #[test]
    fn valid_transition_passes() {
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
    fn empty_actions_are_rejected() {
        let mut transition = make_transition(100);
        let TokenBurnFromPoolTransition::V0(v0) = &mut transition;
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
    fn empty_proof_is_rejected() {
        let mut transition = make_transition(100);
        let TokenBurnFromPoolTransition::V0(v0) = &mut transition;
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
}
