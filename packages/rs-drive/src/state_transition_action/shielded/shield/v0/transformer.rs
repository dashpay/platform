use crate::state_transition_action::shielded::shield::v0::ShieldTransitionActionV0;
use dpp::address_funds::PlatformAddress;
use dpp::fee::Credits;
use dpp::prelude::{AddressNonce, ConsensusValidationResult};
use dpp::state_transition::state_transitions::shielded::shield_transition::v0::ShieldTransitionV0;
use std::collections::BTreeMap;

impl ShieldTransitionActionV0 {
    /// Transforms the shield transition into an action
    pub fn try_from_transition(
        value: &ShieldTransitionV0,
        inputs_with_remaining_balance: BTreeMap<PlatformAddress, (AddressNonce, Credits)>,
        shield_amount: Credits,
        note_commitments: Vec<[u8; 32]>,
        encrypted_notes: Vec<Vec<u8>>,
        current_total_balance: Credits,
    ) -> ConsensusValidationResult<Self> {
        ConsensusValidationResult::new_with_data(ShieldTransitionActionV0 {
            inputs_with_remaining_balance,
            shield_amount,
            note_commitments,
            encrypted_notes,
            fee_strategy: value.fee_strategy.clone(),
            user_fee_increase: value.user_fee_increase,
            current_total_balance,
        })
    }
}
