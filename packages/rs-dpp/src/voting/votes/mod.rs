pub mod contested_document_resource_vote;

use crate::identity::state_transition::asset_lock_proof::{Decode, Encode};
use crate::voting::votes::contested_document_resource_vote::ContestedDocumentResourceVote;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Encode, Decode, PartialEq)]
#[cfg_attr(
    feature = "vote-serde-conversion",
    derive(Serialize, Deserialize),
    serde(rename_all = "camelCase")
)]
pub enum Vote {
    ContestedDocumentResourceVote(ContestedDocumentResourceVote),
}

impl Default for Vote {
    fn default() -> Self {
        Vote::ContestedDocumentResourceVote(ContestedDocumentResourceVote::default())
    }
}
