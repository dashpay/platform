use crate::voting::resource_vote::ResourceVote;
use crate::voting::Vote::ContestedDocumentResourceVote;
use crate::ProtocolError;
use bincode::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use platform_value::{Identifier, Value};
use serde::{Deserialize, Serialize};

pub mod common_vote;
pub mod resource_vote;

#[derive(Debug, Clone, Encode, Decode, PartialEq)]
#[cfg_attr(
    feature = "state-transition-serde-conversion",
    derive(Serialize, Deserialize),
    serde(rename_all = "camelCase")
)]
pub struct ContestedDocumentResourceVotePoll {
    pub contract_id: Identifier,
    pub document_type_name: String,
    pub index_name: String,
    pub index_values: Vec<Value>,
}

impl Default for ContestedDocumentResourceVotePoll {
    fn default() -> Self {
        ContestedDocumentResourceVotePoll {
            contract_id: Default::default(),
            document_type_name: "".to_string(),
            index_name: "".to_string(),
            index_values: vec![],
        }
    }
}

#[derive(Debug, Clone, Encode, Decode, PlatformDeserialize, PlatformSerialize, PartialEq)]
#[cfg_attr(
    feature = "state-transition-serde-conversion",
    derive(Serialize, Deserialize),
    serde(rename_all = "camelCase")
)]
#[platform_serialize(unversioned)]
pub struct ContestedDocumentResourceVoteType {
    pub vote_poll: ContestedDocumentResourceVotePoll,
    pub resource_vote: ResourceVote,
}

impl Default for ContestedDocumentResourceVoteType {
    fn default() -> Self {
        ContestedDocumentResourceVoteType {
            vote_poll: ContestedDocumentResourceVotePoll::default(),
            resource_vote: ResourceVote::Abstain,
        }
    }
}

#[derive(Debug, Clone, Encode, Decode, PartialEq)]
#[cfg_attr(
    feature = "state-transition-serde-conversion",
    derive(Serialize, Deserialize),
    serde(rename_all = "camelCase")
)]
pub enum Vote {
    ContestedDocumentResourceVote(ContestedDocumentResourceVoteType),
}

impl Default for Vote {
    fn default() -> Self {
        ContestedDocumentResourceVote(ContestedDocumentResourceVoteType::default())
    }
}
