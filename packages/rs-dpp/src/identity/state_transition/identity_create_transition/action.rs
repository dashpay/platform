use crate::identifier::Identifier;
use crate::identity::IdentityPublicKey;
use serde::{Deserialize, Serialize};
use crate::identity::state_transition::identity_create_transition::IdentityCreateTransition;
use crate::identity::state_transition::identity_public_key_transitions::IdentityPublicKeyWithWitness;

pub const IDENTITY_CREATE_TRANSITION_ACTION_VERSION: u32 = 0;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IdentityCreateTransitionAction {
    pub version: u32,
    pub public_keys: Vec<IdentityPublicKey>,
    pub initial_balance_amount: u64,
    pub identity_id: Identifier,
}

impl IdentityCreateTransitionAction {
    pub fn from(value: IdentityCreateTransition, initial_balance_amount: u64) -> Self {
        let IdentityCreateTransition {
            public_keys,
            identity_id,
            ..
        } = value;
        IdentityCreateTransitionAction {
            version: IDENTITY_CREATE_TRANSITION_ACTION_VERSION,
            public_keys: public_keys
                .into_iter()
                .map(IdentityPublicKeyWithWitness::to_identity_public_key)
                .collect(),
            initial_balance_amount,
            identity_id,
        }
    }

    pub fn from_borrowed(value: &IdentityCreateTransition, initial_balance_amount: u64) -> Self {
        let IdentityCreateTransition {
            public_keys,
            identity_id,
            ..
        } = value;
        IdentityCreateTransitionAction {
            version: IDENTITY_CREATE_TRANSITION_ACTION_VERSION,
            public_keys: public_keys
                .iter()
                .map(|key| key.clone().to_identity_public_key())
                .collect(),
            initial_balance_amount,
            identity_id: *identity_id,
        }
    }
}
