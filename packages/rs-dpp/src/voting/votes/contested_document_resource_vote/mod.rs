use crate::identity::state_transition::asset_lock_proof::{Decode, Encode};
use crate::voting::votes::contested_document_resource_vote::v0::ContestedDocumentResourceVoteV0;
use crate::ProtocolError;
use derive_more::From;
#[cfg(feature = "vote-serialization")]
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
#[cfg(feature = "vote-serde-conversion")]
use serde::{Deserialize, Serialize};

pub mod accessors;
mod v0;

#[derive(Debug, Clone, PartialEq, From)]
#[cfg_attr(
    feature = "vote-serde-conversion",
    derive(Serialize, Deserialize),
    serde(tag = "$version")
)]
#[cfg_attr(
    feature = "vote-serialization",
    derive(Encode, Decode, PlatformDeserialize, PlatformSerialize),
    platform_serialize(limit = 15000, unversioned)
)]
pub enum ContestedDocumentResourceVote {
    #[cfg_attr(feature = "vote-serde-conversion", serde(rename = "0"))]
    V0(ContestedDocumentResourceVoteV0),
}

impl Default for ContestedDocumentResourceVote {
    fn default() -> Self {
        Self::V0(ContestedDocumentResourceVoteV0::default())
    }
}
