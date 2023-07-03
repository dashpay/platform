use std::fmt::Debug;
use crate::identity::{KeyID, SecurityLevel};
use crate::serialization_traits::Signable;
use crate::state_transition::{StateTransition, StateTransitionFieldTypes};


pub trait StateTransitionIdentitySigned
{
    fn signature_public_key_id(&self) -> Option<KeyID>;
    fn set_signature_public_key_id(&mut self, key_id: KeyID);
    fn security_level_requirement(&self) -> Vec<SecurityLevel>;
}