use crate::consensus::basic::token::InvalidTokenNoteTooBigError;
use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::state_transition::batch_transition::batched_transition::validation::validate_public_note;
use crate::state_transition::batch_transition::token_unfreeze_transition::v0::v0_methods::TokenUnfreezeTransitionV0Methods;
use crate::state_transition::batch_transition::TokenUnfreezeTransition;
use crate::tokens::MAX_TOKEN_NOTE_LEN;
use crate::validation::SimpleConsensusValidationResult;
use crate::ProtocolError;

pub(super) trait TokenUnfreezeTransitionStructureValidationV0 {
    fn validate_structure_v0(&self) -> Result<SimpleConsensusValidationResult, ProtocolError>;
}
impl TokenUnfreezeTransitionStructureValidationV0 for TokenUnfreezeTransition {
    fn validate_structure_v0(&self) -> Result<SimpleConsensusValidationResult, ProtocolError> {
        let mut result = SimpleConsensusValidationResult::new();

        if let Some(public_note) = self.public_note() {
            result.merge(validate_public_note(public_note));
        }

        Ok(result)
    }
}
