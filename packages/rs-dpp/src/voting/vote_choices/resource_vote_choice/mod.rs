#[cfg(feature = "json-conversion")]
use crate::serialization::JsonConvertible;
#[cfg(feature = "value-conversion")]
use crate::serialization::ValueConvertible;
use crate::voting::vote_choices::resource_vote_choice::ResourceVoteChoice::{
    Abstain, Lock, TowardsIdentity,
};
use crate::ProtocolError;
use bincode::{Decode, Encode};
use platform_value::Identifier;
#[cfg(feature = "serde-conversion")]
use serde::{Deserialize, Serialize};
use std::fmt;

/// A resource votes is a votes determining what we should do with a contested resource.
/// For example Alice and Bob both want the username "Malaka"
/// Some would vote for Alice to get it by putting in her Identifier.
/// Some would vote for Bob to get it by putting in Bob's Identifier.
/// Let's say someone voted, but is now not quite sure of their votes, they can abstain.
/// Lock is there to signal that the shared resource should be given to no one.
/// In this case Malaka might have a bad connotation in Greek, hence some might votes to Lock
/// the name.
///
#[derive(Debug, Clone, Copy, Encode, Decode, Ord, Eq, PartialOrd, PartialEq, Default)]
// Custom `Serialize` / `Deserialize` below — `derive(Serialize, Deserialize)`
// can't produce the desired flat wire shape because the `TowardsIdentity`
// variant wraps `Identifier` (a tuple struct that serializes as a base58
// string, not a map), so internal tagging doesn't apply. The custom impl
// emits a flat `{"$type": ..., "identity": ...}` shape with a synthesized
// `identity` field name. Bincode `Encode` / `Decode` derives are untouched
// (consensus binary format is unaffected).
#[cfg_attr(feature = "value-conversion", derive(ValueConvertible))]
pub enum ResourceVoteChoice {
    TowardsIdentity(Identifier),
    #[default]
    Abstain,
    Lock,
}

#[cfg(feature = "serde-conversion")]
impl Serialize for ResourceVoteChoice {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeMap;
        match self {
            ResourceVoteChoice::TowardsIdentity(id) => {
                let mut m = serializer.serialize_map(Some(2))?;
                m.serialize_entry("$type", "towardsIdentity")?;
                m.serialize_entry("identity", id)?;
                m.end()
            }
            ResourceVoteChoice::Abstain => {
                let mut m = serializer.serialize_map(Some(1))?;
                m.serialize_entry("$type", "abstain")?;
                m.end()
            }
            ResourceVoteChoice::Lock => {
                let mut m = serializer.serialize_map(Some(1))?;
                m.serialize_entry("$type", "lock")?;
                m.end()
            }
        }
    }
}

#[cfg(feature = "serde-conversion")]
impl<'de> Deserialize<'de> for ResourceVoteChoice {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        use serde::de::{self, MapAccess, Visitor};

        struct V;

        impl<'de> Visitor<'de> for V {
            type Value = ResourceVoteChoice;

            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                f.write_str("ResourceVoteChoice as a map with `type` discriminator")
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
                    "towardsIdentity" => {
                        let id = identity.ok_or_else(|| de::Error::missing_field("identity"))?;
                        Ok(ResourceVoteChoice::TowardsIdentity(id))
                    }
                    "abstain" => Ok(ResourceVoteChoice::Abstain),
                    "lock" => Ok(ResourceVoteChoice::Lock),
                    other => Err(de::Error::unknown_variant(
                        other,
                        &["towardsIdentity", "abstain", "lock"],
                    )),
                }
            }
        }

        deserializer.deserialize_map(V)
    }
}

impl fmt::Display for ResourceVoteChoice {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ResourceVoteChoice::TowardsIdentity(identifier) => {
                write!(f, "TowardsIdentity({})", identifier)
            }
            ResourceVoteChoice::Abstain => write!(f, "Abstain"),
            ResourceVoteChoice::Lock => write!(f, "Lock"),
        }
    }
}

// Manual impl because ResourceVoteChoice is a flat enum (not versioned V0/V1).
#[cfg(feature = "json-conversion")]
impl JsonConvertible for ResourceVoteChoice {}

#[cfg(all(test, feature = "json-conversion"))]
mod tests {
    use super::*;
    use crate::serialization::JsonConvertible;

    #[test]
    fn resource_vote_choice_towards_identity_json_round_trip() {
        let id = Identifier::from([0x42u8; 32]);
        let choice = ResourceVoteChoice::TowardsIdentity(id);

        let json = choice.to_json().expect("to_json should succeed");
        let json_str = serde_json::to_string(&json).unwrap();
        let expected_base58 = id.to_string(platform_value::string_encoding::Encoding::Base58);
        assert!(
            json_str.contains(&expected_base58),
            "JSON should contain base58 identifier {}, got: {}",
            expected_base58,
            json_str
        );

        let restored = ResourceVoteChoice::from_json(json).expect("from_json should succeed");
        assert_eq!(choice, restored);
    }

    #[test]
    fn resource_vote_choice_abstain_json_round_trip() {
        let choice = ResourceVoteChoice::Abstain;
        let json = choice.to_json().expect("to_json should succeed");
        let restored = ResourceVoteChoice::from_json(json).expect("from_json should succeed");
        assert_eq!(choice, restored);
    }

    #[test]
    fn resource_vote_choice_lock_json_round_trip() {
        let choice = ResourceVoteChoice::Lock;
        let json = choice.to_json().expect("to_json should succeed");
        let restored = ResourceVoteChoice::from_json(json).expect("from_json should succeed");
        assert_eq!(choice, restored);
    }
}

impl TryFrom<(i32, Option<Vec<u8>>)> for ResourceVoteChoice {
    type Error = ProtocolError;

    fn try_from(value: (i32, Option<Vec<u8>>)) -> Result<Self, Self::Error> {
        match value.0 {
            0 => Ok(TowardsIdentity(value.1.ok_or(ProtocolError::DecodingError("identifier needed when trying to cast from an i32 to a resource vote choice".to_string()))?.try_into()?)),
            1 => Ok(Abstain),
            2 => Ok(Lock),
            n => Err(ProtocolError::DecodingError(format!("identifier must be 0, 1, or 2, got {}", n)))
        }
    }
}

#[cfg(all(
    test,
    feature = "json-conversion",
    feature = "value-conversion",
    feature = "serde-conversion"
))]
mod json_convertible_tests_resourcevotechoice {
    use super::*;

    #[test]
    fn json_round_trip_resourcevotechoice() {
        use crate::serialization::JsonConvertible;
        let original = ResourceVoteChoice::default();
        let json = original.to_json().expect("to_json");
        let recovered = ResourceVoteChoice::from_json(json).expect("from_json");
        assert_eq!(original, recovered);
    }

    #[test]
    fn value_round_trip_resourcevotechoice() {
        use crate::serialization::ValueConvertible;
        let original = ResourceVoteChoice::default();
        let value = original.to_object().expect("to_object");
        let recovered = ResourceVoteChoice::from_object(value).expect("from_object");
        assert_eq!(original, recovered);
    }
}
