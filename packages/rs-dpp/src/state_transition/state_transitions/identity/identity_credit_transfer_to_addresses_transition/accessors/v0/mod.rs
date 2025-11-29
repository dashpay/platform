use std::collections::BTreeMap;

use crate::address_funds::PlatformAddress;
use crate::fee::Credits;
use crate::prelude::IdentityNonce;
use platform_value::Identifier;

pub trait IdentityCreditTransferToAddressesTransitionAccessorsV0 {
    fn identity_id(&self) -> Identifier;
    fn set_identity_id(&mut self, identity_id: Identifier);
    fn recipient_addresses(&self) -> &BTreeMap<PlatformAddress, Credits>;
    fn set_recipient_addresses(&mut self, recipient_addresses: BTreeMap<PlatformAddress, Credits>);
    fn set_nonce(&mut self, nonce: IdentityNonce);
    fn nonce(&self) -> IdentityNonce;
}
