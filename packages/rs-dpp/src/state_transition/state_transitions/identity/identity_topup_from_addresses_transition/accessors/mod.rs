mod v0;

use std::collections::BTreeMap;
pub use v0::*;

use crate::address_funds::PlatformAddress;
use crate::fee::Credits;
use crate::prelude::KeyOfTypeNonce;
use crate::state_transition::identity_topup_from_addresses_transition::IdentityTopUpFromAddressesTransition;
use crate::state_transition::StateTransitionAddressInputs;
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

impl StateTransitionAddressInputs for IdentityTopUpFromAddressesTransition {
    fn inputs(&self) -> &BTreeMap<PlatformAddress, (KeyOfTypeNonce, Credits)> {
        match self {
            IdentityTopUpFromAddressesTransition::V0(transition) => transition.inputs(),
        }
    }

    fn inputs_mut(&mut self) -> &mut BTreeMap<PlatformAddress, (KeyOfTypeNonce, Credits)> {
        match self {
            IdentityTopUpFromAddressesTransition::V0(transition) => transition.inputs_mut(),
        }
    }

    fn set_inputs(&mut self, inputs: BTreeMap<PlatformAddress, (KeyOfTypeNonce, Credits)>) {
        match self {
            IdentityTopUpFromAddressesTransition::V0(transition) => transition.set_inputs(inputs),
        }
    }
}
