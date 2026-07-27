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

#[cfg(all(
    test,
    feature = "json-conversion",
    feature = "value-conversion",
    feature = "serde-conversion"
))]
mod json_convertible_tests_resource_vote {
    use super::*;
    use crate::voting::vote_choices::resource_vote_choice::ResourceVoteChoice;
    use crate::voting::vote_polls::contested_document_resource_vote_poll::ContestedDocumentResourceVotePoll;
    use crate::voting::vote_polls::VotePoll;
    use platform_value::{platform_value, Identifier, Value};
    use serde_json::json;

    /// Non-default values per inner field (named contract / index / values
    /// inside the poll, plus a `TowardsIdentity` choice with non-zero
    /// identifier) so the wire-shape assertion catches silent zero-out /
    /// variant flip on round-trip.
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

    #[test]
    fn json_round_trip_with_full_wire_shape() {
        use crate::serialization::JsonConvertible;
        let original = fixture();
        let json = original.to_json().expect("to_json");
        // `VotePoll` uses internal tagging (`tag = "$type"`), so its variant
        // body fields are flattened next to the `$type` discriminator.
        // `ResourceVoteChoice` uses a custom Serialize/Deserialize that
        // emits `{"$type": "towardsIdentity", "identity": <id>}` for the
        // newtype variant. Identifiers render as base58 strings in JSON.
        assert_eq!(
            json,
            json!({
                "$formatVersion": "0",
                "votePoll": {
                    "$type": "contestedDocumentResourceVotePoll",
                    "contractId": "E3M3d7sy8ZKivUGxBexL9wxE7ebqzGWFqkdeFMedCJFS",
                    "documentTypeName": "preorder",
                    "indexName": "parentNameAndLabel",
                    "indexValues": ["dash"],
                },
                "resourceVoteChoice": {
                    "$type": "towardsIdentity",
                    "identity": "CZ8YUVdk7znjrUmnb5n7kgySk9yRAsQDYmyCxzfSky9t",
                },
            })
        );
        let recovered = ResourceVote::from_json(json).expect("from_json");
        assert_eq!(original, recovered);
    }

    #[test]
    fn value_round_trip_with_full_wire_shape() {
        use crate::serialization::ValueConvertible;
        let original = fixture();
        let value = original.to_object().expect("to_object");
        // platform_value preserves typed `Identifier` variants. Interpolate
        // through the macro so Serialize emits `Value::Identifier`.
        let contract_id = Identifier::new([0xc1; 32]);
        let voter_id = Identifier::new([0xab; 32]);
        assert_eq!(
            value,
            platform_value!({
                "$formatVersion": "0",
                "votePoll": {
                    "$type": "contestedDocumentResourceVotePoll",
                    "contractId": contract_id,
                    "documentTypeName": "preorder",
                    "indexName": "parentNameAndLabel",
                    "indexValues": ["dash"],
                },
                "resourceVoteChoice": {
                    "$type": "towardsIdentity",
                    "identity": voter_id,
                },
            })
        );
        let recovered = ResourceVote::from_object(value).expect("from_object");
        assert_eq!(original, recovered);
    }
}
