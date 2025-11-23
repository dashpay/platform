use crate::state_transition_action::identity::identity_create_from_addresses::v0::IdentityCreateFromAddressesTransitionActionV0;
use crate::state_transition_action::identity::identity_create_from_addresses::IdentityCreateFromAddressesTransitionAction;
use dpp::consensus::ConsensusError;
use dpp::state_transition::identity_create_from_addresses_transition::IdentityCreateFromAddressesTransition;

impl IdentityCreateFromAddressesTransitionAction {
    /// try from
    pub fn try_from(value: IdentityCreateFromAddressesTransition) -> Result<Self, ConsensusError> {
        match value {
            IdentityCreateFromAddressesTransition::V0(v0) => {
                Ok(IdentityCreateFromAddressesTransitionActionV0::try_from(v0)?.into())
            }
        }
    }

    /// try from borrowed
    pub fn try_from_borrowed(
        value: &IdentityCreateFromAddressesTransition,
    ) -> Result<Self, ConsensusError> {
        match value {
            IdentityCreateFromAddressesTransition::V0(v0) => {
                Ok(IdentityCreateFromAddressesTransitionActionV0::try_from_borrowed(v0)?.into())
            }
        }
    }
}
