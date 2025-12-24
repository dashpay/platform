use crate::state_transition_action::identity::identity_create_from_addresses::v0::IdentityCreateFromAddressesTransitionActionV0;
use crate::state_transition_action::identity::identity_create_from_addresses::IdentityCreateFromAddressesTransitionAction;
use dpp::address_funds::PlatformAddress;
use dpp::fee::Credits;
use dpp::prelude::{AddressNonce, ConsensusValidationResult};
use dpp::state_transition::identity_create_from_addresses_transition::IdentityCreateFromAddressesTransition;
use std::collections::BTreeMap;

impl IdentityCreateFromAddressesTransitionAction {
    /// Transforms the state transition into an action by validating inputs against provided balances.
    pub fn try_from_transition(
        value: &IdentityCreateFromAddressesTransition,
        inputs_with_remaining_balance: BTreeMap<PlatformAddress, (AddressNonce, Credits)>,
    ) -> ConsensusValidationResult<Self> {
        match value {
            IdentityCreateFromAddressesTransition::V0(v0) => {
                let result = IdentityCreateFromAddressesTransitionActionV0::try_from_transition(
                    v0,
                    inputs_with_remaining_balance,
                );
                result.map(|action| action.into())
            }
        }
    }
}
