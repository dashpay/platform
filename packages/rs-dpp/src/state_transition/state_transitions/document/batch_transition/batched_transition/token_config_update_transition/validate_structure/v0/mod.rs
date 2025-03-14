use crate::errors::consensus::basic::token::{
    InvalidTokenConfigUpdateNoChangeError, InvalidTokenNoteTooBigError,
};
use crate::errors::consensus::basic::BasicError;
use crate::errors::consensus::ConsensusError;
use crate::data_contract::associated_token::token_configuration_item::TokenConfigurationChangeItem;
use crate::state_transition::state_transitions::document::batch_transition::batched_transition::token_config_update_transition::v0::v0_methods::TokenConfigUpdateTransitionV0Methods;
use crate::state_transition::state_transitions::document::batch_transition::batched_transition::token_config_update_transition::TokenConfigUpdateTransition;
use crate::tokens::MAX_TOKEN_NOTE_LEN;
use crate::validation::SimpleConsensusValidationResult;
use crate::ProtocolError;

pub(super) trait TokenConfigUpdateTransitionStructureValidationV0 {
    fn validate_structure_v0(&self) -> Result<SimpleConsensusValidationResult, ProtocolError>;
}
impl TokenConfigUpdateTransitionStructureValidationV0 for TokenConfigUpdateTransition {
    fn validate_structure_v0(&self) -> Result<SimpleConsensusValidationResult, ProtocolError> {
        if matches!(
            self.update_token_configuration_item(),
            TokenConfigurationChangeItem::TokenConfigurationNoChange
        ) {
            return Ok(SimpleConsensusValidationResult::new_with_error(
                ConsensusError::BasicError(BasicError::InvalidTokenConfigUpdateNoChangeError(
                    InvalidTokenConfigUpdateNoChangeError::new(),
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
