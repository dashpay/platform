use std::collections::BTreeMap;
use std::fmt::Debug;

use dashcore::signer;

use platform_value::{BinaryData, IntegerReplacementType, ReplacementType, Value, ValueMapHelper};
use serde::Serialize;
use serde_json::Value as JsonValue;

use crate::consensus::signature::InvalidStateTransitionSignatureError;
use crate::consensus::signature::SignatureError;
use crate::consensus::ConsensusError;

use crate::serialization_traits::{PlatformSerializable, Signable};
use crate::state_transition::errors::{
    InvalidIdentityPublicKeyTypeError, StateTransitionIsNotSignedError,
};
#[cfg(feature = "cbor")]
use crate::util::cbor_serializer;
use crate::version::FeatureVersion;
use crate::{
    identity::KeyType,
    prelude::{Identifier, ProtocolError},
    util::hash,
    BlsModule,
};
use crate::data_contract::DataContract;
use crate::identity::KeyID;
use crate::state_transition::data_contract_create_transition::DataContractCreateTransitionV0;

use super::{StateTransition, StateTransitionType};

const PROPERTY_SIGNATURE: &str = "signature";
const PROPERTY_PROTOCOL_VERSION: &str = "protocolVersion";

pub const DOCUMENT_TRANSITION_TYPES: [StateTransitionType; 1] =
    [StateTransitionType::DocumentsBatch];

pub const IDENTITY_TRANSITION_TYPE: [StateTransitionType; 4] = [
    StateTransitionType::IdentityCreate,
    StateTransitionType::IdentityTopUp,
    StateTransitionType::IdentityUpdate,
    StateTransitionType::IdentityCreditTransfer,
];

pub const DATA_CONTRACT_TRANSITION_TYPES: [StateTransitionType; 2] = [
    StateTransitionType::DataContractCreate,
    StateTransitionType::DataContractUpdate,
];

