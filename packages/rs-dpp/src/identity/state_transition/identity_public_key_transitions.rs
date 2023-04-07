use crate::identity::{IdentityPublicKey, KeyID, KeyType, Purpose, SecurityLevel};
use ciborium::value::Value as CborValue;
use dashcore::signer;
use std::collections::BTreeMap;
use std::convert::{TryFrom, TryInto};

use bincode::{Decode, Encode};
use bls_signatures::Serialize as BlsSerialize;
use dashcore::secp256k1::ecdsa::Signature;
use dashcore::secp256k1::PublicKey;
use platform_value::btreemap_extensions::BTreeValueMapHelper;
use platform_value::btreemap_extensions::BTreeValueRemoveFromMapHelper;
use platform_value::{BinaryData, ReplacementType, Value, ValueMapHelper};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

use crate::consensus::signature::SignatureError;
use crate::consensus::ConsensusError;
use crate::errors::ProtocolError;
use crate::identity::signer::Signer;
use crate::state_transition::errors::InvalidIdentityPublicKeyTypeError;
use crate::util::cbor_value::{CborCanonicalMap, CborMapExtension};
use crate::util::{serializer, vec};
use crate::validation::{
    ConsensusValidationResult, SimpleConsensusValidationResult, SimpleValidationResult,
    ValidationResult,
};
use crate::{
    BlsModule, Convertible, InvalidVectorSizeError, PublicKeyValidationError, SerdeParsingError,
};

pub const BINARY_DATA_FIELDS: [&str; 2] = ["data", "signature"];

#[derive(Debug, Serialize, Deserialize, Encode, Decode, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct IdentityPublicKeyWithWitness {
    pub id: KeyID,
    pub purpose: Purpose,
    pub security_level: SecurityLevel,
    #[serde(rename = "type")]
    pub key_type: KeyType,
    pub data: BinaryData,
    pub read_only: bool,
    /// The signature is needed for ECDSA_SECP256K1 Key type and BLS12_381 Key type
    pub signature: BinaryData,
}

impl Convertible for IdentityPublicKeyWithWitness {
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

    fn to_buffer(&self) -> Result<Vec<u8>, ProtocolError> {
        let mut object = self.to_cleaned_object()?;
        object
            .to_map_mut()
            .unwrap()
            .sort_by_lexicographical_byte_ordering_keys_and_inner_maps();

        serializer::serializable_value_to_cbor(&object, None)
    }
}

