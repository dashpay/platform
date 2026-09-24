#[cfg(feature = "json-conversion")]
use crate::serialization::json_safe_fields;
use crate::voting::vote_choices::yes_no_abstain_vote_choice::YesNoAbstainVoteChoice;
use crate::voting::vote_polls::yes_no_vote_poll::YesNoVotePoll;
use crate::ProtocolError;
use bincode::{Decode, DecodeUntrusted, Encode};
use platform_serialization_derive::{
    PlatformDeserializeTrusted, PlatformDeserializeUntrusted, PlatformSerialize,
};
use platform_value::Identifier;
#[cfg(feature = "serde-conversion")]
use serde::{Deserialize, Serialize};

/// Version 0 of a yes/no vote: the poll answered and the answer.
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
    /// The poll answered.
    pub vote_poll: YesNoVotePoll,
    /// The answer.
    pub vote_choice: YesNoAbstainVoteChoice,
}

impl YesNoVoteV0 {
    pub fn vote_poll_unique_id(&self) -> Result<Identifier, ProtocolError> {
        self.vote_poll.unique_id()
    }
}
