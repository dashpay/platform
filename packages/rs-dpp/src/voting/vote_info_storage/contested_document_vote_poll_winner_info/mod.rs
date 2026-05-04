#[cfg(feature = "json-conversion")]
use crate::serialization::JsonConvertible;
#[cfg(feature = "value-conversion")]
use crate::serialization::ValueConvertible;
use bincode::{Decode, Encode};
use platform_value::Identifier;
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, PartialEq, Eq, Clone, Copy, Default, Encode, Decode, Serialize, Deserialize)]
#[cfg_attr(feature = "value-conversion", derive(ValueConvertible))]
#[serde(tag = "type", content = "data", rename_all = "camelCase")]
pub enum ContestedDocumentVotePollWinnerInfo {
    #[default]
    NoWinner,
    WonByIdentity(Identifier),
    Locked,
}

impl fmt::Display for ContestedDocumentVotePollWinnerInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ContestedDocumentVotePollWinnerInfo::NoWinner => write!(f, "NoWinner"),
            ContestedDocumentVotePollWinnerInfo::WonByIdentity(identifier) => {
                write!(f, "WonByIdentity({})", identifier)
            }
            ContestedDocumentVotePollWinnerInfo::Locked => write!(f, "Locked"),
        }
    }
}

// Manual impl because ContestedDocumentVotePollWinnerInfo is a flat enum
// (not versioned V0/V1).
#[cfg(feature = "json-conversion")]
impl JsonConvertible for ContestedDocumentVotePollWinnerInfo {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_no_winner() {
        let default: ContestedDocumentVotePollWinnerInfo = Default::default();
        assert_eq!(default, ContestedDocumentVotePollWinnerInfo::NoWinner);
    }

    #[test]
    fn display_no_winner() {
        let s = ContestedDocumentVotePollWinnerInfo::NoWinner.to_string();
        assert_eq!(s, "NoWinner");
    }

    #[test]
    fn display_locked() {
        let s = ContestedDocumentVotePollWinnerInfo::Locked.to_string();
        assert_eq!(s, "Locked");
    }

    #[test]
    fn display_won_by_identity() {
        let id = Identifier::new([5u8; 32]);
        let s = ContestedDocumentVotePollWinnerInfo::WonByIdentity(id).to_string();
        assert!(s.starts_with("WonByIdentity("));
        assert!(s.contains(&format!("{}", id)));
    }

    #[test]
    fn equality_and_copy() {
        let a = ContestedDocumentVotePollWinnerInfo::Locked;
        let b = a; // Copy
        assert_eq!(a, b);

        let id1 = Identifier::new([1u8; 32]);
        let id2 = Identifier::new([2u8; 32]);
        assert_ne!(
            ContestedDocumentVotePollWinnerInfo::WonByIdentity(id1),
            ContestedDocumentVotePollWinnerInfo::WonByIdentity(id2)
        );
        assert_ne!(
            ContestedDocumentVotePollWinnerInfo::NoWinner,
            ContestedDocumentVotePollWinnerInfo::Locked
        );
    }

    #[test]
    fn bincode_roundtrip() {
        use bincode::config;
        let cfg = config::standard();
        for v in [
            ContestedDocumentVotePollWinnerInfo::NoWinner,
            ContestedDocumentVotePollWinnerInfo::Locked,
            ContestedDocumentVotePollWinnerInfo::WonByIdentity(Identifier::new([9u8; 32])),
        ] {
            let bytes = bincode::encode_to_vec(v, cfg).expect("encode");
            let (decoded, _): (ContestedDocumentVotePollWinnerInfo, _) =
                bincode::decode_from_slice(&bytes, cfg).expect("decode");
            assert_eq!(v, decoded);
        }
    }

    #[test]
    fn serde_roundtrip_won_by_identity() {
        let id = Identifier::new([3u8; 32]);
        let value = ContestedDocumentVotePollWinnerInfo::WonByIdentity(id);
        let json = serde_json::to_string(&value).expect("serialize");
        let back: ContestedDocumentVotePollWinnerInfo =
            serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back, value);
    }
}

#[cfg(all(test, feature = "json-conversion", feature = "value-conversion", feature = "serde-conversion"))]
mod json_convertible_tests {
    use super::*;

    /// Non-default variant (`WonByIdentity` with a non-zero identifier) so
    /// per-property assertions catch silent variant-flip / identifier-zero
    /// on round-trip — the previous fixture used `Default` (`NoWinner`),
    /// which carries no inner state.
    fn fixture() -> ContestedDocumentVotePollWinnerInfo {
        ContestedDocumentVotePollWinnerInfo::WonByIdentity(Identifier::new([0xab; 32]))
    }

    fn assert_per_property(actual: &ContestedDocumentVotePollWinnerInfo) {
        match actual {
            ContestedDocumentVotePollWinnerInfo::WonByIdentity(id) => {
                assert_eq!(*id, Identifier::new([0xab; 32]), "WonByIdentity.id");
            }
            other => panic!("expected WonByIdentity, got {:?}", other),
        }
    }

    #[test]
    fn json_round_trip_with_per_property_assertions() {
        use crate::serialization::JsonConvertible;
        let original = fixture();
        let json = original.to_json().expect("to_json");
        let recovered = ContestedDocumentVotePollWinnerInfo::from_json(json).expect("from_json");
        assert_eq!(original, recovered);
        assert_per_property(&recovered);
    }

    #[test]
    fn value_round_trip_with_per_property_assertions() {
        use crate::serialization::ValueConvertible;
        let original = fixture();
        let value = original.to_object().expect("to_object");
        let recovered = ContestedDocumentVotePollWinnerInfo::from_object(value).expect("from_object");
        assert_eq!(original, recovered);
        assert_per_property(&recovered);
    }
}
