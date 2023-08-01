use derive_more::From;
use serde::{Deserialize, Serialize};

pub use abstract_state_transition::state_transition_helpers;

use platform_value::{BinaryData, Identifier, Value};
pub use state_transition_types::*;

use bincode::{config, Decode, Encode};
use dashcore::signer;
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize, PlatformSignable};

mod abstract_state_transition;
use crate::{BlsModule, ProtocolError};

mod state_transition_types;

pub mod errors;
use crate::util::hash::hash_to_vec;

mod serialization;
mod state_transitions;
mod traits;
// pub mod state_transition_fee;

pub use traits::*;

use crate::consensus::signature::{
    InvalidStateTransitionSignatureError, PublicKeyIsDisabledError, SignatureError,
};
use crate::consensus::ConsensusError;
use crate::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use crate::identity::{IdentityPublicKey, KeyID, KeyType};
pub use state_transitions::*;

use crate::serialization::{PlatformDeserializable, PlatformSerializable, Signable};
use crate::state_transition::data_contract_create_transition::{
    DataContractCreateTransition, DataContractCreateTransitionSignable,
};
use crate::state_transition::data_contract_update_transition::{
    DataContractUpdateTransition, DataContractUpdateTransitionSignable,
};
use crate::state_transition::documents_batch_transition::{
    DocumentsBatchTransition, DocumentsBatchTransitionSignable,
};
use crate::state_transition::errors::{
    InvalidIdentityPublicKeyTypeError, PublicKeyMismatchError, StateTransitionIsNotSignedError,
};
use crate::state_transition::identity_create_transition::{
    IdentityCreateTransition, IdentityCreateTransitionSignable,
};
use crate::state_transition::identity_credit_transfer_transition::{
    IdentityCreditTransferTransition, IdentityCreditTransferTransitionSignable,
};
use crate::state_transition::identity_credit_withdrawal_transition::{
    IdentityCreditWithdrawalTransition, IdentityCreditWithdrawalTransitionSignable,
};
use crate::state_transition::identity_topup_transition::{
    IdentityTopUpTransition, IdentityTopUpTransitionSignable,
};
use crate::state_transition::identity_update_transition::{
    IdentityUpdateTransition, IdentityUpdateTransitionSignable,
};

macro_rules! call_method {
    ($state_transition:expr, $method:ident, $args:tt ) => {
        match $state_transition {
            StateTransition::DataContractCreate(st) => st.$method($args),
            StateTransition::DataContractUpdate(st) => st.$method($args),
            StateTransition::DocumentsBatch(st) => st.$method($args),
            StateTransition::IdentityCreate(st) => st.$method($args),
            StateTransition::IdentityTopUp(st) => st.$method($args),
            StateTransition::IdentityCreditWithdrawal(st) => st.$method($args),
            StateTransition::IdentityUpdate(st) => st.$method($args),
            StateTransition::IdentityCreditTransfer(st) => st.$method($args),
        }
    };
    ($state_transition:expr, $method:ident ) => {
        match $state_transition {
            StateTransition::DataContractCreate(st) => st.$method(),
            StateTransition::DataContractUpdate(st) => st.$method(),
            StateTransition::DocumentsBatch(st) => st.$method(),
            StateTransition::IdentityCreate(st) => st.$method(),
            StateTransition::IdentityTopUp(st) => st.$method(),
            StateTransition::IdentityCreditWithdrawal(st) => st.$method(),
            StateTransition::IdentityUpdate(st) => st.$method(),
            StateTransition::IdentityCreditTransfer(st) => st.$method(),
        }
    };
}

macro_rules! call_method_identity_signed {
    ($state_transition:expr, $method:ident, $args:tt ) => {
        match $state_transition {
            StateTransition::DataContractCreate(st) => Some(st.$method($args)),
            StateTransition::DataContractUpdate(st) => Some(st.$method($args)),
            StateTransition::DocumentsBatch(st) => Some(st.$method($args)),
            StateTransition::IdentityCreate(st) => None,
            StateTransition::IdentityTopUp(st) => None,
            StateTransition::IdentityCreditWithdrawal(st) => Some(st.$method($args)),
            StateTransition::IdentityUpdate(st) => Some(st.$method($args)),
            StateTransition::IdentityCreditTransfer(st) => Some(st.$method($args)),
        }
    };
    ($state_transition:expr, $method:ident ) => {
        match $state_transition {
            StateTransition::DataContractCreate(st) => Some(st.$method()),
            StateTransition::DataContractUpdate(st) => Some(st.$method()),
            StateTransition::DocumentsBatch(st) => Some(st.$method()),
            StateTransition::IdentityCreate(st) => None,
            StateTransition::IdentityTopUp(st) => None,
            StateTransition::IdentityCreditWithdrawal(st) => Some(st.$method()),
            StateTransition::IdentityUpdate(st) => Some(st.$method()),
            StateTransition::IdentityCreditTransfer(st) => Some(st.$method()),
        }
    };
}

