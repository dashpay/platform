use crate::state_transition_action::shielded::token_unshield_with_shielded_fee::v0::TokenUnshieldWithShieldedFeeTransitionActionV0;
use crate::state_transition_action::shielded::token_unshield_with_shielded_fee::TokenUnshieldWithShieldedFeeTransitionAction;
use dpp::fee::Credits;

use dpp::prelude::ConsensusValidationResult;
use dpp::state_transition::token_unshield_with_shielded_fee_transition::TokenUnshieldWithShieldedFeeTransition;

impl TokenUnshieldWithShieldedFeeTransitionAction {
    /// Transforms the state transition into an action
    pub fn try_from_transition(
        value: &TokenUnshieldWithShieldedFeeTransition,
        fee_amount: Credits,
        current_credit_pool_balance: Credits,
    ) -> ConsensusValidationResult<Self> {
        match value {
            TokenUnshieldWithShieldedFeeTransition::V0(v0) => {
                TokenUnshieldWithShieldedFeeTransitionActionV0::try_from_transition(
                    v0,
                    fee_amount,
                    current_credit_pool_balance,
                )
                .map(|action| action.into())
            }
        }
    }
}
