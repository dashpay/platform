use crate::drive::votes::resolved::vote_polls::contested_document_resource_vote_poll::ContestedDocumentResourceVotePollWithContractInfo;
use crate::drive::votes::resolved::vote_polls::identity_contender_vote_poll::IdentityContenderVotePollEndOutcome;
use derive_more::From;
use dpp::identifier::Identifier;
use dpp::voting::vote_choices::resource_vote_choice::ResourceVoteChoice;
use dpp::voting::vote_polls::identity_contender_vote_poll::IdentityContenderVotePoll;
use dpp::voting::vote_polls::VotePoll;
use dpp::ProtocolError;
use std::collections::BTreeMap;

/// Module containing logic for contested document resource vote polls.
pub mod contested_document_resource_vote_poll;
/// Module containing the paths and the end outcome of identity contender vote polls.
pub mod identity_contender_vote_poll;

/// Module containing logic to resolve various components.
#[cfg(feature = "server")]
pub mod resolve;

/// Represents a resolved vote poll in the system.
// The contested variant carries a resolved contract and dwarfs the identity contender one;
// the enum lives for one vote or one block's clean-up and is never stored, so no box.
#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone, PartialEq, From)]
pub enum ResolvedVotePoll {
    /// A resolved vote poll with contract information for a contested document resource.
    ContestedDocumentResourceVotePollWithContractInfo(
        ContestedDocumentResourceVotePollWithContractInfo,
    ),
    /// An identity contender vote poll, which needs nothing from state to resolve.
    IdentityContenderVotePoll(IdentityContenderVotePoll),
}

impl From<&ResolvedVotePoll> for VotePoll {
    fn from(value: &ResolvedVotePoll) -> Self {
        match value {
            ResolvedVotePoll::ContestedDocumentResourceVotePollWithContractInfo(
                contested_document_resource_vote_poll,
            ) => VotePoll::ContestedDocumentResourceVotePoll(
                contested_document_resource_vote_poll.into(),
            ),
            ResolvedVotePoll::IdentityContenderVotePoll(identity_contender_vote_poll) => {
                VotePoll::IdentityContenderVotePoll(identity_contender_vote_poll.clone())
            }
        }
    }
}

/// Represents a resolved vote poll in the system that also contains votes.
// The contested variant carries a resolved contract and dwarfs the identity contender one;
// the enum lives for one vote or one block's clean-up and is never stored, so no box.
#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone, PartialEq, From)]
pub enum ResolvedVotePollWithVotes {
    /// A resolved vote poll with contract information for a contested document resource.
    ContestedDocumentResourceVotePollWithContractInfoAndVotes(
        ContestedDocumentResourceVotePollWithContractInfo,
        BTreeMap<ResourceVoteChoice, Vec<Identifier>>,
    ),
    /// An identity contender vote poll whose join or vote phase just ended, with what the end
    /// produced.
    IdentityContenderVotePollWithEndOutcome(
        IdentityContenderVotePoll,
        IdentityContenderVotePollEndOutcome,
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
            ResolvedVotePoll::IdentityContenderVotePoll(identity_contender_vote_poll) => {
                Ok(Some(identity_contender_vote_poll.specialized_balance_id()?))
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
            ResolvedVotePoll::IdentityContenderVotePoll(identity_contender_vote_poll) => {
                identity_contender_vote_poll.unique_id()
            }
        }
    }
}
