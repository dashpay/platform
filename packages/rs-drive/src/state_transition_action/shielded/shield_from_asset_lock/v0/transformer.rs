use crate::state_transition_action::shielded::shield_from_asset_lock::v0::ShieldFromAssetLockTransitionActionV0;
use dpp::fee::Credits;
use dpp::prelude::ConsensusValidationResult;
use dpp::state_transition::state_transitions::shielded::shield_from_asset_lock_transition::v0::ShieldFromAssetLockTransitionV0;

impl ShieldFromAssetLockTransitionActionV0 {
    /// Transforms the shield from asset lock transition into an action
    pub fn try_from_transition(
        value: &ShieldFromAssetLockTransitionV0,
        asset_lock_outpoint: [u8; 36],
        asset_lock_value_to_be_consumed: Credits,
        signable_bytes_hasher: [u8; 32],
        shield_amount: Credits,
        nullifiers: Vec<[u8; 32]>,
        note_commitments: Vec<[u8; 32]>,
        encrypted_notes: Vec<Vec<u8>>,
        current_total_balance: Credits,
    ) -> ConsensusValidationResult<Self> {
        ConsensusValidationResult::new_with_data(ShieldFromAssetLockTransitionActionV0 {
            asset_lock_outpoint,
            asset_lock_value_to_be_consumed,
            signable_bytes_hasher,
            shield_amount,
            nullifiers,
            note_commitments,
            encrypted_notes,
            user_fee_increase: value.user_fee_increase,
            current_total_balance,
        })
    }
}
