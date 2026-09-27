use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::errors::ProtocolError;
use crate::identity::KeyID;
use bincode::{Decode, DecodeUntrusted, Encode};
use platform_serialization_derive::{
    PlatformDeserializeTrusted, PlatformDeserializeUntrusted, PlatformSerialize,
};
use thiserror::Error;

/// `IdentityCreateFromShieldedPool` binds its keys, bounds included, into the Orchard sighash
/// preimage, whose layout predates contract group bounds. A key bound to a contract group must
/// be added with an identity update instead.
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
#[error("Key {key_id} is bound to a contract group, which an identity created from the shielded pool cannot register; add the key with an identity update")]
#[platform_serialize(unversioned)]
pub struct ContractGroupBoundKeyNotAllowedInShieldedIdentityCreationError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    key_id: KeyID,
}

impl ContractGroupBoundKeyNotAllowedInShieldedIdentityCreationError {
    pub fn new(key_id: KeyID) -> Self {
        Self { key_id }
    }

    pub fn key_id(&self) -> KeyID {
        self.key_id
    }
}

impl From<ContractGroupBoundKeyNotAllowedInShieldedIdentityCreationError> for ConsensusError {
    fn from(err: ContractGroupBoundKeyNotAllowedInShieldedIdentityCreationError) -> Self {
        Self::BasicError(
            BasicError::ContractGroupBoundKeyNotAllowedInShieldedIdentityCreationError(err),
        )
    }
}
