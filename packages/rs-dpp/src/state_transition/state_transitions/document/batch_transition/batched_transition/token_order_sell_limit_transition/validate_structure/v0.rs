use crate::consensus::basic::token::ZeroTokenPriceError;
use crate::validation::SimpleConsensusValidationResult;
use crate::ProtocolError;
use crate::state_transition::batch_transition::batched_transition::token_order_sell_limit_transition::transition::TokenOrderSellLimitTransition;
use crate::state_transition::batch_transition::batched_transition::token_order_sell_limit_transition::v0::accessors::TokenOrderSellLimitTransitionV0Methods;
use crate::state_transition::batch_transition::batched_transition::validate_token_amount::ValidateTokenAmountV0;

pub(super) trait TokenOrderSellLimitTransitionStructureValidationV0:
    ValidateTokenAmountV0
{
    fn validate_structure_v0(&self) -> Result<SimpleConsensusValidationResult, ProtocolError>;
}

impl ValidateTokenAmountV0 for TokenOrderSellLimitTransition {}

impl TokenOrderSellLimitTransitionStructureValidationV0 for TokenOrderSellLimitTransition {
    fn validate_structure_v0(&self) -> Result<SimpleConsensusValidationResult, ProtocolError> {
        if let Some(consensus_error) = self.validate_token_amount_v0(self.token_amount()) {
            return Ok(SimpleConsensusValidationResult::new_with_error(
                consensus_error,
            ));
        }

        if self.token_min_price() == 0 {
            return Ok(SimpleConsensusValidationResult::new_with_error(
                ZeroTokenPriceError::new().into(),
            ));
        }

        Ok(SimpleConsensusValidationResult::default())
    }
}
