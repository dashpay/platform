use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::errors::ProtocolError;
use bincode::{Decode, DecodeUntrusted, Encode};
use platform_serialization_derive::{
    PlatformDeserializeTrusted, PlatformDeserializeUntrusted, PlatformSerialize,
};
use platform_version::version::ProtocolVersion;
use thiserror::Error;

/// A transition asks for a random value (a random token transfer or mint, or a create of a
/// document whose type declares a `sys.random` property) in the block that switches to
/// `protocol_version`, the first whose blocks draw random values (protocol version 15).
/// Tenderdash applies the consensus parameters a block returns from the next height on, so that
/// block still runs under the consensus version of the protocol version before it, which sends
/// no random beacon: nothing would draw the value. The transition is refused without being
/// charged, and is accepted from the next block on.
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
    "{subject} asks for a random value, which the block switching to protocol version \
     {protocol_version} cannot draw: Tenderdash sends that block no random beacon. It is \
     accepted from the next block on"
)]
#[platform_serialize(unversioned)]
pub struct RandomBeaconUnavailableError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    subject: String,
    protocol_version: ProtocolVersion,
}

impl RandomBeaconUnavailableError {
    pub fn new(subject: impl Into<String>, protocol_version: ProtocolVersion) -> Self {
        Self {
            subject: subject.into(),
            protocol_version,
        }
    }

    /// What asks for the random value: the transition, and the token or document type.
    pub fn subject(&self) -> &str {
        &self.subject
    }

    /// The protocol version the block switches to.
    pub fn protocol_version(&self) -> ProtocolVersion {
        self.protocol_version
    }
}

impl From<RandomBeaconUnavailableError> for ConsensusError {
    fn from(err: RandomBeaconUnavailableError) -> Self {
        Self::BasicError(BasicError::RandomBeaconUnavailableError(err))
    }
}
