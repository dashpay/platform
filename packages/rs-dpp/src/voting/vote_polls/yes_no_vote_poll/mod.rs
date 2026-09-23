#[cfg(feature = "json-conversion")]
use crate::serialization::json_safe_fields;
#[cfg(feature = "json-conversion")]
use crate::serialization::JsonConvertible;
use crate::serialization::PlatformSerializable;
#[cfg(feature = "value-conversion")]
use crate::serialization::ValueConvertible;
use crate::util::hash::hash_double;
use crate::voting::vote_polls::VotePoll;
use crate::ProtocolError;
use bincode::{Decode, DecodeUntrusted, Encode};
use platform_serialization_derive::{
    PlatformDeserializeTrusted, PlatformDeserializeUntrusted, PlatformSerialize,
};
use platform_value::{BinaryData, Identifier};
use platform_version::version::PlatformVersion;
#[cfg(feature = "serde-conversion")]
use serde::{Deserialize, Serialize};
use std::fmt;

/// Voting power: a masternode vote weighs 1, an evonode vote weighs 4. The sum over the whole
/// masternode list fits a `u32`.
pub type VotingPower = u32;

/// A poll the masternodes answer with yes, no or abstain.
///
/// The poll is keyed by a resource path the feature that opens it chooses (a contract id and a
/// purpose, for example). Two polls with the same resource path and the same parameters are the
/// same poll, so a feature that may run the same question again includes something that
/// distinguishes the rounds in the path.
///
/// The poll passes when the yes voting power is strictly more than
/// `supermajority_numerator / supermajority_denominator` of the yes plus no voting power, the
/// abstaining power left out of both sides, and the yes plus no voting power is at least
/// `minimum_voting_power`. Otherwise it fails. Both numbers are carried by the poll so every
/// feature sets its own rule. Two thirds with a floor of 400 is numerator 2, denominator 3 and
/// minimum 400.
///
/// The parameters are part of the poll's identity: [`Self::unique_id`] hashes the serialized
/// [`VotePoll`] that carries the poll, so a vote names the exact poll it answers. The prefunded
/// balance that pays for the votes has the same id.
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
    Eq,
    DecodeUntrusted,
)]
#[cfg_attr(
    feature = "serde-conversion",
    derive(Serialize, Deserialize),
    serde(rename_all = "camelCase")
)]
#[cfg_attr(feature = "value-conversion", derive(ValueConvertible))]
#[platform_serialize(unversioned)]
#[platform_serialize(limit = 100000)]
pub struct YesNoVotePoll {
    /// What the poll decides on, as opaque segments chosen by the feature that opened it.
    pub resource_path: Vec<BinaryData>,
    /// The numerator of the share of yes plus no voting power that yes must exceed.
    pub supermajority_numerator: u8,
    /// The denominator of the share of yes plus no voting power that yes must exceed.
    pub supermajority_denominator: u8,
    /// The least yes plus no voting power for the poll to pass.
    pub minimum_voting_power: VotingPower,
}

impl fmt::Display for YesNoVotePoll {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let resource_path: Vec<String> = self
            .resource_path
            .iter()
            .map(|segment| hex::encode(segment.as_slice()))
            .collect();
        write!(
            f,
            "YesNoVotePoll {{ resource_path: [{}], supermajority: {}/{}, minimum_voting_power: {} }}",
            resource_path.join("/"),
            self.supermajority_numerator,
            self.supermajority_denominator,
            self.minimum_voting_power
        )
    }
}

