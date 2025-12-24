mod v0;

use std::collections::BTreeMap;
pub use v0::*;

use crate::address_funds::PlatformAddress;
use crate::fee::Credits;
use crate::prelude::IdentityNonce;
use crate::state_transition::identity_credit_transfer_to_addresses_transition::IdentityCreditTransferToAddressesTransition;
use platform_value::Identifier;

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

    fn recipient_addresses(&self) -> &BTreeMap<PlatformAddress, Credits> {
        match self {
            IdentityCreditTransferToAddressesTransition::V0(transition) => {
                &transition.recipient_addresses
            }
        }
    }

    fn set_recipient_addresses(&mut self, recipient_addresses: BTreeMap<PlatformAddress, Credits>) {
        match self {
            IdentityCreditTransferToAddressesTransition::V0(transition) => {
                transition.recipient_addresses = recipient_addresses;
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
