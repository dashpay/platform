use anyhow::anyhow;
use dashcore::signer;
use serde::Serialize;
use std::fmt::Debug;

use crate::{
    identity::KeyType,
    prelude::ProtocolError,
    util::{
        hash,
        json_value::{JsonValueExt, ReplaceWith},
        serializer,
    },
};
use serde_json::Value as JsonValue;
use std::convert::TryInto;

use super::{StateTransition, StateTransitionType};
use bls_signatures::{
    verify_messages, PrivateKey as BLSPrivateKey, PublicKey as BLSPublicKey,
    Serialize as BLSSerialize,
};

const PROPERTY_PROTOCOL_VERSION: &str = "protocolVersion";

pub const DOCUMENT_TRANSITION_TYPES: [StateTransitionType; 1] =
    [StateTransitionType::DocumentsBatch];

pub const IDENTITY_TRANSITION_TYPE: [StateTransitionType; 2] = [
    StateTransitionType::IdentityCreate,
    StateTransitionType::IdentityTopUp,
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
    fn get_signature(&self) -> &Vec<u8>;
    /// set a new signature
    fn set_signature(&mut self, signature: Vec<u8>);
    /// Calculates the ST fee in credits
    fn calculate_fee(&self) -> Result<u64, ProtocolError>;

    /// Signs data with the private key
    fn sign_by_private_key(
        &mut self,
        private_key: &[u8],
        key_type: KeyType,
    ) -> Result<(), ProtocolError> {
        let data = self.to_buffer(true)?;
        match key_type {
            KeyType::BLS12_381 => {
                let fixed_len_key: [u8; 32] = private_key
                    .try_into()
                    .map_err(|_| anyhow!("the BLS private key must be 32 bytes long"))?;
                let pk = BLSPrivateKey::from_bytes(&fixed_len_key).map_err(anyhow::Error::msg)?;
                self.set_signature(pk.sign(data).as_bytes())
            }

            KeyType::ECDSA_SECP256K1 => {
                let signature = signer::sign(&data, private_key)?;
                self.set_signature(signature.to_vec());
            }

            KeyType::ECDSA_HASH160 => {
                return Err(anyhow!("Invalid key type of private key: {:?}", key_type).into())
            }
        };
        Ok(())
    }

    fn verify_ecdsa_hash_160_signature_by_public_key_hash(
        &self,
        public_key_hash: &[u8],
    ) -> Result<(), ProtocolError> {
        if self.get_signature().is_empty() {
            return Err(ProtocolError::StateTransitionIsNotIsSignedError {
                state_transition: self.clone().into(),
            });
        }
        let data_hash = self.hash(true)?;
        Ok(signer::verify_hash_signature(
            &data_hash,
            self.get_signature(),
            public_key_hash,
        )?)
    }

    /// Verifies an ECDSA signature with the public key
    fn verify_ecdsa_signature_by_public_key(&self, public_key: &[u8]) -> Result<(), ProtocolError> {
        if self.get_signature().is_empty() {
            return Err(ProtocolError::StateTransitionIsNotIsSignedError {
                state_transition: self.clone().into(),
            });
        }
        let data = self.to_buffer(true)?;
        Ok(signer::verify_data_signature(
            &data,
            self.get_signature(),
            public_key,
        )?)
    }

    /// Verifies a BLS signature with the public key
    fn verify_bls_signature_by_public_key(&self, public_key: &[u8]) -> Result<bool, ProtocolError> {
        if self.get_signature().is_empty() {
            return Err(ProtocolError::StateTransitionIsNotIsSignedError {
                state_transition: self.clone().into(),
            });
        }

        let data = self.to_buffer(true)?;
        let pk = BLSPublicKey::from_bytes(public_key).map_err(anyhow::Error::msg)?;
        let signature = bls_signatures::Signature::from_bytes(self.get_signature())
            .map_err(anyhow::Error::msg)?;

        Ok(verify_messages(&signature, &[&data], &[pk]))
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
}

// TODO remove 'unimplemented' when get rid of state transition mocks
/// The trait contains methods related to conversion of StateTransition into different formats
pub trait StateTransitionConvert: Serialize {
    fn signature_property_paths() -> Vec<&'static str>;
    fn identifiers_property_paths() -> Vec<&'static str>;
    fn binary_property_paths() -> Vec<&'static str>;

    /// Returns the [`serde_json::Value`] instance that preserves the `Vec<u8>` representation
    /// for Identifiers and binary data
    fn to_object(&self, skip_signature: bool) -> Result<JsonValue, ProtocolError> {
        let mut json_value: JsonValue = serde_json::to_value(self)?;
        if skip_signature {
            if let JsonValue::Object(ref mut o) = json_value {
                for path in Self::signature_property_paths() {
                    o.remove(path);
                }
            }
        }
        Ok(json_value)
    }
    /// Returns the [`serde_json::Value`] instance that encodes:
    ///  - Identifiers  - with base58
    ///  - Binary data  - with base64
    fn to_json(&self) -> Result<JsonValue, ProtocolError> {
        let mut json_value: JsonValue = serde_json::to_value(self)?;
        json_value.replace_binary_paths(Self::binary_property_paths(), ReplaceWith::Base64)?;
        json_value
            .replace_identifier_paths(Self::identifiers_property_paths(), ReplaceWith::Base58)?;

        Ok(json_value)
    }

    // Returns the cibor-encoded bytes representation of the object. The data is  prefixed by 4 bytes containing the Protocol Version
    fn to_buffer(&self, skip_signature: bool) -> Result<Vec<u8>, ProtocolError> {
        let mut json_value = self.to_object(skip_signature)?;
        let protocol_version = json_value.remove_u32(PROPERTY_PROTOCOL_VERSION)?;
        serializer::value_to_cbor(json_value, Some(protocol_version))
    }

    // Returns the hash of cibor-encoded bytes representation of the object
    fn hash(&self, skip_signature: bool) -> Result<Vec<u8>, ProtocolError> {
        Ok(hash::hash(self.to_buffer(skip_signature)?))
    }
}
