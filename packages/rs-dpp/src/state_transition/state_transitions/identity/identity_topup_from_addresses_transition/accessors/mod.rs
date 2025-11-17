mod v0;

pub use v0::*;

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
}
