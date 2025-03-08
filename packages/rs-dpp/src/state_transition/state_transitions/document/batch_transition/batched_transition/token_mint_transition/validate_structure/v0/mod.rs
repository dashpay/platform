use crate::consensus::basic::token::{InvalidTokenAmountError, InvalidTokenNoteTooBigError};
use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
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
        if self.amount() > i64::MAX as u64 {
            return Ok(SimpleConsensusValidationResult::new_with_error(
                ConsensusError::BasicError(BasicError::InvalidTokenAmountError(
                    InvalidTokenAmountError::new(i64::MAX as u64, self.amount()),
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
        }
        Ok(SimpleConsensusValidationResult::default())
    }
}
