use crate::state_transition_action::identity::identity_topup_from_addresses::v0::IdentityTopUpFromAddressesTransitionActionV0;
use crate::state_transition_action::identity::identity_topup_from_addresses::IdentityTopUpFromAddressesTransitionAction;
use dpp::address_funds::PlatformAddress;
use dpp::fee::Credits;
use dpp::prelude::{AddressNonce, ConsensusValidationResult};
use dpp::state_transition::identity_topup_from_addresses_transition::IdentityTopUpFromAddressesTransition;
use std::collections::BTreeMap;

impl IdentityTopUpFromAddressesTransitionAction {
    /// Transforms the state transition into an action by validating inputs against provided balances.
    pub fn try_from_transition(
        value: &IdentityTopUpFromAddressesTransition,
        inputs_with_remaining_balance: BTreeMap<PlatformAddress, (AddressNonce, Credits)>,
    ) -> ConsensusValidationResult<Self> {
        match value {
            IdentityTopUpFromAddressesTransition::V0(v0) => {
                let result = IdentityTopUpFromAddressesTransitionActionV0::try_from_transition(
                    v0,
                    inputs_with_remaining_balance,
                );
                result.map(|action| action.into())
            }
        }
    }
}
