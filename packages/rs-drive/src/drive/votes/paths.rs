use crate::drive::votes::resolved::vote_polls::contested_document_resource_vote_poll::{
    ContestedDocumentResourceVotePollWithContractInfo,
    ContestedDocumentResourceVotePollWithContractInfoAllowBorrowed,
};
use crate::drive::votes::ResourceVoteChoiceToKeyTrait;
use crate::drive::RootTree;
use crate::error::Error;
use crate::util::common::encode::encode_u64;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::document_type::methods::DocumentTypeV0Methods;
use dpp::data_contract::document_type::IndexProperty;
use dpp::identity::TimestampMillis;
use dpp::voting::vote_choices::resource_vote_choice::ResourceVoteChoice;
use platform_version::version::PlatformVersion;

/// The votes tree structure looks like this
///
/// ```text
/// Votes
///
/// |- End date Queries [key: "e"]
/// |- Decisions [key: "d"]
/// |- Contested Resource [key: "c"]
///    |- Active polls [key: "p"]
///    |- Identifier Votes Query [key: "i"]
/// ```
///

/// A subtree made for polls to the network that represent decisions.
pub const VOTE_DECISIONS_TREE_KEY: char = 'd';

/// A subtree made for contested resources that will be voted on.
pub const CONTESTED_RESOURCE_TREE_KEY: char = 'c';

/// A subtree made for being able to query the end date of votes
pub const END_DATE_QUERIES_TREE_KEY: char = 'e';

/// The currently active polls
pub const ACTIVE_POLLS_TREE_KEY: char = 'p';

/// A subtree made for being able to query votes that an identity has made
pub const IDENTITY_VOTES_TREE_KEY: char = 'i';

/// The finished info
pub const RESOURCE_STORED_INFO_KEY_U8_32: [u8; 32] = [
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
];

/// In the active vote poll this will contain votes for abstaining on the vote for the contested resource
pub const RESOURCE_ABSTAIN_VOTE_TREE_KEY_U8_32: [u8; 32] = [
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1,
];

/// In the active vote poll this will contain votes for locking the contested resource
pub const RESOURCE_LOCK_VOTE_TREE_KEY_U8_32: [u8; 32] = [
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2,
];

/// The tree key for storage of contested documents
pub const CONTESTED_DOCUMENT_STORAGE_TREE_KEY: u8 = 0;

/// The tree key for the indexes of contested documents
pub const CONTESTED_DOCUMENT_INDEXES_TREE_KEY: u8 = 1;

/// The tree key for storage
pub const VOTING_STORAGE_TREE_KEY: u8 = 1;

/// Convenience methods to be easily able to get a path when we know the vote poll
pub trait VotePollPaths {
    /// The root path, under this there should be the documents area and the contract itself
    fn contract_path(&self) -> [&[u8]; 4];

    /// The root path, under this there should be the documents area and the contract itself as a vec
    fn contract_path_vec(&self) -> Vec<Vec<u8>>;

    /// The documents path, under this you should have the various document types
    fn document_type_path(&self) -> [&[u8]; 5];

    /// The documents path, under this you should have the various document types as a vec
    fn document_type_path_vec(&self) -> Vec<Vec<u8>>;

    /// The documents storage path
    fn documents_storage_path(&self) -> [&[u8]; 6];

    /// The documents storage path as a vec
    fn documents_storage_path_vec(&self) -> Vec<Vec<u8>>;

    /// The contenders path as a vec
    fn contenders_path(&self, platform_version: &PlatformVersion) -> Result<Vec<Vec<u8>>, Error>;

