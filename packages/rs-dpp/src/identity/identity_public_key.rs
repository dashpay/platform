#![allow(clippy::from_over_into)]

use std::convert::TryInto;
use std::{collections::HashMap, convert::TryFrom, hash::Hash};

use anyhow::{anyhow, bail};
use ciborium::value::Value as CborValue;
use dashcore::PublicKey;
use lazy_static::lazy_static;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::Value as JsonValue;
use serde_repr::{Deserialize_repr, Serialize_repr};

use crate::errors::{InvalidVectorSizeError, ProtocolError};
use crate::util::cbor_value::{CborCanonicalMap, CborMapExtension};
use crate::util::json_value::{JsonValueExt, ReplaceWith};
use crate::util::vec;
use crate::SerdeParsingError;

pub type KeyID = u64;
pub type TimestampMillis = u64;

#[allow(non_camel_case_types)]
#[repr(u8)]
#[derive(Debug, PartialEq, Eq, Clone, Copy, Serialize_repr, Deserialize_repr)]
pub enum KeyType {
    ECDSA_SECP256K1 = 0,
    BLS12_381 = 1,
    ECDSA_HASH160 = 2,
    BIP13_SCRIPT_HASH = 3,
}

impl std::default::Default for KeyType {
    fn default() -> Self {
        KeyType::ECDSA_SECP256K1
    }
}

impl std::fmt::Display for KeyType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl TryFrom<u8> for KeyType {
    type Error = anyhow::Error;
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::ECDSA_SECP256K1),
            1 => Ok(Self::BLS12_381),
            2 => Ok(Self::ECDSA_HASH160),
            3 => Ok(Self::BIP13_SCRIPT_HASH),
            value => bail!("unrecognized security level: {}", value),
        }
    }
}

impl Into<CborValue> for KeyType {
    fn into(self) -> CborValue {
        CborValue::from(self as u128)
    }
}

pub const BINARY_DATA_FIELDS: [&str; 2] = ["data", "signature"];

#[repr(u8)]
#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash, Serialize_repr, Deserialize_repr)]
pub enum Purpose {
    /// at least one authentication key must be registered for all security levels
    AUTHENTICATION = 0,
    /// this key cannot be used for signing documents
    ENCRYPTION = 1,
    /// this key cannot be used for signing documents
    DECRYPTION = 2,
    WITHDRAW = 3,
}

impl TryFrom<u8> for Purpose {
    type Error = anyhow::Error;
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::AUTHENTICATION),
            1 => Ok(Self::ENCRYPTION),
            2 => Ok(Self::DECRYPTION),
            value => bail!("unrecognized security level: {}", value),
        }
    }
}

impl Into<CborValue> for Purpose {
    fn into(self) -> CborValue {
        CborValue::from(self as u128)
    }
}
impl std::fmt::Display for Purpose {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[repr(u8)]
#[derive(
    Debug, PartialEq, Eq, Clone, Copy, Hash, Serialize_repr, Deserialize_repr, PartialOrd, Ord,
)]
pub enum SecurityLevel {
    MASTER = 0,
    CRITICAL = 1,
    HIGH = 2,
    MEDIUM = 3,
}

impl TryFrom<usize> for SecurityLevel {
    type Error = anyhow::Error;
    fn try_from(value: usize) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::MASTER),
            1 => Ok(Self::CRITICAL),
            2 => Ok(Self::HIGH),
            3 => Ok(Self::MEDIUM),
            value => bail!("unrecognized security level: {}", value),
        }
    }
}

impl Into<CborValue> for SecurityLevel {
    fn into(self) -> CborValue {
        CborValue::from(self as u128)
    }
}

impl TryFrom<u8> for SecurityLevel {
    type Error = anyhow::Error;
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::MASTER),
            1 => Ok(Self::CRITICAL),
            2 => Ok(Self::HIGH),
            3 => Ok(Self::MEDIUM),
            value => bail!("unrecognized security level: {}", value),
        }
    }
}

impl SecurityLevel {
    pub fn lowest_level() -> SecurityLevel {
        Self::MEDIUM
    }
    pub fn highest_level() -> SecurityLevel {
        Self::MASTER
    }
}

impl std::fmt::Display for SecurityLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

