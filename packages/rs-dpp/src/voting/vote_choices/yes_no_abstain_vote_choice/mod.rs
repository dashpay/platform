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

    // `YesNoAbstainVoteChoice` is a unit-only enum with `rename_all = "camelCase"`,
    // so each variant serializes as a plain string: `"yes"` / `"no"` / `"abstain"`.

    // Surprise wire shape: the variants are SCREAMING_CASE in source
    // (`YES`/`NO`/`ABSTAIN`) and the type carries `rename_all = "camelCase"`.
    // serde's camelCase rule lowercases the FIRST letter only, leaving the
    // rest as-is — so the wire emits `"yES"` / `"nO"` / `"aBSTAIN"` rather
    // than the lowercase-clean strings a casual reader would expect. These
    // tests pin that behaviour so a future "looks-like-a-typo" rename to
    // lowercase doesn't silently change the on-the-wire format.

    #[test]
    fn json_round_trip_yes() {
        use crate::serialization::JsonConvertible;
        let original = YesNoAbstainVoteChoice::YES;
        let json = original.to_json().expect("to_json");
        assert_eq!(json, json!("yES"));
        let recovered = YesNoAbstainVoteChoice::from_json(json).expect("from_json");
        assert_eq!(original, recovered);
    }

    #[test]
    fn json_round_trip_no() {
        use crate::serialization::JsonConvertible;
        let original = YesNoAbstainVoteChoice::NO;
        let json = original.to_json().expect("to_json");
        assert_eq!(json, json!("nO"));
        let recovered = YesNoAbstainVoteChoice::from_json(json).expect("from_json");
        assert_eq!(original, recovered);
    }

    #[test]
    fn json_round_trip_abstain() {
        use crate::serialization::JsonConvertible;
        let original = YesNoAbstainVoteChoice::ABSTAIN;
        let json = original.to_json().expect("to_json");
        assert_eq!(json, json!("aBSTAIN"));
        let recovered = YesNoAbstainVoteChoice::from_json(json).expect("from_json");
        assert_eq!(original, recovered);
    }

    #[test]
    fn value_round_trip_yes() {
        use crate::serialization::ValueConvertible;
        let original = YesNoAbstainVoteChoice::YES;
        let value = original.to_object().expect("to_object");
        assert_eq!(value, platform_value!("yES"));
        let recovered = YesNoAbstainVoteChoice::from_object(value).expect("from_object");
        assert_eq!(original, recovered);
    }

    #[test]
    fn value_round_trip_no() {
        use crate::serialization::ValueConvertible;
        let original = YesNoAbstainVoteChoice::NO;
        let value = original.to_object().expect("to_object");
        assert_eq!(value, platform_value!("nO"));
        let recovered = YesNoAbstainVoteChoice::from_object(value).expect("from_object");
        assert_eq!(original, recovered);
    }

    #[test]
    fn value_round_trip_abstain() {
        use crate::serialization::ValueConvertible;
        let original = YesNoAbstainVoteChoice::ABSTAIN;
        let value = original.to_object().expect("to_object");
        assert_eq!(value, platform_value!("aBSTAIN"));
        let recovered = YesNoAbstainVoteChoice::from_object(value).expect("from_object");
        assert_eq!(original, recovered);
    }
}
