use crate::state_transition_action::identity::identity_topup::v0::IdentityTopUpTransitionActionV0;
use crate::state_transition_action::identity::identity_topup::IdentityTopUpTransitionAction;
use dpp::asset_lock::reduced_asset_lock_value::AssetLockValue;
use dpp::consensus::ConsensusError;
use dpp::state_transition::identity_topup_transition::IdentityTopUpTransition;

impl IdentityTopUpTransitionAction {
    /// try from
    pub fn try_from(
        value: IdentityTopUpTransition,
        top_up_asset_lock_value: AssetLockValue,
    ) -> Result<Self, ConsensusError> {
        match value {
            IdentityTopUpTransition::V0(v0) => {
                Ok(IdentityTopUpTransitionActionV0::try_from(v0, top_up_asset_lock_value)?.into())
            }
        }
    }

    /// try from borrowed
    pub fn try_from_borrowed(
        value: &IdentityTopUpTransition,
        top_up_asset_lock_value: AssetLockValue,
    ) -> Result<Self, ConsensusError> {
        match value {
            IdentityTopUpTransition::V0(v0) => Ok(
                IdentityTopUpTransitionActionV0::try_from_borrowed(v0, top_up_asset_lock_value)?
                    .into(),
            ),
        }
    }
}
