use crate::ProtocolError;
use bincode::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use serde::{Deserialize, Serialize};
use crate::voting::vote_choices::resource_vote_choice::ResourceVoteChoice;
use crate::voting::vote_polls::contested_document_resource_vote_poll::ContestedDocumentResourceVotePoll;

#[derive(Debug, Clone, Encode, Decode, PlatformDeserialize, PlatformSerialize, PartialEq)]
#[cfg_attr(
feature = "state-transition-serde-conversion",
derive(Serialize, Deserialize),
serde(rename_all = "camelCase")
)]
#[platform_serialize(unversioned)]
pub struct ContestedDocumentResourceVoteV0 {
    pub vote_poll: ContestedDocumentResourceVotePoll,
    pub resource_vote_choice: ResourceVoteChoice,
}

impl Default for ContestedDocumentResourceVoteV0 {
    fn default() -> Self {
        ContestedDocumentResourceVoteV0 {
            vote_poll: ContestedDocumentResourceVotePoll::default(),
            resource_vote_choice: ResourceVoteChoice::Abstain,
        }
    }
}