impl YesNoVotePoll {
    /// Refuses parameters no poll can be opened with: an empty or oversized resource path (the
    /// poll rides in every vote, whose wire size is bounded), a zero denominator, or a
    /// numerator that is not below the denominator (yes could never exceed the whole of yes
    /// plus no).
    pub fn validate_parameters(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<(), ProtocolError> {
        if self.resource_path.is_empty() {
            return Err(ProtocolError::VoteError(
                "a yes/no vote poll needs a resource path".to_string(),
            ));
        }
        let max_segments = platform_version
            .system_limits
            .max_yes_no_vote_poll_resource_path_segments;
        if self.resource_path.len() > max_segments as usize {
            return Err(ProtocolError::VoteError(format!(
                "a yes/no vote poll resource path has at most {} segments, got {}",
                max_segments,
                self.resource_path.len()
            )));
        }
        let max_bytes = platform_version
            .system_limits
            .max_yes_no_vote_poll_resource_path_bytes;
        let bytes: usize = self.resource_path.iter().map(|segment| segment.len()).sum();
        if bytes > max_bytes as usize {
            return Err(ProtocolError::VoteError(format!(
                "a yes/no vote poll resource path holds at most {} bytes, got {}",
                max_bytes, bytes
            )));
        }
        if self.supermajority_denominator == 0 {
            return Err(ProtocolError::VoteError(
                "a yes/no vote poll supermajority denominator must not be zero".to_string(),
            ));
        }
        if self.supermajority_numerator >= self.supermajority_denominator {
            return Err(ProtocolError::VoteError(format!(
                "a yes/no vote poll supermajority numerator ({}) must be below its denominator ({})",
                self.supermajority_numerator, self.supermajority_denominator
            )));
        }
        Ok(())
    }

    /// Whether the poll passes with these final tallies. Abstaining power plays no part.
    pub fn passes(&self, yes_voting_power: VotingPower, no_voting_power: VotingPower) -> bool {
        let yes = yes_voting_power as u64;
        let cast = yes + no_voting_power as u64;
        if cast < self.minimum_voting_power as u64 {
            return false;
        }
        yes * self.supermajority_denominator as u64 > cast * self.supermajority_numerator as u64
    }

    /// The double SHA-256 of the serialized `VotePoll` carrying this poll.
    pub fn sha256_2_hash(&self) -> Result<[u8; 32], ProtocolError> {
        let encoded = VotePoll::YesNoVotePoll(self.clone()).serialize_to_bytes()?;
        Ok(hash_double(encoded))
    }

    /// The prefunded balance the votes on this poll are paid from: the poll's unique id.
    pub fn specialized_balance_id(&self) -> Result<Identifier, ProtocolError> {
        self.unique_id()
    }

    /// The id the poll is stored and voted on under.
    pub fn unique_id(&self) -> Result<Identifier, ProtocolError> {
        self.sha256_2_hash().map(Identifier::new)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn poll() -> YesNoVotePoll {
        YesNoVotePoll {
            resource_path: vec![
                BinaryData::new(vec![0xc1; 32]),
                BinaryData::new(b"challenge".to_vec()),
            ],
            supermajority_numerator: 2,
            supermajority_denominator: 3,
            minimum_voting_power: 400,
        }
    }

    #[test]
    fn should_pass_strictly_above_two_thirds_and_fail_at_or_below_it() {
        // The minimum is met by every tally below; only the share decides.
        let poll = YesNoVotePoll {
            minimum_voting_power: 300,
            ..poll()
        };
        // 300 cast: two thirds is 200, so 201 passes and 200 does not.
        assert!(poll.passes(201, 99));
        assert!(!poll.passes(200, 100));
        // 301 cast: two thirds is 200.67, so 201 passes and 200 does not.
        assert!(poll.passes(201, 100));
        assert!(!poll.passes(200, 101));
    }

    #[test]
    fn should_fail_below_the_minimum_voting_power_even_when_unanimous() {
        let poll = poll();
        assert!(!poll.passes(399, 0));
        assert!(poll.passes(400, 0));
    }

    #[test]
    fn should_not_overflow_on_the_largest_tallies() {
        let poll = YesNoVotePoll {
            supermajority_numerator: 254,
            supermajority_denominator: 255,
            ..poll()
        };
        assert!(poll.passes(u32::MAX, 0));
        assert!(!poll.passes(u32::MAX, u32::MAX));
    }

    #[test]
    fn should_refuse_parameters_no_poll_can_run_with() {
        let platform_version = PlatformVersion::latest();
        assert!(poll().validate_parameters(platform_version).is_ok());
        assert!(YesNoVotePoll {
            resource_path: vec![],
            ..poll()
        }
        .validate_parameters(platform_version)
        .is_err());
        assert!(YesNoVotePoll {
            supermajority_denominator: 0,
            ..poll()
        }
        .validate_parameters(platform_version)
        .is_err());
        assert!(YesNoVotePoll {
            supermajority_numerator: 3,
            supermajority_denominator: 3,
            ..poll()
        }
        .validate_parameters(platform_version)
        .is_err());
        // A simple majority is a valid rule.
        assert!(YesNoVotePoll {
            supermajority_numerator: 1,
            supermajority_denominator: 2,
            ..poll()
        }
        .validate_parameters(platform_version)
        .is_ok());
    }

    #[test]
    fn should_bound_the_resource_path() {
        let platform_version = PlatformVersion::latest();
        let max_segments = platform_version
            .system_limits
            .max_yes_no_vote_poll_resource_path_segments as usize;
        let max_bytes = platform_version
            .system_limits
            .max_yes_no_vote_poll_resource_path_bytes as usize;
        let at_most_segments = YesNoVotePoll {
            resource_path: vec![BinaryData::new(vec![1]); max_segments],
            ..poll()
        };
        assert!(at_most_segments
            .validate_parameters(platform_version)
            .is_ok());
        let too_many_segments = YesNoVotePoll {
            resource_path: vec![BinaryData::new(vec![1]); max_segments + 1],
            ..poll()
        };
        assert!(too_many_segments
            .validate_parameters(platform_version)
            .is_err());
        let at_most_bytes = YesNoVotePoll {
            resource_path: vec![BinaryData::new(vec![1; max_bytes])],
            ..poll()
        };
        assert!(at_most_bytes.validate_parameters(platform_version).is_ok());
        let too_many_bytes = YesNoVotePoll {
            resource_path: vec![
                BinaryData::new(vec![1; max_bytes]),
                BinaryData::new(vec![1]),
            ],
            ..poll()
        };
        assert!(too_many_bytes
            .validate_parameters(platform_version)
            .is_err());
    }

    #[test]
    fn should_change_the_id_with_every_parameter() {
        let base = poll().unique_id().expect("id");
        let other_path = YesNoVotePoll {
            resource_path: vec![BinaryData::new(vec![0xc1; 32])],
            ..poll()
        };
        let other_rule = YesNoVotePoll {
            supermajority_numerator: 1,
            supermajority_denominator: 2,
            ..poll()
        };
        let other_minimum = YesNoVotePoll {
            minimum_voting_power: 401,
            ..poll()
        };
        for other in [other_path, other_rule, other_minimum] {
            assert_ne!(other.unique_id().expect("id"), base);
        }
        assert_eq!(poll().specialized_balance_id().expect("balance id"), base);
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

    fn fixture() -> YesNoVotePoll {
        YesNoVotePoll {
            resource_path: vec![
                BinaryData::new(vec![0xc1; 4]),
                BinaryData::new(b"challenge".to_vec()),
            ],
            supermajority_numerator: 2,
            supermajority_denominator: 3,
            minimum_voting_power: 400,
        }
    }

    #[test]
    fn json_round_trip_with_full_wire_shape() {
        use crate::serialization::JsonConvertible;
        let original = fixture();
        let json = original.to_json().expect("to_json");
        // Path segments are `BinaryData`, base64 strings in JSON.
        assert_eq!(
            json,
            json!({
                "resourcePath": ["wcHBwQ==", "Y2hhbGxlbmdl"],
                "supermajorityNumerator": 2,
                "supermajorityDenominator": 3,
                "minimumVotingPower": 400,
            })
        );
        let recovered = YesNoVotePoll::from_json(json).expect("from_json");
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
                "resourcePath": [
                    platform_value::Value::Bytes(vec![0xc1; 4]),
                    platform_value::Value::Bytes(b"challenge".to_vec()),
                ],
                "supermajorityNumerator": 2u8,
                "supermajorityDenominator": 3u8,
                "minimumVotingPower": 400u32,
            })
        );
        let recovered = YesNoVotePoll::from_object(value).expect("from_object");
        assert_eq!(original, recovered);
    }
}
