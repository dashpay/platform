use crate::prelude::TimestampMillis;
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
use platform_version::version::PlatformVersion;
#[cfg(feature = "serde-conversion")]
use serde::{Deserialize, Serialize};
use std::fmt;
use std::ops::RangeInclusive;

/// Voting power: a masternode vote weighs 1, an evonode vote weighs 4. The sum over the whole
/// masternode list fits a `u32`.
pub type VotingPower = u32;

/// Prefixed to the serialized poll before its id is hashed. A contested poll's id hashes its
/// bare encoding, which starts with a 32-byte contract id; matching this 33-byte tag would take
/// a contract id equal to its first 32 bytes, so the two kinds never share an id (and with it a
/// prefunded balance and an end date index entry).
const YES_NO_VOTE_POLL_ID_DOMAIN: &[u8] = b"dash platform yes/no vote poll id";

/// Which way a fraction of the total voting power rounds to a whole voting power.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Encode, Decode, DecodeUntrusted)]
#[cfg_attr(
    feature = "serde-conversion",
    derive(Serialize, Deserialize),
    serde(rename_all = "camelCase")
)]
pub enum VotingPowerRounding {
    /// Toward zero: 1/3 of 10 is 3, 1/3 of 9 is 3.
    Down,
    /// Away from zero: 1/3 of 10 is 4, 1/3 of 9 is 3.
    Up,
    /// Rounded down, plus one: 1/3 of 10 is 4, 1/3 of 9 is 4. Half the total this way is a
    /// strict majority of it.
    DownPlusOne,
    /// Rounded up, minus one, never below zero: 1/3 of 10 is 3, 1/3 of 9 is 2.
    UpMinusOne,
}

/// The least yes plus no voting power a yes/no poll needs to pass.
///
/// Its JSON and Value shape is internally tagged, as every sum type's is:
/// `{ "$type": "absolute", "votingPower": 400 }` or
/// `{ "$type": "fractionOfTotal", "numerator": 1, "denominator": 3, "rounding": "up" }`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Encode, Decode, DecodeUntrusted)]
#[cfg_attr(
    feature = "serde-conversion",
    derive(Serialize, Deserialize),
    serde(tag = "$type", rename_all = "camelCase")
)]
pub enum YesNoMinimumVotingPower {
    /// A fixed voting power, at most `max_yes_no_vote_poll_minimum_voting_power`.
    #[cfg_attr(feature = "serde-conversion", serde(rename_all = "camelCase"))]
    Absolute { voting_power: VotingPower },
    /// A share of the total voting power of the masternode list (masternode 1, evonode 4) in
    /// the block that closes the poll, rounded as asked, at most
    /// `max_yes_no_vote_poll_minimum_voting_power_percent_of_total` percent of it.
    FractionOfTotal {
        numerator: u8,
        denominator: u8,
        rounding: VotingPowerRounding,
    },
}

impl YesNoMinimumVotingPower {
    /// The minimum as a voting power, given the total voting power of the masternode list when
    /// the poll closes, as `YesNoVotePoll::required_voting_power` version 0 resolves it. A
    /// fraction with a zero denominator (refused when the poll opens) needs more than any poll
    /// can gather.
    fn resolve_v0(&self, total_voting_power: VotingPower) -> VotingPower {
        match *self {
            YesNoMinimumVotingPower::Absolute { voting_power } => voting_power,
            YesNoMinimumVotingPower::FractionOfTotal {
                numerator,
                denominator,
                rounding,
            } => {
                if denominator == 0 {
                    return VotingPower::MAX;
                }
                let share = total_voting_power as u64 * numerator as u64;
                let denominator = denominator as u64;
                let resolved = match rounding {
                    VotingPowerRounding::Down => share / denominator,
                    VotingPowerRounding::Up => share.div_ceil(denominator),
                    VotingPowerRounding::DownPlusOne => share / denominator + 1,
                    VotingPowerRounding::UpMinusOne => {
                        share.div_ceil(denominator).saturating_sub(1)
                    }
                };
                VotingPower::try_from(resolved).unwrap_or(VotingPower::MAX)
            }
        }
    }
}

