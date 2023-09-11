use crate::voting::resource_vote::ResourceVote;
use crate::voting::Vote::ContestedDocumentResourceVote;
use bincode::{Decode, Encode};
use platform_value::{Identifier, Value};
use serde::{Deserialize, Serialize};

pub mod common_vote;
pub mod resource_vote;

type ContractId = Identifier;
type DocumentTypeName = String;

type IndexName = String;

type IndexValues = Vec<Value>;

#[derive(Debug, Clone, Encode, Decode, PartialEq)]
#[cfg_attr(
    feature = "state-transition-serde-conversion",
    derive(Serialize, Deserialize),
    serde(rename_all = "camelCase")
)]
pub enum Vote {
    ContestedDocumentResourceVote(
        ContractId,
        DocumentTypeName,
        IndexName,
        IndexValues,
        ResourceVote,
    ),
}

impl Default for Vote {
    fn default() -> Self {
        ContestedDocumentResourceVote(
            Identifier::default(),
            String::default(),
            String::default(),
            Vec::default(),
            ResourceVote::Abstain,
        )
    }
}
