use crate::consensus::basic::token::InvalidTokenNoteTooBigError;
use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::state_transition::batch_transition::batched_transition::validation::{
    validate_public_note, validate_token_amount_v0,
};
use crate::state_transition::batch_transition::token_burn_transition::v0::v0_methods::TokenBurnTransitionV0Methods;
use crate::state_transition::batch_transition::TokenBurnTransition;
use crate::tokens::MAX_TOKEN_NOTE_LEN;
use crate::validation::SimpleConsensusValidationResult;
use crate::ProtocolError;

pub(super) trait TokenBurnTransitionActionStructureValidationV0 {
    fn validate_structure_v0(&self) -> Result<SimpleConsensusValidationResult, ProtocolError>;
}

impl TokenBurnTransitionActionStructureValidationV0 for TokenBurnTransition {
    fn validate_structure_v0(&self) -> Result<SimpleConsensusValidationResult, ProtocolError> {
        let mut result = validate_token_amount_v0(self.burn_amount());

        if let Some(public_note) = self.public_note() {
            result.merge(validate_public_note(public_note));
        };

        Ok(result)
    }
}
