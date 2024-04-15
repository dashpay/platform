use crate::state_transition_action::identity::identity_topup::v0::IdentityTopUpTransitionActionV0;
use crate::state_transition_action::identity::identity_topup::IdentityTopUpTransitionAction;
use dpp::asset_lock::reduced_asset_lock_value::AssetLockValue;
use dpp::consensus::basic::value_error::ValueError;
use dpp::consensus::basic::BasicError;
use dpp::consensus::ConsensusError;
use dpp::serialization::Signable;
use dpp::state_transition::identity_topup_transition::IdentityTopUpTransition;

impl IdentityTopUpTransitionAction {
    /// try from
    pub fn try_from(
        value: IdentityTopUpTransition,
        top_up_asset_lock_value: AssetLockValue,
    ) -> Result<Self, ConsensusError> {
        //todo: this is already done in signature verification, ideally we should reuse it
        let signable_bytes = value.signable_bytes().map_err(|e| {
            ConsensusError::BasicError(BasicError::ValueError(ValueError::new_from_string(
                e.to_string(),
            )))
        })?;

        match value {
            IdentityTopUpTransition::V0(v0) => Ok(IdentityTopUpTransitionActionV0::try_from(
                v0,
                signable_bytes,
                top_up_asset_lock_value,
            )?
            .into()),
        }
    }

    /// try from borrowed
    pub fn try_from_borrowed(
        value: &IdentityTopUpTransition,
        top_up_asset_lock_value: AssetLockValue,
    ) -> Result<Self, ConsensusError> {
        //todo: this is already done in signature verification, ideally we should reuse it
        let signable_bytes = value.signable_bytes().map_err(|e| {
            ConsensusError::BasicError(BasicError::ValueError(ValueError::new_from_string(
                e.to_string(),
            )))
        })?;

        match value {
            IdentityTopUpTransition::V0(v0) => {
                Ok(IdentityTopUpTransitionActionV0::try_from_borrowed(
                    v0,
                    signable_bytes,
                    top_up_asset_lock_value,
                )?
                .into())
            }
        }
    }
}