macro_rules! call_static_method {
    ($state_transition:expr, $method:ident ) => {
        match $state_transition {
            StateTransition::DataContractCreate(_) => DataContractCreateTransition::$method(),
            StateTransition::DataContractUpdate(_) => DataContractUpdateTransition::$method(),
            StateTransition::DocumentsBatch(_) => DocumentsBatchTransition::$method(),
            StateTransition::IdentityCreate(_) => IdentityCreateTransition::$method(),
            StateTransition::IdentityTopUp(_) => IdentityTopUpTransition::$method(),
            StateTransition::IdentityCreditWithdrawal(_) => {
                IdentityCreditWithdrawalTransition::$method()
            }
            StateTransition::IdentityUpdate(_) => IdentityUpdateTransition::$method(),
            StateTransition::IdentityCreditTransfer(_) => {
                IdentityCreditTransferTransition::$method()
            }
        }
    };
}

#[derive(
    Debug,
    Clone,
    Encode,
    Decode,
    PlatformSerialize,
    PlatformDeserialize,
    PlatformSignable,
    From,
    PartialEq,
)]
#[cfg_attr(
    feature = "state-transition-serde-conversion",
    derive(Serialize, Deserialize),
    serde(untagged)
)]
#[platform_serialize(unversioned)] //versioned directly, no need to use platform_version
#[platform_serialize(limit = 100000)]
pub enum StateTransition {
    DataContractCreate(DataContractCreateTransition),
    DataContractUpdate(DataContractUpdateTransition),
    DocumentsBatch(DocumentsBatchTransition),
    IdentityCreate(IdentityCreateTransition),
    IdentityTopUp(IdentityTopUpTransition),
    IdentityCreditWithdrawal(IdentityCreditWithdrawalTransition),
    IdentityUpdate(IdentityUpdateTransition),
    IdentityCreditTransfer(IdentityCreditTransferTransition),
}

impl StateTransition {
    fn hash(&self, skip_signature: bool) -> Result<Vec<u8>, ProtocolError> {
        if skip_signature {
            Ok(hash_to_vec(self.signable_bytes()?))
        } else {
            Ok(hash_to_vec(self.serialize()?))
        }
    }

    /// returns the signature as a byte-array
    fn signature(&self) -> &BinaryData {
        call_method!(self, signature)
    }

    /// returns the signature as a byte-array
    pub fn signature_public_key_id(&self) -> Option<KeyID> {
        call_method_identity_signed!(self, signature_public_key_id)
    }

    /// returns the signature as a byte-array
    pub fn owner_id(&self) -> Option<Identifier> {
        call_method_identity_signed!(self, owner_id)
    }

    /// set a new signature
    fn set_signature(&mut self, signature: BinaryData) {
        call_method!(self, set_signature, signature)
    }

    #[cfg(feature = "state-transition-signing")]
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

