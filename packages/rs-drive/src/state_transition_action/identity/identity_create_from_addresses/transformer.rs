use crate::state_transition_action::identity::identity_create_from_addresses::v0::IdentityCreateFromAddressesTransitionActionV0;
use crate::state_transition_action::identity::identity_create_from_addresses::IdentityCreateFromAddressesTransitionAction;
use dpp::asset_lock::reduced_asset_lock_value::AssetLockValue;
use dpp::consensus::ConsensusError;
use dpp::state_transition::identity_create_from_addresses_transition::IdentityCreateFromAddressesTransition;
use dpp::state_transition::signable_bytes_hasher::SignableBytesHasher;

impl IdentityCreateFromAddressesTransitionAction {
    /// try from
    pub fn try_from(
        value: IdentityCreateFromAddressesTransition,
        signable_bytes_hasher: SignableBytesHasher,
        asset_lock_value_to_be_consumed: AssetLockValue,
    ) -> Result<Self, ConsensusError> {
        match value {
            IdentityCreateFromAddressesTransition::V0(v0) => {
                Ok(IdentityCreateFromAddressesTransitionActionV0::try_from(
                    v0,
                    signable_bytes_hasher,
                    asset_lock_value_to_be_consumed,
                )?
                .into())
            }
        }
    }

    /// try from borrowed
    pub fn try_from_borrowed(
        value: &IdentityCreateFromAddressesTransition,
        signable_bytes_hasher: SignableBytesHasher,
        asset_lock_value_to_be_consumed: AssetLockValue,
    ) -> Result<Self, ConsensusError> {
        match value {
            IdentityCreateFromAddressesTransition::V0(v0) => Ok(
                IdentityCreateFromAddressesTransitionActionV0::try_from_borrowed(
                    v0,
                    signable_bytes_hasher,
                    asset_lock_value_to_be_consumed,
                )?
                .into(),
            ),
        }
    }
}
