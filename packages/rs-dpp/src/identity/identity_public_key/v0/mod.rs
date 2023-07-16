mod accessors;
mod conversion;
mod methods;

use std::convert::{TryFrom, TryInto};

use crate::serialization_traits::{PlatformDeserializable, PlatformSerializable};

use platform_value::{BinaryData, Bytes20, ReplacementType, Value};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::Value as JsonValue;

use crate::errors::{InvalidVectorSizeError, ProtocolError};
pub use crate::identity::KeyType;
pub use crate::identity::Purpose;
pub use crate::identity::SecurityLevel;
use crate::util::hash::ripemd160_sha256;
use crate::Convertible;
use bincode::{config, Decode, Encode};
use dashcore::hashes::Hash;

use crate::identity::{KeyID, TimestampMillis};
use crate::util::vec;
use platform_serialization::{PlatformDeserialize, PlatformSerialize};

#[derive(
    Debug, Serialize, Deserialize, Encode, Decode, Clone, PartialEq, Eq, Ord, PartialOrd, Hash,
)]
#[serde(rename_all = "camelCase")]
pub struct IdentityPublicKeyV0 {
    pub id: KeyID,
    pub purpose: Purpose,
    pub security_level: SecurityLevel,
    #[serde(rename = "type")]
    pub key_type: KeyType,
    pub read_only: bool,
    pub data: BinaryData,
    #[serde(default)]
    pub disabled_at: Option<TimestampMillis>,
}

#[cfg(feature = "state-transitions")]
impl Into<IdentityPublicKeyV0InCreation> for &IdentityPublicKeyV0 {
    fn into(self) -> IdentityPublicKeyV0InCreation {
        IdentityPublicKeyV0InCreation {
            id: self.id,
            purpose: self.purpose,
            security_level: self.security_level,
            key_type: self.key_type,
            read_only: self.read_only,
            data: self.data.clone(),
            signature: BinaryData::default(),
        }
    }
}
