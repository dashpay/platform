use crate::consensus::basic::token::InvalidTokenNoteTooBigError;
use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::state_transition::batch_transition::token_claim_transition::v0::v0_methods::TokenClaimTransitionV0Methods;
use crate::state_transition::batch_transition::TokenClaimTransition;
use crate::tokens::MAX_TOKEN_NOTE_LEN;
use crate::validation::SimpleConsensusValidationResult;
use crate::ProtocolError;

pub(super) trait TokenClaimTransitionActionStructureValidationV0 {
    fn validate_structure_v0(&self) -> Result<SimpleConsensusValidationResult, ProtocolError>;
}
impl TokenClaimTransitionActionStructureValidationV0 for TokenClaimTransition {
    fn validate_structure_v0(&self) -> Result<SimpleConsensusValidationResult, ProtocolError> {
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
        }
        Ok(SimpleConsensusValidationResult::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data_contract::associated_token::token_distribution_key::TokenDistributionType;
    use crate::state_transition::batch_transition::token_base_transition::v0::TokenBaseTransitionV0;
    use crate::state_transition::batch_transition::token_base_transition::TokenBaseTransition;
    use crate::state_transition::batch_transition::token_claim_transition::v0::TokenClaimTransitionV0;
    use platform_value::Identifier;

    fn make_transition(public_note: Option<String>) -> TokenClaimTransition {
        TokenClaimTransition::V0(TokenClaimTransitionV0 {
            base: TokenBaseTransition::V0(TokenBaseTransitionV0 {
                identity_contract_nonce: 1,
                token_contract_position: 0,
                data_contract_id: Identifier::default(),
                token_id: Identifier::default(),
                using_group_info: None,
            }),
            distribution_type: TokenDistributionType::PreProgrammed,
            public_note,
        })
    }

    #[test]
    fn should_pass_with_no_public_note() {
        let transition = make_transition(None);
        let result = transition.validate_structure_v0().unwrap();
        assert!(result.is_valid());
    }

    #[test]
    fn should_pass_with_short_public_note() {
        let transition = make_transition(Some("hello".to_string()));
        let result = transition.validate_structure_v0().unwrap();
        assert!(result.is_valid());
    }

    #[test]
    fn should_pass_with_note_at_max_length() {
        let note = "a".repeat(MAX_TOKEN_NOTE_LEN);
        let transition = make_transition(Some(note));
        let result = transition.validate_structure_v0().unwrap();
        assert!(result.is_valid());
    }

    #[test]
    fn should_return_error_when_public_note_too_big() {
        let long_note = "a".repeat(MAX_TOKEN_NOTE_LEN + 1);
        let transition = make_transition(Some(long_note));
        let result = transition.validate_structure_v0().unwrap();
        assert!(!result.is_valid());
        let error = result.errors.first().unwrap();
        assert!(matches!(
            error,
            ConsensusError::BasicError(BasicError::InvalidTokenNoteTooBigError(_))
        ));
    }
}
