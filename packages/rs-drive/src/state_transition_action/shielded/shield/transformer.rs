use crate::state_transition_action::shielded::shield::v0::ShieldTransitionActionV0;
use crate::state_transition_action::shielded::shield::ShieldTransitionAction;
use crate::state_transition_action::shielded::ShieldedActionNote;
use dpp::address_funds::PlatformAddress;
use dpp::fee::Credits;
use dpp::prelude::{AddressNonce, ConsensusValidationResult};
use dpp::state_transition::shield_transition::ShieldTransition;
use std::collections::BTreeMap;

impl ShieldTransitionAction {
    /// Transforms the state transition into an action
    pub fn try_from_transition(
        value: &ShieldTransition,
        inputs_with_remaining_balance: BTreeMap<PlatformAddress, (AddressNonce, Credits)>,
        shield_amount: Credits,
        current_total_balance: Credits,
    ) -> ConsensusValidationResult<Self> {
        match value {
            ShieldTransition::V0(v0) => {
                let result = ShieldTransitionActionV0::try_from_transition(
                    v0,
                    inputs_with_remaining_balance,
                    shield_amount,
                    current_total_balance,
                );
                result.map(|action| action.into())
            }
            ShieldTransition::V1(v1) => ConsensusValidationResult::new_with_data(
                ShieldTransitionActionV0 {
                    inputs_with_remaining_balance,
                    shield_amount,
                    notes: v1.actions.iter().map(ShieldedActionNote::from).collect(),
                    fee_strategy: v1.fee_strategy.clone(),
                    user_fee_increase: v1.user_fee_increase,
                    current_total_balance,
                }
                .into(),
            ),
        }
    }
}
