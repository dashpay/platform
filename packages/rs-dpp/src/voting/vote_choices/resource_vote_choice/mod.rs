#[cfg(feature = "vote-serde-conversion")]
use crate::serialization::ValueConvertible;
#[cfg(feature = "json-conversion")]
use crate::serialization::JsonConvertible;
use crate::voting::vote_choices::resource_vote_choice::ResourceVoteChoice::{
    Abstain, Lock, TowardsIdentity,
};
use crate::ProtocolError;
use bincode::{Decode, Encode};
use platform_value::Identifier;
#[cfg(feature = "vote-serde-conversion")]
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
#[cfg_attr(
    feature = "vote-serde-conversion",
    derive(Serialize, Deserialize, ValueConvertible),
    serde(tag = "type", content = "data", rename_all = "camelCase")
)]
pub enum ResourceVoteChoice {
    TowardsIdentity(Identifier),
    #[default]
    Abstain,
    Lock,
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
#[cfg(all(feature = "json-conversion", feature = "vote-serde-conversion"))]
impl JsonConvertible for ResourceVoteChoice {}

#[cfg(all(test, feature = "json-conversion", feature = "vote-serde-conversion"))]
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
