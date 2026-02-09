mod v0;

pub use v0::*;

use crate::address_funds::PlatformAddress;
use crate::fee::Credits;
use crate::state_transition::identity_topup_from_addresses_transition::IdentityTopUpFromAddressesTransition;
use platform_value::Identifier;

impl IdentityTopUpFromAddressesTransitionAccessorsV0 for IdentityTopUpFromAddressesTransition {
    fn set_identity_id(&mut self, identity_id: Identifier) {
        match self {
            IdentityTopUpFromAddressesTransition::V0(transition) => {
                transition.set_identity_id(identity_id)
            }
        }
    }

    fn identity_id(&self) -> &Identifier {
        match self {
            IdentityTopUpFromAddressesTransition::V0(transition) => transition.identity_id(),
        }
    }

    fn output(&self) -> Option<&(PlatformAddress, Credits)> {
        match self {
            IdentityTopUpFromAddressesTransition::V0(transition) => transition.output(),
        }
    }

    fn set_output(&mut self, output: Option<(PlatformAddress, Credits)>) {
        match self {
            IdentityTopUpFromAddressesTransition::V0(transition) => transition.set_output(output),
        }
    }
}
