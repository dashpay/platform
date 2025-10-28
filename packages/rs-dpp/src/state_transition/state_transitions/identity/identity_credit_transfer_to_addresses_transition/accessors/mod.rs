mod v0;

use crate::fee::Credits;
use crate::identity::KeyOfType;
use crate::prelude::IdentityNonce;
use crate::state_transition::identity_credit_transfer_to_addresses_transition::IdentityCreditTransferToAddressesTransition;
use platform_value::Identifier;
use std::collections::BTreeMap;
pub use v0::*;

impl IdentityCreditTransferToAddressesTransitionAccessorsV0
    for IdentityCreditTransferToAddressesTransition
{
    fn identity_id(&self) -> Identifier {
        match self {
            IdentityCreditTransferToAddressesTransition::V0(transition) => transition.identity_id,
        }
    }

    fn set_identity_id(&mut self, identity_id: Identifier) {
        match self {
            IdentityCreditTransferToAddressesTransition::V0(transition) => {
                transition.identity_id = identity_id;
            }
        }
    }

    fn recipient_keys(&self) -> &BTreeMap<KeyOfType, Credits> {
        match self {
            IdentityCreditTransferToAddressesTransition::V0(transition) => {
                &transition.recipient_keys
            }
        }
    }

    fn set_recipient_keys(&mut self, recipient_keys: BTreeMap<KeyOfType, Credits>) {
        match self {
            IdentityCreditTransferToAddressesTransition::V0(transition) => {
                transition.recipient_keys = recipient_keys;
            }
        }
    }

    fn set_nonce(&mut self, nonce: IdentityNonce) {
        match self {
            IdentityCreditTransferToAddressesTransition::V0(transition) => transition.nonce = nonce,
        }
    }

    fn nonce(&self) -> IdentityNonce {
        match self {
            IdentityCreditTransferToAddressesTransition::V0(transition) => transition.nonce,
        }
    }
}
