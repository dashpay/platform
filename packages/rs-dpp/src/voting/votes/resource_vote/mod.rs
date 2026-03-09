#[cfg(all(feature = "json-conversion", feature = "vote-serde-conversion"))]
use crate::serialization::JsonConvertible;
#[cfg(feature = "vote-serde-conversion")]
use crate::serialization::ValueConvertible;
use crate::voting::votes::resource_vote::v0::ResourceVoteV0;
use crate::ProtocolError;
use bincode::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
#[cfg(feature = "vote-serde-conversion")]
use serde::{Deserialize, Serialize};

pub mod accessors;
pub mod v0;

#[cfg_attr(
    all(feature = "json-conversion", feature = "vote-serde-conversion"),
    derive(JsonConvertible)
)]
#[derive(Debug, Clone, Encode, Decode, PlatformSerialize, PlatformDeserialize, PartialEq)]
#[cfg_attr(
    feature = "vote-serde-conversion",
    derive(Serialize, Deserialize, ValueConvertible),
    serde(tag = "$formatVersion")
)]
#[platform_serialize(limit = 15000, unversioned)]
pub enum ResourceVote {
    #[cfg_attr(feature = "vote-serde-conversion", serde(rename = "0"))]
    V0(ResourceVoteV0),
}

impl Default for ResourceVote {
    fn default() -> Self {
        Self::V0(ResourceVoteV0::default())
    }
}
