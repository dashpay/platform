use crate::drive::document::contract_document_type_path;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::data_contract::DataContract;
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
                    &contested_document_vote_poll.contract_id.as_bytes(),
                    &contested_document_vote_poll.document_type_name,
                )
                .to_vec();

                // at this point the path only contains the parts before the index

                let Some(contested_index) = &index.contested_index else {
                    return Err(ProtocolError::VoteError(
                    "we expect the index in a contested document resource votes type to be contested"
                        .to_string(),
                ));
                };

                let mut properties_iter = index.properties.iter();

                while let Some(index_part) = properties_iter.next() {
                    let level_name = if contested_index.contested_field_name == index_part.name {
                        &contested_index.contested_field_temp_replacement_name
                    } else {
                        &index_part.name
                    };

                    path.push(level_name.as_bytes());
                }
                Ok(path)
            }
        }
    }
}
