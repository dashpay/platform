use bincode::{Decode, DecodeUntrusted, Encode};
#[cfg(feature = "serde-conversion")]
use serde::{Deserialize, Serialize};
use std::fmt;

/// The answer a masternode gives a yes/no vote poll. Abstaining counts towards nothing: the
/// poll's supermajority and minimum are measured over yes plus no only.
#[derive(
    Debug,
    Clone,
    Copy,
    Encode,
    Decode,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Default,
    DecodeUntrusted,
)]
#[cfg_attr(
    feature = "serde-conversion",
    derive(Serialize, Deserialize),
    serde(rename_all = "camelCase")
)]
pub enum YesNoAbstainVoteChoice {
    Yes,
    No,
    #[default]
    Abstain,
}

impl YesNoAbstainVoteChoice {
    /// Every choice, in tree key order.
    pub const ALL: [YesNoAbstainVoteChoice; 3] = [
        YesNoAbstainVoteChoice::Yes,
        YesNoAbstainVoteChoice::No,
        YesNoAbstainVoteChoice::Abstain,
    ];
}

impl fmt::Display for YesNoAbstainVoteChoice {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            YesNoAbstainVoteChoice::Yes => write!(f, "Yes"),
            YesNoAbstainVoteChoice::No => write!(f, "No"),
            YesNoAbstainVoteChoice::Abstain => write!(f, "Abstain"),
        }
    }
}

// --- canonical conversion trait impls (unification pass 1) ---
#[cfg(all(feature = "json-conversion", feature = "serde-conversion"))]
impl crate::serialization::JsonConvertible for YesNoAbstainVoteChoice {}

#[cfg(all(feature = "value-conversion", feature = "serde-conversion"))]
impl crate::serialization::ValueConvertible for YesNoAbstainVoteChoice {}

#[cfg(all(
    test,
    feature = "json-conversion",
    feature = "value-conversion",
    feature = "serde-conversion"
))]
mod json_convertible_tests_yesnoabstainvotechoice {
    use super::*;
    use platform_value::platform_value;
    use serde_json::json;

    // A unit-only enum with `rename_all = "camelCase"`: each variant is a plain lowercase
    // string on the wire.

    #[test]
    fn json_round_trip_yes() {
        use crate::serialization::JsonConvertible;
        let original = YesNoAbstainVoteChoice::Yes;
        let json = original.to_json().expect("to_json");
        assert_eq!(json, json!("yes"));
        let recovered = YesNoAbstainVoteChoice::from_json(json).expect("from_json");
        assert_eq!(original, recovered);
    }

    #[test]
    fn json_round_trip_no() {
        use crate::serialization::JsonConvertible;
        let original = YesNoAbstainVoteChoice::No;
        let json = original.to_json().expect("to_json");
        assert_eq!(json, json!("no"));
        let recovered = YesNoAbstainVoteChoice::from_json(json).expect("from_json");
        assert_eq!(original, recovered);
    }

    #[test]
    fn json_round_trip_abstain() {
        use crate::serialization::JsonConvertible;
        let original = YesNoAbstainVoteChoice::Abstain;
        let json = original.to_json().expect("to_json");
        assert_eq!(json, json!("abstain"));
        let recovered = YesNoAbstainVoteChoice::from_json(json).expect("from_json");
        assert_eq!(original, recovered);
    }

    #[test]
    fn value_round_trip_yes() {
        use crate::serialization::ValueConvertible;
        let original = YesNoAbstainVoteChoice::Yes;
        let value = original.to_object().expect("to_object");
        assert_eq!(value, platform_value!("yes"));
        let recovered = YesNoAbstainVoteChoice::from_object(value).expect("from_object");
        assert_eq!(original, recovered);
    }

    #[test]
    fn value_round_trip_no() {
        use crate::serialization::ValueConvertible;
        let original = YesNoAbstainVoteChoice::No;
        let value = original.to_object().expect("to_object");
        assert_eq!(value, platform_value!("no"));
        let recovered = YesNoAbstainVoteChoice::from_object(value).expect("from_object");
        assert_eq!(original, recovered);
    }

    #[test]
    fn value_round_trip_abstain() {
        use crate::serialization::ValueConvertible;
        let original = YesNoAbstainVoteChoice::Abstain;
        let value = original.to_object().expect("to_object");
        assert_eq!(value, platform_value!("abstain"));
        let recovered = YesNoAbstainVoteChoice::from_object(value).expect("from_object");
        assert_eq!(original, recovered);
    }
}
