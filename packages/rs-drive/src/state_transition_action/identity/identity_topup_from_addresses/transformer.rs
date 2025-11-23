use crate::state_transition_action::identity::identity_topup_from_addresses::v0::IdentityTopUpFromAddressesTransitionActionV0;
use crate::state_transition_action::identity::identity_topup_from_addresses::IdentityTopUpFromAddressesTransitionAction;
use dpp::asset_lock::reduced_asset_lock_value::AssetLockValue;
use dpp::consensus::ConsensusError;
use dpp::state_transition::identity_topup_from_addresses_transition::IdentityTopUpFromAddressesTransition;
use dpp::state_transition::signable_bytes_hasher::SignableBytesHasher;

impl IdentityTopUpFromAddressesTransitionAction {
    /// try from
    pub fn try_from(value: IdentityTopUpFromAddressesTransition) -> Result<Self, ConsensusError> {
        match value {
            IdentityTopUpFromAddressesTransition::V0(v0) => {
                Ok(IdentityTopUpFromAddressesTransitionActionV0::try_from(v0)?.into())
            }
        }
    }

    /// try from borrowed
    pub fn try_from_borrowed(
        value: &IdentityTopUpFromAddressesTransition,
    ) -> Result<Self, ConsensusError> {
        match value {
            IdentityTopUpFromAddressesTransition::V0(v0) => {
                Ok(IdentityTopUpFromAddressesTransitionActionV0::try_from_borrowed(v0)?.into())
            }
        }
    }
}
