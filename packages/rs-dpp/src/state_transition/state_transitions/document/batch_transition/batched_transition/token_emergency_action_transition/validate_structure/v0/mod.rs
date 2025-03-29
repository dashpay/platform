use crate::consensus::basic::token::InvalidTokenNoteTooBigError;
use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::state_transition::batch_transition::batched_transition::validation::validate_public_note;
use crate::state_transition::batch_transition::token_emergency_action_transition::v0::v0_methods::TokenEmergencyActionTransitionV0Methods;
use crate::state_transition::batch_transition::TokenEmergencyActionTransition;
use crate::tokens::MAX_TOKEN_NOTE_LEN;
use crate::validation::SimpleConsensusValidationResult;
use crate::ProtocolError;

pub(super) trait TokenEmergencyActionTransitionStructureValidationV0 {
    fn validate_structure_v0(&self) -> Result<SimpleConsensusValidationResult, ProtocolError>;
}
impl TokenEmergencyActionTransitionStructureValidationV0 for TokenEmergencyActionTransition {
    fn validate_structure_v0(&self) -> Result<SimpleConsensusValidationResult, ProtocolError> {
        let mut result = SimpleConsensusValidationResult::new();

        if let Some(public_note) = self.public_note() {
            result.merge(validate_public_note(public_note));
        }

        Ok(result)
    }
}
