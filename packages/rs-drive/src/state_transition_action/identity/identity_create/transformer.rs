use crate::state_transition_action::identity::identity_create::v0::IdentityCreateTransitionActionV0;
use crate::state_transition_action::identity::identity_create::IdentityCreateTransitionAction;
use dpp::asset_lock::reduced_asset_lock_value::AssetLockValue;
use dpp::consensus::ConsensusError;
use dpp::state_transition::identity_create_transition::IdentityCreateTransition;

impl IdentityCreateTransitionAction {
    /// try from
    pub fn try_from(
        value: IdentityCreateTransition,
        asset_lock_value_to_be_consumed: AssetLockValue,
    ) -> Result<Self, ConsensusError> {
        match value {
            IdentityCreateTransition::V0(v0) => Ok(IdentityCreateTransitionActionV0::try_from(
                v0,
                asset_lock_value_to_be_consumed,
            )?
            .into()),
        }
    }

    /// try from borrowed
    pub fn try_from_borrowed(
        value: &IdentityCreateTransition,
        asset_lock_value_to_be_consumed: AssetLockValue,
    ) -> Result<Self, ConsensusError> {
        match value {
            IdentityCreateTransition::V0(v0) => {
                Ok(IdentityCreateTransitionActionV0::try_from_borrowed(
                    v0,
                    asset_lock_value_to_be_consumed,
                )?
                .into())
            }
        }
    }
}
