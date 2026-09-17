use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::errors::ProtocolError;
use crate::identity::KeyID;
use bincode::{Decode, DecodeUntrusted, Encode};
use platform_serialization_derive::{
    PlatformDeserializeTrusted, PlatformDeserializeUntrusted, PlatformSerialize,
};
use thiserror::Error;

/// `IdentityCreateFromShieldedPool` has no identity signature: its keys are bound into the
/// Orchard sighash preimage field by field, and that layout predates the version 1 key. A
/// budget or an expiry would not be covered, so a relay could alter it on a key type that
/// carries no proof of possession. A version 1 key must be added with an identity update
/// instead.
#[derive(
    Error,
    Debug,
    Clone,
    PartialEq,
    Eq,
    Encode,
    Decode,
    PlatformSerialize,
    PlatformDeserializeTrusted,
    PlatformDeserializeUntrusted,
    DecodeUntrusted,
)]
#[error("Key {public_key_id} is a version 1 key, the format that can carry a budget or an expiry, which an identity created from the shielded pool cannot register; add the key with an identity update")]
#[platform_serialize(unversioned)]
pub struct IdentityPublicKeyLimitsNotAllowedInShieldedIdentityCreationError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    public_key_id: KeyID,
}

impl IdentityPublicKeyLimitsNotAllowedInShieldedIdentityCreationError {
    pub fn new(public_key_id: KeyID) -> Self {
        Self { public_key_id }
    }

    pub fn public_key_id(&self) -> KeyID {
        self.public_key_id
    }
}

impl From<IdentityPublicKeyLimitsNotAllowedInShieldedIdentityCreationError> for ConsensusError {
    fn from(err: IdentityPublicKeyLimitsNotAllowedInShieldedIdentityCreationError) -> Self {
        Self::BasicError(
            BasicError::IdentityPublicKeyLimitsNotAllowedInShieldedIdentityCreationError(err),
        )
    }
}
