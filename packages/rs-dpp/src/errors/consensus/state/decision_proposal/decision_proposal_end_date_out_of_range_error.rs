use crate::consensus::state::state_error::StateError;
use crate::consensus::ConsensusError;
use crate::errors::ProtocolError;
use crate::prelude::TimestampMillis;
use bincode::{Decode, DecodeUntrusted, Encode};
use platform_serialization_derive::{
    PlatformDeserializeTrusted, PlatformDeserializeUntrusted, PlatformSerialize,
};
use platform_value::Identifier;
use thiserror::Error;

/// A decision proposal (protocol version 15) whose end date is outside the window the block
/// creating it allows (41300): from `min_yes_no_vote_poll_duration_ms` to
/// `max_yes_no_vote_poll_duration_ms` after the block's time, both ends allowed.
#[derive(
    Error,
    Debug,
    Clone,
    PartialEq,
    Eq,
    Encode,
    Decode,
    PlatformSerialize,
    PlatformDeserializeTrusted,
    PlatformDeserializeUntrusted,
    DecodeUntrusted,
)]
#[error(
    "decision proposal {document_id} ends at {end_date}, it must end from {earliest_end_date} to {latest_end_date}"
)]
#[platform_serialize(unversioned)]
pub struct DecisionProposalEndDateOutOfRangeError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    document_id: Identifier,
    end_date: TimestampMillis,
    earliest_end_date: TimestampMillis,
    latest_end_date: TimestampMillis,
}

impl DecisionProposalEndDateOutOfRangeError {
    pub fn new(
        document_id: Identifier,
        end_date: TimestampMillis,
        earliest_end_date: TimestampMillis,
        latest_end_date: TimestampMillis,
    ) -> Self {
        Self {
            document_id,
            end_date,
            earliest_end_date,
            latest_end_date,
        }
    }

    pub fn document_id(&self) -> Identifier {
        self.document_id
    }

    pub fn end_date(&self) -> TimestampMillis {
        self.end_date
    }

    pub fn earliest_end_date(&self) -> TimestampMillis {
        self.earliest_end_date
    }

    pub fn latest_end_date(&self) -> TimestampMillis {
        self.latest_end_date
    }
}

impl From<DecisionProposalEndDateOutOfRangeError> for ConsensusError {
    fn from(err: DecisionProposalEndDateOutOfRangeError) -> Self {
        Self::StateError(StateError::DecisionProposalEndDateOutOfRangeError(err))
    }
}
