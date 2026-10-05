use crate::state_transition_action::shielded::shield_from_asset_lock::v0::ShieldFromAssetLockTransitionActionV0;
use crate::state_transition_action::shielded::shield_from_asset_lock::ShieldFromAssetLockTransitionAction;
use dpp::fee::Credits;
use dpp::prelude::ConsensusValidationResult;
use dpp::state_transition::shield_from_asset_lock_transition::ShieldFromAssetLockTransition;

impl ShieldFromAssetLockTransitionAction {
    /// Transforms the state transition into an action
    #[allow(clippy::too_many_arguments)]
    pub fn try_from_transition(
        value: &ShieldFromAssetLockTransition,
        asset_lock_outpoint: [u8; 36],
        asset_lock_value_to_be_consumed: Credits,
        signable_bytes_hasher: [u8; 32],
        shield_amount: Credits,
        current_total_balance: Credits,
        surplus_amount: Credits,
    ) -> ConsensusValidationResult<Self> {
        ShieldFromAssetLockTransitionActionV0::try_from_transition(
            value,
            asset_lock_outpoint,
            asset_lock_value_to_be_consumed,
            signable_bytes_hasher,
            shield_amount,
            current_total_balance,
            surplus_amount,
        )
        .map(|action| action.into())
    }
}
