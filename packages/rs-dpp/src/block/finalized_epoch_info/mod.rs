mod getters;
pub mod v0;

use crate::block::finalized_epoch_info::v0::FinalizedEpochInfoV0;
use crate::protocol_error::ProtocolError;
#[cfg(feature = "json-conversion")]
use crate::serialization::JsonConvertible;
use crate::serialization::ValueConvertible;
use bincode::{Decode, Encode};
use derive_more::From;
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use serde::{Deserialize, Serialize};

/// Finalized Epoch information
#[derive(
    Clone,
    Debug,
    PartialEq,
    Serialize,
    Deserialize,
    Encode,
    Decode,
    PlatformSerialize,
    PlatformDeserialize,
    From,
)]
#[platform_serialize(unversioned)] //versioned directly, no need to use platform_version
#[serde(tag = "$formatVersion")]
pub enum FinalizedEpochInfo {
    #[serde(rename = "0")]
    V0(FinalizedEpochInfoV0),
}

#[cfg(feature = "json-conversion")]
impl JsonConvertible for FinalizedEpochInfo {}
impl ValueConvertible for FinalizedEpochInfo {}
