use std::fmt::Debug;
use crate::identity::KeyID;
use crate::serialization_traits::Signable;
use crate::state_transition::{StateTransition, StateTransitionFieldTypes};


pub trait StateTransitionIdentitySigned
{
    fn get_signature_public_key_id(&self) -> Option<KeyID>;
    fn set_signature_public_key_id(&mut self, key_id: KeyID);
}