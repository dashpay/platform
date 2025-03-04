use crate::voting::votes::resource_vote::v0::ResourceVoteV0;
use crate::ProtocolError;
use bincode::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
#[cfg(feature = "vote-serde-conversion")]
use serde::{Deserialize, Serialize};

pub mod accessors;
pub mod v0;

#[derive(Debug, Clone, Encode, Decode, PlatformSerialize, PlatformDeserialize, PartialEq)]
#[cfg_attr(
    feature = "vote-serde-conversion",
    derive(Serialize, Deserialize),
    serde(tag = "$version")
)]
#[platform_serialize(limit = 15000, unversioned)]
#[cfg_attr(feature = "apple", ferment_macro::export)]
pub enum ResourceVote {
    #[cfg_attr(feature = "vote-serde-conversion", serde(rename = "0"))]
    V0(ResourceVoteV0),
}

impl Default for ResourceVote {
    fn default() -> Self {
        Self::V0(ResourceVoteV0::default())
    }
}
