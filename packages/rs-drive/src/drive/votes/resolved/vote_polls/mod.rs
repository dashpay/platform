use crate::drive::votes::resolved::vote_polls::contested_document_resource_vote_poll::ContestedDocumentResourceVotePollWithContractInfo;
use derive_more::From;
use dpp::identifier::Identifier;
use dpp::voting::vote_choices::resource_vote_choice::ResourceVoteChoice;
use dpp::voting::vote_choices::yes_no_abstain_vote_choice::YesNoAbstainVoteChoice;
use dpp::voting::vote_polls::yes_no_vote_poll::YesNoVotePoll;
use dpp::voting::vote_polls::VotePoll;
use dpp::ProtocolError;
use std::collections::BTreeMap;

/// Module containing logic for contested document resource vote polls.
pub mod contested_document_resource_vote_poll;

/// Module containing logic to resolve various components.
#[cfg(feature = "server")]
pub mod resolve;

/// Represents a resolved vote poll in the system.
// The contested variant carries the resolved contract; a yes/no poll is a few bytes.
#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone, PartialEq, From)]
pub enum ResolvedVotePoll {
    /// A resolved vote poll with contract information for a contested document resource.
    ContestedDocumentResourceVotePollWithContractInfo(
        ContestedDocumentResourceVotePollWithContractInfo,
    ),
    /// A yes/no vote poll. It needs nothing from the state to be resolved.
    YesNoVotePoll(YesNoVotePoll),
}

impl From<&ResolvedVotePoll> for VotePoll {
    fn from(value: &ResolvedVotePoll) -> Self {
        match value {
            ResolvedVotePoll::ContestedDocumentResourceVotePollWithContractInfo(
                contested_document_resource_vote_poll,
            ) => VotePoll::ContestedDocumentResourceVotePoll(
                contested_document_resource_vote_poll.into(),
            ),
            ResolvedVotePoll::YesNoVotePoll(yes_no_vote_poll) => {
                VotePoll::YesNoVotePoll(yes_no_vote_poll.clone())
            }
        }
    }
}

/// Represents a resolved vote poll in the system that also contains votes.
// The contested variant carries the resolved contract; a yes/no poll is a few bytes.
#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone, PartialEq, From)]
pub enum ResolvedVotePollWithVotes {
    /// A resolved vote poll with contract information for a contested document resource.
    ContestedDocumentResourceVotePollWithContractInfoAndVotes(
        ContestedDocumentResourceVotePollWithContractInfo,
        BTreeMap<ResourceVoteChoice, Vec<Identifier>>,
    ),
    /// A yes/no vote poll and the masternodes that voted for each choice.
    YesNoVotePollWithVotes(
        YesNoVotePoll,
        BTreeMap<YesNoAbstainVoteChoice, Vec<Identifier>>,
    ),
}

impl ResolvedVotePoll {
    /// Retrieves the specialized balance identifier associated with the resolved vote poll.
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
            ResolvedVotePoll::ContestedDocumentResourceVotePollWithContractInfo(
                contested_document_resource_vote_poll,
            ) => Ok(Some(
                contested_document_resource_vote_poll.specialized_balance_id()?,
            )),
            ResolvedVotePoll::YesNoVotePoll(yes_no_vote_poll) => {
                Ok(Some(yes_no_vote_poll.specialized_balance_id()?))
            }
        }
    }

    /// Retrieves the unique identifier associated with the resolved vote poll.
    ///
    /// # Returns
    ///
    /// * `Ok(identifier)` containing the unique identifier.
    /// * `Err(ProtocolError)` if there is an error retrieving the unique ID.
    ///
    /// # Errors
    ///
    /// Returns a `ProtocolError` if there is an issue retrieving the unique ID.
    pub fn unique_id(&self) -> Result<Identifier, ProtocolError> {
        match self {
            ResolvedVotePoll::ContestedDocumentResourceVotePollWithContractInfo(
                contested_document_resource_vote_poll,
            ) => contested_document_resource_vote_poll.unique_id(),
            ResolvedVotePoll::YesNoVotePoll(yes_no_vote_poll) => yes_no_vote_poll.unique_id(),
        }
    }
}
