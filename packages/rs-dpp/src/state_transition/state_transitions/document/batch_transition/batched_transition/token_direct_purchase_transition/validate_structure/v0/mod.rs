use crate::consensus::basic::token::InvalidTokenAmountError;
use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::data_contract::associated_token::token_perpetual_distribution::distribution_function::MAX_DISTRIBUTION_PARAM;
use crate::state_transition::batch_transition::token_direct_purchase_transition::v0::v0_methods::TokenDirectPurchaseTransitionV0Methods;
use crate::state_transition::batch_transition::TokenDirectPurchaseTransition;
use crate::validation::SimpleConsensusValidationResult;
use crate::ProtocolError;

pub(super) trait TokenDirectPurchaseTransitionActionStructureValidationV0 {
    fn validate_structure_v0(&self) -> Result<SimpleConsensusValidationResult, ProtocolError>;
}
impl TokenDirectPurchaseTransitionActionStructureValidationV0 for TokenDirectPurchaseTransition {
    fn validate_structure_v0(&self) -> Result<SimpleConsensusValidationResult, ProtocolError> {
        if self.token_count() > MAX_DISTRIBUTION_PARAM || self.token_count() == 0 {
            return Ok(SimpleConsensusValidationResult::new_with_error(
                ConsensusError::BasicError(BasicError::InvalidTokenAmountError(
                    InvalidTokenAmountError::new(MAX_DISTRIBUTION_PARAM, self.token_count()),
                )),
            ));
        }
        Ok(SimpleConsensusValidationResult::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state_transition::batch_transition::token_base_transition::v0::TokenBaseTransitionV0;
    use crate::state_transition::batch_transition::token_base_transition::TokenBaseTransition;
    use crate::state_transition::batch_transition::token_direct_purchase_transition::v0::TokenDirectPurchaseTransitionV0;
    use platform_value::Identifier;

    fn make_transition(token_count: u64) -> TokenDirectPurchaseTransition {
        TokenDirectPurchaseTransition::V0(TokenDirectPurchaseTransitionV0 {
            base: TokenBaseTransition::V0(TokenBaseTransitionV0 {
                identity_contract_nonce: 1,
                token_contract_position: 0,
                data_contract_id: Identifier::default(),
                token_id: Identifier::default(),
                using_group_info: None,
            }),
            token_count,
            total_agreed_price: 1000,
        })
    }

    #[test]
    fn should_pass_with_valid_token_count() {
        let transition = make_transition(100);
        let result = transition.validate_structure_v0().unwrap();
        assert!(result.is_valid());
    }

    #[test]
    fn should_pass_with_token_count_of_one() {
        let transition = make_transition(1);
        let result = transition.validate_structure_v0().unwrap();
        assert!(result.is_valid());
    }

    #[test]
    fn should_pass_with_token_count_at_max() {
        let transition = make_transition(MAX_DISTRIBUTION_PARAM);
        let result = transition.validate_structure_v0().unwrap();
        assert!(result.is_valid());
    }

    #[test]
    fn should_return_error_when_token_count_is_zero() {
        let transition = make_transition(0);
        let result = transition.validate_structure_v0().unwrap();
        assert!(!result.is_valid());
        let error = result.errors.first().unwrap();
        assert!(matches!(
            error,
            ConsensusError::BasicError(BasicError::InvalidTokenAmountError(_))
        ));
    }

    #[test]
    fn should_return_error_when_token_count_exceeds_max() {
        let transition = make_transition(MAX_DISTRIBUTION_PARAM + 1);
        let result = transition.validate_structure_v0().unwrap();
        assert!(!result.is_valid());
        let error = result.errors.first().unwrap();
        assert!(matches!(
            error,
            ConsensusError::BasicError(BasicError::InvalidTokenAmountError(_))
        ));
    }
}
