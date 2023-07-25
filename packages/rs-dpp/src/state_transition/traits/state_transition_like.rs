use std::fmt::Debug;

use dashcore::signer;

use platform_value::{BinaryData, ReplacementType, Value, ValueMapHelper};

use crate::consensus::signature::InvalidStateTransitionSignatureError;
use crate::consensus::signature::SignatureError;
use crate::consensus::ConsensusError;

use crate::serialization::{PlatformSerializable, Signable};
use crate::version::FeatureVersion;
use crate::{
    identity::KeyType,
    prelude::{Identifier, ProtocolError},
    util::hash,
    BlsModule,
};

use crate::identity::KeyID;
#[cfg(any(
    feature = "state-transition-validation",
    feature = "state-transition-signing"
))]
use crate::state_transition::errors::InvalidIdentityPublicKeyTypeError;
#[cfg(any(
    feature = "state-transition-validation",
    feature = "state-transition-signing"
))]
use crate::state_transition::errors::StateTransitionIsNotSignedError;
use crate::state_transition::StateTransitionType;
use crate::state_transition::{StateTransition, StateTransitionFieldTypes};

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
    StateTransitionFieldTypes + Clone + Debug + Into<StateTransition> + Signable
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
    fn owner_id(&self) -> &Identifier;
}