impl fmt::Display for YesNoMinimumVotingPower {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            YesNoMinimumVotingPower::Absolute { voting_power } => write!(f, "{}", voting_power),
            YesNoMinimumVotingPower::FractionOfTotal {
                numerator,
                denominator,
                rounding,
            } => write!(
                f,
                "{}/{} of the total, rounded {}",
                numerator,
                denominator,
                match rounding {
                    VotingPowerRounding::Down => "down",
                    VotingPowerRounding::Up => "up",
                    VotingPowerRounding::DownPlusOne => "down plus one",
                    VotingPowerRounding::UpMinusOne => "up minus one",
                }
            ),
        }
    }
}

/// What a yes/no poll decides: one variant per feature that opens polls.
///
/// A poll's id is derived from its purpose alone ([`YesNoVotePoll::unique_id`]), so a purpose
/// has one poll: a feature that may ask the same question again puts what tells the rounds apart
/// in its variant. A later feature appends its variant, and the code acting on a finished poll
/// (`on_yes_no_vote_poll_finished`) matches every variant, so a new one cannot be left out.
///
/// Append only: the bincode discriminant is the variant position, and the serialized purpose
/// names the poll's id, so its storage paths, end date index entry and prefunded balance.
/// Reordering, removing or reshaping a variant moves every poll opened with it.
///
/// Its JSON and Value shape is internally tagged, as every sum type's is:
/// `{ "$type": "decisionProposal", "documentId": "<base58 id>" }`.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Encode,
    Decode,
    DecodeUntrusted,
    PlatformSerialize,
    PlatformDeserializeTrusted,
    PlatformDeserializeUntrusted,
)]
#[cfg_attr(
    feature = "serde-conversion",
    derive(Serialize, Deserialize),
    serde(tag = "$type", rename_all = "camelCase")
)]
#[platform_serialize(unversioned)]
pub enum YesNoVotePollPurpose {
    /// A decision proposal (protocol version 15): a question anyone puts to the masternodes,
    /// asked by the proposal document with this id. Signaling only: nothing acts on the result,
    /// which stays in the poll's stored info.
    #[cfg_attr(feature = "serde-conversion", serde(rename_all = "camelCase"))]
    DecisionProposal { document_id: Identifier },
}

impl fmt::Display for YesNoVotePollPurpose {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            YesNoVotePollPurpose::DecisionProposal { document_id } => {
                write!(f, "decision proposal {}", document_id)
            }
        }
    }
}

/// A poll the masternodes answer with yes, no or abstain.
///
/// The poll passes when the yes voting power is strictly more than
/// `supermajority_numerator / supermajority_denominator` of the yes plus no voting power, the
/// abstaining power left out of both sides, and the yes plus no voting power is at least
/// `minimum_voting_power`. Otherwise it fails. The minimum is a fixed voting power or a share of
/// the masternode list's total voting power when the poll closes, banned masternodes left out.
/// The whole rule is carried by the poll so every feature sets its own. Two thirds with a floor
/// of 400 is numerator 2, denominator 3 and an absolute minimum of 400.
///
/// Only the purpose is the poll's identity: [`Self::unique_id`] version 0 is the double SHA-256
/// of the domain tag `dash platform yes/no vote poll id` followed by the serialized purpose. A
/// vote names the poll by that id alone; the rule is written to the poll's stored info when it
/// opens and cannot change after that. The prefunded balance that pays for the votes has the
/// same id.
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
// One attribute: the derives read only the first `platform_serialize` attribute.
#[platform_serialize(limit = 100000, unversioned)]
pub struct YesNoVotePoll {
    /// What the poll decides. The poll's id is derived from it alone.
    pub purpose: YesNoVotePollPurpose,
    /// The numerator of the share of yes plus no voting power that yes must exceed.
    pub supermajority_numerator: u8,
    /// The denominator of the share of yes plus no voting power that yes must exceed.
    pub supermajority_denominator: u8,
    /// The least yes plus no voting power for the poll to pass.
    pub minimum_voting_power: YesNoMinimumVotingPower,
}

impl fmt::Display for YesNoVotePoll {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "YesNoVotePoll {{ purpose: {}, supermajority: {}/{}, minimum_voting_power: {} }}",
            self.purpose,
            self.supermajority_numerator,
            self.supermajority_denominator,
            self.minimum_voting_power
        )
    }
}

