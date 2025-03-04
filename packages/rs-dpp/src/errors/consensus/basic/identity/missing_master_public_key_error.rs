use crate::errors::consensus::basic::BasicError;
use crate::errors::consensus::ConsensusError;
use crate::errors::ProtocolError;
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use thiserror::Error;

use bincode::{Decode, Encode};

#[derive(
    Error,
    Debug,
    Clone,
    PartialEq,
    Eq,
    Default,
    Encode,
    Decode,
    PlatformSerialize,
    PlatformDeserialize,
)]
#[error(
    "Identity doesn't contain any master key, thus can not be updated. Please add a master key"
)]
#[platform_serialize(unversioned)]
#[cfg_attr(feature = "apple", ferment_macro::export)]
pub struct MissingMasterPublicKeyError;

/*

DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

*/

impl MissingMasterPublicKeyError {
    pub fn new() -> Self {
        Self
    }
}
impl From<MissingMasterPublicKeyError> for ConsensusError {
    fn from(err: MissingMasterPublicKeyError) -> Self {
        Self::BasicError(BasicError::MissingMasterPublicKeyError(err))
    }
}
