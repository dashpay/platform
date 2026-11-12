use crate::state_transition_action::shielded::token_purchase_from_shielded_pool::v0::TokenPurchaseFromShieldedPoolTransitionActionV0;
use crate::state_transition_action::shielded::token_purchase_from_shielded_pool::TokenPurchaseFromShieldedPoolTransitionAction;
use dpp::fee::Credits;
use dpp::platform_value::Identifier;
use dpp::prelude::ConsensusValidationResult;
use dpp::state_transition::token_purchase_from_shielded_pool_transition::TokenPurchaseFromShieldedPoolTransition;

impl TokenPurchaseFromShieldedPoolTransitionAction {
    /// Transforms the state transition into an action
    pub fn try_from_transition(
        value: &TokenPurchaseFromShieldedPoolTransition,
        fee_amount: Credits,
        current_credit_pool_balance: Credits,
        contract_owner_id: Identifier,
        allow_first_mint: bool,
    ) -> ConsensusValidationResult<Self> {
        match value {
            TokenPurchaseFromShieldedPoolTransition::V0(v0) => {
                TokenPurchaseFromShieldedPoolTransitionActionV0::try_from_transition(
                    v0,
                    fee_amount,
                    current_credit_pool_balance,
                    contract_owner_id,
                    allow_first_mint,
                )
                .map(|action| action.into())
            }
        }
    }
}
