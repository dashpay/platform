use bincode::{Decode, Encode};
#[cfg(feature = "serde-conversion")]
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Encode, Decode, PartialEq, Default)]
#[cfg_attr(
    feature = "serde-conversion",
    derive(Serialize, Deserialize),
    serde(rename_all = "camelCase")
)]
pub enum YesNoAbstainVoteChoice {
    YES,
    NO,
    #[default]
    ABSTAIN,
}

// --- canonical conversion trait impls (unification pass 1) ---
#[cfg(all(feature = "json-conversion", feature = "serde-conversion"))]
impl crate::serialization::JsonConvertible for YesNoAbstainVoteChoice {}

#[cfg(all(feature = "value-conversion", feature = "serde-conversion"))]
impl crate::serialization::ValueConvertible for YesNoAbstainVoteChoice {}


#[cfg(all(test, feature = "json-conversion", feature = "value-conversion", feature = "serde-conversion"))]
mod json_convertible_tests_yesnoabstainvotechoice {
    use super::*;

    #[test]
    fn json_round_trip_yesnoabstainvotechoice() {
        use crate::serialization::JsonConvertible;
        let original = YesNoAbstainVoteChoice::default();
        let json = original.to_json().expect("to_json");
        let recovered = YesNoAbstainVoteChoice::from_json(json).expect("from_json");
        assert_eq!(original, recovered);
    }

    #[test]
    fn value_round_trip_yesnoabstainvotechoice() {
        use crate::serialization::ValueConvertible;
        let original = YesNoAbstainVoteChoice::default();
        let value = original.to_object().expect("to_object");
        let recovered = YesNoAbstainVoteChoice::from_object(value).expect("from_object");
        assert_eq!(original, recovered);
    }
}