    /// The path that would store the contender information for a single contender
    fn contender_path(
        &self,
        vote_choice: &ResourceVoteChoice,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<Vec<u8>>, Error>;

    /// The path that would store the votes for a single contender
    fn contender_voting_path(
        &self,
        vote_choice: &ResourceVoteChoice,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<Vec<u8>>, Error>;
}

impl VotePollPaths for ContestedDocumentResourceVotePollWithContractInfo {
    fn contract_path(&self) -> [&[u8]; 4] {
        vote_contested_resource_active_polls_contract_tree_path(
            self.contract.as_ref().id_ref().as_slice(),
        )
    }

    fn contract_path_vec(&self) -> Vec<Vec<u8>> {
        vote_contested_resource_active_polls_contract_tree_path_vec(
            self.contract.as_ref().id_ref().as_slice(),
        )
    }

    fn document_type_path(&self) -> [&[u8]; 5] {
        vote_contested_resource_active_polls_contract_document_tree_path(
            self.contract.as_ref().id_ref().as_slice(),
            self.document_type_name.as_str(),
        )
    }

    fn document_type_path_vec(&self) -> Vec<Vec<u8>> {
        vote_contested_resource_active_polls_contract_document_tree_path_vec(
            self.contract.as_ref().id_ref().as_slice(),
            self.document_type_name.as_str(),
        )
    }

    fn documents_storage_path(&self) -> [&[u8]; 6] {
        vote_contested_resource_contract_documents_storage_path(
            self.contract.as_ref().id_ref().as_slice(),
            self.document_type_name.as_str(),
        )
    }

    fn documents_storage_path_vec(&self) -> Vec<Vec<u8>> {
        vote_contested_resource_contract_documents_storage_path_vec(
            self.contract.as_ref().id_ref().as_slice(),
            self.document_type_name.as_str(),
        )
    }

    fn contenders_path(&self, platform_version: &PlatformVersion) -> Result<Vec<Vec<u8>>, Error> {
        let mut root = vote_contested_resource_contract_documents_indexes_path_vec(
            self.contract.as_ref().id_ref().as_slice(),
            self.document_type_name.as_str(),
        );
        let document_type = self.document_type()?;
        root.append(
            &mut self
                .index()?
                .properties
                .iter()
                .zip(self.index_values.iter())
                .map(|(IndexProperty { name, .. }, value)| {
                    document_type
                        .serialize_value_for_key(name, value, platform_version)
                        .map_err(Error::Protocol)
                })
                .collect::<Result<Vec<Vec<u8>>, Error>>()?,
        );
        Ok(root)
    }

    fn contender_path(
        &self,
        vote_choice: &ResourceVoteChoice,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<Vec<u8>>, Error> {
        let mut contenders_path = self.contenders_path(platform_version)?;
        contenders_path.push(vote_choice.to_key());
        Ok(contenders_path)
    }

    fn contender_voting_path(
        &self,
        vote_choice: &ResourceVoteChoice,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<Vec<u8>>, Error> {
        let key = vote_choice.to_key();
        let mut contender_voting_path = self.contenders_path(platform_version)?;
        contender_voting_path.push(key);
        contender_voting_path.push(vec![VOTING_STORAGE_TREE_KEY]);
        Ok(contender_voting_path)
    }
}

impl<'a> VotePollPaths for ContestedDocumentResourceVotePollWithContractInfoAllowBorrowed<'a> {
    fn contract_path(&self) -> [&[u8]; 4] {
        vote_contested_resource_active_polls_contract_tree_path(
            self.contract.as_ref().id_ref().as_slice(),
        )
    }

    fn contract_path_vec(&self) -> Vec<Vec<u8>> {
        vote_contested_resource_active_polls_contract_tree_path_vec(
            self.contract.as_ref().id_ref().as_slice(),
        )
    }

    fn document_type_path(&self) -> [&[u8]; 5] {
        vote_contested_resource_active_polls_contract_document_tree_path(
            self.contract.as_ref().id_ref().as_slice(),
            self.document_type_name.as_str(),
        )
    }

    fn document_type_path_vec(&self) -> Vec<Vec<u8>> {
        vote_contested_resource_active_polls_contract_document_tree_path_vec(
            self.contract.as_ref().id_ref().as_slice(),
            self.document_type_name.as_str(),
        )
    }

    fn documents_storage_path(&self) -> [&[u8]; 6] {
        vote_contested_resource_contract_documents_storage_path(
            self.contract.as_ref().id_ref().as_slice(),
            self.document_type_name.as_str(),
        )
    }

    fn documents_storage_path_vec(&self) -> Vec<Vec<u8>> {
        vote_contested_resource_contract_documents_storage_path_vec(
            self.contract.as_ref().id_ref().as_slice(),
            self.document_type_name.as_str(),
        )
    }

    fn contenders_path(&self, platform_version: &PlatformVersion) -> Result<Vec<Vec<u8>>, Error> {
        let mut root = vote_contested_resource_contract_documents_indexes_path_vec(
            self.contract.as_ref().id_ref().as_slice(),
            self.document_type_name.as_str(),
        );
        let document_type = self.document_type()?;
        root.append(
            &mut self
                .index()?
                .properties
                .iter()
                .zip(self.index_values.iter())
                .map(|(IndexProperty { name, .. }, value)| {
                    document_type
                        .serialize_value_for_key(name, value, platform_version)
                        .map_err(Error::Protocol)
                })
                .collect::<Result<Vec<Vec<u8>>, Error>>()?,
        );
        Ok(root)
    }

    fn contender_path(
        &self,
        vote_choice: &ResourceVoteChoice,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<Vec<u8>>, Error> {
        let mut contenders_path = self.contenders_path(platform_version)?;
        contenders_path.push(vote_choice.to_key());
        Ok(contenders_path)
    }

    fn contender_voting_path(
        &self,
        vote_choice: &ResourceVoteChoice,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<Vec<u8>>, Error> {
        let key = vote_choice.to_key();
        let mut contender_voting_path = self.contenders_path(platform_version)?;
        contender_voting_path.push(key);
        contender_voting_path.push(vec![VOTING_STORAGE_TREE_KEY]);
        Ok(contender_voting_path)
    }
}

/// the root path of the voting branch
pub fn vote_root_path<'a>() -> [&'a [u8]; 1] {
    [Into::<&[u8; 1]>::into(RootTree::Votes)]
}

/// the root path of the voting branch as a vec
pub fn vote_root_path_vec() -> Vec<Vec<u8>> {
    vec![vec![RootTree::Votes as u8]]
}

/// the decisions path of the voting branch
pub fn vote_decisions_tree_path<'a>() -> [&'a [u8]; 2] {
    [
        Into::<&[u8; 1]>::into(RootTree::Votes),
        &[VOTE_DECISIONS_TREE_KEY as u8],
    ]
}

/// the decisions path of the voting branch as a vec
pub fn vote_decisions_tree_path_vec() -> Vec<Vec<u8>> {
    vec![
        vec![RootTree::Votes as u8],
        vec![VOTE_DECISIONS_TREE_KEY as u8],
    ]
}

/// the contested resource tree path of the voting branch
pub fn vote_contested_resource_tree_path<'a>() -> [&'a [u8]; 2] {
    [
        Into::<&[u8; 1]>::into(RootTree::Votes),
        &[CONTESTED_RESOURCE_TREE_KEY as u8],
    ]
}

/// the contested resource tree path of the voting branch as a vec
pub fn vote_contested_resource_tree_path_vec() -> Vec<Vec<u8>> {
    vec![
        vec![RootTree::Votes as u8],
        vec![CONTESTED_RESOURCE_TREE_KEY as u8],
    ]
}

/// the end dates of contested resources
pub fn vote_end_date_queries_tree_path<'a>() -> [&'a [u8]; 2] {
    [
        Into::<&[u8; 1]>::into(RootTree::Votes),
        &[END_DATE_QUERIES_TREE_KEY as u8],
    ]
}

/// the end dates of contested resources as a vec
pub fn vote_end_date_queries_tree_path_vec() -> Vec<Vec<u8>> {
    vec![
        vec![RootTree::Votes as u8],
        vec![END_DATE_QUERIES_TREE_KEY as u8],
    ]
}

/// this is the path where votes are actually kept
pub fn vote_contested_resource_active_polls_tree_path<'a>() -> [&'a [u8]; 3] {
    [
        Into::<&[u8; 1]>::into(RootTree::Votes),
        &[CONTESTED_RESOURCE_TREE_KEY as u8],
        &[ACTIVE_POLLS_TREE_KEY as u8],
    ]
}

/// this is the path where votes are actually kept as a vec
pub fn vote_contested_resource_active_polls_tree_path_vec() -> Vec<Vec<u8>> {
    vec![
        vec![RootTree::Votes as u8],
        vec![CONTESTED_RESOURCE_TREE_KEY as u8],
        vec![ACTIVE_POLLS_TREE_KEY as u8],
    ]
}

/// Returns the path to root of a contract in the contested resource active polls.
pub fn vote_contested_resource_active_polls_contract_tree_path(contract_id: &[u8]) -> [&[u8]; 4] {
    [
        Into::<&[u8; 1]>::into(RootTree::Votes), // 1
        &[CONTESTED_RESOURCE_TREE_KEY as u8],    // 1
        &[ACTIVE_POLLS_TREE_KEY as u8],          // 1
        contract_id,                             // 32
    ]
}

/// Returns the path to root of a contract in the contested resource active polls.
pub fn vote_contested_resource_active_polls_contract_tree_path_vec(
    contract_id: &[u8],
) -> Vec<Vec<u8>> {
    vec![
        vec![RootTree::Votes as u8],
        vec![CONTESTED_RESOURCE_TREE_KEY as u8],
        vec![ACTIVE_POLLS_TREE_KEY as u8],
        contract_id.to_vec(),
    ]
}

/// Returns the path to the primary keys of a contract document type.
pub fn vote_contested_resource_active_polls_contract_document_tree_path<'a>(
    contract_id: &'a [u8],
    document_type_name: &'a str,
) -> [&'a [u8]; 5] {
    [
        Into::<&[u8; 1]>::into(RootTree::Votes), // 1
        &[CONTESTED_RESOURCE_TREE_KEY as u8],    // 1
        &[ACTIVE_POLLS_TREE_KEY as u8],          // 1
        contract_id,                             // 32
        document_type_name.as_bytes(),
    ]
}

