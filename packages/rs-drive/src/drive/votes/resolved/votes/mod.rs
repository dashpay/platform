#[cfg(feature = "server")]
pub(crate) mod resolve;
/// Resolved resource vote module
pub mod resolved_resource_vote;
/// Resolved yes/no vote module
pub mod resolved_yes_no_vote;

use crate::drive::votes::resolved::vote_polls::ResolvedVotePoll;
use crate::drive::votes::resolved::votes::resolved_resource_vote::accessors::v0::ResolvedResourceVoteGettersV0;
use crate::drive::votes::resolved::votes::resolved_resource_vote::ResolvedResourceVote;
use crate::drive::votes::resolved::votes::resolved_yes_no_vote::ResolvedYesNoVote;
use dpp::identifier::Identifier;
use dpp::voting::vote_polls::VotePoll;
use dpp::voting::votes::resource_vote::v0::ResourceVoteV0;
use dpp::voting::votes::resource_vote::ResourceVote;
use dpp::voting::votes::Vote;
use dpp::ProtocolError;

/// Represents the different types of resolved votes within the system.
// A resource vote carries the resolved contract of its poll; a yes/no vote is a few bytes.
#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone, PartialEq)]
pub enum ResolvedVote {
    /// A resolved vote for a specific resource.
    ResolvedResourceVote(ResolvedResourceVote),
    /// A vote on a yes/no poll.
    YesNoVote(ResolvedYesNoVote),
}

impl ResolvedVote {
    /// Retrieves the specialized balance identifier associated with the resolved vote.
    ///
    /// # Returns
    ///
    /// * `Ok(Some(identifier))` if a specialized balance ID is available.
    /// * `Ok(None)` if no specialized balance ID is associated.
    /// * `Err(ProtocolError)` if there is an error retrieving the balance ID.
    ///
    /// # Errors
    ///
    /// Returns a `ProtocolError` if there is an issue retrieving the specialized balance ID.
    pub fn specialized_balance_id(&self) -> Result<Option<Identifier>, ProtocolError> {
        match self {
            ResolvedVote::ResolvedResourceVote(resource_vote) => {
                resource_vote.vote_poll().specialized_balance_id()
            }
            ResolvedVote::YesNoVote(yes_no_vote) => {
                Ok(Some(yes_no_vote.vote_poll.specialized_balance_id()?))
            }
        }
    }
}

impl TryFrom<ResolvedVote> for Vote {
    type Error = ProtocolError;

    /// The vote as it was cast. A resource vote resolves to the contested document resource
    /// poll it carried, so a resolved resource vote holding a yes/no poll is a programming
    /// error, not a vote.
    fn try_from(value: ResolvedVote) -> Result<Self, Self::Error> {
        match value {
            ResolvedVote::ResolvedResourceVote(resolved_resource_vote) => {
                match resolved_resource_vote {
                    ResolvedResourceVote::V0(v0) => {
                        let vote_choice = v0.resource_vote_choice;
                        match v0.resolved_vote_poll {
                            ResolvedVotePoll::ContestedDocumentResourceVotePollWithContractInfo(
                                contested_document_resource,
                            ) => Ok(Self::ResourceVote(ResourceVote::V0(ResourceVoteV0 {
                                vote_poll: VotePoll::ContestedDocumentResourceVotePoll(
                                    contested_document_resource.into(),
                                ),
                                resource_vote_choice: vote_choice,
                            }))),
                            ResolvedVotePoll::YesNoVotePoll(_) => {
                                Err(ProtocolError::CorruptedCodeExecution(
                                    "a resolved resource vote cannot hold a yes/no vote poll"
                                        .to_string(),
                                ))
                            }
                        }
                    }
                }
            }
            ResolvedVote::YesNoVote(yes_no_vote) => Ok(Self::YesNoVote(yes_no_vote.into())),
        }
    }
}
