use crate::identity::IdentityPublicKey;
use crate::state_transition::public_key_in_creation::v0::IdentityPublicKeyInCreationV0;
use crate::state_transition::public_key_in_creation::v0::IdentityPublicKeyInCreationV0Signable;
use crate::ProtocolError;
use bincode::{config, Decode, Encode};
use platform_serialization_derive::PlatformSignable;
use serde::{Deserialize, Serialize};

pub mod accessors;
mod fields;
pub mod v0;
mod v0_methods;

#[derive(Debug, Serialize, Deserialize, Encode, Decode, PlatformSignable, Clone, PartialEq, Eq)]

pub enum IdentityPublicKeyInCreation {
    V0(IdentityPublicKeyInCreationV0),
}

impl From<&IdentityPublicKeyInCreation> for IdentityPublicKey {
    fn from(val: &IdentityPublicKeyInCreation) -> Self {
        match val {
            val => val.into(),
        }
    }
}

impl From<IdentityPublicKey> for IdentityPublicKeyInCreation {
    fn from(val: IdentityPublicKey) -> Self {
        match val {
            IdentityPublicKey::V0(v0) => v0.into(),
        }
    }
}
