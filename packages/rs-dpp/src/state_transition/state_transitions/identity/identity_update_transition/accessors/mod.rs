mod v0;

use crate::identity::KeyID;
use crate::prelude::{IdentityNonce, Revision};
use crate::state_transition::identity_update_transition::IdentityUpdateTransition;
use crate::state_transition::public_key_in_creation::IdentityPublicKeyInCreation;
use platform_value::Identifier;
pub use v0::*;

impl IdentityUpdateTransitionAccessorsV0 for IdentityUpdateTransition {
    fn set_identity_id(&mut self, id: Identifier) {
        match self {
            IdentityUpdateTransition::V0(transition) => transition.set_identity_id(id),
        }
    }

    fn identity_id(&self) -> Identifier {
        match self {
            IdentityUpdateTransition::V0(transition) => transition.identity_id(),
        }
    }

    fn set_revision(&mut self, revision: Revision) {
        match self {
            IdentityUpdateTransition::V0(transition) => transition.set_revision(revision),
        }
    }

    fn revision(&self) -> Revision {
        match self {
            IdentityUpdateTransition::V0(transition) => transition.revision(),
        }
    }

    fn set_nonce(&mut self, nonce: IdentityNonce) {
        match self {
            IdentityUpdateTransition::V0(transition) => transition.set_nonce(nonce),
        }
    }

    fn nonce(&self) -> IdentityNonce {
        match self {
            IdentityUpdateTransition::V0(transition) => transition.nonce(),
        }
    }

    fn set_public_keys_to_add(&mut self, add_public_keys: Vec<IdentityPublicKeyInCreation>) {
        match self {
            IdentityUpdateTransition::V0(transition) => {
                transition.set_public_keys_to_add(add_public_keys)
            }
        }
    }

    fn public_keys_to_add(&self) -> &[IdentityPublicKeyInCreation] {
        match self {
            IdentityUpdateTransition::V0(transition) => transition.public_keys_to_add(),
        }
    }

    fn public_keys_to_add_mut(&mut self) -> &mut [IdentityPublicKeyInCreation] {
        match self {
            IdentityUpdateTransition::V0(transition) => transition.public_keys_to_add_mut(),
        }
    }

    fn set_public_key_ids_to_disable(&mut self, disable_public_keys: Vec<KeyID>) {
        match self {
            IdentityUpdateTransition::V0(transition) => {
                transition.set_public_key_ids_to_disable(disable_public_keys)
            }
        }
    }

    fn public_key_ids_to_disable(&self) -> &[KeyID] {
        match self {
            IdentityUpdateTransition::V0(transition) => transition.public_key_ids_to_disable(),
        }
    }

    fn owner_id(&self) -> Identifier {
        match self {
            IdentityUpdateTransition::V0(transition) => transition.owner_id(),
        }
    }
}
