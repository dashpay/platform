#[cfg(feature = "json-conversion")]
use crate::serialization::json_safe_fields;
use crate::voting::vote_choices::resource_vote_choice::ResourceVoteChoice;
use crate::voting::vote_polls::VotePoll;
use crate::ProtocolError;
use bincode::{Decode, DecodeUntrusted, Encode};
use platform_serialization_derive::{
    PlatformDeserializeTrusted, PlatformDeserializeUntrusted, PlatformSerialize,
};
use platform_value::Identifier;
use platform_version::version::PlatformVersion;
#[cfg(feature = "serde-conversion")]
use serde::{Deserialize, Serialize};

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
    DecodeUntrusted,
)]
#[cfg_attr(
    feature = "serde-conversion",
    derive(Serialize, Deserialize),
    serde(rename_all = "camelCase"),
    serde(try_from = "ResourceVoteV0Inner")
)]
#[platform_serialize(unversioned)]
pub struct ResourceVoteV0 {
    pub vote_poll: VotePoll,
    pub resource_vote_choice: ResourceVoteChoice,
}

/// The fields of a resource vote read from a plain object or JSON, built into one through
/// [`ResourceVoteV0::try_new`] so those conversions refuse a yes/no poll as `try_new` does.
#[cfg(feature = "serde-conversion")]
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ResourceVoteV0Inner {
    vote_poll: VotePoll,
    resource_vote_choice: ResourceVoteChoice,
}

#[cfg(feature = "serde-conversion")]
impl TryFrom<ResourceVoteV0Inner> for ResourceVoteV0 {
    type Error = ProtocolError;

    fn try_from(value: ResourceVoteV0Inner) -> Result<Self, Self::Error> {
        ResourceVoteV0::try_new(value.vote_poll, value.resource_vote_choice)
    }
}

impl Default for ResourceVoteV0 {
    fn default() -> Self {
        ResourceVoteV0 {
            vote_poll: VotePoll::default(),
            resource_vote_choice: ResourceVoteChoice::Abstain,
        }
    }
}

impl ResourceVoteV0 {
    /// A resource vote on `vote_poll`, refused when the poll is a yes/no poll, which a yes/no
    /// vote answers. The fields stay public for decoding; consensus refuses a resource vote
    /// naming a yes/no poll however it was built.
    pub fn try_new(
        vote_poll: VotePoll,
        resource_vote_choice: ResourceVoteChoice,
    ) -> Result<Self, ProtocolError> {
        match vote_poll {
            VotePoll::ContestedDocumentResourceVotePoll(_) => Ok(ResourceVoteV0 {
                vote_poll,
                resource_vote_choice,
            }),
            VotePoll::YesNoVotePoll(_) => Err(ProtocolError::VoteError(
                "a resource vote answers a contested document resource vote poll; a yes/no vote \
                 poll is answered by a yes/no vote"
                    .to_string(),
            )),
        }
    }

    /// The id of the poll this vote answers.
    pub fn vote_poll_unique_id(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<Identifier, ProtocolError> {
        self.vote_poll.unique_id(platform_version)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::voting::vote_polls::contested_document_resource_vote_poll::ContestedDocumentResourceVotePoll;
    use crate::voting::vote_polls::yes_no_vote_poll::{YesNoMinimumVotingPower, YesNoVotePoll};
    use platform_value::BinaryData;

    #[test]
    fn should_build_a_resource_vote_only_on_a_contested_poll() {
        let contested_poll = VotePoll::ContestedDocumentResourceVotePoll(
            ContestedDocumentResourceVotePoll::default(),
        );
        let vote = ResourceVoteV0::try_new(contested_poll.clone(), ResourceVoteChoice::Abstain)
            .expect("expected a resource vote on a contested poll");
        assert_eq!(vote.vote_poll, contested_poll);

        let yes_no_poll = VotePoll::YesNoVotePoll(YesNoVotePoll {
            resource_path: vec![BinaryData::new(b"challenge".to_vec())],
            supermajority_numerator: 2,
            supermajority_denominator: 3,
            minimum_voting_power: YesNoMinimumVotingPower::Absolute { voting_power: 0 },
        });
        ResourceVoteV0::try_new(yes_no_poll, ResourceVoteChoice::Abstain)
            .expect_err("expected a resource vote on a yes/no poll to be refused");
    }
}
