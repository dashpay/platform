use crate::identity::signer::Signer;
use crate::identity::{IdentityPublicKey, SecurityLevel};
use crate::state_transition::public_key_in_creation::accessors::IdentityPublicKeyInCreationV0Getters;
use crate::state_transition::public_key_in_creation::v0::IdentityPublicKeyInCreationV0;
use crate::{BlsModule, ProtocolError};

pub trait IdentityPublicKeyInCreationMethodsV0: IdentityPublicKeyInCreationV0Getters {
    fn into_identity_public_key(self) -> IdentityPublicKey;
    fn from_public_key_signed_with_private_key(
        public_key: IdentityPublicKey,
        state_transition_bytes: &[u8],
        private_key: &[u8],
        bls: &impl BlsModule,
    ) -> Result<Self, ProtocolError>
    where
        Self: Sized;

    fn from_public_key_signed_external<S: Signer>(
        public_key: IdentityPublicKey,
        state_transition_bytes: &[u8],
        signer: &S,
    ) -> Result<Self, ProtocolError>
    where
        Self: Sized;

    /// Checks if public key security level is MASTER
    fn is_master(&self) -> bool {
        self.security_level() == SecurityLevel::MASTER
    }
    /// Get the original public key hash
    fn hash(&self) -> Result<[u8; 20], ProtocolError>;
    /// Get the original public key hash
    fn hash_as_vec(&self) -> Result<Vec<u8>, ProtocolError>;
}
