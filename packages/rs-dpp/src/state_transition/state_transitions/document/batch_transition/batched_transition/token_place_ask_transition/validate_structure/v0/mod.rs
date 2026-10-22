use crate::state_transition::batch_transition::batched_transition::token_order_terms::validate_token_order_terms;
use crate::state_transition::batch_transition::token_place_ask_transition::v0::v0_methods::TokenPlaceAskTransitionV0Methods;
use crate::state_transition::batch_transition::TokenPlaceAskTransition;
use crate::validation::SimpleConsensusValidationResult;
use crate::ProtocolError;

pub(super) trait TokenPlaceAskTransitionStructureValidationV0 {
    fn validate_structure_v0(&self) -> Result<SimpleConsensusValidationResult, ProtocolError>;
}

impl TokenPlaceAskTransitionStructureValidationV0 for TokenPlaceAskTransition {
    /// The listing's terms, checked as an offer's are: see `validate_token_order_terms`.
    fn validate_structure_v0(&self) -> Result<SimpleConsensusValidationResult, ProtocolError> {
        Ok(validate_token_order_terms(
            self.amount(),
            self.price(),
            self.min_fill(),
            self.expires_at(),
        ))
    }
}