impl YesNoVotePoll {
    /// Refuses parameters no poll can be opened with, by the rules the platform version selects
    /// (`dpp.voting_versions.yes_no_vote_poll_parameters`).
    ///
    /// # Parameters
    /// - `platform_version`: the platform version, whose system limits bound the parameters.
    ///
    /// # Returns
    /// `Ok(())` for parameters a poll can run with, or the error refusing them.
    pub fn validate_parameters(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<(), ProtocolError> {
        match platform_version
            .dpp
            .voting_versions
            .yes_no_vote_poll_parameters
        {
            Some(0) => self.validate_parameters_v0(platform_version),
            Some(version) => Err(ProtocolError::UnknownVersionMismatch {
                method: "YesNoVotePoll::validate_parameters".to_string(),
                known_versions: vec![0],
                received: version,
            }),
            None => Err(ProtocolError::UnknownVersionError(
                "YesNoVotePoll::validate_parameters is not active before protocol version 15"
                    .to_string(),
            )),
        }
    }

    /// Version 0 refuses a zero denominator, a zero numerator (any single yes would pass
    /// against every no), a numerator that is not below the denominator (yes could never exceed
    /// the whole of yes plus no), a threshold below `min_yes_no_vote_poll_supermajority_percent`
    /// (a poll passing with fewer yes than no), and a minimum above its cap (a voting power, or
    /// a share of the total with a zero denominator or above the allowed percentage). The
    /// purpose needs no check: every variant has a fixed size.
    fn validate_parameters_v0(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<(), ProtocolError> {
        if self.supermajority_denominator == 0 {
            return Err(ProtocolError::VoteError(
                "a yes/no vote poll supermajority denominator must not be zero".to_string(),
            ));
        }
        if self.supermajority_numerator == 0 {
            return Err(ProtocolError::VoteError(
                "a yes/no vote poll supermajority numerator must not be zero".to_string(),
            ));
        }
        if self.supermajority_numerator >= self.supermajority_denominator {
            return Err(ProtocolError::VoteError(format!(
                "a yes/no vote poll supermajority numerator ({}) must be below its denominator ({})",
                self.supermajority_numerator, self.supermajority_denominator
            )));
        }
        let min_percent = platform_version
            .system_limits
            .min_yes_no_vote_poll_supermajority_percent;
        if (self.supermajority_numerator as u32) * 100
            < (self.supermajority_denominator as u32) * (min_percent as u32)
        {
            return Err(ProtocolError::VoteError(format!(
                "a yes/no vote poll supermajority is at least {}% of yes plus no, got {}/{}",
                min_percent, self.supermajority_numerator, self.supermajority_denominator
            )));
        }
        match self.minimum_voting_power {
            YesNoMinimumVotingPower::Absolute { voting_power } => {
                let max_voting_power = platform_version
                    .system_limits
                    .max_yes_no_vote_poll_minimum_voting_power;
                if voting_power > max_voting_power {
                    return Err(ProtocolError::VoteError(format!(
                        "a yes/no vote poll minimum voting power is at most {}, got {}",
                        max_voting_power, voting_power
                    )));
                }
            }
            YesNoMinimumVotingPower::FractionOfTotal {
                numerator,
                denominator,
                ..
            } => {
                if denominator == 0 {
                    return Err(ProtocolError::VoteError(
                        "a yes/no vote poll minimum share denominator must not be zero".to_string(),
                    ));
                }
                let max_percent = platform_version
                    .system_limits
                    .max_yes_no_vote_poll_minimum_voting_power_percent_of_total;
                if numerator as u32 * 100 > denominator as u32 * max_percent as u32 {
                    return Err(ProtocolError::VoteError(format!(
                        "a yes/no vote poll minimum share is at most {}% of the total voting power, got {}/{}",
                        max_percent, numerator, denominator
                    )));
                }
            }
        }
        Ok(())
    }

    /// The end dates a yes/no poll opened in a block at `opening_time_ms` may have, by the rule
    /// the platform version selects (`dpp.voting_versions.yes_no_vote_poll_parameters`).
    ///
    /// # Parameters
    /// - `opening_time_ms`: the time of the block that opens the poll.
    /// - `platform_version`: the platform version, whose system limits bound the window.
    ///
    /// # Returns
    /// The earliest and the latest end date, both allowed.
    pub fn end_date_window(
        opening_time_ms: TimestampMillis,
        platform_version: &PlatformVersion,
    ) -> Result<RangeInclusive<TimestampMillis>, ProtocolError> {
        match platform_version
            .dpp
            .voting_versions
            .yes_no_vote_poll_parameters
        {
            Some(0) => Ok(Self::end_date_window_v0(opening_time_ms, platform_version)),
            Some(version) => Err(ProtocolError::UnknownVersionMismatch {
                method: "YesNoVotePoll::end_date_window".to_string(),
                known_versions: vec![0],
                received: version,
            }),
            None => Err(ProtocolError::UnknownVersionError(
                "YesNoVotePoll::end_date_window is not active before protocol version 15"
                    .to_string(),
            )),
        }
    }

    /// Version 0: from `min_yes_no_vote_poll_duration_ms` (and at least 1 ms) after the opening
    /// block to `max_yes_no_vote_poll_duration_ms` after it. A poll that ends in the block that
    /// opens it, or earlier, would close at the end of that block with no vote cast; one that
    /// ends too soon would close before masternode operators see it; one that runs too long
    /// would hold its fund and trees for as long. The latest end date is also at most
    /// `i64::MAX`, past which the end date index (which flips its top bit, `encode_u64`) would
    /// sort it before every block time.
    fn end_date_window_v0(
        opening_time_ms: TimestampMillis,
        platform_version: &PlatformVersion,
    ) -> RangeInclusive<TimestampMillis> {
        let limits = &platform_version.system_limits;
        let earliest =
            opening_time_ms.saturating_add(limits.min_yes_no_vote_poll_duration_ms.max(1));
        let latest = opening_time_ms
            .saturating_add(limits.max_yes_no_vote_poll_duration_ms)
            .min(i64::MAX as TimestampMillis);
        earliest..=latest
    }

    /// The least yes plus no voting power the poll needs, given the total voting power of the
    /// masternode list in the block that closes it, by the rule the platform version selects
    /// (`dpp.voting_versions.yes_no_vote_poll_decision`).
    ///
    /// # Parameters
    /// - `total_voting_power`: the masternode list's total voting power in the closing block.
    /// - `platform_version`: the platform version.
    ///
    /// # Returns
    /// The required voting power.
    pub fn required_voting_power(
        &self,
        total_voting_power: VotingPower,
        platform_version: &PlatformVersion,
    ) -> Result<VotingPower, ProtocolError> {
        match platform_version
            .dpp
            .voting_versions
            .yes_no_vote_poll_decision
        {
            Some(0) => Ok(self.required_voting_power_v0(total_voting_power)),
            Some(version) => Err(ProtocolError::UnknownVersionMismatch {
                method: "YesNoVotePoll::required_voting_power".to_string(),
                known_versions: vec![0],
                received: version,
            }),
            None => Err(ProtocolError::UnknownVersionError(
                "YesNoVotePoll::required_voting_power is not active before protocol version 15"
                    .to_string(),
            )),
        }
    }

    /// Whether the poll passes with these final tallies, given the total voting power of the
    /// masternode list in the block that closes it, by the rule the platform version selects
    /// (`dpp.voting_versions.yes_no_vote_poll_decision`). Abstaining power plays no part.
    ///
    /// # Parameters
    /// - `yes_voting_power`: the yes voting power cast.
    /// - `no_voting_power`: the no voting power cast.
    /// - `total_voting_power`: the masternode list's total voting power in the closing block.
    /// - `platform_version`: the platform version.
    ///
    /// # Returns
    /// Whether the poll passes.
    pub fn passes(
        &self,
        yes_voting_power: VotingPower,
        no_voting_power: VotingPower,
        total_voting_power: VotingPower,
        platform_version: &PlatformVersion,
    ) -> Result<bool, ProtocolError> {
        match platform_version
            .dpp
            .voting_versions
            .yes_no_vote_poll_decision
        {
            Some(0) => Ok(self.passes_v0(yes_voting_power, no_voting_power, total_voting_power)),
            Some(version) => Err(ProtocolError::UnknownVersionMismatch {
                method: "YesNoVotePoll::passes".to_string(),
                known_versions: vec![0],
                received: version,
            }),
            None => Err(ProtocolError::UnknownVersionError(
                "YesNoVotePoll::passes is not active before protocol version 15".to_string(),
            )),
        }
    }

    /// Version 0: the minimum resolved against the total, rounded as the poll asks.
    fn required_voting_power_v0(&self, total_voting_power: VotingPower) -> VotingPower {
        self.minimum_voting_power.resolve_v0(total_voting_power)
    }

    /// Version 0: yes plus no must reach the required voting power, and yes must be strictly
    /// more than the supermajority share of yes plus no.
    fn passes_v0(
        &self,
        yes_voting_power: VotingPower,
        no_voting_power: VotingPower,
        total_voting_power: VotingPower,
    ) -> bool {
        let yes = yes_voting_power as u64;
        let cast = yes + no_voting_power as u64;
        if cast < self.required_voting_power_v0(total_voting_power) as u64 {
            return false;
        }
        yes * self.supermajority_denominator as u64 > cast * self.supermajority_numerator as u64
    }

    /// The id the poll is stored and voted on under, which also keys its end date index entry
    /// and names its prefunded balance, derived as the platform version selects
    /// (`dpp.voting_versions.yes_no_vote_poll_id`).
    ///
    /// # Parameters
    /// - `platform_version`: the platform version.
    ///
    /// # Returns
    /// The poll's id.
    pub fn unique_id(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<Identifier, ProtocolError> {
        match platform_version.dpp.voting_versions.yes_no_vote_poll_id {
            Some(0) => self.unique_id_v0(),
            Some(version) => Err(ProtocolError::UnknownVersionMismatch {
                method: "YesNoVotePoll::unique_id".to_string(),
                known_versions: vec![0],
                received: version,
            }),
            None => Err(ProtocolError::UnknownVersionError(
                "YesNoVotePoll::unique_id is not active before protocol version 15".to_string(),
            )),
        }
    }

    /// The prefunded balance the votes on this poll are paid from: the poll's unique id.
    ///
    /// # Parameters
    /// - `platform_version`: the platform version, which selects how the id is derived.
    ///
    /// # Returns
    /// The balance's id.
    pub fn specialized_balance_id(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<Identifier, ProtocolError> {
        self.unique_id(platform_version)
    }

    /// Version 0: the double SHA-256 of the yes/no poll id domain tag followed by the serialized
    /// purpose. The rule is left out, so a vote naming the id answers whatever rule the poll
    /// opened with, and a purpose opens one poll whatever rule it is asked with.
    fn unique_id_v0(&self) -> Result<Identifier, ProtocolError> {
        let mut preimage = YES_NO_VOTE_POLL_ID_DOMAIN.to_vec();
        preimage.extend(self.purpose.serialize_to_bytes()?);
        Ok(Identifier::new(hash_double(preimage)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn poll() -> YesNoVotePoll {
        YesNoVotePoll {
            purpose: YesNoVotePollPurpose::DecisionProposal {
                document_id: Identifier::new([0xc1; 32]),
            },
            supermajority_numerator: 2,
            supermajority_denominator: 3,
            minimum_voting_power: YesNoMinimumVotingPower::Absolute { voting_power: 400 },
        }
    }

    #[test]
    fn should_pass_strictly_above_two_thirds_and_fail_at_or_below_it() {
        // The minimum is met by every tally below; only the share decides.
        let poll = YesNoVotePoll {
            minimum_voting_power: YesNoMinimumVotingPower::Absolute { voting_power: 300 },
            ..poll()
        };
        // 300 cast: two thirds is 200, so 201 passes and 200 does not.
        assert!(poll.passes_v0(201, 99, 0));
        assert!(!poll.passes_v0(200, 100, 0));
        // 301 cast: two thirds is 200.67, so 201 passes and 200 does not.
        assert!(poll.passes_v0(201, 100, 0));
        assert!(!poll.passes_v0(200, 101, 0));
    }

    #[test]
    fn should_fail_below_the_minimum_voting_power_even_when_unanimous() {
        let poll = poll();
        assert!(!poll.passes_v0(399, 0, 0));
        assert!(poll.passes_v0(400, 0, 0));
    }

    #[test]
    fn should_resolve_a_share_of_the_total_rounded_as_asked() {
        let third = |rounding| YesNoMinimumVotingPower::FractionOfTotal {
            numerator: 1,
            denominator: 3,
            rounding,
        };
        assert_eq!(third(VotingPowerRounding::Down).resolve_v0(10), 3);
        assert_eq!(third(VotingPowerRounding::Up).resolve_v0(10), 4);
        // An exact share is the same either way.
        assert_eq!(third(VotingPowerRounding::Down).resolve_v0(9), 3);
        assert_eq!(third(VotingPowerRounding::Up).resolve_v0(9), 3);
        assert_eq!(third(VotingPowerRounding::Up).resolve_v0(0), 0);
        // Rounded down plus one, and rounded up minus one (never below zero).
        assert_eq!(third(VotingPowerRounding::DownPlusOne).resolve_v0(10), 4);
        assert_eq!(third(VotingPowerRounding::DownPlusOne).resolve_v0(9), 4);
        assert_eq!(third(VotingPowerRounding::DownPlusOne).resolve_v0(0), 1);
        assert_eq!(third(VotingPowerRounding::UpMinusOne).resolve_v0(10), 3);
        assert_eq!(third(VotingPowerRounding::UpMinusOne).resolve_v0(9), 2);
        assert_eq!(third(VotingPowerRounding::UpMinusOne).resolve_v0(0), 0);
        // Half rounded down plus one is a strict majority.
        let majority = YesNoMinimumVotingPower::FractionOfTotal {
            numerator: 1,
            denominator: 2,
            rounding: VotingPowerRounding::DownPlusOne,
        };
        assert_eq!(majority.resolve_v0(10), 6);
        assert_eq!(majority.resolve_v0(9), 5);
        assert_eq!(majority.resolve_v0(u32::MAX), u32::MAX / 2 + 1);
        assert_eq!(
            YesNoMinimumVotingPower::Absolute { voting_power: 400 }.resolve_v0(10),
            400
        );
        // No overflow on the largest total.
        let half = YesNoMinimumVotingPower::FractionOfTotal {
            numerator: 1,
            denominator: 2,
            rounding: VotingPowerRounding::Up,
        };
        assert_eq!(half.resolve_v0(u32::MAX), u32::MAX / 2 + 1);
    }

    #[test]
    fn should_judge_a_share_minimum_against_the_total_at_close() {
        // Two thirds with at least 1/10 of the total cast, rounded up.
        let poll = YesNoVotePoll {
            minimum_voting_power: YesNoMinimumVotingPower::FractionOfTotal {
                numerator: 1,
                denominator: 10,
                rounding: VotingPowerRounding::Up,
            },
            ..poll()
        };
        // A total of 3,801 needs 381 cast.
        assert_eq!(poll.required_voting_power_v0(3_801), 381);
        assert!(!poll.passes_v0(380, 0, 3_801));
        assert!(poll.passes_v0(381, 0, 3_801));
        // The same tally fails once the list has grown.
        assert!(!poll.passes_v0(381, 0, 3_811));
    }

    #[test]
    fn should_bound_the_minimum() {
        let platform_version = PlatformVersion::latest();
        let max_voting_power = platform_version
            .system_limits
            .max_yes_no_vote_poll_minimum_voting_power;
        let with_minimum = |minimum_voting_power| YesNoVotePoll {
            minimum_voting_power,
            ..poll()
        };
        let share = |numerator, denominator| YesNoMinimumVotingPower::FractionOfTotal {
            numerator,
            denominator,
            rounding: VotingPowerRounding::Up,
        };
        for accepted in [
            YesNoMinimumVotingPower::Absolute { voting_power: 0 },
            YesNoMinimumVotingPower::Absolute {
                voting_power: max_voting_power,
            },
            share(0, 1),
            share(1, 3),
            share(1, 2),
            share(50, 100),
        ] {
            assert!(
                with_minimum(accepted)
                    .validate_parameters(platform_version)
                    .is_ok(),
                "{accepted} should be accepted"
            );
        }
        for refused in [
            YesNoMinimumVotingPower::Absolute {
                voting_power: max_voting_power + 1,
            },
            share(1, 0),
            share(51, 100),
            share(2, 3),
        ] {
            assert!(
                with_minimum(refused)
                    .validate_parameters(platform_version)
                    .is_err(),
                "{refused} should be refused"
            );
        }
    }

    #[test]
    fn should_not_overflow_on_the_largest_tallies() {
        let poll = YesNoVotePoll {
            supermajority_numerator: 254,
            supermajority_denominator: 255,
            ..poll()
        };
        assert!(poll.passes_v0(u32::MAX, 0, 0));
        assert!(!poll.passes_v0(u32::MAX, u32::MAX, 0));
    }

    #[test]
    fn should_refuse_parameters_no_poll_can_run_with() {
        let platform_version = PlatformVersion::latest();
        assert!(poll().validate_parameters(platform_version).is_ok());
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
        // A zero numerator would let one yes pass against any amount of no.
        assert!(YesNoVotePoll {
            supermajority_numerator: 0,
            ..poll()
        }
        .validate_parameters(platform_version)
        .is_err());
        // A simple majority is a valid rule, the lowest one.
        assert!(YesNoVotePoll {
            supermajority_numerator: 1,
            supermajority_denominator: 2,
            ..poll()
        }
        .validate_parameters(platform_version)
        .is_ok());
        assert!(YesNoVotePoll {
            supermajority_numerator: 128,
            supermajority_denominator: 255,
            ..poll()
        }
        .validate_parameters(platform_version)
        .is_ok());
        // Below half, a poll could pass with fewer yes than no.
        assert!(YesNoVotePoll {
            supermajority_numerator: 127,
            supermajority_denominator: 255,
            ..poll()
        }
        .validate_parameters(platform_version)
        .is_err());
        assert!(YesNoVotePoll {
            supermajority_numerator: 1,
            supermajority_denominator: 3,
            ..poll()
        }
        .validate_parameters(platform_version)
        .is_err());
    }

    #[test]
    fn should_allow_end_dates_from_one_to_four_weeks_after_the_opening_block() {
        let platform_version = PlatformVersion::latest();
        const WEEK: u64 = 604_800_000;
        let opening = 1_000;
        let window =
            YesNoVotePoll::end_date_window(opening, platform_version).expect("active at 15");
        assert_eq!(window, (opening + WEEK)..=(opening + 4 * WEEK));
        assert!(!window.contains(&(opening + WEEK - 1)));
        assert!(!window.contains(&(opening + 4 * WEEK + 1)));
        // Near the end of time the latest end date stops at what the end date index can sort.
        let late = YesNoVotePoll::end_date_window(i64::MAX as u64 - WEEK, platform_version)
            .expect("active at 15");
        assert_eq!(late, (i64::MAX as u64)..=(i64::MAX as u64));
        let platform_version = PlatformVersion::get(14).expect("expected protocol version 14");
        assert!(matches!(
            YesNoVotePoll::end_date_window(opening, platform_version),
            Err(ProtocolError::UnknownVersionError(_))
        ));
    }

    #[test]
    fn should_derive_the_id_from_the_purpose_alone() {
        let platform_version = PlatformVersion::latest();
        let base = poll().unique_id(platform_version).expect("id");
        let other_purpose = YesNoVotePoll {
            purpose: YesNoVotePollPurpose::DecisionProposal {
                document_id: Identifier::new([0xc2; 32]),
            },
            ..poll()
        };
        assert_ne!(other_purpose.unique_id(platform_version).expect("id"), base);
        // The rule is not part of the id: the same purpose asked with another rule names the
        // same poll, which then cannot be opened a second time.
        let other_rule = YesNoVotePoll {
            supermajority_numerator: 1,
            supermajority_denominator: 2,
            ..poll()
        };
        let other_minimum = YesNoVotePoll {
            minimum_voting_power: YesNoMinimumVotingPower::Absolute { voting_power: 401 },
            ..poll()
        };
        for same in [other_rule, other_minimum] {
            assert_eq!(same.unique_id(platform_version).expect("id"), base);
        }
        assert_eq!(
            poll()
                .specialized_balance_id(platform_version)
                .expect("balance id"),
            base
        );
    }

    #[test]
    fn should_hash_the_id_under_the_yes_no_domain_tag() {
        let encoded = poll()
            .purpose
            .serialize_to_bytes()
            .expect("encoded purpose");
        // A tag byte for the variant, then the 32-byte document id. `DecisionProposal` is
        // variant 0: reordering the purposes would move every decision proposal's poll.
        assert_eq!(encoded, [[0u8].as_slice(), &[0xc1; 32]].concat());
        let untagged = Identifier::new(hash_double(&encoded));
        let tagged = Identifier::new(hash_double(
            [YES_NO_VOTE_POLL_ID_DOMAIN, encoded.as_slice()].concat(),
        ));
        let id = poll().unique_id(PlatformVersion::latest()).expect("id");
        assert_eq!(id, tagged);
        assert_ne!(id, untagged);
    }

    #[test]
    fn should_report_the_poll_rules_not_active_before_protocol_version_15() {
        let platform_version = PlatformVersion::get(14).expect("expected protocol version 14");
        let not_active = |result: Result<(), ProtocolError>, method: &str| {
            assert!(
                matches!(result, Err(ProtocolError::UnknownVersionError(_))),
                "expected {method} to be not active at protocol version 14, got {result:?}"
            );
        };
        not_active(
            poll().validate_parameters(platform_version),
            "validate_parameters",
        );
        not_active(
            poll()
                .required_voting_power(100, platform_version)
                .map(|_| ()),
            "required_voting_power",
        );
        not_active(
            poll().passes(10, 0, 100, platform_version).map(|_| ()),
            "passes",
        );
        not_active(poll().unique_id(platform_version).map(|_| ()), "unique_id");
        not_active(
            poll().specialized_balance_id(platform_version).map(|_| ()),
            "specialized_balance_id",
        );
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
            purpose: YesNoVotePollPurpose::DecisionProposal {
                document_id: Identifier::new([0xc1; 32]),
            },
            supermajority_numerator: 2,
            supermajority_denominator: 3,
            minimum_voting_power: YesNoMinimumVotingPower::Absolute { voting_power: 400 },
        }
    }

    #[test]
    fn json_round_trip_with_full_wire_shape() {
        use crate::serialization::JsonConvertible;
        let original = fixture();
        let json = original.to_json().expect("to_json");
        // The purpose is internally tagged; its document id is a base58 string in JSON.
        assert_eq!(
            json,
            json!({
                "purpose": {
                    "$type": "decisionProposal",
                    "documentId": Identifier::new([0xc1; 32])
                        .to_string(platform_value::string_encoding::Encoding::Base58),
                },
                "supermajorityNumerator": 2,
                "supermajorityDenominator": 3,
                "minimumVotingPower": { "$type": "absolute", "votingPower": 400 },
            })
        );
        let recovered = YesNoVotePoll::from_json(json).expect("from_json");
        assert_eq!(original, recovered);
    }

    #[test]
    fn should_round_trip_a_share_minimum_through_json() {
        use crate::serialization::JsonConvertible;
        let original = YesNoVotePoll {
            minimum_voting_power: YesNoMinimumVotingPower::FractionOfTotal {
                numerator: 1,
                denominator: 3,
                rounding: VotingPowerRounding::Up,
            },
            ..fixture()
        };
        let json = original.to_json().expect("to_json");
        assert_eq!(
            json["minimumVotingPower"],
            json!({ "$type": "fractionOfTotal", "numerator": 1, "denominator": 3, "rounding": "up" })
        );
        let recovered = YesNoVotePoll::from_json(json).expect("from_json");
        assert_eq!(original, recovered);
    }

    #[test]
    fn should_name_every_rounding_in_camel_case() {
        use crate::serialization::JsonConvertible;
        for (rounding, name) in [
            (VotingPowerRounding::Down, "down"),
            (VotingPowerRounding::Up, "up"),
            (VotingPowerRounding::DownPlusOne, "downPlusOne"),
            (VotingPowerRounding::UpMinusOne, "upMinusOne"),
        ] {
            let original = YesNoVotePoll {
                minimum_voting_power: YesNoMinimumVotingPower::FractionOfTotal {
                    numerator: 1,
                    denominator: 2,
                    rounding,
                },
                ..fixture()
            };
            let json = original.to_json().expect("to_json");
            assert_eq!(json["minimumVotingPower"]["rounding"], json!(name));
            assert_eq!(YesNoVotePoll::from_json(json).expect("from_json"), original);
        }
    }

    #[test]
    fn value_round_trip_with_full_wire_shape() {
        use crate::serialization::ValueConvertible;
        let original = fixture();
        let value = original.to_object().expect("to_object");
        assert_eq!(
            value,
            platform_value!({
                "purpose": {
                    "$type": "decisionProposal",
                    "documentId": Identifier::new([0xc1; 32]),
                },
                "supermajorityNumerator": 2u8,
                "supermajorityDenominator": 3u8,
                "minimumVotingPower": { "$type": "absolute", "votingPower": 400u32 },
            })
        );
        let recovered = YesNoVotePoll::from_object(value).expect("from_object");
        assert_eq!(original, recovered);
    }
}
