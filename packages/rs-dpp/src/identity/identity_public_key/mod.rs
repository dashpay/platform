#![allow(clippy::from_over_into)]

pub mod factory;
mod in_creation;
pub mod key_type;
pub mod purpose;
pub mod security_level;
pub mod serialize;

use std::convert::TryInto;

use anyhow::anyhow;
use ciborium::value::Value as CborValue;
use dashcore::PublicKey as ECDSAPublicKey;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::Value as JsonValue;

use crate::errors::{InvalidVectorSizeError, ProtocolError};
pub use crate::identity::key_type::KeyType;
pub use crate::identity::purpose::Purpose;
pub use crate::identity::security_level::SecurityLevel;
use crate::util::cbor_value::{CborCanonicalMap, CborMapExtension};
use crate::util::hash::ripemd160_sha256;
use crate::util::json_value::{JsonValueExt, ReplaceWith};
use crate::util::vec;
use crate::SerdeParsingError;
use bincode::{deserialize, serialize};

pub use in_creation::IdentityPublicKeyInCreation;

pub type KeyID = u32;
pub type TimestampMillis = u64;

pub const BINARY_DATA_FIELDS: [&str; 2] = ["data", "signature"];

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct IdentityPublicKey {
    pub id: KeyID,
    pub purpose: Purpose,
    pub security_level: SecurityLevel,
    #[serde(rename = "type")]
    pub key_type: KeyType,
    pub read_only: bool,
    pub data: Vec<u8>,
    #[serde(default)]
    pub disabled_at: Option<TimestampMillis>,
}

impl std::convert::Into<IdentityPublicKeyInCreation> for &IdentityPublicKey {
    fn into(self) -> IdentityPublicKeyInCreation {
        IdentityPublicKeyInCreation {
            id: self.id,
            purpose: self.purpose,
            security_level: self.security_level,
            key_type: self.key_type,
            read_only: self.read_only,
            data: self.data.clone(),
            signature: vec![],
        }
    }
}

impl IdentityPublicKey {
    /// Get raw public key
    pub fn get_data(&self) -> &[u8] {
        &self.data
    }

    /// Set raw public key
    pub fn set_data(&mut self, data: Vec<u8>) {
        self.data = data;
    }

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
    pub fn hash(&self) -> Result<Vec<u8>, ProtocolError> {
        if self.data.is_empty() {
            return Err(ProtocolError::EmptyPublicKeyDataError);
        }

        match self.key_type {
            KeyType::ECDSA_SECP256K1 => {
                let key = match self.data.len() {
                    // TODO: We need to update schema and tests for 65 len keys
                    65 | 33 => ECDSAPublicKey::from_slice(&self.data.as_slice())
                        .map_err(|e| anyhow!("unable to create pub key - {}", e))?,
                    _ => {
                        return Err(ProtocolError::ParsingError(format!(
                            "the key length is invalid: {} Allowed sizes: 33 or 65 bytes for ecdsa key",
                            self.data.len()
                        )));
                    }
                };
                Ok(key.pubkey_hash().to_vec())
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
            KeyType::ECDSA_HASH160 | KeyType::BIP13_SCRIPT_HASH => Ok(self.data.clone()),
        }
    }

    pub fn as_ecdsa_array(&self) -> Result<[u8; 33], InvalidVectorSizeError> {
        vec::vec_to_array::<33>(&self.data)
    }

    pub fn from_raw_object(raw_object: JsonValue) -> Result<IdentityPublicKey, ProtocolError> {
        let identity_public_key: IdentityPublicKey = serde_json::from_value(raw_object)?;
        Ok(identity_public_key)
    }

    pub fn from_json_object(mut raw_object: JsonValue) -> Result<IdentityPublicKey, ProtocolError> {
        raw_object.replace_binary_paths(BINARY_DATA_FIELDS, ReplaceWith::Bytes)?;
        let identity_public_key: IdentityPublicKey = serde_json::from_value(raw_object)?;

        Ok(identity_public_key)
    }

    /// Return raw data, with all binary fields represented as arrays
    pub fn to_raw_json_object(&self) -> Result<JsonValue, SerdeParsingError> {
        let mut value = serde_json::to_value(&self)?;

        Ok(value)
    }

    /// Return json with all binary data converted to base64
    pub fn to_json(&self) -> Result<JsonValue, SerdeParsingError> {
        let mut value = self.to_raw_json_object()?;

        value.replace_binary_paths(BINARY_DATA_FIELDS, ReplaceWith::Base64)?;

        Ok(value)
    }

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
            data: public_key_bytes,
            read_only: readonly,
            disabled_at,
        })
    }

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

impl Into<CborValue> for &IdentityPublicKey {
    fn into(self) -> CborValue {
        self.to_cbor_value()
    }
}

pub fn de_base64_to_vec<'de, D: Deserializer<'de>>(d: D) -> Result<Vec<u8>, D::Error> {
    let data: String = Deserialize::deserialize(d)?;
    base64::decode(&data)
        .map_err(|e| serde::de::Error::custom(format!("unable to decode from bas64 - {}", e)))
}

pub fn se_vec_to_base64<S>(buffer: &Vec<u8>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_str(&base64::encode(&buffer))
}
