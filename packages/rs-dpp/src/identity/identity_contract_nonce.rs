use crate::ProtocolError;
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use std::fmt::{Debug, Display, Formatter};

use crate::prelude::IdentityContractNonce;
use bincode::{Decode, Encode};

pub const IDENTITY_CONTRACT_NONCE_VALUE_FILTER: u64 = 0xFFFFFFFFFF;
pub const MISSING_IDENTITY_CONTRACT_REVISIONS_FILTER: u64 = 0xFFFFFF0000000000;
pub const MAX_MISSING_IDENTITY_CONTRACT_REVISIONS: u64 = 20;
pub const MISSING_IDENTITY_CONTRACT_REVISIONS_MAX_BYTES: u64 =
    MAX_MISSING_IDENTITY_CONTRACT_REVISIONS;
pub const IDENTITY_CONTRACT_NONCE_VALUE_FILTER_MAX_BYTES: u64 = 40;

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
/// The result of the merge of the identity contract nonce
pub enum MergeIdentityContractNonceResult {
    /// The nonce is too far in the future
    NonceTooFarInFuture,
    /// The nonce is too far in the past
    NonceTooFarInPast,
    /// The nonce is already present at the tip
    NonceAlreadyPresentAtTip,
    /// The nonce is already present in the past
    NonceAlreadyPresentInPast(u64),
    /// The merge is a success
    MergeIdentityContractNonceSuccess(IdentityContractNonce),
}

impl Display for MergeIdentityContractNonceResult {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.error_message().unwrap_or_else(|| "no error"))
    }
}

impl MergeIdentityContractNonceResult {
    /// Gives a result from the enum
    pub fn error_message(self) -> Option<&'static str> {
        match self {
            MergeIdentityContractNonceResult::NonceTooFarInFuture => {
                Some("nonce too far in future")
            }
            MergeIdentityContractNonceResult::NonceTooFarInPast => Some("nonce too far in past"),
            MergeIdentityContractNonceResult::NonceAlreadyPresentAtTip => {
                Some("nonce already present at tip")
            }
            MergeIdentityContractNonceResult::NonceAlreadyPresentInPast(_) => {
                Some("nonce already present in past")
            }
            MergeIdentityContractNonceResult::MergeIdentityContractNonceSuccess(_) => None,
        }
    }
}
