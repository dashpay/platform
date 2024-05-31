use crate::drive::defaults::{DEFAULT_HASH_SIZE, DEFAULT_HASH_SIZE_USIZE};
use crate::drive::document::paths::contract_document_type_path;
use crate::drive::votes::paths::{
    ACTIVE_POLLS_TREE_KEY, CONTESTED_RESOURCE_TREE_KEY, RESOURCE_ABSTAIN_VOTE_TREE_KEY,
    RESOURCE_LOCK_VOTE_TREE_KEY, VOTE_DECISIONS_TREE_KEY,
};
use crate::drive::RootTree::Votes;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::data_contract::DataContract;
use dpp::identifier::Identifier;
use dpp::voting::vote_choices::resource_vote_choice::ResourceVoteChoice;
use dpp::voting::vote_polls::contested_document_resource_vote_poll::ContestedDocumentResourceVotePoll;
use dpp::voting::vote_polls::VotePoll;
use dpp::voting::votes::resource_vote::accessors::v0::ResourceVoteGettersV0;
use dpp::voting::votes::resource_vote::v0::ResourceVoteV0;
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

/// A trait to convert the vote to a tree path usable in grovedb
pub trait TreePath {
    /// The tree path function
    fn tree_path<'a>(&'a self, contract: &'a DataContract) -> Result<Vec<&'a [u8]>, ProtocolError>;

    /// Construction of the resource vote from the tree oath
    fn try_from_tree_path(path: Vec<Vec<u8>>) -> Result<Self, ProtocolError>
    where
        Self: Sized;
}

impl TreePath for Vote {
    fn tree_path<'a>(&'a self, contract: &'a DataContract) -> Result<Vec<&'a [u8]>, ProtocolError> {
        match self {
            Vote::ResourceVote(resource_vote) => resource_vote.tree_path(contract),
        }
    }

    fn try_from_tree_path(path: Vec<Vec<u8>>) -> Result<Self, ProtocolError>
    where
        Self: Sized,
    {
        if path.len() < 3 {
            return Err(ProtocolError::VoteError(format!(
                "path {} is not long enough to construct vote information",
                path.into_iter()
                    .map(hex::encode)
                    .collect::<Vec<_>>()
                    .join("/")
            )));
        }
        let key_0 = path.get(0).unwrap();
        let key_1 = path.get(1).unwrap();

        let Some(key_0_byte) = key_0.get(0) else {
            return Err(ProtocolError::VoteError(format!(
                "path {} first element must be a byte",
                path.into_iter()
                    .map(hex::encode)
                    .collect::<Vec<_>>()
                    .join("/")
            )));
        };

        if *key_0_byte != Votes as u8 {
            return Err(ProtocolError::VoteError(format!(
                "path {} first element must be a byte for voting {}, got {}",
                path.iter().map(hex::encode).collect::<Vec<_>>().join("/"),
                Votes as u8,
                *key_0_byte
            )));
        };

        let Some(key_1_byte) = key_1.get(0) else {
            return Err(ProtocolError::VoteError(format!(
                "path {} second element must be a byte",
                path.into_iter()
                    .map(hex::encode)
                    .collect::<Vec<_>>()
                    .join("/")
            )));
        };

        match *key_1_byte as char {
            CONTESTED_RESOURCE_TREE_KEY => {
                Ok(Vote::ResourceVote(ResourceVote::try_from_tree_path(path)?))
            }
            VOTE_DECISIONS_TREE_KEY => Err(ProtocolError::NotSupported(
                "decision votes not supported yet".to_string(),
            )),
            _ => Err(ProtocolError::VoteError(format!(
                "path {} second element must be a byte for CONTESTED_RESOURCE_TREE_KEY {}, got {}",
                path.iter().map(hex::encode).collect::<Vec<_>>().join("/"),
                CONTESTED_RESOURCE_TREE_KEY as u8,
                *key_1_byte
            ))),
        }
    }
}