impl IdentityPublicKeyWithWitness {
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
        private_key: &[u8],
        bls: &impl BlsModule,
    ) -> Result<Self, ProtocolError> {
        let key_type = public_key.key_type;
        let mut public_key_with_witness: IdentityPublicKeyWithWitness = public_key.into();
        public_key_with_witness.sign_by_private_key(private_key, key_type, bls)?;
        Ok(public_key_with_witness)
    }

    pub fn from_public_key_signed_external<S: Signer>(
        public_key: IdentityPublicKey,
        signer: &S,
    ) -> Result<Self, ProtocolError> {
        let mut public_key_with_witness: IdentityPublicKeyWithWitness = public_key.clone().into();
        let data = public_key_with_witness.to_buffer()?;
        public_key_with_witness.signature = signer.sign(&public_key, data.as_slice())?;
        Ok(public_key_with_witness)
    }

    /// Signs data with the private key
    fn sign_by_private_key(
        &mut self,
        private_key: &[u8],
        key_type: KeyType,
        bls: &impl BlsModule,
    ) -> Result<(), ProtocolError> {
        let data = self.to_buffer()?;
        match key_type {
            KeyType::BLS12_381 => self.signature = bls.sign(&data, private_key)?.into(),

            // https://github.com/dashevo/platform/blob/9c8e6a3b6afbc330a6ab551a689de8ccd63f9120/packages/js-dpp/lib/stateTransition/AbstractStateTransition.js#L169
            KeyType::ECDSA_SECP256K1 | KeyType::ECDSA_HASH160 => {
                let signature = signer::sign(&data, private_key)?;
                self.signature = signature.to_vec().into();
            }

            // the default behavior from
            // https://github.com/dashevo/platform/blob/6b02b26e5cd3a7c877c5fdfe40c4a4385a8dda15/packages/js-dpp/lib/stateTransition/AbstractStateTransition.js#L187
            // is to return the error for the BIP13_SCRIPT_HASH
            KeyType::BIP13_SCRIPT_HASH => {
                return Err(ProtocolError::InvalidIdentityPublicKeyTypeError(
                    InvalidIdentityPublicKeyTypeError::new(key_type),
                ))
            }
        };
        Ok(())
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

    pub fn verify_signature(&self) -> Result<SimpleConsensusValidationResult, ProtocolError> {
        match self.key_type {
            KeyType::ECDSA_SECP256K1 => {
                let signable_data = self.to_buffer()?;
                if let Err(e) = signer::verify_data_signature(
                    &signable_data,
                    self.signature.as_slice(),
                    self.data.as_slice(),
                ) {
                    Ok(SimpleConsensusValidationResult::new_with_error(
                        ConsensusError::SignatureError(SignatureError::BasicECDSAError(
                            e.to_string(),
                        )),
                    ))
                } else {
                    Ok(SimpleConsensusValidationResult::default())
                }
            }
            KeyType::BLS12_381 => {
                let signable_data = self.to_buffer()?;
                let public_key = match bls_signatures::PublicKey::from_bytes(self.data.as_slice()) {
                    Ok(public_key) => public_key,
                    Err(e) => {
                        return Ok(SimpleConsensusValidationResult::new_with_error(
                            ConsensusError::SignatureError(SignatureError::BasicBLSError(
                                e.to_string(),
                            )),
                        ))
                    }
                };
                let signature = match bls_signatures::Signature::from_bytes(self.data.as_slice()) {
                    Ok(public_key) => public_key,
                    Err(e) => {
                        return Ok(SimpleConsensusValidationResult::new_with_error(
                            ConsensusError::SignatureError(SignatureError::BasicBLSError(
                                e.to_string(),
                            )),
                        ))
                    }
                };
                if !public_key.verify(signature, signable_data) {
                    Ok(SimpleConsensusValidationResult::new_with_error(
                        ConsensusError::SignatureError(SignatureError::BasicBLSError(
                            "bls signature was incorrect".to_string(),
                        )),
                    ))
                } else {
                    Ok(SimpleConsensusValidationResult::default())
                }
            }
            KeyType::ECDSA_HASH160 => {
                if !self.signature.is_empty() {
                    Ok(SimpleConsensusValidationResult::new_with_error(ConsensusError::SignatureError(
                        SignatureError::SignatureShouldNotBePresent("ecdsa_hash160 keys should not have a signature as that would reveal the public key".to_string()),
                    )))
                } else {
                    Ok(SimpleConsensusValidationResult::default())
                }
            }
            KeyType::BIP13_SCRIPT_HASH => {
                if !self.signature.is_empty() {
                    Ok(SimpleConsensusValidationResult::new_with_error(ConsensusError::SignatureError(
                        SignatureError::SignatureShouldNotBePresent("script hash keys should not have a signature as that would reveal the script".to_string()),
                    )))
                } else {
                    Ok(SimpleConsensusValidationResult::default())
                }
            }
        }
    }
}

impl From<&IdentityPublicKeyWithWitness> for IdentityPublicKey {
    fn from(val: &IdentityPublicKeyWithWitness) -> Self {
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

impl From<IdentityPublicKey> for IdentityPublicKeyWithWitness {
    fn from(val: IdentityPublicKey) -> Self {
        IdentityPublicKeyWithWitness {
            id: val.id,
            purpose: val.purpose,
            security_level: val.security_level,
            key_type: val.key_type,
            read_only: val.read_only,
            data: val.data.clone(),
            signature: Default::default(),
        }
    }
}

impl TryFrom<Value> for IdentityPublicKeyWithWitness {
    type Error = platform_value::Error;

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        platform_value::from_value(value)
    }
}

impl TryFrom<IdentityPublicKeyWithWitness> for Value {
    type Error = platform_value::Error;

    fn try_from(value: IdentityPublicKeyWithWitness) -> Result<Self, Self::Error> {
        platform_value::to_value(value)
    }
}

impl TryFrom<&IdentityPublicKeyWithWitness> for Value {
    type Error = platform_value::Error;

    fn try_from(value: &IdentityPublicKeyWithWitness) -> Result<Self, Self::Error> {
        platform_value::to_value(value)
    }
}