/// Returns the path to the root of document type in the contested tree
pub fn vote_contested_resource_active_polls_contract_document_tree_path_vec(
    contract_id: &[u8],
    document_type_name: &str,
) -> Vec<Vec<u8>> {
    vec![
        vec![RootTree::Votes as u8],
        vec![CONTESTED_RESOURCE_TREE_KEY as u8],
        vec![ACTIVE_POLLS_TREE_KEY as u8],
        contract_id.to_vec(),
        document_type_name.as_bytes().to_vec(),
    ]
}

/// Returns the path to the primary keys of a contract document type.
pub fn vote_contested_resource_contract_documents_storage_path<'a>(
    contract_id: &'a [u8],
    document_type_name: &'a str,
) -> [&'a [u8]; 6] {
    [
        Into::<&[u8; 1]>::into(RootTree::Votes), // 1
        &[CONTESTED_RESOURCE_TREE_KEY as u8],    // 1
        &[ACTIVE_POLLS_TREE_KEY as u8],          // 1
        contract_id,                             // 32
        document_type_name.as_bytes(),
        &[CONTESTED_DOCUMENT_STORAGE_TREE_KEY], // 1
    ]
}

/// Returns the path to the primary keys of a contract document type as a vec.
pub fn vote_contested_resource_contract_documents_storage_path_vec(
    contract_id: &[u8],
    document_type_name: &str,
) -> Vec<Vec<u8>> {
    vec![
        vec![RootTree::Votes as u8],
        vec![CONTESTED_RESOURCE_TREE_KEY as u8],
        vec![ACTIVE_POLLS_TREE_KEY as u8],
        contract_id.to_vec(),
        document_type_name.as_bytes().to_vec(),
        vec![CONTESTED_DOCUMENT_STORAGE_TREE_KEY],
    ]
}

