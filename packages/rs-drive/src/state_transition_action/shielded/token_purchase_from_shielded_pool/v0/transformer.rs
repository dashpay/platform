use crate::state_transition_action::shielded::token_purchase_from_shielded_pool::v0::TokenPurchaseFromShieldedPoolTransitionActionV0;
use crate::state_transition_action::shielded::ShieldedActionNote;
use dpp::fee::Credits;
use dpp::platform_value::Identifier;
use dpp::prelude::ConsensusValidationResult;
use dpp::state_transition::state_transitions::shielded::token_purchase_from_shielded_pool_transition::v0::TokenPurchaseFromShieldedPoolTransitionV0;

impl TokenPurchaseFromShieldedPoolTransitionActionV0 {
    /// Builds the v0 action from the v0 transition, the flat fee and the credit pool total
    pub fn try_from_transition(
        value: &TokenPurchaseFromShieldedPoolTransitionV0,
        fee_amount: Credits,
        current_credit_pool_balance: Credits,
        contract_owner_id: Identifier,
        allow_first_mint: bool,
    ) -> ConsensusValidationResult<Self> {
        let token_notes: Vec<ShieldedActionNote> = value
            .token_actions
            .iter()
            .map(ShieldedActionNote::from)
            .collect();
        let fee_notes: Vec<ShieldedActionNote> = value
            .fee_actions
            .iter()
            .map(ShieldedActionNote::from)
            .collect();
        ConsensusValidationResult::new_with_data(TokenPurchaseFromShieldedPoolTransitionActionV0 {
            token_id: value.token_id,
            token_notes,
            token_anchor: value.token_anchor,
            fee_notes,
            fee_anchor: value.fee_anchor,
            fee_amount,
            current_credit_pool_balance,
            contract_owner_id,
            token_count: value.token_count,
            total_agreed_price: value.total_agreed_price,
            allow_first_mint,
        })
    }
}
