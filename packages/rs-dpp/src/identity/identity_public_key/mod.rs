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
pub use key_type::KeyOfType;
pub use purpose::Purpose;
pub use security_level::SecurityLevel;
pub mod accessors;
pub(crate) mod conversion;
pub mod fields;
pub mod v0;
use crate::version::PlatformVersion;
use crate::ProtocolError;
pub use fields::*;
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};

pub mod methods;
pub use methods::*;
pub mod contract_bounds;
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
    Encode,
    Decode,
    PlatformDeserialize,
    PlatformSerialize,
    From,
    Hash,
    Ord,
    PartialOrd,
)]
#[platform_serialize(limit = 2000, unversioned)] //This is not platform versioned automatically
#[serde(tag = "$version")]
pub enum IdentityPublicKey {
    #[serde(rename = "0")]
    V0(IdentityPublicKeyV0),
}

impl IdentityPublicKey {
    /// Checks if public key security level is MASTER
    pub fn is_master(&self) -> bool {
        self.security_level() == SecurityLevel::MASTER
    }

    /// Generates an identity public key with the maximum possible size based on the platform version.
    ///
    /// This method constructs a key of the largest possible size for the given platform version.
    /// This can be useful for stress testing or benchmarking purposes.
    ///
    /// # Parameters
    ///
    /// * `id`: The `KeyID` for the generated key.
    /// * `platform_version`: The platform version which determines the structure of the identity key.
    ///
    /// # Returns
    ///
    /// * `Self`: An instance of the `IdentityPublicKey` struct.
    ///
    pub fn max_possible_size_key(
        id: KeyID,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError> {
        match platform_version
            .dpp
            .identity_versions
            .identity_key_structure_version
        {
            0 => Ok(IdentityPublicKeyV0::max_possible_size_key(id).into()),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "IdentityPublicKey::max_possible_size_key".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }

    pub fn default_versioned(platform_version: &PlatformVersion) -> Result<Self, ProtocolError> {
        match platform_version
            .dpp
            .identity_versions
            .identity_key_structure_version
        {
            0 => Ok(IdentityPublicKeyV0::default().into()),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "IdentityPublicKey::default_versioned".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::identity::identity_public_key::v0::IdentityPublicKeyV0;
    use crate::identity::IdentityPublicKey;
    use crate::serialization::{PlatformDeserializable, PlatformSerializable};
    use platform_version::version::LATEST_PLATFORM_VERSION;
    use rand::SeedableRng;

    #[test]
    fn test_identity_key_serialization_deserialization() {
        let mut rng = rand::rngs::StdRng::from_entropy();
        let key: IdentityPublicKey =
            IdentityPublicKeyV0::random_ecdsa_master_authentication_key_with_rng(
                1,
                &mut rng,
                LATEST_PLATFORM_VERSION,
            )
            .expect("expected a random key")
            .0
            .into();
        let serialized = key.serialize_to_bytes().expect("expected to serialize key");
        let unserialized: IdentityPublicKey =
            PlatformDeserializable::deserialize_from_bytes(serialized.as_slice())
                .expect("expected to deserialize key");
        assert_eq!(key, unserialized)
    }
}
