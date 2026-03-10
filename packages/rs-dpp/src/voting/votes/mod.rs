pub mod resource_vote;

#[cfg(feature = "json-conversion")]
use crate::serialization::JsonConvertible;
#[cfg(feature = "serde-conversion")]
use crate::serialization::ValueConvertible;
use crate::voting::votes::resource_vote::accessors::v0::ResourceVoteGettersV0;
use crate::voting::votes::resource_vote::ResourceVote;
use crate::ProtocolError;
use bincode::{Decode, Encode};
use derive_more::From;
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use platform_value::Identifier;
#[cfg(feature = "serde-conversion")]
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Encode, Decode, PlatformSerialize, PlatformDeserialize, PartialEq, From)]
#[cfg_attr(
    feature = "serde-conversion",
    derive(Serialize, Deserialize, ValueConvertible),
    serde(tag = "type", content = "data", rename_all = "camelCase")
)]
#[platform_serialize(limit = 15000, unversioned)]
pub enum Vote {
    ResourceVote(ResourceVote),
}

// Manual impl because Vote is a flat enum (not versioned V0/V1).
#[cfg(feature = "json-conversion")]
impl JsonConvertible for Vote {}

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

#[cfg(all(test, feature = "json-conversion"))]
mod tests {
    use super::*;
    use crate::serialization::JsonConvertible;
    use crate::voting::vote_choices::resource_vote_choice::ResourceVoteChoice;
    use crate::voting::vote_polls::contested_document_resource_vote_poll::ContestedDocumentResourceVotePoll;
    use crate::voting::vote_polls::VotePoll;
    use crate::voting::votes::resource_vote::v0::ResourceVoteV0;
    use crate::voting::votes::resource_vote::ResourceVote;

    #[test]
    fn vote_json_round_trip() {
        let contract_id = Identifier::from([0x11u8; 32]);
        let towards_id = Identifier::from([0x22u8; 32]);

        let vote = Vote::ResourceVote(ResourceVote::V0(ResourceVoteV0 {
            vote_poll: VotePoll::ContestedDocumentResourceVotePoll(
                ContestedDocumentResourceVotePoll {
                    contract_id,
                    document_type_name: "domain".to_string(),
                    index_name: "parentNameAndLabel".to_string(),
                    index_values: vec![platform_value::Value::Text("dash".to_string())],
                },
            ),
            resource_vote_choice: ResourceVoteChoice::TowardsIdentity(towards_id),
        }));

        let json = vote.to_json().expect("to_json should succeed");

        // Verify it's a valid JSON object
        assert!(json.is_object(), "Vote JSON should be an object");

        // round-trip
        let restored = Vote::from_json(json).expect("from_json should succeed");
        assert_eq!(vote, restored);
    }
}
