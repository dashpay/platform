use crate::identity::{IdentityPublicKey, KeyID, KeyType, Purpose, SecurityLevel};
#[cfg(feature = "cbor")]
use ciborium::value::Value as CborValue;

use std::collections::BTreeMap;
use std::convert::{TryFrom, TryInto};

use bincode::{config, Decode, Encode};

use platform_value::btreemap_extensions::BTreeValueMapHelper;
use platform_value::btreemap_extensions::BTreeValueRemoveFromMapHelper;
use platform_value::{BinaryData, ReplacementType, Value};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

use crate::errors::ProtocolError;
use crate::identity::signer::Signer;
use crate::platform_serialization::PlatformSignable;
use crate::serialization_traits::PlatformMessageSignable;
use crate::serialization_traits::Signable;

#[cfg(feature = "cbor")]
use crate::util::cbor_serializer;
#[cfg(feature = "cbor")]
use crate::util::cbor_value::{CborCanonicalMap, CborMapExtension};
use crate::util::vec;

use crate::{BlsModule, Convertible, InvalidVectorSizeError, SerdeParsingError};

pub const BINARY_DATA_FIELDS: [&str; 2] = ["data", "signature"];

#[derive(Debug, Serialize, Deserialize, Encode, Decode, PlatformSignable, Clone, PartialEq, Eq)]
#[platform_error_type(ProtocolError)]
#[serde(rename_all = "camelCase")]
pub struct IdentityPublicKeyInCreation {
    pub id: KeyID,
    #[serde(rename = "type")]
    pub key_type: KeyType,
    pub purpose: Purpose,
    pub security_level: SecurityLevel,
    pub read_only: bool,
    pub data: BinaryData,
    /// The signature is needed for ECDSA_SECP256K1 Key type and BLS12_381 Key type
    #[platform_signable(exclude_from_sig_hash)]
    pub signature: BinaryData,
}

impl Convertible for IdentityPublicKeyInCreation {
    fn to_object(&self) -> Result<Value, ProtocolError> {
        platform_value::to_value(self).map_err(ProtocolError::ValueError)
    }

    fn to_cleaned_object(&self) -> Result<Value, ProtocolError> {
        platform_value::to_value(self).map_err(ProtocolError::ValueError)
    }

    fn into_object(self) -> Result<Value, ProtocolError> {
        platform_value::to_value(self).map_err(ProtocolError::ValueError)
    }

    fn to_json_object(&self) -> Result<JsonValue, ProtocolError> {
        self.to_cleaned_object()?
            .try_into_validating_json()
            .map_err(ProtocolError::ValueError)
    }

    fn to_json(&self) -> Result<JsonValue, ProtocolError> {
        self.to_cleaned_object()?
            .try_into()
            .map_err(ProtocolError::ValueError)
    }
    #[cfg(feature = "cbor")]
    fn to_cbor_buffer(&self) -> Result<Vec<u8>, ProtocolError> {
        let object = self.to_object()?;

        cbor_serializer::serializable_value_to_cbor(&object, None)
    }
}

impl IdentityPublicKeyInCreation {
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

    pub fn from_raw_object(raw_object: Value) -> Result<Self, ProtocolError> {
        raw_object.try_into().map_err(ProtocolError::ValueError)
    }

    pub fn from_public_key_signed_with_private_key(
        public_key: IdentityPublicKey,
        state_transition_bytes: &[u8],
        private_key: &[u8],
        bls: &impl BlsModule,
    ) -> Result<Self, ProtocolError> {
        let key_type = public_key.key_type;
        let mut public_key_with_witness: IdentityPublicKeyInCreation = public_key.into();
        public_key_with_witness.signature = state_transition_bytes
            .sign_by_private_key(private_key, key_type, bls)?
            .into();
        Ok(public_key_with_witness)
    }

    pub fn from_public_key_signed_external<S: Signer>(
        public_key: IdentityPublicKey,
        state_transition_bytes: &[u8],
        signer: &S,
    ) -> Result<Self, ProtocolError> {
        let mut public_key_with_witness: IdentityPublicKeyInCreation = public_key.clone().into();
        match public_key.key_type {
            KeyType::ECDSA_SECP256K1 | KeyType::BLS12_381 => {
                public_key_with_witness.signature =
                    signer.sign(&public_key, state_transition_bytes)?;
            }
            KeyType::ECDSA_HASH160 | KeyType::BIP13_SCRIPT_HASH | KeyType::EDDSA_25519_HASH160 => {
                // don't sign (on purpose)
            }
        }
        // dbg!(format!("signed signature {} data {} public key {}",hex::encode(public_key_with_witness.signature.as_slice()),  hex::encode(data), hex::encode(public_key.data.as_slice())));
        Ok(public_key_with_witness)
    }

