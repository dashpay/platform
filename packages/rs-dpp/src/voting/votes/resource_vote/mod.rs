#[cfg(feature = "json-conversion")]
use crate::serialization::JsonConvertible;
#[cfg(feature = "value-conversion")]
use crate::serialization::ValueConvertible;
use crate::voting::votes::resource_vote::v0::ResourceVoteV0;
use crate::ProtocolError;
use bincode::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
#[cfg(feature = "serde-conversion")]
use serde::{Deserialize, Serialize};

pub mod accessors;
pub mod v0;

#[cfg_attr(
    all(feature = "json-conversion", feature = "serde-conversion"),
    derive(JsonConvertible)
)]
#[derive(Debug, Clone, Encode, Decode, PlatformSerialize, PlatformDeserialize, PartialEq)]
#[cfg_attr(
    feature = "serde-conversion",
    derive(Serialize, Deserialize),
    serde(tag = "$formatVersion")
)]
#[cfg_attr(feature = "value-conversion", derive(ValueConvertible))]
#[platform_serialize(limit = 15000, unversioned)]
pub enum ResourceVote {
    #[cfg_attr(feature = "serde-conversion", serde(rename = "0"))]
    V0(ResourceVoteV0),
}

impl Default for ResourceVote {
    fn default() -> Self {
        Self::V0(ResourceVoteV0::default())
    }
}

#[cfg(all(test, feature = "json-conversion", feature = "value-conversion", feature = "serde-conversion"))]
mod json_convertible_tests_resource_vote {
    use super::*;
    use crate::voting::vote_choices::resource_vote_choice::ResourceVoteChoice;
    use crate::voting::vote_polls::contested_document_resource_vote_poll::ContestedDocumentResourceVotePoll;
    use crate::voting::vote_polls::VotePoll;
    use platform_value::{Identifier, Value};

    /// Non-default values per inner field (named contract / index / values
    /// inside the poll, plus a `TowardsIdentity` choice with non-zero
    /// identifier) so per-property assertions catch silent zero-out / variant
    /// flip on round-trip.
    fn fixture() -> ResourceVote {
        ResourceVote::V0(ResourceVoteV0 {
            vote_poll: VotePoll::ContestedDocumentResourceVotePoll(
                ContestedDocumentResourceVotePoll {
                    contract_id: Identifier::new([0xc1; 32]),
                    document_type_name: "preorder".to_string(),
                    index_name: "parentNameAndLabel".to_string(),
                    index_values: vec![Value::Text("dash".to_string())],
                },
            ),
            resource_vote_choice: ResourceVoteChoice::TowardsIdentity(Identifier::new([0xab; 32])),
        })
    }

    fn assert_v0_fields(v: &ResourceVote) {
        let ResourceVote::V0(rec) = v;
        match &rec.vote_poll {
            VotePoll::ContestedDocumentResourceVotePoll(p) => {
                assert_eq!(p.contract_id, Identifier::new([0xc1; 32]), "contract_id");
                assert_eq!(p.document_type_name, "preorder", "document_type_name");
                assert_eq!(p.index_name, "parentNameAndLabel", "index_name");
                assert_eq!(p.index_values.len(), 1, "index_values.len");
            }
        }
        match rec.resource_vote_choice {
            ResourceVoteChoice::TowardsIdentity(id) => {
                assert_eq!(id, Identifier::new([0xab; 32]), "resource_vote_choice.id");
            }
            other => panic!("expected TowardsIdentity, got {:?}", other),
        }
    }

    #[test]
    fn json_round_trip_with_per_property_assertions() {
        use crate::serialization::JsonConvertible;
        let original = fixture();
        let json = original.to_json().expect("to_json");
        let recovered = ResourceVote::from_json(json).expect("from_json");
        assert_eq!(original, recovered);
        assert_v0_fields(&recovered);
    }

    #[test]
    fn value_round_trip_with_per_property_assertions() {
        use crate::serialization::ValueConvertible;
        let original = fixture();
        let value = original.to_object().expect("to_object");
        let recovered = ResourceVote::from_object(value).expect("from_object");
        assert_eq!(original, recovered);
        assert_v0_fields(&recovered);
    }
}
