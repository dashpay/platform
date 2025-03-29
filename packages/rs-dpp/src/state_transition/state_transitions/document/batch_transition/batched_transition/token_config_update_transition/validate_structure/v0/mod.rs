use crate::consensus::basic::token::{
    InvalidTokenConfigUpdateNoChangeError, InvalidTokenNoteTooBigError,
};
use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::data_contract::associated_token::token_configuration_item::TokenConfigurationChangeItem;
use crate::state_transition::batch_transition::batched_transition::validation::validate_public_note;
use crate::state_transition::batch_transition::token_config_update_transition::v0::v0_methods::TokenConfigUpdateTransitionV0Methods;
use crate::state_transition::batch_transition::TokenConfigUpdateTransition;
use crate::tokens::MAX_TOKEN_NOTE_LEN;
use crate::validation::SimpleConsensusValidationResult;
use crate::ProtocolError;

pub(super) trait TokenConfigUpdateTransitionStructureValidationV0 {
    fn validate_structure_v0(&self) -> Result<SimpleConsensusValidationResult, ProtocolError>;
}
impl TokenConfigUpdateTransitionStructureValidationV0 for TokenConfigUpdateTransition {
    fn validate_structure_v0(&self) -> Result<SimpleConsensusValidationResult, ProtocolError> {
        let mut result = SimpleConsensusValidationResult::new();

        if matches!(
            self.update_token_configuration_item(),
            TokenConfigurationChangeItem::TokenConfigurationNoChange
        ) {
            result.add_error(InvalidTokenConfigUpdateNoChangeError::new());
        }

        if let Some(public_note) = self.public_note() {
            result.merge(validate_public_note(public_note));
        }

        Ok(result)
    }
}
