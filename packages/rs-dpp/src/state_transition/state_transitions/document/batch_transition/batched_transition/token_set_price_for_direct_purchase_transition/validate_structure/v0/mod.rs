use crate::consensus::basic::token::InvalidTokenNoteTooBigError;
use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::state_transition::batch_transition::token_set_price_for_direct_purchase_transition::v0::v0_methods::TokenSetPriceForDirectPurchaseTransitionV0Methods;
use crate::state_transition::batch_transition::TokenSetPriceForDirectPurchaseTransition;
use crate::tokens::MAX_TOKEN_NOTE_LEN;
use crate::validation::SimpleConsensusValidationResult;
use crate::ProtocolError;

pub(super) trait TokenSetPriceForDirectPurchaseTransitionActionStructureValidationV0 {
    fn validate_structure_v0(&self) -> Result<SimpleConsensusValidationResult, ProtocolError>;
}
impl TokenSetPriceForDirectPurchaseTransitionActionStructureValidationV0 for TokenSetPriceForDirectPurchaseTransition {
    fn validate_structure_v0(&self) -> Result<SimpleConsensusValidationResult, ProtocolError> {
        // There is no need to validate the price because setting a price that is too high just makes the token non purchasable
        
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
