use crate::prelude::{Identity, Revision};
use derive_more::From;
use platform_value::{Bytes36, Identifier};
use crate::identity::{IdentityPublicKey, KeyID, TimestampMillis};
use crate::state_transition::identity_update_transition::IdentityUpdateTransition;
use crate::state_transition::identity_update_transition::v0_action::IdentityUpdateTransitionActionV0;

#[derive(Debug, Clone, From)]
pub enum IdentityUpdateTransitionAction {
    V0(IdentityUpdateTransitionActionV0),
}

impl IdentityUpdateTransitionAction {
    // Public Keys
    pub fn public_keys_to_add(&self) -> &Vec<IdentityPublicKey> {
        match self {
            IdentityUpdateTransitionAction::V0(transition) => &transition.add_public_keys,
        }
    }
    // Disable Public Keys
    pub fn public_keys_to_disable(&self) -> &Vec<KeyID> {
        match self {
            IdentityUpdateTransitionAction::V0(transition) => &transition.disable_public_keys,
        }
    }

    // Public Keys Disabled At
    pub fn public_keys_disabled_at(&self) -> Option<TimestampMillis> {
        match self {
            IdentityUpdateTransitionAction::V0(transition) => transition.public_keys_disabled_at,
        }
    }

    // Identity Id
    pub fn identity_id(&self) -> Identifier {
        match self {
            IdentityUpdateTransitionAction::V0(transition) => transition.identity_id,
        }
    }

    // Revision
    pub fn revision(&self) -> Revision {
        match self {
            IdentityUpdateTransitionAction::V0(transition) => transition.revision,
        }
    }
}