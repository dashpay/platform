use crate::consensus::basic::token::InvalidTokenNoteTooBigError;
use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::state_transition::batch_transition::batched_transition::validation::{
    validate_public_note, validate_token_amount_v0,
};
use crate::state_transition::batch_transition::token_mint_transition::v0::v0_methods::TokenMintTransitionV0Methods;
use crate::state_transition::batch_transition::TokenMintTransition;
use crate::tokens::MAX_TOKEN_NOTE_LEN;
use crate::validation::SimpleConsensusValidationResult;
use crate::ProtocolError;

pub(super) trait TokenMintTransitionActionStructureValidationV0 {
    fn validate_structure_v0(&self) -> Result<SimpleConsensusValidationResult, ProtocolError>;
}

impl TokenMintTransitionActionStructureValidationV0 for TokenMintTransition {
    fn validate_structure_v0(&self) -> Result<SimpleConsensusValidationResult, ProtocolError> {
        let mut result = validate_token_amount_v0(self.amount());

        if let Some(public_note) = self.public_note() {
            result.merge(validate_public_note(public_note));
        };

        Ok(result)
    }
}
