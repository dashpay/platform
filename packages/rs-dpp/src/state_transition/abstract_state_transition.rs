use std::fmt::Debug;

use dashcore::signer;

use platform_value::{BinaryData, Value, ValueMapHelper};
use serde::Serialize;
use serde_json::Value as JsonValue;

use crate::consensus::ConsensusError;
use crate::errors::consensus::signature::SignatureError;
use crate::identity::signer::Signer;

use crate::state_transition::errors::{
    InvalidIdentityPublicKeyTypeError, StateTransitionIsNotSignedError,
};
use crate::{
    identity::KeyType,
    prelude::{Identifier, ProtocolError},
    util::{hash, serializer},
    BlsModule,
};

use super::{StateTransition, StateTransitionType};

const PROPERTY_SIGNATURE: &str = "signature";
const PROPERTY_PROTOCOL_VERSION: &str = "protocolVersion";

pub const DOCUMENT_TRANSITION_TYPES: [StateTransitionType; 1] =
    [StateTransitionType::DocumentsBatch];

pub const IDENTITY_TRANSITION_TYPE: [StateTransitionType; 3] = [
    StateTransitionType::IdentityCreate,
    StateTransitionType::IdentityTopUp,
    StateTransitionType::IdentityUpdate,
];

pub const DATA_CONTRACT_TRANSITION_TYPES: [StateTransitionType; 2] = [
    StateTransitionType::DataContractCreate,
    StateTransitionType::DataContractUpdate,
];

/// The StateTransitionLike represents set of methods that are shared for all types of State Transition.
/// Every type of state transition should also implement Debug, Clone, and support conversion to compounded [`StateTransition`]
pub trait StateTransitionLike:
    StateTransitionConvert + Clone + Debug + Into<StateTransition>
{
    /// returns the protocol version
    fn get_protocol_version(&self) -> u32;
    /// returns the type of State Transition
    fn get_type(&self) -> StateTransitionType;
    /// returns the signature as a byte-array
    fn get_signature(&self) -> &BinaryData;
    /// set a new signature
    fn set_signature(&mut self, signature: BinaryData);
    /// get modified ids list
    fn get_modified_data_ids(&self) -> Vec<Identifier>;

    /// Signs data with the private key
    fn sign_by_private_key(
        &mut self,
        private_key: &[u8],
        key_type: KeyType,
        bls: &impl BlsModule,
    ) -> Result<(), ProtocolError> {
        let data = self.to_cbor_buffer(true)?;
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
        if self.get_signature().is_empty() {
            return Err(ProtocolError::StateTransitionIsNotSignedError(
                StateTransitionIsNotSignedError::new(self.clone().into()),
            ));
        }
        let data_hash = self.hash(true)?;
        signer::verify_hash_signature(&data_hash, self.get_signature().as_slice(), public_key_hash)
            .map_err(|_| {
                ProtocolError::from(ConsensusError::SignatureError(
                    SignatureError::InvalidStateTransitionSignatureError,
                ))
            })
    }

    /// Verifies an ECDSA signature with the public key
    fn verify_ecdsa_signature_by_public_key(&self, public_key: &[u8]) -> Result<(), ProtocolError> {
        if self.get_signature().is_empty() {
            return Err(ProtocolError::StateTransitionIsNotSignedError(
                StateTransitionIsNotSignedError::new(self.clone().into()),
            ));
        }
        let data = self.to_cbor_buffer(true)?;
        signer::verify_data_signature(&data, self.get_signature().as_slice(), public_key).map_err(
            |_| {
                ProtocolError::from(ConsensusError::SignatureError(
                    SignatureError::InvalidStateTransitionSignatureError,
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
        if self.get_signature().is_empty() {
            return Err(ProtocolError::StateTransitionIsNotSignedError(
                StateTransitionIsNotSignedError::new(self.clone().into()),
            ));
        }

        let data = self.to_cbor_buffer(true)?;

        bls.verify_signature(self.get_signature().as_slice(), &data, public_key)
            .map(|_| ())
            .map_err(|_| {
                ProtocolError::from(ConsensusError::SignatureError(
                    SignatureError::InvalidStateTransitionSignatureError,
                ))
            })
    }

    /// returns true if state transition is a document state transition
    fn is_document_state_transition(&self) -> bool {
        DOCUMENT_TRANSITION_TYPES.contains(&self.get_type())
    }
    /// returns true if state transition is a data contract state transition
    fn is_data_contract_state_transition(&self) -> bool {
        DATA_CONTRACT_TRANSITION_TYPES.contains(&self.get_type())
    }
    /// return true if state transition is an identity state transition
    fn is_identity_state_transition(&self) -> bool {
        IDENTITY_TRANSITION_TYPE.contains(&self.get_type())
    }

    fn set_signature_bytes(&mut self, signature: Vec<u8>);
}

/// The trait contains methods related to conversion of StateTransition into different formats
pub trait StateTransitionConvert: Serialize {
    // TODO remove this as it is not necessary and can be hardcoded
    fn signature_property_paths() -> Vec<&'static str>;
    fn identifiers_property_paths() -> Vec<&'static str>;
    fn binary_property_paths() -> Vec<&'static str>;

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

    // Returns the cbor-encoded bytes representation of the object. The data is  prefixed by 4 bytes containing the Protocol Version
    fn to_cbor_buffer(&self, skip_signature: bool) -> Result<Vec<u8>, ProtocolError> {
        let mut value = self.to_canonical_cleaned_object(skip_signature)?;
        let protocol_version = value.remove_integer(PROPERTY_PROTOCOL_VERSION)?;

        serializer::serializable_value_to_cbor(&value, Some(protocol_version))
    }

    // Returns the hash of cibor-encoded bytes representation of the object
    fn hash(&self, skip_signature: bool) -> Result<Vec<u8>, ProtocolError> {
        Ok(hash::hash_to_vec(self.to_cbor_buffer(skip_signature)?))
    }

    fn to_cleaned_object(&self, skip_signature: bool) -> Result<Value, ProtocolError> {
        self.to_object(skip_signature)
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
