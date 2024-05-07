pub mod resource_vote;

use crate::voting::votes::resource_vote::ResourceVote;
use serde::{Deserialize, Serialize};
#[cfg(feature = "vote-serialization")]
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
#[cfg(feature = "vote-serialization")]
use bincode::{Decode, Encode};
#[cfg(feature = "vote-serialization")]
use crate::ProtocolError;

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(
    feature = "vote-serde-conversion",
    derive(Serialize, Deserialize),
    serde(rename_all = "camelCase")
)]
#[cfg_attr(
feature = "vote-serialization",
derive(Encode, Decode, PlatformDeserialize, PlatformSerialize),
platform_serialize(limit = 15000, unversioned)
)]
pub enum Vote {
    ResourceVote(ResourceVote),
}

impl Default for Vote {
    fn default() -> Self {
        Vote::ResourceVote(ResourceVote::default())
    }
}
