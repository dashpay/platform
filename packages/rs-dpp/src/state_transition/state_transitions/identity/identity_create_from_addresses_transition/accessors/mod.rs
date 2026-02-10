mod v0;

use crate::address_funds::PlatformAddress;
use crate::fee::Credits;
use crate::state_transition::identity_create_from_addresses_transition::IdentityCreateFromAddressesTransition;
use crate::state_transition::public_key_in_creation::IdentityPublicKeyInCreation;
use crate::state_transition::StateTransitionIdentityIdFromInputs;
pub use v0::*;

impl IdentityCreateFromAddressesTransitionAccessorsV0 for IdentityCreateFromAddressesTransition {
    fn public_keys(&self) -> &[IdentityPublicKeyInCreation] {
        match self {
            IdentityCreateFromAddressesTransition::V0(transition) => transition.public_keys(),
        }
    }

    fn public_keys_mut(&mut self) -> &mut Vec<IdentityPublicKeyInCreation> {
        match self {
            IdentityCreateFromAddressesTransition::V0(transition) => transition.public_keys_mut(),
        }
    }

    fn set_public_keys(&mut self, public_keys: Vec<IdentityPublicKeyInCreation>) {
        match self {
            IdentityCreateFromAddressesTransition::V0(transition) => {
                transition.set_public_keys(public_keys)
            }
        }
    }

    fn add_public_keys(&mut self, public_keys: &mut Vec<IdentityPublicKeyInCreation>) {
        match self {
            IdentityCreateFromAddressesTransition::V0(transition) => {
                transition.add_public_keys(public_keys)
            }
        }
    }

    fn output(&self) -> Option<&(PlatformAddress, Credits)> {
        match self {
            IdentityCreateFromAddressesTransition::V0(transition) => transition.output(),
        }
    }

    fn set_output(&mut self, output: Option<(PlatformAddress, Credits)>) {
        match self {
            IdentityCreateFromAddressesTransition::V0(transition) => transition.set_output(output),
        }
    }
}

impl StateTransitionIdentityIdFromInputs for IdentityCreateFromAddressesTransition {}
