use crate::identifier::Identifier;
use crate::identity::state_transition::identity_public_key_transitions::IdentityPublicKeyInCreation;
use crate::identity::state_transition::identity_update_transition::identity_update_transition::IdentityUpdateTransition;
use crate::identity::{IdentityPublicKey, KeyID, TimestampMillis};
use crate::prelude::Revision;
use serde::{Deserialize, Serialize};

pub const IDENTITY_UPDATE_TRANSITION_ACTION_VERSION: u32 = 0;

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IdentityUpdateTransitionAction {
    pub version: u32,
    pub add_public_keys: Vec<IdentityPublicKey>,
    pub disable_public_keys: Vec<KeyID>,
    pub public_keys_disabled_at: Option<TimestampMillis>,
    pub identity_id: Identifier,
    pub revision: Revision,
}

impl From<IdentityUpdateTransition> for IdentityUpdateTransitionAction {
    fn from(value: IdentityUpdateTransition) -> Self {
        let IdentityUpdateTransition {
            identity_id,
            add_public_keys,
            disable_public_keys,
            public_keys_disabled_at,
            revision,
            ..
        } = value;
        IdentityUpdateTransitionAction {
            version: IDENTITY_UPDATE_TRANSITION_ACTION_VERSION,
            add_public_keys: add_public_keys
                .into_iter()
                .map(IdentityPublicKeyInCreation::to_identity_public_key)
                .collect(),
            disable_public_keys,
            public_keys_disabled_at,
            identity_id,
            revision,
        }
    }
}

impl From<&IdentityUpdateTransition> for IdentityUpdateTransitionAction {
    fn from(value: &IdentityUpdateTransition) -> Self {
        let IdentityUpdateTransition {
            identity_id,
            add_public_keys,
            disable_public_keys,
            public_keys_disabled_at,
            revision,
            ..
        } = value;
        IdentityUpdateTransitionAction {
            version: IDENTITY_UPDATE_TRANSITION_ACTION_VERSION,
            add_public_keys: add_public_keys
                .iter()
                .map(|key| key.clone().to_identity_public_key())
                .collect(),
            disable_public_keys: disable_public_keys.clone(),
            public_keys_disabled_at: *public_keys_disabled_at,
            identity_id: *identity_id,
            revision: *revision,
        }
    }
}
