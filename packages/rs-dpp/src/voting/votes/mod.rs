pub mod resource_vote;

use crate::voting::votes::resource_vote::accessors::v0::ResourceVoteGettersV0;
use crate::voting::votes::resource_vote::ResourceVote;
#[cfg(feature = "vote-serialization")]
use crate::ProtocolError;
#[cfg(feature = "vote-serialization")]
use bincode::{Decode, Encode};
#[cfg(feature = "vote-serialization")]
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use platform_value::Identifier;
use platform_version::version::PlatformVersion;
use serde::{Deserialize, Serialize};

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

impl Vote {
    pub fn specialized_balance_id(&self) -> Result<Option<Identifier>, ProtocolError> {
        match self {
            Vote::ResourceVote(resource_vote) => resource_vote.vote_poll().specialized_balance_id(),
        }
    }
}
