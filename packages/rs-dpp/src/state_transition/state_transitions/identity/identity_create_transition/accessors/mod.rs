mod v0;

use crate::state_transition::identity_create_transition::IdentityCreateTransition;
use crate::state_transition::public_key_in_creation::IdentityPublicKeyInCreation;

use platform_value::Identifier;
pub use v0::*;

impl IdentityCreateTransitionAccessorsV0 for IdentityCreateTransition {
    fn public_keys(&self) -> &[IdentityPublicKeyInCreation] {
        match self {
            IdentityCreateTransition::V0(transition) => transition.public_keys(),
        }
    }

    fn public_keys_mut(&mut self) -> &mut Vec<IdentityPublicKeyInCreation> {
        match self {
            IdentityCreateTransition::V0(transition) => transition.public_keys_mut(),
        }
    }

    fn set_public_keys(&mut self, public_keys: Vec<IdentityPublicKeyInCreation>) {
        match self {
            IdentityCreateTransition::V0(transition) => transition.set_public_keys(public_keys),
        }
    }

    fn add_public_keys(&mut self, public_keys: &mut Vec<IdentityPublicKeyInCreation>) {
        match self {
            IdentityCreateTransition::V0(transition) => transition.add_public_keys(public_keys),
        }
    }

    fn identity_id(&self) -> Identifier {
        match self {
            IdentityCreateTransition::V0(transition) => transition.identity_id(),
        }
    }

    fn owner_id(&self) -> Identifier {
        match self {
            IdentityCreateTransition::V0(transition) => transition.owner_id(),
        }
    }
}
