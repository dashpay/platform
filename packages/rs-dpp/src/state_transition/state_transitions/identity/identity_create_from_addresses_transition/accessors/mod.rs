mod v0;

use std::collections::BTreeMap;

use crate::state_transition::identity_create_from_addresses_transition::IdentityCreateFromAddressesTransition;
use crate::state_transition::public_key_in_creation::IdentityPublicKeyInCreation;

use crate::fee::Credits;
use crate::identity::KeyOfType;
use platform_value::Identifier;
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

    fn identity_id(&self) -> Identifier {
        match self {
            IdentityCreateFromAddressesTransition::V0(transition) => transition.identity_id(),
        }
    }

    fn inputs(&self) -> &[KeyOfType] {
        match self {
            IdentityCreateFromAddressesTransition::V0(transition) => transition.inputs(),
        }
    }

    fn inputs_mut(&mut self) -> &mut Vec<KeyOfType> {
        match self {
            IdentityCreateFromAddressesTransition::V0(transition) => transition.inputs_mut(),
        }
    }

    fn set_inputs(&mut self, inputs: Vec<KeyOfType>) {
        match self {
            IdentityCreateFromAddressesTransition::V0(transition) => transition.set_inputs(inputs),
        }
    }

    fn outputs(&self) -> &BTreeMap<KeyOfType, Credits> {
        match self {
            IdentityCreateFromAddressesTransition::V0(transition) => transition.outputs(),
        }
    }

    fn outputs_mut(&mut self) -> &mut BTreeMap<KeyOfType, Credits> {
        match self {
            IdentityCreateFromAddressesTransition::V0(transition) => transition.outputs_mut(),
        }
    }

    fn set_outputs(&mut self, outputs: BTreeMap<KeyOfType, Credits>) {
        match self {
            IdentityCreateFromAddressesTransition::V0(transition) => {
                transition.set_outputs(outputs)
            }
        }
    }
}
