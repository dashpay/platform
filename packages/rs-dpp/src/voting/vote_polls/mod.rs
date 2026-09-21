#[cfg(feature = "json-conversion")]
use crate::serialization::JsonConvertible;
#[cfg(feature = "value-conversion")]
use crate::serialization::ValueConvertible;
use crate::voting::vote_polls::contested_document_resource_vote_poll::ContestedDocumentResourceVotePoll;
use crate::voting::vote_polls::identity_contender_vote_poll::IdentityContenderVotePoll;
use crate::ProtocolError;
use bincode::{Decode, DecodeUntrusted, Encode};
use derive_more::From;
use platform_serialization_derive::{
    PlatformDeserializeTrusted, PlatformDeserializeUntrusted, PlatformSerialize,
};
use platform_value::Identifier;
#[cfg(feature = "serde-conversion")]
use serde::{Deserialize, Serialize};
use std::fmt;

pub mod contested_document_resource_vote_poll;
pub mod identity_contender_vote_poll;

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
    From,
    DecodeUntrusted,
)]
#[cfg_attr(
    feature = "serde-conversion",
    derive(Serialize, Deserialize),
    // Internally tagged with a `$type` discriminator; inner
    // `ContestedDocumentResourceVotePoll` fields flatten at the same level.
    serde(tag = "$type", rename_all = "camelCase")
)]
#[cfg_attr(feature = "value-conversion", derive(ValueConvertible))]
#[platform_serialize(unversioned)]
#[platform_serialize(limit = 100000)]
pub enum VotePoll {
    ContestedDocumentResourceVotePoll(ContestedDocumentResourceVotePoll),
    /// A poll electing one identity among contenders over a resource path, without a Lock
    /// choice, resolved by plurality with the earliest contender winning a tie (protocol
    /// version 14).
    IdentityContenderVotePoll(IdentityContenderVotePoll),
}

impl fmt::Display for VotePoll {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            VotePoll::ContestedDocumentResourceVotePoll(poll) => {
                write!(f, "ContestedDocumentResourceVotePoll({})", poll)
            }
            VotePoll::IdentityContenderVotePoll(poll) => {
                write!(f, "IdentityContenderVotePoll({})", poll)
            }
        }
    }
}

impl Default for VotePoll {
    fn default() -> Self {
        ContestedDocumentResourceVotePoll::default().into()
    }
}

impl VotePoll {
    pub fn specialized_balance_id(&self) -> Result<Option<Identifier>, ProtocolError> {
        match self {
            VotePoll::ContestedDocumentResourceVotePoll(contested_document_resource_vote_poll) => {
                Ok(Some(
                    contested_document_resource_vote_poll.specialized_balance_id()?,
                ))
            }
            VotePoll::IdentityContenderVotePoll(identity_contender_vote_poll) => {
                Ok(Some(identity_contender_vote_poll.specialized_balance_id()?))
            }
        }
    }

    pub fn unique_id(&self) -> Result<Identifier, ProtocolError> {
        match self {
            VotePoll::ContestedDocumentResourceVotePoll(contested_document_resource_vote_poll) => {
                contested_document_resource_vote_poll.unique_id()
            }
            VotePoll::IdentityContenderVotePoll(identity_contender_vote_poll) => {
                identity_contender_vote_poll.unique_id()
            }
        }
    }

    /// Whether the poll offers the Lock choice. Only contested document resource polls do: an
    /// identity contender poll is decided among its contenders or abstained on.
    pub fn allows_lock_vote(&self) -> bool {
        match self {
            VotePoll::ContestedDocumentResourceVotePoll(_) => true,
            VotePoll::IdentityContenderVotePoll(_) => false,
        }
    }
}

#[cfg(all(
    test,
    feature = "json-conversion",
    feature = "value-conversion",
    feature = "serde-conversion"
))]
mod json_convertible_tests_votepoll {
    use super::*;

    #[test]
    fn json_round_trip_votepoll() {
        use crate::serialization::JsonConvertible;
        let original = VotePoll::default();
        let json = original.to_json().expect("to_json");
        let recovered = VotePoll::from_json(json).expect("from_json");
        assert_eq!(original, recovered);
    }

    #[test]
    fn value_round_trip_votepoll() {
        use crate::serialization::ValueConvertible;
        let original = VotePoll::default();
        let value = original.to_object().expect("to_object");
        let recovered = VotePoll::from_object(value).expect("from_object");
        assert_eq!(original, recovered);
    }

    #[test]
    fn json_round_trip_identity_contender_vote_poll_with_full_wire_shape() {
        use crate::serialization::JsonConvertible;
        use serde_json::json;
        let original = VotePoll::IdentityContenderVotePoll(IdentityContenderVotePoll::new(vec![
            vec![0xc1; 2],
            b"el".to_vec(),
        ]));
        let json = original.to_json().expect("to_json");
        assert_eq!(
            json,
            json!({
                "$type": "identityContenderVotePoll",
                "resourcePath": [[193, 193], [101, 108]],
            })
        );
        let recovered = VotePoll::from_json(json).expect("from_json");
        assert_eq!(original, recovered);
    }

    #[test]
    fn value_round_trip_identity_contender_vote_poll() {
        use crate::serialization::ValueConvertible;
        let original = VotePoll::IdentityContenderVotePoll(IdentityContenderVotePoll::new(vec![
            vec![0xc1; 32],
            b"election".to_vec(),
        ]));
        let value = original.to_object().expect("to_object");
        let recovered = VotePoll::from_object(value).expect("from_object");
        assert_eq!(original, recovered);
    }
}
