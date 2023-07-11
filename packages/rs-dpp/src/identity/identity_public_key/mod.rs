#![allow(clippy::from_over_into)]

pub mod key_type;
pub mod purpose;
pub mod security_level;

use std::convert::{TryFrom, TryInto};

use crate::serialization_traits::{PlatformDeserializable, PlatformSerializable};
use anyhow::anyhow;
#[cfg(feature = "cbor")]
use ciborium::value::Value as CborValue;
use dashcore::PublicKey as ECDSAPublicKey;
use platform_value::{BinaryData, Bytes20, ReplacementType, Value};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::Value as JsonValue;

use crate::errors::{InvalidVectorSizeError, ProtocolError};
pub use crate::identity::key_type::KeyType;
pub use crate::identity::purpose::Purpose;
pub use crate::identity::security_level::SecurityLevel;
#[cfg(feature = "cbor")]
use crate::util::cbor_serializer;
#[cfg(feature = "cbor")]
use crate::util::cbor_value::{CborCanonicalMap, CborMapExtension};
use crate::util::hash::ripemd160_sha256;
use crate::Convertible;
use bincode::{config, Decode, Encode};
use dashcore::hashes::Hash;

#[cfg(feature = "state-transitions")]
use crate::state_transition::identity_public_key_transitions::IdentityPublicKeyInCreation;
use crate::util::vec;
use platform_serialization::{PlatformDeserialize, PlatformSerialize};

pub type KeyID = u32;
pub type TimestampMillis = u64;

pub const BINARY_DATA_FIELDS: [&str; 1] = ["data"];

