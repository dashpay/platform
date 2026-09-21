#[cfg(feature = "json-conversion")]
use crate::serialization::json_safe_fields;
#[cfg(feature = "json-conversion")]
use crate::serialization::JsonConvertible;
use crate::serialization::PlatformSerializable;
#[cfg(feature = "value-conversion")]
use crate::serialization::ValueConvertible;
use crate::util::hash::hash_double;
use crate::ProtocolError;
use bincode::{Decode, DecodeUntrusted, Encode};
use platform_serialization_derive::{
    PlatformDeserializeTrusted, PlatformDeserializeUntrusted, PlatformSerialize,
};
use platform_value::Identifier;
#[cfg(feature = "serde-conversion")]
use serde::{Deserialize, Serialize};
use std::fmt;

/// A masternode vote poll that elects one identity among contenders, keyed by a resource path
/// the opener chooses (the moderation teams key their polls by the moderated contract id).
///
/// Unlike a contested document resource poll it has no Lock choice: masternodes vote towards a
/// contender or abstain. Contenders join during the join phase, votes are cast during the vote
/// phase, and the poll resolves by plurality with the earliest contender winning a tie. With a
/// single contender at the end of the join phase there is no vote phase at all.
#[cfg_attr(feature = "json-conversion", json_safe_fields)]
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
    Default,
    DecodeUntrusted,
)]
#[cfg_attr(
    feature = "serde-conversion",
    derive(Serialize, Deserialize),
    serde(rename_all = "camelCase")
)]
#[cfg_attr(feature = "value-conversion", derive(ValueConvertible))]
#[platform_serialize(unversioned)] //versioned directly, no need to use platform_version
#[platform_serialize(limit = 100000)]
pub struct IdentityContenderVotePoll {
    /// What the poll decides, as the opener describes it: a path of byte strings, such as the
    /// moderated contract's id followed by what the election is for. Two polls with the same
    /// path are the same poll, so an opener that can run several polls on one subject tells
    /// them apart here. Byte strings rather than values, so that a poll rebuilt from JSON hashes
    /// to the same id as the one in state.
    pub resource_path: Vec<Vec<u8>>,
}

impl fmt::Display for IdentityContenderVotePoll {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let resource_path: Vec<String> = self.resource_path.iter().map(hex::encode).collect();
        write!(
            f,
            "IdentityContenderVotePoll {{ resource_path: [{}] }}",
            resource_path.join(", ")
        )
    }
}

impl IdentityContenderVotePoll {
    /// The poll over the given resource path.
    pub fn new(resource_path: Vec<Vec<u8>>) -> Self {
        Self { resource_path }
    }

    /// The double sha256 of the serialized poll.
    pub fn sha256_2_hash(&self) -> Result<[u8; 32], ProtocolError> {
        let encoded = self.serialize_to_bytes()?;
        Ok(hash_double(encoded))
    }

    /// The prefunded specialized balance the votes on this poll are paid from.
    pub fn specialized_balance_id(&self) -> Result<Identifier, ProtocolError> {
        self.unique_id()
    }

    /// The id that keys the poll in state: the double sha256 of the serialized poll.
    pub fn unique_id(&self) -> Result<Identifier, ProtocolError> {
        self.sha256_2_hash().map(Identifier::new)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unique_id_should_commit_to_the_resource_path() {
        let contract_id = Identifier::new([0xc1; 32]);
        let poll = IdentityContenderVotePoll::new(vec![contract_id.to_vec()]);
        let other =
            IdentityContenderVotePoll::new(vec![contract_id.to_vec(), b"challenge".to_vec()]);

        assert_eq!(
            poll.unique_id().expect("unique id"),
            poll.specialized_balance_id().expect("balance id")
        );
        assert_ne!(
            poll.unique_id().expect("unique id"),
            other.unique_id().expect("unique id")
        );
    }

    #[test]
    fn should_round_trip_through_platform_serialization() {
        use crate::serialization::PlatformDeserializableUntrusted;

        let poll = IdentityContenderVotePoll::new(vec![
            vec![0xc1; 32],
            b"election".to_vec(),
            3u64.to_be_bytes().to_vec(),
        ]);
        let bytes = poll.serialize_to_bytes().expect("serialize");
        let recovered =
            IdentityContenderVotePoll::deserialize_from_bytes_untrusted(&bytes).expect("decode");
        assert_eq!(poll, recovered);
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

    fn fixture() -> IdentityContenderVotePoll {
        IdentityContenderVotePoll::new(vec![vec![0xc1; 2], b"el".to_vec()])
    }

    #[test]
    fn json_round_trip_with_full_wire_shape() {
        use crate::serialization::JsonConvertible;
        let original = fixture();
        let json = original.to_json().expect("to_json");
        // Byte strings are arrays of numbers on the wire: exact in both directions
        assert_eq!(
            json,
            json!({
                "resourcePath": [[193, 193], [101, 108]],
            })
        );
        let recovered = IdentityContenderVotePoll::from_json(json).expect("from_json");
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
                "resourcePath": [[193u8, 193u8], [101u8, 108u8]],
            })
        );
        let recovered = IdentityContenderVotePoll::from_object(value).expect("from_object");
        assert_eq!(original, recovered);
    }
}
