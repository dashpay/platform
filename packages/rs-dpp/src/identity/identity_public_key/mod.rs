#![allow(clippy::from_over_into)]

use crate::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use crate::identity::identity_public_key::v0::IdentityPublicKeyV0;
use bincode::{Decode, Encode};
use serde::{Deserialize, Serialize};

mod key_type;
mod purpose;
mod security_level;
pub use key_type::KeyType;
pub use purpose::Purpose;
pub use security_level::SecurityLevel;
pub mod accessors;
mod conversion;
mod fields;
mod v0;
pub use fields::*;
pub mod methods;

pub type KeyID = u32;
pub type TimestampMillis = u64;

#[derive(Debug, Clone, Eq, PartialEq, Encode, Decode, Serialize, Deserialize)]
pub enum IdentityPublicKey {
    V0(IdentityPublicKeyV0),
}

impl IdentityPublicKey {
    /// Checks if public key security level is MASTER
    pub fn is_master(&self) -> bool {
        self.security_level() == SecurityLevel::MASTER
    }
}
