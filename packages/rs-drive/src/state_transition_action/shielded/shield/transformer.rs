use crate::state_transition_action::shielded::shield::v0::ShieldTransitionActionV0;
use crate::state_transition_action::shielded::shield::ShieldTransitionAction;
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
        nullifiers: Vec<[u8; 32]>,
        note_commitments: Vec<[u8; 32]>,
        encrypted_notes: Vec<Vec<u8>>,
        current_total_balance: Credits,
    ) -> ConsensusValidationResult<Self> {
        match value {
            ShieldTransition::V0(v0) => {
                let result = ShieldTransitionActionV0::try_from_transition(
                    v0,
                    inputs_with_remaining_balance,
                    shield_amount,
                    nullifiers,
                    note_commitments,
                    encrypted_notes,
                    current_total_balance,
                );
                result.map(|action| action.into())
            }
        }
    }
}
