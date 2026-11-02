use crate::state_transition_action::shielded::token_unshield_with_shielded_fee::v0::TokenUnshieldWithShieldedFeeTransitionActionV0;
use crate::state_transition_action::shielded::ShieldedActionNote;
use dpp::fee::Credits;

use dpp::prelude::ConsensusValidationResult;
use dpp::state_transition::state_transitions::shielded::token_unshield_with_shielded_fee_transition::v0::TokenUnshieldWithShieldedFeeTransitionV0;

impl TokenUnshieldWithShieldedFeeTransitionActionV0 {
    /// Builds the v0 action from the v0 transition, the flat fee and the credit pool total
    pub fn try_from_transition(
        value: &TokenUnshieldWithShieldedFeeTransitionV0,
        fee_amount: Credits,
        current_credit_pool_balance: Credits,
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
        ConsensusValidationResult::new_with_data(TokenUnshieldWithShieldedFeeTransitionActionV0 {
            token_id: value.token_id,
            token_notes,
            token_anchor: value.token_anchor,
            fee_notes,
            fee_anchor: value.fee_anchor,
            fee_amount,
            current_credit_pool_balance,
            recipient_id: value.recipient_id,
            amount: value.amount,
        })
    }
}