impl TreePath for ResourceVote {
    fn try_from_tree_path(path: Vec<Vec<u8>>) -> Result<Self, ProtocolError>
    where
        Self: Sized,
    {
        if path.len() < 8 {
            return Err(ProtocolError::VoteError(format!(
                "path {} is not long enough to construct vote information",
                path.into_iter()
                    .map(hex::encode)
                    .collect::<Vec<_>>()
                    .join("/")
            )));
        }

        let key_2 = path.get(2).unwrap(); // active_vote_polls
        let key_contract_id = path.get(3).unwrap(); // contract_id
        let key_document_type_name = path.get(4).unwrap(); // document_type_name
        let key_vote_choice = path.get(path.len() - 3).unwrap(); // this is the vote choice

        let Some(key_2_byte) = key_2.get(0) else {
            return Err(ProtocolError::VoteError(format!(
                "path {} third element must be a byte",
                path.into_iter()
                    .map(hex::encode)
                    .collect::<Vec<_>>()
                    .join("/")
            )));
        };

        if *key_2_byte != ACTIVE_POLLS_TREE_KEY as u8 {
            return Err(ProtocolError::VoteError(format!(
                "path {} third element must be a byte for ACTIVE_POLLS_TREE_KEY {}, got {}",
                path.iter().map(hex::encode).collect::<Vec<_>>().join("/"),
                ACTIVE_POLLS_TREE_KEY as u8,
                *key_2_byte
            )));
        };

        if key_contract_id.len() != DEFAULT_HASH_SIZE_USIZE {
            return Err(ProtocolError::VoteError(format!(
                "path {} fourth element must be a contract id but isn't 32 bytes long",
                path.into_iter()
                    .map(hex::encode)
                    .collect::<Vec<_>>()
                    .join("/")
            )));
        }

        let contract_id = Identifier::from_vec(key_contract_id.clone())?;

        let document_type_name = String::from_utf8(key_document_type_name.clone()).map_err(|_| ProtocolError::VoteError(format!("path {} fifth element must be a document type name but couldn't be converted to a string", path.iter().map(hex::encode).collect::<Vec<_>>().join("/"))))?;

        let resource_vote_choice = if key_vote_choice.len() == 32 {
            ResourceVoteChoice::TowardsIdentity(Identifier::from_vec(key_vote_choice.clone())?)
        } else if key_vote_choice.len() == 1 {
            let char = (*key_vote_choice.first().unwrap()) as char;
            match char {
                RESOURCE_ABSTAIN_VOTE_TREE_KEY => ResourceVoteChoice::Abstain,
                RESOURCE_LOCK_VOTE_TREE_KEY => ResourceVoteChoice::Lock,
                _ => return Err(ProtocolError::VoteError(format!("path {} 2 before last element must be an identifier or RESOURCE_ABSTAIN_VOTE_TREE_KEY/RESOURCE_LOCK_VOTE_TREE_KEY", path.into_iter().map(hex::encode).collect::<Vec<_>>().join("/")))),
            }
        } else {
            return Err(ProtocolError::VoteError(format!("path {} 2 before last element must be an identifier or RESOURCE_ABSTAIN_VOTE_TREE_KEY/RESOURCE_LOCK_VOTE_TREE_KEY", path.into_iter().map(hex::encode).collect::<Vec<_>>().join("/"))));
        };

        let vote_poll =
            VotePoll::ContestedDocumentResourceVotePoll(ContestedDocumentResourceVotePoll {
                contract_id,
                document_type_name,
                index_name: "".to_string(),
                index_values: vec![],
            });

        Ok(ResourceVote::V0(ResourceVoteV0 {
            vote_poll,
            resource_vote_choice,
        }))
    }
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
            ResourceVoteChoice::Abstain => vec![RESOURCE_ABSTAIN_VOTE_TREE_KEY as u8],
            ResourceVoteChoice::Lock => vec![RESOURCE_LOCK_VOTE_TREE_KEY as u8],
        }
    }
}
