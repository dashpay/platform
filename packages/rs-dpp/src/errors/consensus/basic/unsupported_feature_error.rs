use crate::errors::consensus::basic::BasicError;
use crate::errors::consensus::ConsensusError;
use crate::errors::ProtocolError;
use bincode::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};

use thiserror::Error;

#[derive(
    Error, Debug, Clone, PartialEq, Eq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[error("feature {feature_name} is not supported. This validation was done in protocol version {current_protocol_version}")]
#[platform_serialize(unversioned)]
#[cfg_attr(feature = "apple", ferment_macro::export)]
pub struct UnsupportedFeatureError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    pub feature_name: String,
    pub current_protocol_version: u32,
}

impl UnsupportedFeatureError {
    pub fn new(feature_name: String, current_protocol_version: u32) -> Self {
        Self {
            feature_name,
            current_protocol_version,
        }
    }

    pub fn feature(&self) -> &String {
        &self.feature_name
    }

    pub fn current_protocol_version(&self) -> u32 {
        self.current_protocol_version
    }
}

impl From<UnsupportedFeatureError> for ConsensusError {
    fn from(err: UnsupportedFeatureError) -> Self {
        Self::BasicError(BasicError::UnsupportedFeatureError(err))
    }
}
