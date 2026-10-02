mod v0;

use crate::util::hash::hash_double;
use crate::voting::readiness::payer::ReadinessPayer;
use crate::ProtocolError;
use bincode::{Decode, DecodeUntrusted, Encode};
use derive_more::From;
use platform_serialization_derive::{
    PlatformDeserializeTrusted, PlatformDeserializeUntrusted, PlatformSerialize,
};
use platform_value::Identifier;
use platform_version::version::PlatformVersion;
use std::fmt;
pub use v0::{ReadinessEvaluation, ReadinessRoundStatus, ReadinessRoundV0};

/// Domain separator of the readiness fund id derived from a round id. Provisional under the
/// allocation register's prefunded purpose row.
pub const READINESS_FUND_ID_DOMAIN: &[u8] = b"dashvm-readiness-fund-v1";

/// One compilation readiness round: the pending executable bundle of a contract, the reports
/// the evonodes sent for it, the last membership view it was judged against and, once it has
/// crossed the threshold, its activation deadline.
#[derive(
    Debug,
    PartialEq,
    Eq,
    Clone,
    From,
    Encode,
    Decode,
    DecodeUntrusted,
    PlatformSerialize,
    PlatformDeserializeTrusted,
    PlatformDeserializeUntrusted,
)]
#[platform_serialize(unversioned)]
pub enum ReadinessRound {
    /// Version 0.
    V0(ReadinessRoundV0),
}

impl fmt::Display for ReadinessRound {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ReadinessRound::V0(round) => write!(f, "V0({})", round),
        }
    }
}

/// The inputs that identify a round when it opens.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReadinessRoundOpening {
    /// The contract whose bundle is being prepared.
    pub contract_id: Identifier,
    /// The contract version the bundle activates.
    pub version: u32,
    /// The canonical bundle (manifest) digest every report binds.
    pub bundle_digest: [u8; 32],
    /// The preparation profile generation the reports must be compiled against.
    pub preparation_profile: u16,
    /// The committed block time at which the bundle was accepted.
    pub accepted_at_ms: u64,
    /// The block height at which the bundle was accepted.
    pub accepted_at_height: u64,
    /// The party that funds the round and receives the unused remainder.
    pub payer: ReadinessPayer,
}