    #[cfg(all(feature = "state-transition-validation"))]
    fn verify_by_raw_public_key<T: BlsModule>(
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

    #[cfg(feature = "state-transition-validation")]
    pub fn verify_signature(
        &self,
        public_key: &IdentityPublicKey,
        bls: &impl BlsModule,
    ) -> Result<(), ProtocolError> {
        // self.verify_public_key_level_and_purpose(public_key)?;
        if public_key.disabled_at().is_some() {
            return Err(ProtocolError::PublicKeyIsDisabledError(
                PublicKeyIsDisabledError::new(public_key.id()),
            ));
        }

        let signature = self.signature();
        if signature.is_empty() {
            return Err(ProtocolError::StateTransitionIsNotSignedError(
                StateTransitionIsNotSignedError::new(self.clone().into()),
            ));
        }

        if self.signature_public_key_id() != Some(public_key.id()) {
            return Err(ProtocolError::PublicKeyMismatchError(
                PublicKeyMismatchError::new(public_key.clone()),
            ));
        }

        let public_key_bytes = public_key.data().as_slice();
        match public_key.key_type() {
            KeyType::ECDSA_HASH160 => {
                self.verify_ecdsa_hash_160_signature_by_public_key_hash(public_key_bytes)
            }

            KeyType::ECDSA_SECP256K1 => self.verify_ecdsa_signature_by_public_key(public_key_bytes),

            KeyType::BLS12_381 => self.verify_bls_signature_by_public_key(public_key_bytes, bls),

            // per https://github.com/dashevo/platform/pull/353, signing and verification is not supported
            KeyType::BIP13_SCRIPT_HASH | KeyType::EDDSA_25519_HASH160 => Ok(()),
        }
    }

    #[cfg(all(feature = "state-transition-validation"))]
    fn verify_ecdsa_hash_160_signature_by_public_key_hash(
        &self,
        public_key_hash: &[u8],
    ) -> Result<(), ProtocolError> {
        if self.signature().is_empty() {
            return Err(ProtocolError::StateTransitionIsNotSignedError(
                StateTransitionIsNotSignedError::new(self.clone().into()),
            ));
        }
        let data = self.signable_bytes()?;
        signer::verify_data_signature(&data, self.signature().as_slice(), public_key_hash).map_err(
            |_| {
                ProtocolError::from(ConsensusError::SignatureError(
                    SignatureError::InvalidStateTransitionSignatureError(
                        InvalidStateTransitionSignatureError::new(),
                    ),
                ))
            },
        )
    }

    #[cfg(all(feature = "state-transition-validation"))]
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

    #[cfg(all(feature = "state-transition-validation"))]
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
}
//
// impl StateTransition {
//     fn signature_property_paths(&self) -> Vec<&'static str> {
//         call_static_method!(self, signature_property_paths)
//     }
//
//     fn identifiers_property_paths(&self) -> Vec<&'static str> {
//         call_static_method!(self, identifiers_property_paths)
//     }
//
//     fn binary_property_paths(&self) -> Vec<&'static str> {
//         call_static_method!(self, binary_property_paths)
//     }
//
//     pub fn get_owner_id(&self) -> &Identifier {
//         call_method!(self, get_owner_id)
//     }
// }
//
// impl StateTransitionFieldTypes for StateTransition {
//     fn hash(&self, skip_signature: bool) -> Result<Vec<u8>, ProtocolError> {
//         if skip_signature {
//             Ok(hash::hash_to_vec(self.signable_bytes()?))
//         } else {
//             Ok(hash::hash_to_vec(PlatformSerializable::serialize(self)?))
//         }
//     }
//
//     #[cfg(feature = "state-transition-cbor-conversion")]
//     fn to_cbor_buffer(&self, _skip_signature: bool) -> Result<Vec<u8>, crate::ProtocolError> {
//         call_method!(self, to_cbor_buffer, true)
//     }
//
//     #[cfg(feature = "state-transition-json-conversion")]
//     fn to_json(&self, skip_signature: bool) -> Result<serde_json::Value, crate::ProtocolError> {
//         call_method!(self, to_json, skip_signature)
//     }
//
//     #[cfg(feature = "state-transition-value-conversion")]
//     fn to_object(
//         &self,
//         skip_signature: bool,
//     ) -> Result<platform_value::Value, crate::ProtocolError> {
//         call_method!(self, to_object, skip_signature)
//     }
//
//     fn signature_property_paths() -> Vec<&'static str> {
//         panic!("Static call is not supported")
//     }
//
//     fn identifiers_property_paths() -> Vec<&'static str> {
//         panic!("Static call is not supported")
//     }
//
//     fn binary_property_paths() -> Vec<&'static str> {
//         panic!("Static call is not supported")
//     }
//
//     #[cfg(feature = "state-transition-value-conversion")]
//     fn to_cleaned_object(&self, skip_signature: bool) -> Result<Value, ProtocolError> {
//         call_method!(self, to_cleaned_object, skip_signature)
//     }
// }
//
// impl StateTransitionLike for StateTransition {
//     fn state_transition_protocol_version(&self) -> FeatureVersion {
//         call_method!(self, state_transition_protocol_version)
//     }
//     /// returns the type of State Transition
//     fn state_transition_type(&self) -> StateTransitionType {
//         call_method!(self, state_transition_type)
//     }
//     /// returns the signature as a byte-array
//     fn signature(&self) -> &BinaryData {
//         call_method!(self, signature)
//     }
//
//     /// set a new signature
//     fn set_signature(&mut self, signature: BinaryData) {
//         call_method!(self, set_signature, signature)
//     }
//
//     fn set_signature_bytes(&mut self, signature: Vec<u8>) {
//         call_method!(self, set_signature_bytes, signature)
//     }
//
//     fn modified_data_ids(&self) -> Vec<crate::prelude::Identifier> {
//         call_method!(self, modified_data_ids)
//     }
// }
