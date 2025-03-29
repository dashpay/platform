use crate::validation::SimpleConsensusValidationResult;
use crate::ProtocolError;
use crate::state_transition::batch_transition::batched_transition::token_order_buy_limit_transition::transition::TokenOrderBuyLimitTransition;
use crate::state_transition::batch_transition::batched_transition::token_order_buy_limit_transition::v0::accessors::TokenOrderBuyLimitTransitionV0Methods;
use crate::state_transition::batch_transition::batched_transition::validation::{validate_token_amount_v0, validate_token_price_v0};

pub(super) trait TokenOrderBuyLimitTransitionStructureValidationV0 {
    fn validate_structure_v0(&self) -> Result<SimpleConsensusValidationResult, ProtocolError>;
}

impl TokenOrderBuyLimitTransitionStructureValidationV0 for TokenOrderBuyLimitTransition {
    fn validate_structure_v0(&self) -> Result<SimpleConsensusValidationResult, ProtocolError> {
        let mut result = validate_token_amount_v0(self.token_amount());

        result.merge(validate_token_price_v0(self.token_max_price()));

        Ok(result)
    }
}
