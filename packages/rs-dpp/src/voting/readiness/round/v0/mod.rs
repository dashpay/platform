use crate::voting::readiness::payer::ReadinessPayer;
use bincode::{Decode, DecodeUntrusted, Encode};
use platform_value::Identifier;
use std::fmt;

/// The membership view and raw report count a round was last judged against. A round whose
/// mark differs from the block's view or raw count needs reconsideration; one whose mark
/// matches is skipped at the cost of one record read.
#[derive(Debug, PartialEq, Eq, Clone, Copy, Encode, Decode, DecodeUntrusted)]
pub struct ReadinessEvaluation {
    /// The core height whose masternode list was the membership view.
    pub core_height: u32,
    /// The raw distinct report count at the time.
    pub raw_count: u64,
}

/// Where a round stands.
#[derive(Debug, PartialEq, Eq, Clone, Copy, Encode, Decode, DecodeUntrusted)]
pub enum ReadinessRoundStatus {
    /// Collecting reports; never expires on its own.
    Pending,
    /// A fully validated crossing was recorded; the bundle activates at the first block
    /// boundary whose time is at or past the deadline.
    Crossed {
        /// The committed block time of the crossing.
        crossing_ms: u64,
        /// The committed block time at or after which the bundle activates.
        deadline_ms: u64,
    },
}

impl fmt::Display for ReadinessRoundStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ReadinessRoundStatus::Pending => write!(f, "Pending"),
            ReadinessRoundStatus::Crossed {
                crossing_ms,
                deadline_ms,
            } => write!(
                f,
                "Crossed {{ crossing_ms: {}, deadline_ms: {} }}",
                crossing_ms, deadline_ms
            ),
        }
    }
}

/// Version 0 of a compilation readiness round.
#[derive(Debug, PartialEq, Eq, Clone, Encode, Decode, DecodeUntrusted)]
pub struct ReadinessRoundV0 {
    /// The contract whose bundle is being prepared.
    pub contract_id: Identifier,
    /// The round id every report binds; see `ReadinessRound::derive_round_id`.
    pub round_id: [u8; 32],
    /// The canonical bundle digest every report binds.
    pub bundle_digest: [u8; 32],
    /// The contract version the bundle activates.
    pub version: u32,
    /// The preparation profile the reports must be compiled against.
    pub preparation_profile: u16,
    /// The committed block time at which the bundle was accepted.
    pub accepted_at_ms: u64,
    /// The block height at which the bundle was accepted.
    pub accepted_at_height: u64,
    /// Where the round stands.
    pub status: ReadinessRoundStatus,
    /// The membership view and raw count the round was last judged against; `None` until the
    /// first evaluation.
    pub last_evaluated: Option<ReadinessEvaluation>,
    /// The fund could not pay for the last membership page; the round waits for a top-up.
    pub awaiting_funding: bool,
    /// The readiness fund the verification work is paid from.
    pub funding_id: Identifier,
    /// The party the unused fund is refunded to.
    pub payer: ReadinessPayer,
}

impl fmt::Display for ReadinessRoundV0 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "ReadinessRoundV0 {{ contract_id: {}, round_id: {}, bundle_digest: {}, version: {}, preparation_profile: {}, accepted_at_ms: {}, accepted_at_height: {}, status: {}, last_evaluated: {:?}, awaiting_funding: {}, funding_id: {}, payer: {} }}",
            self.contract_id,
            hex::encode(self.round_id),
            hex::encode(self.bundle_digest),
            self.version,
            self.preparation_profile,
            self.accepted_at_ms,
            self.accepted_at_height,
            self.status,
            self.last_evaluated,
            self.awaiting_funding,
            self.funding_id,
            self.payer
        )
    }
}
