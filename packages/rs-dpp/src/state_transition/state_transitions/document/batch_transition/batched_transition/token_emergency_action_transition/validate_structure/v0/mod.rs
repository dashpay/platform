use crate::errors::consensus::basic::token::InvalidTokenNoteTooBigError;
use crate::errors::consensus::basic::BasicError;
use crate::errors::consensus::ConsensusError;
use crate::state_transition::state_transitions::document::batch_transition::batched_transition::token_emergency_action_transition::v0::v0_methods::TokenEmergencyActionTransitionV0Methods;
use crate::state_transition::state_transitions::document::batch_transition::batched_transition::token_emergency_action_transition::TokenEmergencyActionTransition;
use crate::tokens::MAX_TOKEN_NOTE_LEN;
use crate::validation::SimpleConsensusValidationResult;
use crate::ProtocolError;

pub(super) trait TokenEmergencyActionTransitionStructureValidationV0 {
    fn validate_structure_v0(&self) -> Result<SimpleConsensusValidationResult, ProtocolError>;
}
impl TokenEmergencyActionTransitionStructureValidationV0 for TokenEmergencyActionTransition {
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
