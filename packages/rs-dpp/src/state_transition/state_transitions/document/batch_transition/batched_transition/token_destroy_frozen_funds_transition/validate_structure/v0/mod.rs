use crate::ProtocolError;
use crate::state_transition::batch_transition::TokenDestroyFrozenFundsTransition;
use crate::validation::SimpleConsensusValidationResult;
use crate::consensus::basic::BasicError;
use crate::consensus::basic::token::InvalidTokenNoteTooBigError;
use crate::consensus::ConsensusError;
use crate::state_transition::batch_transition::batched_transition::validation::validate_public_note;
use crate::state_transition::batch_transition::token_destroy_frozen_funds_transition::v0::v0_methods::TokenDestroyFrozenFundsTransitionV0Methods;
use crate::tokens::MAX_TOKEN_NOTE_LEN;

pub(super) trait TokenDestroyFrozenFundsTransitionStructureValidationV0 {
    fn validate_structure_v0(&self) -> Result<SimpleConsensusValidationResult, ProtocolError>;
}
impl TokenDestroyFrozenFundsTransitionStructureValidationV0 for TokenDestroyFrozenFundsTransition {
    fn validate_structure_v0(&self) -> Result<SimpleConsensusValidationResult, ProtocolError> {
        let mut result = SimpleConsensusValidationResult::new();

        if let Some(public_note) = self.public_note() {
            result.merge(validate_public_note(public_note));
        }

        Ok(result)
    }
}
