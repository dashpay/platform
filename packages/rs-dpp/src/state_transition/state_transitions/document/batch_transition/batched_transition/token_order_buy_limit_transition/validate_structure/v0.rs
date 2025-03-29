use crate::consensus::basic::token::ZeroTokenPriceError;
use crate::validation::SimpleConsensusValidationResult;
use crate::ProtocolError;
use crate::state_transition::batch_transition::batched_transition::token_order_buy_limit_transition::transition::TokenOrderBuyLimitTransition;
use crate::state_transition::batch_transition::batched_transition::token_order_buy_limit_transition::v0::accessors::TokenOrderBuyLimitTransitionV0Methods;
use crate::state_transition::batch_transition::batched_transition::validate_token_amount::ValidateTokenAmountV0;

pub(super) trait TokenOrderBuyLimitTransitionStructureValidationV0:
    ValidateTokenAmountV0
{
    fn validate_structure_v0(&self) -> Result<SimpleConsensusValidationResult, ProtocolError>;
}

impl ValidateTokenAmountV0 for TokenOrderBuyLimitTransition {}

impl TokenOrderBuyLimitTransitionStructureValidationV0 for TokenOrderBuyLimitTransition {
    fn validate_structure_v0(&self) -> Result<SimpleConsensusValidationResult, ProtocolError> {
        if let Some(consensus_error) = self.validate_token_amount_v0(self.token_amount()) {
            return Ok(SimpleConsensusValidationResult::new_with_error(
                consensus_error,
            ));
        }

        if self.token_max_price() == 0 {
            return Ok(SimpleConsensusValidationResult::new_with_error(
                ZeroTokenPriceError::new().into(),
            ));
        }

        Ok(SimpleConsensusValidationResult::default())
    }
}
