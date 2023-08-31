/// transformer
pub mod transformer;
/// v0
pub mod v0;

use crate::state_transition_action::identity::identity_update::v0::IdentityUpdateTransitionActionV0;
use derive_more::From;
use dpp::identity::{IdentityPublicKey, KeyID, TimestampMillis};
use dpp::platform_value::Identifier;
use dpp::prelude::Revision;

/// action
#[derive(Debug, Clone, From)]
pub enum IdentityUpdateTransitionAction {
    /// v0
    V0(IdentityUpdateTransitionActionV0),
}

impl IdentityUpdateTransitionAction {
    /// Public Keys
    pub fn public_keys_to_add(&self) -> &Vec<IdentityPublicKey> {
        match self {
            IdentityUpdateTransitionAction::V0(transition) => &transition.add_public_keys,
        }
    }
    /// Disable Public Keys
    pub fn public_keys_to_disable(&self) -> &Vec<KeyID> {
        match self {
            IdentityUpdateTransitionAction::V0(transition) => &transition.disable_public_keys,
        }
    }

    /// Public Keys to Add and Disable Owned
    pub fn public_keys_to_add_and_disable_owned(self) -> (Vec<IdentityPublicKey>, Vec<KeyID>) {
        match self {
            IdentityUpdateTransitionAction::V0(transition) => {
                (transition.add_public_keys, transition.disable_public_keys)
            }
        }
    }

    /// Public Keys Disabled At
    pub fn public_keys_disabled_at(&self) -> Option<TimestampMillis> {
        match self {
            IdentityUpdateTransitionAction::V0(transition) => transition.public_keys_disabled_at,
        }
    }

    /// Identity Id
    pub fn identity_id(&self) -> Identifier {
        match self {
            IdentityUpdateTransitionAction::V0(transition) => transition.identity_id,
        }
    }

    /// Revision
    pub fn revision(&self) -> Revision {
        match self {
            IdentityUpdateTransitionAction::V0(transition) => transition.revision,
        }
    }
}