/// Returns the path to the primary keys of a contract document type.
pub fn vote_contested_resource_contract_documents_indexes_path<'a>(
    contract_id: &'a [u8],
    document_type_name: &'a str,
) -> [&'a [u8]; 6] {
    [
        Into::<&[u8; 1]>::into(RootTree::Votes), // 1
        &[CONTESTED_RESOURCE_TREE_KEY as u8],    // 1
        &[ACTIVE_POLLS_TREE_KEY as u8],          // 1
        contract_id,                             // 32
        document_type_name.as_bytes(),
        &[CONTESTED_DOCUMENT_INDEXES_TREE_KEY], // 1
    ]
}

/// Returns the path to the primary keys of a contract document type as a vec.
pub fn vote_contested_resource_contract_documents_indexes_path_vec(
    contract_id: &[u8],
    document_type_name: &str,
) -> Vec<Vec<u8>> {
    vec![
        vec![RootTree::Votes as u8],
        vec![CONTESTED_RESOURCE_TREE_KEY as u8],
        vec![ACTIVE_POLLS_TREE_KEY as u8],
        contract_id.to_vec(),
        document_type_name.as_bytes().to_vec(),
        vec![CONTESTED_DOCUMENT_INDEXES_TREE_KEY],
    ]
}

/// the specific end date path query of a contested resources as a vec
/// there is no need to ever add a key to this
pub fn vote_contested_resource_end_date_queries_at_time_tree_path_vec(
    time: TimestampMillis,
) -> Vec<Vec<u8>> {
    vec![
        vec![RootTree::Votes as u8],
        vec![END_DATE_QUERIES_TREE_KEY as u8],
        encode_u64(time),
    ]
}