#[derive(
    Debug,
    Serialize,
    Deserialize,
    Decode,
    PlatformDeserialize,
    PlatformSerialize,
    Clone,
    PartialEq,
    Eq,
    Ord,
    PartialOrd,
    Hash,
)]
#[serde(rename_all = "camelCase")]
#[platform_error_type(ProtocolError)]
#[platform_deserialize_limit(2000)]
#[platform_serialize(limit = 2000, allow_nested)]
pub struct IdentityPublicKey {
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
impl Into<IdentityPublicKeyInCreation> for &IdentityPublicKey {
    fn into(self) -> IdentityPublicKeyInCreation {
        IdentityPublicKeyInCreation {
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

impl Convertible for IdentityPublicKey {
    #[cfg(feature = "platform-value")]
    fn to_object(&self) -> Result<Value, ProtocolError> {
        platform_value::to_value(self).map_err(ProtocolError::ValueError)
    }

    #[cfg(feature = "platform-value")]
    fn to_cleaned_object(&self) -> Result<Value, ProtocolError> {
        let mut value = platform_value::to_value(self).map_err(ProtocolError::ValueError)?;
        if self.disabled_at.is_none() {
            value
                .remove("disabledAt")
                .map_err(ProtocolError::ValueError)?;
        }
        Ok(value)
    }

    #[cfg(feature = "platform-value")]
    fn into_object(self) -> Result<Value, ProtocolError> {
        platform_value::to_value(self).map_err(ProtocolError::ValueError)
    }

    #[cfg(feature = "json-object")]
    fn to_json_object(&self) -> Result<JsonValue, ProtocolError> {
        self.to_cleaned_object()?
            .try_into_validating_json()
            .map_err(ProtocolError::ValueError)
    }

    #[cfg(feature = "json-object")]
    fn to_json(&self) -> Result<JsonValue, ProtocolError> {
        self.to_cleaned_object()?
            .try_into()
            .map_err(ProtocolError::ValueError)
    }

    #[cfg(feature = "cbor")]
    fn to_cbor_buffer(&self) -> Result<Vec<u8>, ProtocolError> {
        let mut object = self.to_cleaned_object()?;
        object
            .to_map_mut()
            .unwrap()
            .sort_by_lexicographical_byte_ordering_keys_and_inner_maps();

        cbor_serializer::serializable_value_to_cbor(&object, None)
    }
}

impl IdentityPublicKey {
    /// Set disabledAt
    pub fn set_disabled_at(&mut self, timestamp_millis: u64) {
        self.disabled_at = Some(timestamp_millis);
    }

    /// Is public key disabled
    pub fn is_disabled(&self) -> bool {
        self.disabled_at.is_some()
    }

    /// Checks if public key security level is MASTER
    pub fn is_master(&self) -> bool {
        self.security_level == SecurityLevel::MASTER
    }

    /// Get the original public key hash
    pub fn hash(&self) -> Result<[u8; 20], ProtocolError> {
        if self.data.is_empty() {
            return Err(ProtocolError::EmptyPublicKeyDataError);
        }

        match self.key_type {
            KeyType::ECDSA_SECP256K1 => {
                let key = match self.data.len() {
                    // TODO: We need to update schema and tests for 65 len keys
                    65 | 33 => ECDSAPublicKey::from_slice(self.data.as_slice())
                        .map_err(|e| anyhow!("unable to create pub key - {}", e))?,
                    _ => {
                        return Err(ProtocolError::ParsingError(format!(
                            "the key length is invalid: {} Allowed sizes: 33 or 65 bytes for ecdsa key",
                            self.data.len()
                        )));
                    }
                };
                Ok(key.pubkey_hash().into_inner())
            }
            KeyType::BLS12_381 => {
                if self.data.len() != 48 {
                    Err(ProtocolError::ParsingError(format!(
                        "the key length is invalid: {} Allowed sizes: 48 bytes for bls key",
                        self.data.len()
                    )))
                } else {
                    Ok(ripemd160_sha256(self.data.as_slice()))
                }
            }
            KeyType::ECDSA_HASH160 | KeyType::BIP13_SCRIPT_HASH | KeyType::EDDSA_25519_HASH160 => {
                Ok(Bytes20::from_vec(self.data.to_vec())?.into_buffer())
            }
        }
    }

    pub fn as_ecdsa_array(&self) -> Result<[u8; 33], InvalidVectorSizeError> {
        vec::vec_to_array::<33>(self.data.as_slice())
    }

    #[cfg(feature = "platform-value")]
    pub fn from_value(value: Value) -> Result<IdentityPublicKey, ProtocolError> {
        value.try_into().map_err(ProtocolError::ValueError)
    }

    #[cfg(feature = "json-object")]
    pub fn from_json_object(raw_object: JsonValue) -> Result<IdentityPublicKey, ProtocolError> {
        let mut value: Value = raw_object.into();
        value.replace_at_paths(BINARY_DATA_FIELDS, ReplacementType::BinaryBytes)?;
        Self::from_value(value)
    }

    #[cfg(feature = "cbor")]
    pub fn from_cbor_value(cbor_value: &CborValue) -> Result<Self, ProtocolError> {
        let key_value_map = cbor_value.as_map().ok_or_else(|| {
            ProtocolError::DecodingError(String::from(
                "Expected identity public key to be a key value map",
            ))
        })?;

        let id = key_value_map.as_u16("id", "A key must have an uint16 id")?;
        let key_type = key_value_map.as_u8("type", "Identity public key must have a type")?;
        let purpose = key_value_map.as_u8("purpose", "Identity public key must have a purpose")?;
        let security_level = key_value_map.as_u8(
            "securityLevel",
            "Identity public key must have a securityLevel",
        )?;
        let readonly =
            key_value_map.as_bool("readOnly", "Identity public key must have a readOnly")?;
        let public_key_bytes =
            key_value_map.as_bytes("data", "Identity public key must have a data")?;
        let disabled_at = key_value_map.as_u64("disabledAt", "").ok();

        Ok(IdentityPublicKey {
            id: id.into(),
            purpose: purpose.try_into()?,
            security_level: security_level.try_into()?,
            key_type: key_type.try_into()?,
            data: BinaryData::new(public_key_bytes),
            read_only: readonly,
            disabled_at,
        })
    }

    #[cfg(feature = "cbor")]
    pub fn to_cbor_value(&self) -> CborValue {
        let mut pk_map = CborCanonicalMap::new();

        pk_map.insert("id", self.id);
        pk_map.insert("data", self.data.as_slice());
        pk_map.insert("type", self.key_type);
        pk_map.insert("purpose", self.purpose);
        pk_map.insert("readOnly", self.read_only);
        pk_map.insert("securityLevel", self.security_level);
        if let Some(ts) = self.disabled_at {
            pk_map.insert("disabledAt", ts)
        }

        pk_map.to_value_sorted()
    }
}

#[cfg(feature = "cbor")]
impl Into<CborValue> for &IdentityPublicKey {
    fn into(self) -> CborValue {
        self.to_cbor_value()
    }
}

impl TryFrom<&IdentityPublicKey> for Value {
    type Error = platform_value::Error;

    fn try_from(value: &IdentityPublicKey) -> Result<Self, Self::Error> {
        platform_value::to_value(value)
    }
}

impl TryFrom<IdentityPublicKey> for Value {
    type Error = platform_value::Error;

    fn try_from(value: IdentityPublicKey) -> Result<Self, Self::Error> {
        platform_value::to_value(value)
    }
}

impl TryFrom<Value> for IdentityPublicKey {
    type Error = platform_value::Error;

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        platform_value::from_value(value)
    }
}

impl TryFrom<&str> for IdentityPublicKey {
    type Error = ProtocolError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let mut platform_value: Value = serde_json::from_str::<JsonValue>(value)
            .map_err(|e| ProtocolError::StringDecodeError(e.to_string()))?
            .into();
        platform_value.replace_at_paths(BINARY_DATA_FIELDS, ReplacementType::BinaryBytes)?;
        platform_value.try_into().map_err(ProtocolError::ValueError)
    }
}

pub fn de_base64_to_vec<'de, D: Deserializer<'de>>(d: D) -> Result<Vec<u8>, D::Error> {
    let data: String = Deserialize::deserialize(d)?;
    base64::decode(data)
        .map_err(|e| serde::de::Error::custom(format!("unable to decode from bas64 - {}", e)))
}

pub fn se_vec_to_base64<S>(buffer: &Vec<u8>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_str(&base64::encode(buffer))
}

#[cfg(test)]
mod tests {
    use crate::identity::IdentityPublicKey;
    use crate::serialization_traits::{PlatformDeserializable, PlatformSerializable};

    #[test]
    fn test_identity_key_serialization_deserialization() {
        let key = IdentityPublicKey::random_key(1, Some(500));
        let serialized = key.serialize().expect("expected to serialize key");
        let unserialized = IdentityPublicKey::deserialize(serialized.as_slice())
            .expect("expected to deserialize key");
        assert_eq!(key, unserialized)
    }
}
