use std::collections::BTreeMap;
use crate::prelude::IdentityNonce;

use platform_value::Identifier;
use crate::fee::Credits;
use crate::identity::KeyOfType;

pub trait IdentityCreditTransferToSingleKeyTransitionAccessorsV0 {
    fn identity_id(&self) -> Identifier;
    fn set_identity_id(&mut self, identity_id: Identifier);
    fn recipient_keys(&self) -> &BTreeMap<KeyOfType, Credits>;
    fn set_recipient_keys(&mut self, recipient_keys: BTreeMap<KeyOfType, Credits>);
    fn set_nonce(&mut self, nonce: IdentityNonce);
    fn nonce(&self) -> IdentityNonce;
}
