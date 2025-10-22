mod v0;

use std::collections::BTreeMap;
use crate::prelude::IdentityNonce;
use crate::state_transition::identity_credit_transfer_to_single_key_transition::IdentityCreditTransferToSingleKeyTransition;
use platform_value::Identifier;
pub use v0::*;
use crate::fee::Credits;
use crate::identity::KeyOfType;

impl IdentityCreditTransferToSingleKeyTransitionAccessorsV0 for IdentityCreditTransferToSingleKeyTransition {

    fn identity_id(&self) -> Identifier {
        match self {
            IdentityCreditTransferToSingleKeyTransition::V0(transition) => transition.identity_id,
        }
    }

    fn set_identity_id(&mut self, identity_id: Identifier) {
        match self {
            IdentityCreditTransferToSingleKeyTransition::V0(transition) => {
                transition.identity_id = identity_id;
            }
        }
    }

    fn recipient_keys(&self) -> &BTreeMap<KeyOfType, Credits> {
        match self {
            IdentityCreditTransferToSingleKeyTransition::V0(transition) => &transition.recipient_keys,
        }
    }

    fn set_recipient_keys(&mut self, recipient_keys: BTreeMap<KeyOfType, Credits>) {
        match self {
            IdentityCreditTransferToSingleKeyTransition::V0(transition) => {
                transition.recipient_keys = recipient_keys;
            }
        }
    }

    fn set_nonce(&mut self, nonce: IdentityNonce) {
        match self {
            IdentityCreditTransferToSingleKeyTransition::V0(transition) => transition.nonce = nonce,
        }
    }

    fn nonce(&self) -> IdentityNonce {
        match self {
            IdentityCreditTransferToSingleKeyTransition::V0(transition) => transition.nonce,
        }
    }
}
