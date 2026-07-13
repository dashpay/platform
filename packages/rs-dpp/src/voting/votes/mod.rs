pub mod resource_vote;

#[cfg(feature = "json-conversion")]
use crate::serialization::JsonConvertible;
#[cfg(feature = "value-conversion")]
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
    derive(Serialize, Deserialize),
    // Internal tagging with `$type` — system-field convention. Drops the
    // `data` wrapper. Inner `ResourceVote` is `tag = "$formatVersion"`
    // (different key, no collision).
    serde(tag = "$type", rename_all = "camelCase")
)]
#[cfg_attr(feature = "value-conversion", derive(ValueConvertible))]
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

#[cfg(all(
    test,
    feature = "json-conversion",
    feature = "value-conversion",
    feature = "serde-conversion"
))]
mod json_convertible_tests_vote {
    use super::*;

    #[test]
    fn json_round_trip_vote() {
        use crate::serialization::JsonConvertible;
        let original = Vote::default();
        let json = original.to_json().expect("to_json");
        let recovered = Vote::from_json(json).expect("from_json");
        assert_eq!(original, recovered);
    }

    #[test]
    fn value_round_trip_vote() {
        use crate::serialization::ValueConvertible;
        let original = Vote::default();
        let value = original.to_object().expect("to_object");
        let recovered = Vote::from_object(value).expect("from_object");
        assert_eq!(original, recovered);
    }
}