/// the identity votes of contested resources
pub fn vote_contested_resource_identity_votes_tree_path<'a>() -> [&'a [u8]; 3] {
    [
        Into::<&[u8; 1]>::into(RootTree::Votes),
        &[CONTESTED_RESOURCE_TREE_KEY as u8],
        &[IDENTITY_VOTES_TREE_KEY as u8],
    ]
}

/// the identity votes of contested resources as a vec
pub fn vote_contested_resource_identity_votes_tree_path_vec() -> Vec<Vec<u8>> {
    vec![
        vec![RootTree::Votes as u8],
        vec![CONTESTED_RESOURCE_TREE_KEY as u8],
        vec![IDENTITY_VOTES_TREE_KEY as u8],
    ]
}

/// the qualified identity vote of contested resources
/// this is with the identity id at the end already
pub fn vote_contested_resource_identity_votes_tree_path_for_identity(
    identity_id: &[u8; 32],
) -> [&[u8]; 4] {
    [
        Into::<&[u8; 1]>::into(RootTree::Votes),
        &[CONTESTED_RESOURCE_TREE_KEY as u8],
        &[IDENTITY_VOTES_TREE_KEY as u8],
        identity_id,
    ]
}

/// the qualified identity vote of contested resources as a vec
/// this is with the identity id at the end already
pub fn vote_contested_resource_identity_votes_tree_path_for_identity_vec(
    identity_id: &[u8; 32],
) -> Vec<Vec<u8>> {
    vec![
        vec![RootTree::Votes as u8],
        vec![CONTESTED_RESOURCE_TREE_KEY as u8],
        vec![IDENTITY_VOTES_TREE_KEY as u8],
        identity_id.to_vec(),
    ]
}
