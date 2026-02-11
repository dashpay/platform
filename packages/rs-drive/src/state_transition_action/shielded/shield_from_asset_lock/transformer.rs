use crate::state_transition_action::shielded::shield_from_asset_lock::v0::ShieldFromAssetLockTransitionActionV0;
use crate::state_transition_action::shielded::shield_from_asset_lock::ShieldFromAssetLockTransitionAction;
use dpp::fee::Credits;
use dpp::prelude::ConsensusValidationResult;
use dpp::state_transition::shield_from_asset_lock_transition::ShieldFromAssetLockTransition;

impl ShieldFromAssetLockTransitionAction {
    /// Transforms the state transition into an action
    pub fn try_from_transition(
        value: &ShieldFromAssetLockTransition,
        asset_lock_outpoint: [u8; 36],
        asset_lock_value_to_be_consumed: Credits,
        signable_bytes_hasher: [u8; 32],
        shield_amount: Credits,
        note_commitments: Vec<[u8; 32]>,
        encrypted_notes: Vec<Vec<u8>>,
        current_total_balance: Credits,
    ) -> ConsensusValidationResult<Self> {
        match value {
            ShieldFromAssetLockTransition::V0(v0) => {
                let result = ShieldFromAssetLockTransitionActionV0::try_from_transition(
                    v0,
                    asset_lock_outpoint,
                    asset_lock_value_to_be_consumed,
                    signable_bytes_hasher,
                    shield_amount,
                    note_commitments,
                    encrypted_notes,
                    current_total_balance,
                );
                result.map(|action| action.into())
            }
        }
    }
}