/// The StateTransitionLike represents set of methods that are shared for all types of State Transition.
/// Every type of state transition should also implement Debug, Clone, and support conversion to compounded [`StateTransition`]
pub trait StateTransitionLike:
    StateTransitionConvert + Clone + Debug + Into<StateTransition> + Signable
{
    /// returns the protocol version
    fn state_transition_protocol_version(&self) -> FeatureVersion;
    /// returns the type of State Transition
    fn state_transition_type(&self) -> StateTransitionType;
    /// returns the signature as a byte-array
    fn signature(&self) -> &BinaryData;
    /// set a new signature
    fn set_signature(&mut self, signature: BinaryData);
    /// get modified ids list
    fn modified_data_ids(&self) -> Vec<Identifier>;

    /// Signs data with the private key
    fn sign_by_private_key(
        &mut self,
        private_key: &[u8],
        key_type: KeyType,
        bls: &impl BlsModule,
    ) -> Result<(), ProtocolError> {
        let data = self.signable_bytes()?;
        match key_type {
            KeyType::BLS12_381 => self.set_signature(bls.sign(&data, private_key)?.into()),

            // https://github.com/dashevo/platform/blob/9c8e6a3b6afbc330a6ab551a689de8ccd63f9120/packages/js-dpp/lib/stateTransition/AbstractStateTransition.js#L169
            KeyType::ECDSA_SECP256K1 | KeyType::ECDSA_HASH160 => {
                let signature = signer::sign(&data, private_key)?;
                self.set_signature(signature.to_vec().into());
            }

            // the default behavior from
            // https://github.com/dashevo/platform/blob/6b02b26e5cd3a7c877c5fdfe40c4a4385a8dda15/packages/js-dpp/lib/stateTransition/AbstractStateTransition.js#L187
            // is to return the error for the BIP13_SCRIPT_HASH
            KeyType::BIP13_SCRIPT_HASH | KeyType::EDDSA_25519_HASH160 => {
                return Err(ProtocolError::InvalidIdentityPublicKeyTypeError(
                    InvalidIdentityPublicKeyTypeError::new(key_type),
                ))
            }
        };
        Ok(())
    }

    fn verify_by_public_key<T: BlsModule>(
        &self,
        public_key: &[u8],
        public_key_type: KeyType,
        bls: &T,
    ) -> Result<(), ProtocolError> {
        match public_key_type {
            KeyType::ECDSA_SECP256K1 => self.verify_ecdsa_signature_by_public_key(public_key),
            KeyType::ECDSA_HASH160 => {
                self.verify_ecdsa_hash_160_signature_by_public_key_hash(public_key)
            }
            KeyType::BLS12_381 => self.verify_bls_signature_by_public_key(public_key, bls),
            KeyType::BIP13_SCRIPT_HASH | KeyType::EDDSA_25519_HASH160 => {
                Err(ProtocolError::InvalidIdentityPublicKeyTypeError(
                    InvalidIdentityPublicKeyTypeError::new(public_key_type),
                ))
            }
        }
    }

    fn verify_ecdsa_hash_160_signature_by_public_key_hash(
        &self,
        public_key_hash: &[u8],
    ) -> Result<(), ProtocolError> {
        if self.signature().is_empty() {
            return Err(ProtocolError::StateTransitionIsNotSignedError(
                StateTransitionIsNotSignedError::new(self.clone().into()),
            ));
        }
        let data_hash = self.hash(true)?;
        signer::verify_hash_signature(&data_hash, self.signature().as_slice(), public_key_hash)
            .map_err(|_| {
                ProtocolError::from(ConsensusError::SignatureError(
                    SignatureError::InvalidStateTransitionSignatureError(
                        InvalidStateTransitionSignatureError::new(),
                    ),
                ))
            })
    }

    /// Verifies an ECDSA signature with the public key
    fn verify_ecdsa_signature_by_public_key(&self, public_key: &[u8]) -> Result<(), ProtocolError> {
        if self.signature().is_empty() {
            return Err(ProtocolError::StateTransitionIsNotSignedError(
                StateTransitionIsNotSignedError::new(self.clone().into()),
            ));
        }
        let data = self.signable_bytes()?;
        signer::verify_data_signature(&data, self.signature().as_slice(), public_key).map_err(
            |_| {
                // TODO: it shouldn't respond with consensus error

                ProtocolError::from(ConsensusError::SignatureError(
                    SignatureError::InvalidStateTransitionSignatureError(
                        InvalidStateTransitionSignatureError::new(),
                    ),
                ))
            },
        )
    }

    /// Verifies a BLS signature with the public key
    fn verify_bls_signature_by_public_key<T: BlsModule>(
        &self,
        public_key: &[u8],
        bls: &T,
    ) -> Result<(), ProtocolError> {
        if self.signature().is_empty() {
            return Err(ProtocolError::StateTransitionIsNotSignedError(
                StateTransitionIsNotSignedError::new(self.clone().into()),
            ));
        }

        let data = self.signable_bytes()?;

        bls.verify_signature(self.signature().as_slice(), &data, public_key)
            .map(|_| ())
            .map_err(|_| {
                // TODO: it shouldn't respond with consensus error
                ProtocolError::from(ConsensusError::SignatureError(
                    SignatureError::InvalidStateTransitionSignatureError(
                        InvalidStateTransitionSignatureError::new(),
                    ),
                ))
            })
    }

    /// returns true if state transition is a document state transition
    fn is_document_state_transition(&self) -> bool {
        DOCUMENT_TRANSITION_TYPES.contains(&self.state_transition_type())
    }
    /// returns true if state transition is a data contract state transition
    fn is_data_contract_state_transition(&self) -> bool {
        DATA_CONTRACT_TRANSITION_TYPES.contains(&self.state_transition_type())
    }
    /// return true if state transition is an identity state transition
    fn is_identity_state_transition(&self) -> bool {
        IDENTITY_TRANSITION_TYPE.contains(&self.state_transition_type())
    }

    fn set_signature_bytes(&mut self, signature: Vec<u8>);

    fn hash(&self, skip_signature: bool) -> Result<Vec<u8>, ProtocolError> {
        if skip_signature {
            Ok(hash::hash_to_vec(self.signable_bytes()?))
        } else {
            Ok(hash::hash_to_vec(PlatformSerializable::serialize(self)?))
        }
    }
    /// Get owner ID
    fn get_owner_id(&self) -> &Identifier;
    fn get_signature_public_key_id(&self) -> Option<KeyID>;
    fn set_signature_public_key_id(&mut self, key_id: crate::identity::KeyID);
}

/// The trait contains methods related to conversion of StateTransition into different formats
pub trait StateTransitionConvert: Serialize + Signable + PlatformSerializable {
    // TODO remove this as it is not necessary and can be hardcoded
    fn signature_property_paths() -> Vec<&'static str>;
    fn identifiers_property_paths() -> Vec<&'static str>;
    fn binary_property_paths() -> Vec<&'static str>;
}

#[cfg(feature = "platform-value")]
/// The trait contains methods related to conversion of StateTransition into different formats
pub trait StateTransitionValueConvert: Serialize + Signable + PlatformSerializable {
    /// Returns the [`platform_value::Value`] instance that preserves the `Vec<u8>` representation
    /// for Identifiers and binary data
    fn to_object(&self, skip_signature: bool) -> Result<Value, ProtocolError> {
        let skip_signature_paths = if skip_signature {
            Self::signature_property_paths()
        } else {
            vec![]
        };
        state_transition_helpers::to_object(self, skip_signature_paths)
    }

