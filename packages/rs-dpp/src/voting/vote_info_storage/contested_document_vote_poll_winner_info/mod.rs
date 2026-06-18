#[cfg(feature = "json-conversion")]
use crate::serialization::JsonConvertible;
#[cfg(feature = "value-conversion")]
use crate::serialization::ValueConvertible;
use bincode::{Decode, Encode};
use platform_value::Identifier;
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, PartialEq, Eq, Clone, Copy, Default, Encode, Decode)]
// Custom `Serialize` / `Deserialize` below — same pattern as
// `ResourceVoteChoice`. The `WonByIdentity` variant wraps `Identifier`
// (a tuple struct that serializes as a base58 string, not a map), so
// internal tagging doesn't apply natively. The custom impl emits a flat
// `{"$type": ..., "identity": ...}` shape with a synthesized `identity`
// field name. Bincode `Encode` / `Decode` derives are untouched.
#[cfg_attr(feature = "value-conversion", derive(ValueConvertible))]
pub enum ContestedDocumentVotePollWinnerInfo {
    #[default]
    NoWinner,
    WonByIdentity(Identifier),
    Locked,
}

impl Serialize for ContestedDocumentVotePollWinnerInfo {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeMap;
        match self {
            ContestedDocumentVotePollWinnerInfo::NoWinner => {
                let mut m = serializer.serialize_map(Some(1))?;
                m.serialize_entry("$type", "noWinner")?;
                m.end()
            }
            ContestedDocumentVotePollWinnerInfo::WonByIdentity(id) => {
                let mut m = serializer.serialize_map(Some(2))?;
                m.serialize_entry("$type", "wonByIdentity")?;
                m.serialize_entry("identity", id)?;
                m.end()
            }
            ContestedDocumentVotePollWinnerInfo::Locked => {
                let mut m = serializer.serialize_map(Some(1))?;
                m.serialize_entry("$type", "locked")?;
                m.end()
            }
        }
    }
}

impl<'de> Deserialize<'de> for ContestedDocumentVotePollWinnerInfo {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        use serde::de::{self, MapAccess, Visitor};

        struct V;

        impl<'de> Visitor<'de> for V {
            type Value = ContestedDocumentVotePollWinnerInfo;

            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                f.write_str(
                    "ContestedDocumentVotePollWinnerInfo as a map with `type` discriminator",
                )
            }

            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
                let mut variant: Option<String> = None;
                let mut identity: Option<Identifier> = None;

                while let Some(key) = map.next_key::<String>()? {
                    match key.as_str() {
                        "$type" => {
                            if variant.is_some() {
                                return Err(de::Error::duplicate_field("$type"));
                            }
                            variant = Some(map.next_value()?);
                        }
                        "identity" => {
                            if identity.is_some() {
                                return Err(de::Error::duplicate_field("identity"));
                            }
                            identity = Some(map.next_value()?);
                        }
                        _ => {
                            let _: serde::de::IgnoredAny = map.next_value()?;
                        }
                    }
                }

                let variant = variant.ok_or_else(|| de::Error::missing_field("$type"))?;
                match variant.as_str() {
                    "noWinner" => Ok(ContestedDocumentVotePollWinnerInfo::NoWinner),
                    "wonByIdentity" => {
                        let id = identity.ok_or_else(|| de::Error::missing_field("identity"))?;
                        Ok(ContestedDocumentVotePollWinnerInfo::WonByIdentity(id))
                    }
                    "locked" => Ok(ContestedDocumentVotePollWinnerInfo::Locked),
                    other => Err(de::Error::unknown_variant(
                        other,
                        &["noWinner", "wonByIdentity", "locked"],
                    )),
                }
            }
        }

        deserializer.deserialize_map(V)
    }
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

#[cfg(all(
    test,
    feature = "json-conversion",
    feature = "value-conversion",
    feature = "serde-conversion"
))]
mod json_convertible_tests {
    use super::*;
    use platform_value::platform_value;
    use serde_json::json;

    /// Non-default variant (`WonByIdentity` with a non-zero identifier) so
    /// the wire-shape assertion catches silent variant-flip / identifier-zero
    /// on round-trip — the previous fixture used `Default` (`NoWinner`),
    /// which carries no inner state.
    fn fixture() -> ContestedDocumentVotePollWinnerInfo {
        ContestedDocumentVotePollWinnerInfo::WonByIdentity(Identifier::new([0xab; 32]))
    }

    #[test]
    fn json_round_trip_with_full_wire_shape() {
        use crate::serialization::JsonConvertible;
        let original = fixture();
        let json = original.to_json().expect("to_json");
        // `ContestedDocumentVotePollWinnerInfo` has a custom Serialize/
        // Deserialize that emits a flat shape with a synthesized `identity`
        // field. `Identifier` -> base58 string in JSON.
        assert_eq!(
            json,
            json!({
                "$type": "wonByIdentity",
                "identity": "CZ8YUVdk7znjrUmnb5n7kgySk9yRAsQDYmyCxzfSky9t",
            })
        );
        let recovered = ContestedDocumentVotePollWinnerInfo::from_json(json).expect("from_json");
        assert_eq!(original, recovered);
    }

    #[test]
    fn value_round_trip_with_full_wire_shape() {
        use crate::serialization::ValueConvertible;
        let original = fixture();
        let value = original.to_object().expect("to_object");
        // platform_value preserves typed `Identifier` variants — interpolate
        // through the macro so Serialize emits `Value::Identifier`.
        let id = Identifier::new([0xab; 32]);
        assert_eq!(
            value,
            platform_value!({
                "$type": "wonByIdentity",
                "identity": id,
            })
        );
        let recovered =
            ContestedDocumentVotePollWinnerInfo::from_object(value).expect("from_object");
        assert_eq!(original, recovered);
    }
}
