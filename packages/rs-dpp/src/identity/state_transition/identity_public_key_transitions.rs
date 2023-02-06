use crate::identity::{IdentityPublicKey, KeyID, KeyType, Purpose, SecurityLevel};
use ciborium::value::Value as CborValue;
use std::convert::TryInto;

use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

use crate::errors::ProtocolError;
use crate::util::cbor_value::{CborCanonicalMap, CborMapExtension};
use crate::util::json_value::{JsonValueExt, ReplaceWith};
use crate::SerdeParsingError;

pub const BINARY_DATA_FIELDS: [&str; 2] = ["data", "signature"];

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct IdentityPublicKeyCreateTransition {
    pub id: KeyID,
    pub purpose: Purpose,
    pub security_level: SecurityLevel,
    #[serde(rename = "type")]
    pub key_type: KeyType,
    pub data: Vec<u8>,
    pub read_only: bool,
    /// The signature is needed for ECDSA_SECP256K1 Key type and BLS12_381 Key type
    pub signature: Vec<u8>,
}

impl IdentityPublicKeyCreateTransition {
    pub fn to_identity_public_key(self) -> IdentityPublicKey {
        let Self {
            id,
            purpose,
            security_level,
            key_type,
            data,
            read_only,
            ..
        } = self;
        IdentityPublicKey {
            id,
            purpose,
            security_level,
            key_type,
            data,
            read_only,
            disabled_at: None,
        }
    }

    pub fn from_raw_object(raw_object: JsonValue) -> Result<Self, ProtocolError> {
        let identity_public_key: Self = serde_json::from_value(raw_object)?;
        Ok(identity_public_key)
    }

    pub fn from_json_object(mut raw_object: JsonValue) -> Result<Self, ProtocolError> {
        raw_object.replace_binary_paths(BINARY_DATA_FIELDS, ReplaceWith::Bytes)?;
        let identity_public_key: Self = serde_json::from_value(raw_object)?;

        Ok(identity_public_key)
    }

    /// Return raw data, with all binary fields represented as arrays
    pub fn to_raw_json_object(&self, skip_signature: bool) -> Result<JsonValue, SerdeParsingError> {
        let mut value = serde_json::to_value(self)?;

        if skip_signature {
            if let JsonValue::Object(ref mut o) = value {
                o.remove("signature");
            }
        }

        Ok(value)
    }

    /// Checks if public key security level is MASTER
    pub fn is_master(&self) -> bool {
        self.security_level == SecurityLevel::MASTER
    }

    /// Get the original public key hash
    pub fn hash(&self) -> Result<Vec<u8>, ProtocolError> {
        Into::<IdentityPublicKey>::into(self).hash()
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

        Ok(Self {
            id: id.into(),
            purpose: purpose.try_into()?,
            security_level: security_level.try_into()?,
            key_type: key_type.try_into()?,
            data: public_key_bytes,
            read_only: readonly,
            signature: signature_bytes,
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

        if !self.signature.is_empty() {
            pk_map.insert("signature", self.signature.as_slice())
        }

        pk_map.to_value_sorted()
    }
}

impl Into<IdentityPublicKey> for &IdentityPublicKeyCreateTransition {
    fn into(self) -> IdentityPublicKey {
        IdentityPublicKey {
            id: self.id,
            purpose: self.purpose,
            security_level: self.security_level,
            key_type: self.key_type,
            read_only: self.read_only,
            data: self.data.clone(),
            disabled_at: None,
        }
    }
}
