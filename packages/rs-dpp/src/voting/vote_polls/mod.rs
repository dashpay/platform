use derive_more::From;
use serde::{Deserialize, Serialize};
use crate::identity::state_transition::asset_lock_proof::{Decode, Encode};
use crate::voting::vote_polls::contested_document_resource_vote_poll::ContestedDocumentResourceVotePoll;

pub mod contested_document_resource_vote_poll;

#[derive(Debug, Clone, Encode, Decode, PartialEq, From)]
#[cfg_attr(
feature = "state-transition-serde-conversion",
derive(Serialize, Deserialize),
serde(rename_all = "camelCase")
)]
pub enum VotePoll {
    ContestedDocumentResourceVotePoll(ContestedDocumentResourceVotePoll)
}

impl Default for VotePoll {
    fn default() -> Self {
        ContestedDocumentResourceVotePoll::default().into()
    }
}