    /// Returns the [`platform_value::Value`] instance that preserves the `Vec<u8>` representation
    /// for Identifiers and binary data
    fn to_canonical_object(&self, skip_signature: bool) -> Result<Value, ProtocolError> {
        let skip_signature_paths = if skip_signature {
            Self::signature_property_paths()
        } else {
            vec![]
        };
        let mut object = state_transition_helpers::to_object(self, skip_signature_paths)?;

        object.as_map_mut_ref().unwrap().sort_by_keys();
        Ok(object)
    }

    /// Returns the [`platform_value::Value`] instance that preserves the `Vec<u8>` representation
    /// for Identifiers and binary data
    fn to_canonical_cleaned_object(&self, skip_signature: bool) -> Result<Value, ProtocolError> {
        let skip_signature_paths = if skip_signature {
            Self::signature_property_paths()
        } else {
            vec![]
        };
        let mut object = state_transition_helpers::to_cleaned_object(self, skip_signature_paths)?;

        object.as_map_mut_ref().unwrap().sort_by_keys();
        Ok(object)
    }

    fn to_cleaned_object(&self, skip_signature: bool) -> Result<Value, ProtocolError> {
        self.to_object(skip_signature)
    }
    fn from_raw_object(
        raw_object: Value,
    ) -> Result<Self, ProtocolError>;
    fn from_value_map(
        raw_data_contract_create_transition: BTreeMap<String, Value>,
    ) -> Result<Self, ProtocolError>;
    fn clean_value(value: &mut Value) -> Result<(), ProtocolError>;
}

#[cfg(feature = "json-object")]
/// The trait contains methods related to conversion of StateTransition into different formats
pub trait StateTransitionJsonConvert: Serialize + Signable + PlatformSerializable {
    /// Returns the [`serde_json::Value`] instance that encodes:
    ///  - Identifiers  - with base58
    ///  - Binary data  - with base64
    fn to_json(&self, skip_signature: bool) -> Result<JsonValue, ProtocolError> {
        let skip_signature_paths = if skip_signature {
            Self::signature_property_paths()
        } else {
            vec![]
        };
        state_transition_helpers::to_json(self, skip_signature_paths)
    }
}

#[cfg(feature = "cbor")]
/// The trait contains methods related to conversion of StateTransition into different formats
pub trait StateTransitionCborConvert: Serialize + Signable + PlatformSerializable {
    // Returns the cbor-encoded bytes representation of the object. The data is  prefixed by 4 bytes containing the Protocol Version
    fn to_cbor_buffer(&self, skip_signature: bool) -> Result<Vec<u8>, ProtocolError> {
        let mut value = self.to_canonical_cleaned_object(skip_signature)?;
        let protocol_version = value.remove_integer(PROPERTY_PROTOCOL_VERSION)?;

        cbor_serializer::serializable_value_to_cbor(&value, Some(protocol_version))
    }
}

pub mod state_transition_helpers {
    use super::*;
    use std::convert::TryInto;

    pub fn to_json<'a, I: IntoIterator<Item = &'a str>>(
        serializable: impl Serialize,
        skip_signature_paths: I,
    ) -> Result<JsonValue, ProtocolError> {
        to_object(serializable, skip_signature_paths)
            .and_then(|v| v.try_into().map_err(ProtocolError::ValueError))
    }

    pub fn to_object<'a, I: IntoIterator<Item = &'a str>>(
        serializable: impl Serialize,
        skip_signature_paths: I,
    ) -> Result<Value, ProtocolError> {
        let mut value: Value = platform_value::to_value(serializable)?;
        skip_signature_paths.into_iter().try_for_each(|path| {
            value
                .remove_values_matching_path(path)
                .map_err(ProtocolError::ValueError)
                .map(|_| ())
        })?;
        Ok(value)
    }

    pub fn to_cleaned_object<'a, I: IntoIterator<Item = &'a str>>(
        serializable: impl Serialize,
        skip_signature_paths: I,
    ) -> Result<Value, ProtocolError> {
        let mut value: Value = platform_value::to_value(serializable)?;

        value = value.clean_recursive()?;

        skip_signature_paths.into_iter().try_for_each(|path| {
            value
                .remove_values_matching_path(path)
                .map_err(ProtocolError::ValueError)
                .map(|_| ())
        })?;
        Ok(value)
    }
}
