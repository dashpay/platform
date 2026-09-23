#[cfg(feature = "json-conversion")]
use crate::serialization::JsonConvertible;
#[cfg(feature = "value-conversion")]
use crate::serialization::ValueConvertible;
use crate::voting::votes::yes_no_vote::v0::YesNoVoteV0;
use crate::ProtocolError;
use bincode::{Decode, DecodeUntrusted, Encode};
use platform_serialization_derive::{
    PlatformDeserializeTrusted, PlatformDeserializeUntrusted, PlatformSerialize,
};
#[cfg(feature = "serde-conversion")]
use serde::{Deserialize, Serialize};

pub mod accessors;
pub mod v0;

/// A masternode's answer to a yes/no vote poll (protocol version 14).
#[cfg_attr(
    all(feature = "json-conversion", feature = "serde-conversion"),
    derive(JsonConvertible)
)]
#[derive(
    Debug,
    Clone,
    Encode,
    Decode,
    PlatformSerialize,
    PlatformDeserializeTrusted,
    PlatformDeserializeUntrusted,
    PartialEq,
    Eq,
    DecodeUntrusted,
)]
#[cfg_attr(
    feature = "serde-conversion",
    derive(Serialize, Deserialize),
    serde(tag = "$formatVersion")
)]
#[cfg_attr(feature = "value-conversion", derive(ValueConvertible))]
#[platform_serialize(limit = 15000, unversioned)]
pub enum YesNoVote {
    #[cfg_attr(feature = "serde-conversion", serde(rename = "0"))]
    V0(YesNoVoteV0),
}

#[cfg(all(
    test,
    feature = "json-conversion",
    feature = "value-conversion",
    feature = "serde-conversion"
))]
mod json_convertible_tests_yes_no_vote {
    use super::*;
    use crate::voting::vote_choices::yes_no_abstain_vote_choice::YesNoAbstainVoteChoice;
    use crate::voting::vote_polls::yes_no_vote_poll::YesNoVotePoll;
    use platform_value::{platform_value, BinaryData};
    use serde_json::json;

    fn fixture() -> YesNoVote {
        YesNoVote::V0(YesNoVoteV0 {
            vote_poll: YesNoVotePoll {
                resource_path: vec![BinaryData::new(vec![0xc1; 4])],
                supermajority_numerator: 2,
                supermajority_denominator: 3,
                minimum_voting_power: 400,
            },
            vote_choice: YesNoAbstainVoteChoice::Yes,
        })
    }

    #[test]
    fn json_round_trip_with_full_wire_shape() {
        use crate::serialization::JsonConvertible;
        let original = fixture();
        let json = original.to_json().expect("to_json");
        assert_eq!(
            json,
            json!({
                "$formatVersion": "0",
                "votePoll": {
                    "resourcePath": ["wcHBwQ=="],
                    "supermajorityNumerator": 2,
                    "supermajorityDenominator": 3,
                    "minimumVotingPower": 400,
                },
                "voteChoice": "yes",
            })
        );
        let recovered = YesNoVote::from_json(json).expect("from_json");
        assert_eq!(original, recovered);
    }

    #[test]
    fn value_round_trip_with_full_wire_shape() {
        use crate::serialization::ValueConvertible;
        let original = fixture();
        let value = original.to_object().expect("to_object");
        assert_eq!(
            value,
            platform_value!({
                "$formatVersion": "0",
                "votePoll": {
                    "resourcePath": [platform_value::Value::Bytes(vec![0xc1; 4])],
                    "supermajorityNumerator": 2u8,
                    "supermajorityDenominator": 3u8,
                    "minimumVotingPower": 400u32,
                },
                "voteChoice": "yes",
            })
        );
        let recovered = YesNoVote::from_object(value).expect("from_object");
        assert_eq!(original, recovered);
    }
}
