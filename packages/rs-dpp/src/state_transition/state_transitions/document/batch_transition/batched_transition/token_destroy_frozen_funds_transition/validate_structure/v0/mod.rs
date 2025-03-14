use crate::ProtocolError;
use crate::state_transition::state_transitions::document::batch_transition::batched_transition::token_destroy_frozen_funds_transition::TokenDestroyFrozenFundsTransition;
use crate::validation::SimpleConsensusValidationResult;
use crate::errors::consensus::basic::BasicError;
use crate::errors::consensus::basic::token::InvalidTokenNoteTooBigError;
use crate::errors::consensus::ConsensusError;
use crate::state_transition::state_transitions::document::batch_transition::batched_transition::token_destroy_frozen_funds_transition::v0::v0_methods::TokenDestroyFrozenFundsTransitionV0Methods;
use crate::tokens::MAX_TOKEN_NOTE_LEN;

pub(super) trait TokenDestroyFrozenFundsTransitionStructureValidationV0 {
    fn validate_structure_v0(&self) -> Result<SimpleConsensusValidationResult, ProtocolError>;
}
impl TokenDestroyFrozenFundsTransitionStructureValidationV0 for TokenDestroyFrozenFundsTransition {
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
