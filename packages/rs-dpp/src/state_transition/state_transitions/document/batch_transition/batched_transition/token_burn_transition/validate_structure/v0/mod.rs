use crate::errors::consensus::basic::token::{InvalidTokenAmountError, InvalidTokenNoteTooBigError};
use crate::errors::consensus::basic::BasicError;
use crate::errors::consensus::ConsensusError;
use crate::data_contract::associated_token::token_perpetual_distribution::distribution_function::MAX_DISTRIBUTION_PARAM;
use crate::state_transition::state_transitions::document::batch_transition::batched_transition::token_burn_transition::v0::v0_methods::TokenBurnTransitionV0Methods;
use crate::state_transition::state_transitions::document::batch_transition::batched_transition::token_burn_transition::TokenBurnTransition;
use crate::tokens::MAX_TOKEN_NOTE_LEN;
use crate::validation::SimpleConsensusValidationResult;
use crate::ProtocolError;

pub(super) trait TokenBurnTransitionActionStructureValidationV0 {
    fn validate_structure_v0(&self) -> Result<SimpleConsensusValidationResult, ProtocolError>;
}
impl TokenBurnTransitionActionStructureValidationV0 for TokenBurnTransition {
    fn validate_structure_v0(&self) -> Result<SimpleConsensusValidationResult, ProtocolError> {
        if self.burn_amount() > MAX_DISTRIBUTION_PARAM || self.burn_amount() == 0 {
            return Ok(SimpleConsensusValidationResult::new_with_error(
                ConsensusError::BasicError(BasicError::InvalidTokenAmountError(
                    InvalidTokenAmountError::new(MAX_DISTRIBUTION_PARAM, self.burn_amount()),
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