impl ReadinessRound {
    /// Opens a round in the structure version the platform version selects.
    ///
    /// The round id is `hash_double(network_magic || contract_id || version || bundle_digest
    /// || accepted_at_height)`, so a replacement of the very same bundle in a later block is a
    /// different round, and a report signed for a round on one network is meaningless on
    /// another. The fund id derives from the round id under a fixed domain.
    pub fn new(
        network_magic: u32,
        opening: ReadinessRoundOpening,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError> {
        match platform_version
            .dpp
            .voting_versions
            .readiness_round_default_structure_version
        {
            0 => {
                let round_id = Self::derive_round_id(network_magic, &opening);
                let funding_id = Self::derive_fund_id(round_id);
                Ok(ReadinessRoundV0 {
                    contract_id: opening.contract_id,
                    round_id,
                    bundle_digest: opening.bundle_digest,
                    version: opening.version,
                    preparation_profile: opening.preparation_profile,
                    accepted_at_ms: opening.accepted_at_ms,
                    accepted_at_height: opening.accepted_at_height,
                    status: ReadinessRoundStatus::Pending,
                    last_evaluated: None,
                    awaiting_funding: false,
                    funding_id,
                    payer: opening.payer,
                }
                .into())
            }
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "ReadinessRound::new".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }

    /// The round id of an opening on a network.
    pub fn derive_round_id(network_magic: u32, opening: &ReadinessRoundOpening) -> [u8; 32] {
        let mut payload = Vec::with_capacity(4 + 32 + 4 + 32 + 8);
        payload.extend_from_slice(&network_magic.to_be_bytes());
        payload.extend_from_slice(opening.contract_id.as_slice());
        payload.extend_from_slice(&opening.version.to_be_bytes());
        payload.extend_from_slice(&opening.bundle_digest);
        payload.extend_from_slice(&opening.accepted_at_height.to_be_bytes());
        hash_double(payload)
    }

    /// The readiness fund id of a round.
    pub fn derive_fund_id(round_id: [u8; 32]) -> Identifier {
        let mut payload = Vec::with_capacity(READINESS_FUND_ID_DOMAIN.len() + 32);
        payload.extend_from_slice(READINESS_FUND_ID_DOMAIN);
        payload.extend_from_slice(&round_id);
        Identifier::from(hash_double(payload))
    }

    /// The contract whose bundle is being prepared.
    pub fn contract_id(&self) -> Identifier {
        match self {
            ReadinessRound::V0(round) => round.contract_id,
        }
    }

    /// The round id.
    pub fn round_id(&self) -> [u8; 32] {
        match self {
            ReadinessRound::V0(round) => round.round_id,
        }
    }

    /// The canonical bundle digest every report binds.
    pub fn bundle_digest(&self) -> [u8; 32] {
        match self {
            ReadinessRound::V0(round) => round.bundle_digest,
        }
    }

    /// The contract version the bundle activates.
    pub fn version(&self) -> u32 {
        match self {
            ReadinessRound::V0(round) => round.version,
        }
    }

    /// The preparation profile the reports must be compiled against.
    pub fn preparation_profile(&self) -> u16 {
        match self {
            ReadinessRound::V0(round) => round.preparation_profile,
        }
    }

    /// The committed block time at which the bundle was accepted.
    pub fn accepted_at_ms(&self) -> u64 {
        match self {
            ReadinessRound::V0(round) => round.accepted_at_ms,
        }
    }

    /// The block height at which the bundle was accepted.
    pub fn accepted_at_height(&self) -> u64 {
        match self {
            ReadinessRound::V0(round) => round.accepted_at_height,
        }
    }

    /// The round's status.
    pub fn status(&self) -> ReadinessRoundStatus {
        match self {
            ReadinessRound::V0(round) => round.status,
        }
    }

    /// Whether the round is still collecting reports.
    pub fn is_pending(&self) -> bool {
        matches!(self.status(), ReadinessRoundStatus::Pending)
    }

    /// The activation deadline, once the round has crossed.
    pub fn deadline_ms(&self) -> Option<u64> {
        match self.status() {
            ReadinessRoundStatus::Pending => None,
            ReadinessRoundStatus::Crossed { deadline_ms, .. } => Some(deadline_ms),
        }
    }

    /// The membership view and raw count the round was last judged against.
    pub fn last_evaluated(&self) -> Option<ReadinessEvaluation> {
        match self {
            ReadinessRound::V0(round) => round.last_evaluated,
        }
    }

    /// Whether the fund could not pay for the last membership page.
    pub fn awaiting_funding(&self) -> bool {
        match self {
            ReadinessRound::V0(round) => round.awaiting_funding,
        }
    }

    /// The readiness fund the round's verification work is paid from.
    pub fn funding_id(&self) -> Identifier {
        match self {
            ReadinessRound::V0(round) => round.funding_id,
        }
    }

    /// The party the unused fund is refunded to.
    pub fn payer(&self) -> ReadinessPayer {
        match self {
            ReadinessRound::V0(round) => round.payer,
        }
    }

    /// Records the membership view and raw count the round was judged against, and whether
    /// the fund ran short.
    pub fn set_evaluation(&mut self, evaluation: ReadinessEvaluation, awaiting_funding: bool) {
        match self {
            ReadinessRound::V0(round) => {
                round.last_evaluated = Some(evaluation);
                round.awaiting_funding = awaiting_funding;
            }
        }
    }

    /// Marks the round crossed at `crossing_ms` with an activation deadline computed as
    /// `crossing_ms + clamp(crossing_ms - accepted_at_ms, min_wait_ms, max_wait_ms)`.
    ///
    /// The subtraction saturates (the acceptance is always earlier than the crossing) and the
    /// addition is checked. Returns the deadline. A round crosses once: a crossed round is
    /// refused and keeps its deadline.
    pub fn record_crossing(
        &mut self,
        crossing_ms: u64,
        min_wait_ms: u64,
        max_wait_ms: u64,
    ) -> Result<u64, ProtocolError> {
        if !self.is_pending() {
            return Err(ProtocolError::CorruptedCodeExecution(
                "recording a crossing on a readiness round that already crossed".to_string(),
            ));
        }
        if min_wait_ms > max_wait_ms {
            return Err(ProtocolError::CorruptedCodeExecution(
                "readiness wait bounds are inverted".to_string(),
            ));
        }
        let deadline_ms = match self {
            ReadinessRound::V0(round) => {
                let elapsed = crossing_ms.saturating_sub(round.accepted_at_ms);
                let wait = elapsed.clamp(min_wait_ms, max_wait_ms);
                let deadline_ms = crossing_ms
                    .checked_add(wait)
                    .ok_or(ProtocolError::Overflow("readiness activation deadline"))?;
                round.status = ReadinessRoundStatus::Crossed {
                    crossing_ms,
                    deadline_ms,
                };
                deadline_ms
            }
        };
        Ok(deadline_ms)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::serialization::{
        PlatformDeserializableTrusted, PlatformDeserializableUntrusted, PlatformSerializable,
    };

    fn opening() -> ReadinessRoundOpening {
        ReadinessRoundOpening {
            contract_id: Identifier::from([1u8; 32]),
            version: 2,
            bundle_digest: [3u8; 32],
            preparation_profile: 1,
            accepted_at_ms: 1_000_000,
            accepted_at_height: 50,
            payer: ReadinessPayer::Identity(Identifier::from([9u8; 32])),
        }
    }

    #[test]
    fn should_round_trip_a_pending_round_through_serialization() {
        let platform_version = PlatformVersion::latest();
        let round = ReadinessRound::new(0xBD6B0CBF, opening(), platform_version).expect("round");
        let bytes = round.serialize_to_bytes().expect("serialize");
        let restored = ReadinessRound::deserialize_from_bytes_trusted(&bytes).expect("deserialize");
        assert_eq!(restored, round);
        assert_eq!(
            ReadinessRound::deserialize_from_bytes_untrusted(&bytes)
                .expect("deserialize untrusted"),
            restored
        );
        assert!(restored.is_pending());
        assert_eq!(restored.deadline_ms(), None);
        assert_eq!(restored.last_evaluated(), None);
        assert!(!restored.awaiting_funding());
        assert_eq!(
            restored.funding_id(),
            ReadinessRound::derive_fund_id(restored.round_id())
        );
    }

    #[test]
    fn should_round_trip_a_crossed_round_with_an_evaluation_mark() {
        let platform_version = PlatformVersion::latest();
        let mut round =
            ReadinessRound::new(0xBD6B0CBF, opening(), platform_version).expect("round");
        round.set_evaluation(
            ReadinessEvaluation {
                core_height: 700,
                raw_count: 320,
            },
            true,
        );
        let deadline = round
            .record_crossing(1_600_000, 120_000, 3_600_000)
            .expect("crossing");
        assert_eq!(deadline, 1_600_000 + 600_000);
        let bytes = round.serialize_to_bytes().expect("serialize");
        let restored = ReadinessRound::deserialize_from_bytes_trusted(&bytes).expect("deserialize");
        assert_eq!(restored, round);
        assert_eq!(
            ReadinessRound::deserialize_from_bytes_untrusted(&bytes)
                .expect("deserialize untrusted"),
            restored
        );
        assert!(!restored.is_pending());
        assert_eq!(restored.deadline_ms(), Some(2_200_000));
        assert_eq!(
            restored.last_evaluated(),
            Some(ReadinessEvaluation {
                core_height: 700,
                raw_count: 320
            })
        );
        assert!(restored.awaiting_funding());
    }

    #[test]
    fn should_clamp_the_additional_wait_between_the_bounds() {
        let platform_version = PlatformVersion::latest();
        let cases = [
            (30_000u64, 120_000u64),
            (600_000, 600_000),
            (7_200_000, 3_600_000),
        ];
        for (elapsed, expected_wait) in cases {
            let mut round =
                ReadinessRound::new(0xBD6B0CBF, opening(), platform_version).expect("round");
            let crossing_ms = 1_000_000 + elapsed;
            let deadline = round
                .record_crossing(crossing_ms, 120_000, 3_600_000)
                .expect("crossing");
            assert_eq!(deadline, crossing_ms + expected_wait, "elapsed {elapsed}");
        }
    }

    #[test]
    fn should_refuse_a_second_crossing_and_keep_the_first_deadline() {
        let platform_version = PlatformVersion::latest();
        let mut round =
            ReadinessRound::new(0xBD6B0CBF, opening(), platform_version).expect("round");
        let deadline = round
            .record_crossing(1_600_000, 120_000, 3_600_000)
            .expect("crossing");
        let crossed = round.clone();
        assert!(matches!(
            round.record_crossing(2_000_000, 120_000, 3_600_000),
            Err(ProtocolError::CorruptedCodeExecution(_))
        ));
        assert_eq!(round, crossed);
        assert_eq!(round.deadline_ms(), Some(deadline));
    }

    #[test]
    fn should_saturate_when_the_crossing_time_precedes_acceptance() {
        let platform_version = PlatformVersion::latest();
        let mut round =
            ReadinessRound::new(0xBD6B0CBF, opening(), platform_version).expect("round");
        let deadline = round
            .record_crossing(500, 120_000, 3_600_000)
            .expect("crossing");
        assert_eq!(deadline, 500 + 120_000);
    }

    #[test]
    fn should_report_overflow_on_the_deadline_and_reject_inverted_bounds() {
        let platform_version = PlatformVersion::latest();
        let mut round =
            ReadinessRound::new(0xBD6B0CBF, opening(), platform_version).expect("round");
        assert!(matches!(
            round.record_crossing(u64::MAX, 120_000, 3_600_000),
            Err(ProtocolError::Overflow(_))
        ));
        assert!(matches!(
            round.record_crossing(2_000_000, 3_600_000, 120_000),
            Err(ProtocolError::CorruptedCodeExecution(_))
        ));
        assert!(round.is_pending());
    }

    #[test]
    fn should_derive_a_different_round_id_per_network_height_digest_and_version() {
        let base = opening();
        let id = ReadinessRound::derive_round_id(1, &base);
        assert_ne!(id, ReadinessRound::derive_round_id(2, &base));
        assert_ne!(
            id,
            ReadinessRound::derive_round_id(
                1,
                &ReadinessRoundOpening {
                    accepted_at_height: 51,
                    ..base
                }
            )
        );
        assert_ne!(
            id,
            ReadinessRound::derive_round_id(
                1,
                &ReadinessRoundOpening {
                    bundle_digest: [4u8; 32],
                    ..base
                }
            )
        );
        assert_ne!(
            id,
            ReadinessRound::derive_round_id(1, &ReadinessRoundOpening { version: 3, ..base })
        );
        assert_eq!(id, ReadinessRound::derive_round_id(1, &base));
    }
}
