use crate::drive::document::paths::contract_document_type_path;
use crate::drive::votes::paths::{
    RESOURCE_ABSTAIN_VOTE_TREE_KEY_U8_32, RESOURCE_LOCK_VOTE_TREE_KEY_U8_32,
};
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::data_contract::DataContract;
use dpp::voting::vote_choices::resource_vote_choice::ResourceVoteChoice;
use dpp::voting::vote_polls::VotePoll;
use dpp::voting::votes::resource_vote::accessors::v0::ResourceVoteGettersV0;
use dpp::voting::votes::resource_vote::ResourceVote;
use dpp::voting::votes::Vote;
use dpp::ProtocolError;

#[cfg(feature = "server")]
mod cleanup;

#[cfg(feature = "server")]
mod insert;

/// Paths important for the module
#[cfg(any(feature = "server", feature = "verify"))]
pub mod paths;

#[cfg(feature = "server")]
mod setup;

#[cfg(any(feature = "server", feature = "verify"))]
/// Resolve contested document resource vote poll module
pub mod resolved;
#[cfg(any(feature = "server", feature = "verify"))]
/// Storage form
pub mod storage_form;
#[cfg(any(feature = "server", feature = "verify"))]
/// Tree path storage form
pub mod tree_path_storage_form;

#[cfg(feature = "server")]
mod fetch;

/// A trait to convert the vote to a tree path usable in grovedb
pub trait TreePath {
    /// The tree path function
    fn tree_path<'a>(&'a self, contract: &'a DataContract) -> Result<Vec<&'a [u8]>, ProtocolError>;
}

impl TreePath for Vote {
    fn tree_path<'a>(&'a self, contract: &'a DataContract) -> Result<Vec<&'a [u8]>, ProtocolError> {
        match self {
            Vote::ResourceVote(resource_vote) => resource_vote.tree_path(contract),
        }
    }
}

impl TreePath for ResourceVote {
    fn tree_path<'a>(&'a self, contract: &'a DataContract) -> Result<Vec<&'a [u8]>, ProtocolError> {
        let vote_poll = self.vote_poll();

        match vote_poll {
            VotePoll::ContestedDocumentResourceVotePoll(contested_document_vote_poll) => {
                if contract.id() != contested_document_vote_poll.contract_id {
                    return Err(ProtocolError::VoteError(format!(
                        "contract id of votes {} does not match supplied contract {}",
                        contested_document_vote_poll.contract_id,
                        contract.id()
                    )));
                }
                let document_type = contract.document_type_borrowed_for_name(
                    &contested_document_vote_poll.document_type_name,
                )?;
                let index = document_type
                    .indexes()
                    .get(&contested_document_vote_poll.index_name)
                    .ok_or(ProtocolError::UnknownContestedIndexResolution(format!(
                        "no index named {} for document type {} on contract with id {}",
                        &contested_document_vote_poll.index_name,
                        document_type.name(),
                        contract.id()
                    )))?;
                let mut path = contract_document_type_path(
                    contested_document_vote_poll.contract_id.as_bytes(),
                    &contested_document_vote_poll.document_type_name,
                )
                .to_vec();

                // at this point the path only contains the parts before the index

                let mut properties_iter = index.properties.iter();

                while let Some(index_part) = properties_iter.next() {
                    path.push(index_part.name.as_bytes());
                }
                Ok(path)
            }
        }
    }
}

/// A helper trait to get the key for a resource vote
pub trait ResourceVoteChoiceToKeyTrait {
    /// A helper function to get the key for a resource vote
    fn to_key(&self) -> Vec<u8>;
}

impl ResourceVoteChoiceToKeyTrait for ResourceVoteChoice {
    fn to_key(&self) -> Vec<u8> {
        match self {
            ResourceVoteChoice::TowardsIdentity(identity_id) => identity_id.to_vec(),
            ResourceVoteChoice::Abstain => RESOURCE_ABSTAIN_VOTE_TREE_KEY_U8_32.to_vec(),
            ResourceVoteChoice::Lock => RESOURCE_LOCK_VOTE_TREE_KEY_U8_32.to_vec(),
        }
    }
}
