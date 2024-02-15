use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::errors::ProtocolError;
use bincode::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use thiserror::Error;
use crate::prelude::IdentityContractNonce;

#[derive(
Error, Debug, Clone, PartialEq, Eq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[error(
"Document transitions with duplicate unique properties: {:?}",
references
)]
#[platform_serialize(unversioned)]
pub struct IdentityContractNonceOutOfBoundsError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    identity_contract_nonce: IdentityContractNonce,
}

impl IdentityContractNonceOutOfBoundsError {
    pub fn new(identity_contract_nonce: IdentityContractNonce) -> Self {
        Self { identity_contract_nonce }
    }

    pub fn identity_contract_nonce(&self) -> IdentityContractNonce {
        self.identity_contract_nonce
    }
}

impl From<IdentityContractNonceOutOfBoundsError> for ConsensusError {
    fn from(err: IdentityContractNonceOutOfBoundsError) -> Self {
        Self::BasicError(BasicError::IdentityContractNonceOutOfBoundsError(
            err,
        ))
    }
}
