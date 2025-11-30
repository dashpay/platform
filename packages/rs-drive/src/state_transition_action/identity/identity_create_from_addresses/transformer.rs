use crate::state_transition_action::identity::identity_create_from_addresses::v0::IdentityCreateFromAddressesTransitionActionV0;
use crate::state_transition_action::identity::identity_create_from_addresses::IdentityCreateFromAddressesTransitionAction;
use dpp::address_funds::PlatformAddress;
use dpp::fee::Credits;
use dpp::prelude::ConsensusValidationResult;
use dpp::state_transition::identity_create_from_addresses_transition::IdentityCreateFromAddressesTransition;
use std::collections::BTreeMap;

impl IdentityCreateFromAddressesTransitionAction {
    /// Transforms the state transition into an action by validating inputs against provided balances.
    pub fn try_from_transition(
        value: &IdentityCreateFromAddressesTransition,
        input_balances: BTreeMap<PlatformAddress, Credits>,
    ) -> ConsensusValidationResult<Self> {
        match value {
            IdentityCreateFromAddressesTransition::V0(v0) => {
                let result = IdentityCreateFromAddressesTransitionActionV0::try_from_transition(
                    v0,
                    input_balances,
                );
                result.map(|action| action.into())
            }
        }
    }
}
