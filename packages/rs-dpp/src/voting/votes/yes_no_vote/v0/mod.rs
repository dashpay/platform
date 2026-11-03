#[cfg(feature = "json-conversion")]
use crate::serialization::json_safe_fields;
use crate::voting::vote_choices::yes_no_abstain_vote_choice::YesNoAbstainVoteChoice;
use crate::ProtocolError;
use bincode::{Decode, DecodeUntrusted, Encode};
use platform_serialization_derive::{
    PlatformDeserializeTrusted, PlatformDeserializeUntrusted, PlatformSerialize,
};
use platform_value::Identifier;
#[cfg(feature = "serde-conversion")]
use serde::{Deserialize, Serialize};

/// Version 0 of a yes/no vote: the id of the poll answered and the answer. The poll's rule is
/// not carried: it is in the poll's stored info, written when the poll opened.
#[cfg_attr(feature = "json-conversion", json_safe_fields)]
#[derive(
    Debug,
    Clone,
    Encode,
    Decode,
    PlatformDeserializeTrusted,
    PlatformDeserializeUntrusted,
    PlatformSerialize,
    PartialEq,
    Eq,
    DecodeUntrusted,
)]
#[cfg_attr(
    feature = "serde-conversion",
    derive(Serialize, Deserialize),
    serde(rename_all = "camelCase")
)]
#[platform_serialize(unversioned)]
pub struct YesNoVoteV0 {
    /// The id of the poll answered (`YesNoVotePoll::unique_id`).
    pub vote_poll_id: Identifier,
    /// The answer.
    pub vote_choice: YesNoAbstainVoteChoice,
}
