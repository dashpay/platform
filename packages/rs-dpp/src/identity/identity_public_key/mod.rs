#![allow(clippy::from_over_into)]

use crate::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use crate::identity::identity_public_key::v0::IdentityPublicKeyV0;
use bincode::{Decode, Encode};
use derive_more::From;
use serde::{Deserialize, Serialize};

mod key_type;
mod purpose;
mod security_level;
pub use key_type::KeyType;
pub use purpose::Purpose;
pub use security_level::SecurityLevel;
pub mod accessors;
pub(crate) mod conversion;
mod fields;
mod v0;
use crate::ProtocolError;
pub use fields::*;
use platform_serialization::{PlatformDeserialize, PlatformSerialize};

pub mod methods;
#[cfg(feature = "random-public-keys")]
mod random;

pub type KeyID = u32;
pub type KeyCount = KeyID;
pub type TimestampMillis = u64;

#[derive(
    Debug,
    Clone,
    Eq,
    PartialEq,
    Serialize,
    Deserialize,
    PlatformDeserialize,
    PlatformSerialize,
    From,
)]
#[platform_error_type(ProtocolError)]
#[platform_serialize(limit = 2000, allow_nested, untagged)]
pub enum IdentityPublicKey {
    V0(IdentityPublicKeyV0),
}

impl IdentityPublicKey {
    /// Checks if public key security level is MASTER
    pub fn is_master(&self) -> bool {
        self.security_level() == SecurityLevel::MASTER
    }
}

#[cfg(test)]
mod tests {
    use crate::identity::identity_public_key::v0::IdentityPublicKeyV0;
    use crate::identity::IdentityPublicKey;
    use crate::serialization_traits::{PlatformDeserializable, PlatformSerializable};
    use serde::Deserialize;

    #[test]
    fn test_identity_key_serialization_deserialization() {
        let key: IdentityPublicKey = IdentityPublicKeyV0::random_key(1, Some(500)).into();
        let serialized = key.serialize().expect("expected to serialize key");
        let unserialized = IdentityPublicKey::deserialize(serialized.as_slice())
            .expect("expected to deserialize key");
        assert_eq!(key, unserialized)
    }
}