lazy_static! {
    pub static ref ALLOWED_SECURITY_LEVELS: HashMap<Purpose, Vec<SecurityLevel>> = {
        let mut m = HashMap::new();
        m.insert(
            Purpose::AUTHENTICATION,
            vec![
                SecurityLevel::MASTER,
                SecurityLevel::CRITICAL,
                SecurityLevel::HIGH,
                SecurityLevel::MEDIUM,
            ],
        );
        m.insert(Purpose::ENCRYPTION, vec![SecurityLevel::MEDIUM]);
        m.insert(Purpose::DECRYPTION, vec![SecurityLevel::MEDIUM]);
        m.insert(Purpose::WITHDRAW, vec![SecurityLevel::CRITICAL]);
        m
    };
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct IdentityPublicKey {
    pub id: KeyID,
    pub purpose: Purpose,
    pub security_level: SecurityLevel,
    #[serde(rename = "type")]
    pub key_type: KeyType,
    pub data: Vec<u8>,
    pub read_only: bool,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub disabled_at: Option<TimestampMillis>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub signature: Vec<u8>,
}

//? do we really need that???
#[derive(Serialize, Deserialize, Debug)]
pub struct JsonIdentityPublicKey {
    pub id: KeyID,
    pub purpose: Purpose,
    pub security_level: SecurityLevel,
    pub key_type: KeyType,
    pub data: String,
    pub read_only: bool,
}

impl std::convert::Into<JsonIdentityPublicKey> for &IdentityPublicKey {
    fn into(self) -> JsonIdentityPublicKey {
        JsonIdentityPublicKey {
            id: self.id,
            purpose: self.purpose,
            security_level: self.security_level,
            key_type: self.key_type,
            read_only: self.read_only,
            data: base64::encode(&self.data),
        }
    }
}

impl IdentityPublicKey {
    /// Get key ID
    pub fn get_id(&self) -> KeyID {
        self.id
    }

    /// Set key ID
    pub fn set_id(mut self, id: KeyID) -> Self {
        self.id = id;
        self
    }

    /// Get key type
    pub fn get_type(&self) -> KeyType {
        self.key_type
    }

    /// Set key type
    pub fn set_type(mut self, key_type: KeyType) -> Self {
        self.key_type = key_type;
        self
    }

    /// Get raw public key
    pub fn get_data(&self) -> &[u8] {
        &self.data
    }

    /// Set raw public key
    pub fn set_data(mut self, data: Vec<u8>) -> Self {
        self.data = data;
        self
    }

    /// Get the purpose value
    pub fn get_purpose(&self) -> Purpose {
        self.purpose
    }

    /// Set the purpose value
    pub fn set_purpose(mut self, purpose: Purpose) -> Self {
        self.purpose = purpose;
        self
    }

    /// Get the raw security level value. A uint8 number
    pub fn get_security_level(&self) -> SecurityLevel {
        self.security_level
    }

    /// Set the raw security level
    //? maybe we should replace the enum with impl TryInto<SecurityLevel> or Into<SecurityLevel>
    pub fn set_security_level(mut self, security_level: SecurityLevel) -> Self {
        self.security_level = security_level;
        self
    }

    /// Get readOnly flag
    pub fn get_readonly(&self) -> bool {
        self.read_only
    }

    /// Set readOnly flag
    pub fn set_readonly(mut self, ro: bool) -> Self {
        self.read_only = ro;
        self
    }

    /// Get disabledAt
    pub fn get_disabled_at(&self) -> Option<TimestampMillis> {
        self.disabled_at
    }

    /// Set disabledAt
    pub fn set_disabled_at(mut self, timestamp_millis: u64) -> Self {
        self.disabled_at = Some(timestamp_millis);
        self
    }

    /// Checks if public key security level is MASTER
    pub fn is_master(&self) -> bool {
        self.security_level == SecurityLevel::MASTER
    }

    /// Returns the signature
    pub fn get_signature(&self) -> &[u8] {
        &self.signature
    }

    /// Set the signature
    pub fn set_signature(&mut self, signature: Vec<u8>) {
        self.signature = signature;
    }

    /// Get the original public key hash
    pub fn hash(&self) -> Result<Vec<u8>, ProtocolError> {
        if self.data.is_empty() {
            return Err(ProtocolError::EmptyPublicKeyDataError);
        }

        if self.key_type == KeyType::ECDSA_HASH160 || self.key_type == KeyType::BIP13_SCRIPT_HASH {
            return Ok(self.data.clone());
        }

        let original_key = match self.data.len() {
            65 => {
                let public_key = vec::vec_to_array::<65>(&self.data)
                    .map_err(|e| ProtocolError::ParsingError(e.to_string()))?;

                PublicKey::from_slice(&public_key)
                    .map_err(|e| anyhow!("unable to create pub key - {}", e))?
            }

            33 => {
                let public_key = vec::vec_to_array::<33>(&self.data)
                    .map_err(|e| ProtocolError::ParsingError(e.to_string()))?;
                PublicKey::from_slice(&public_key)
                    .map_err(|e| anyhow!("unable to create pub key - {}", e))?
            }
            _ => {
                return Err(ProtocolError::ParsingError(format!(
                    "the key length is invalid: {} Allowed sizes: 33 or 65 bytes",
                    self.data.len()
                )));
            }
        };
        Ok(original_key.pubkey_hash().to_vec())
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
    pub fn to_raw_json_object(
        &self,
        skip_signatures: bool,
    ) -> Result<JsonValue, SerdeParsingError> {
        let mut value = serde_json::to_value(&self)?;
        if skip_signatures {
            let _ = value.remove("signature");
        }

        Ok(value)
    }

    /// Return json with all binary data converted to base64
    pub fn to_json(&self) -> Result<JsonValue, SerdeParsingError> {
        let mut value = self.to_raw_json_object(false)?;

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
        let signature_bytes = key_value_map.as_bytes("signature", "").unwrap_or_default();
        let disabled_at = key_value_map.as_u64("disabledAt", "").ok();

        Ok(IdentityPublicKey {
            id: id.into(),
            purpose: purpose.try_into()?,
            security_level: security_level.try_into()?,
            key_type: key_type.try_into()?,
            data: public_key_bytes,
            read_only: readonly,
            disabled_at,
            signature: signature_bytes,
        })
    }

    pub fn to_cbor_value(&self) -> CborValue {
        let mut pk_map = CborCanonicalMap::new();

        pk_map.insert("id", self.get_id());
        pk_map.insert("data", self.get_data());
        pk_map.insert("type", self.get_type());
        pk_map.insert("purpose", self.get_purpose());
        pk_map.insert("readOnly", self.get_readonly());
        pk_map.insert("securityLevel", self.get_security_level());
        if let Some(ts) = self.get_disabled_at() {
            pk_map.insert("disabledAt", ts)
        }

        if !self.get_signature().is_empty() {
            pk_map.insert("signature", self.get_signature().to_owned())
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
