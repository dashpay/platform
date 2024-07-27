use crate::identity::state_transition::asset_lock_proof::{Decode, Encode};
use crate::voting::vote_polls::contested_document_resource_vote_poll::ContestedDocumentResourceVotePoll;
use crate::ProtocolError;
use derive_more::From;
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use platform_value::Identifier;
#[cfg(feature = "vote-serde-conversion")]
use serde::{Deserialize, Serialize};
use std::fmt;

pub mod contested_document_resource_vote_poll;

#[derive(Debug, Clone, Encode, Decode, PlatformSerialize, PlatformDeserialize, PartialEq, From)]
#[cfg_attr(
    feature = "vote-serde-conversion",
    derive(Serialize, Deserialize),
    serde(rename_all = "camelCase")
)]
#[platform_serialize(unversioned)]
#[platform_serialize(limit = 100000)]
#[ferment_macro::export]
pub enum VotePoll {
    ContestedDocumentResourceVotePoll(ContestedDocumentResourceVotePoll),
}

impl fmt::Display for VotePoll {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            VotePoll::ContestedDocumentResourceVotePoll(poll) => {
                write!(f, "ContestedDocumentResourceVotePoll({})", poll)
            }
        }
    }
}

impl Default for VotePoll {
    fn default() -> Self {
        ContestedDocumentResourceVotePoll::default().into()
    }
}

impl VotePoll {
    pub fn specialized_balance_id(&self) -> Result<Option<Identifier>, ProtocolError> {
        match self {
            VotePoll::ContestedDocumentResourceVotePoll(contested_document_resource_vote_poll) => {
                Ok(Some(
                    contested_document_resource_vote_poll.specialized_balance_id()?,
                ))
            }
        }
    }

    pub fn unique_id(&self) -> Result<Identifier, ProtocolError> {
        match self {
            VotePoll::ContestedDocumentResourceVotePoll(contested_document_resource_vote_poll) => {
                contested_document_resource_vote_poll.unique_id()
            }
        }
    }
}
