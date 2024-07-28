pub mod resource_vote;

use crate::voting::votes::resource_vote::accessors::v0::ResourceVoteGettersV0;
use crate::voting::votes::resource_vote::ResourceVote;
use crate::ProtocolError;
use bincode::{Decode, Encode};
use derive_more::From;
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use platform_value::Identifier;
#[cfg(feature = "vote-serde-conversion")]
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Encode, Decode, PlatformSerialize, PlatformDeserialize, PartialEq, From)]
#[cfg_attr(
    feature = "vote-serde-conversion",
    derive(Serialize, Deserialize),
    serde(rename_all = "camelCase")
)]
#[platform_serialize(limit = 15000, unversioned)]
#[ferment_macro::export]
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
    pub fn vote_poll_unique_id(&self) -> Result<Identifier, ProtocolError> {
        match self {
            Vote::ResourceVote(resource_vote) => resource_vote.vote_poll().unique_id(),
        }
    }
}
