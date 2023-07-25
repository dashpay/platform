use crate::platform_serialization::PlatformSignable;
use crate::state_transition::public_key_in_creation::v0::IdentityPublicKeyInCreationV0;
use crate::state_transition::public_key_in_creation::v0::IdentityPublicKeyInCreationV0Signable;
use crate::ProtocolError;
use bincode::{config, Decode, Encode};
use serde::{Deserialize, Serialize};

pub mod accessors;
mod fields;
pub mod v0;
mod v0_methods;

#[derive(Debug, Serialize, Deserialize, Encode, Decode, PlatformSignable, Clone, PartialEq, Eq)]
#[platform_error_type(ProtocolError)]
pub enum IdentityPublicKeyInCreation {
    V0(IdentityPublicKeyInCreationV0),
}
