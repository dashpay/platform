use crate::prelude::IdentityNonce;
use crate::state_transition::public_key_in_creation::IdentityPublicKeyInCreation;
use crate::{
    identity::KeyID,
    prelude::{Identifier, Revision},
};

pub trait IdentityUpdateTransitionAccessorsV0 {
    fn set_identity_id(&mut self, id: Identifier);
    fn identity_id(&self) -> Identifier;
    fn set_revision(&mut self, revision: Revision);
    fn revision(&self) -> Revision;
    fn set_nonce(&mut self, nonce: IdentityNonce);
    fn nonce(&self) -> IdentityNonce;
    fn set_public_keys_to_add(&mut self, add_public_keys: Vec<IdentityPublicKeyInCreation>);
    fn public_keys_to_add(&self) -> &[IdentityPublicKeyInCreation];
    fn public_keys_to_add_mut(&mut self) -> &mut [IdentityPublicKeyInCreation];
    fn set_public_key_ids_to_disable(&mut self, disable_public_keys: Vec<KeyID>);
    fn public_key_ids_to_disable(&self) -> &[KeyID];
    fn owner_id(&self) -> Identifier;
}