    pub fn from_value_map(mut value_map: BTreeMap<String, Value>) -> Result<Self, ProtocolError> {
        Ok(Self {
            id: value_map
                .get_integer("id")
                .map_err(ProtocolError::ValueError)?,
            purpose: value_map
                .get_integer::<u8>("purpose")
                .map_err(ProtocolError::ValueError)?
                .try_into()?,
            security_level: value_map
                .get_integer::<u8>("securityLevel")
                .map_err(ProtocolError::ValueError)?
                .try_into()?,
            key_type: value_map
                .get_integer::<u8>("keyType")
                .map_err(ProtocolError::ValueError)?
                .try_into()?,
            data: value_map
                .remove_binary_data("data")
                .map_err(ProtocolError::ValueError)?,
            read_only: value_map
                .get_bool("readOnly")
                .map_err(ProtocolError::ValueError)?,
            signature: value_map
                .remove_binary_data("signature")
                .map_err(ProtocolError::ValueError)?,
        })
    }

    pub fn from_raw_json_object(raw_object: JsonValue) -> Result<Self, ProtocolError> {
        let identity_public_key: Self = serde_json::from_value(raw_object)?;
        Ok(identity_public_key)
    }

    pub fn from_json_object(raw_object: JsonValue) -> Result<Self, ProtocolError> {
        let mut value: Value = raw_object.into();
        value.replace_at_paths(BINARY_DATA_FIELDS, ReplacementType::BinaryBytes)?;
        value.try_into().map_err(ProtocolError::ValueError)
    }

    /// Return raw data, with all binary fields represented as arrays
    pub fn to_raw_object(&self, skip_signature: bool) -> Result<Value, ProtocolError> {
        let mut value = self.to_object()?;

        if skip_signature || self.signature.is_empty() {
            value
                .remove("signature")
                .map_err(ProtocolError::ValueError)?;
        }

        Ok(value)
    }

    /// Return raw data, with all binary fields represented as arrays
    pub fn to_raw_cleaned_object(&self, skip_signature: bool) -> Result<Value, ProtocolError> {
        let mut value = self.to_cleaned_object()?;

        if skip_signature || self.signature.is_empty() {
            value
                .remove("signature")
                .map_err(ProtocolError::ValueError)?;
        }

        Ok(value)
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

    pub fn to_ecdsa_array(&self) -> Result<[u8; 33], InvalidVectorSizeError> {
        vec::vec_to_array::<33>(self.data.as_slice())
    }

    /// Get the original public key hash
    pub fn hash_as_vec(&self) -> Result<Vec<u8>, ProtocolError> {
        Into::<IdentityPublicKey>::into(self).hash()
    }

    /// Get the original public key hash
    pub fn hash(&self) -> Result<[u8; 20], ProtocolError> {
        Into::<IdentityPublicKey>::into(self)
            .hash()?
            .try_into()
            .map_err(|_| {
                ProtocolError::CorruptedCodeExecution(
                    "hash should always output 20 bytes".to_string(),
                )
            })
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
        let signature_bytes = key_value_map.as_bytes("signature", "").unwrap_or_default();

        Ok(Self {
            id: id.into(),
            purpose: purpose.try_into()?,
            security_level: security_level.try_into()?,
            key_type: key_type.try_into()?,
            data: BinaryData::from(public_key_bytes),
            read_only: readonly,
            signature: BinaryData::from(signature_bytes),
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

        if !self.signature.is_empty() {
            pk_map.insert("signature", self.signature.as_slice())
        }

        pk_map.to_value_sorted()
    }
}

impl From<&IdentityPublicKeyInCreation> for IdentityPublicKey {
    fn from(val: &IdentityPublicKeyInCreation) -> Self {
        IdentityPublicKey {
            id: val.id,
            purpose: val.purpose,
            security_level: val.security_level,
            key_type: val.key_type,
            read_only: val.read_only,
            data: val.data.clone(),
            disabled_at: None,
        }
    }
}

impl From<IdentityPublicKey> for IdentityPublicKeyInCreation {
    fn from(val: IdentityPublicKey) -> Self {
        IdentityPublicKeyInCreation {
            id: val.id,
            purpose: val.purpose,
            security_level: val.security_level,
            key_type: val.key_type,
            read_only: val.read_only,
            data: val.data,
            signature: Default::default(),
        }
    }
}

impl TryFrom<Value> for IdentityPublicKeyInCreation {
    type Error = platform_value::Error;

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        platform_value::from_value(value)
    }
}

impl TryFrom<IdentityPublicKeyInCreation> for Value {
    type Error = platform_value::Error;

    fn try_from(value: IdentityPublicKeyInCreation) -> Result<Self, Self::Error> {
        platform_value::to_value(value)
    }
}

impl TryFrom<&IdentityPublicKeyInCreation> for Value {
    type Error = platform_value::Error;

    fn try_from(value: &IdentityPublicKeyInCreation) -> Result<Self, Self::Error> {
        platform_value::to_value(value)
    }
}
