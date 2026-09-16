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
use crate::drive::votes::resolved::vote_polls::contested_document_resource_vote_poll::{
    ContestedDocumentResourceVotePollWithContractInfo,
    ContestedDocumentResourceVotePollWithContractInfoAllowBorrowed,
};
use crate::drive::votes::ResourceVoteChoiceToKeyTrait;
use crate::drive::RootTree;
use crate::error::query::QuerySyntaxError;
use crate::error::Error;
use crate::util::common::encode::encode_u64;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::document_type::methods::DocumentTypeV0Methods;
use dpp::data_contract::document_type::IndexProperty;
use dpp::identity::TimestampMillis;
use dpp::voting::vote_choices::resource_vote_choice::ResourceVoteChoice;
use platform_version::version::PlatformVersion;

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

    /// the last index path as a path vec and a key
    // TODO: Use type alias or struct
    #[allow(clippy::type_complexity)]
    fn last_index_path(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<(Vec<Vec<u8>>, Option<Vec<u8>>), Error> {
        let mut contenders_path = self.contenders_path(platform_version)?;
        if contenders_path.is_empty() {
            Ok((vec![], None))
        } else {
            let last_index = contenders_path.remove(contenders_path.len() - 1);
            Ok((contenders_path, Some(last_index)))
        }
    }

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
        let index = self.index()?;
        if index.properties.len() != self.index_values.len() {
            return Err(Error::Query(QuerySyntaxError::InvalidParameter(format!(
                "contested index expects {} values, received {}",
                index.properties.len(),
                self.index_values.len()
            ))));
        }
        root.append(
            &mut index
                .properties
                .iter()
                .zip(self.index_values.iter())
                .map(|(IndexProperty { name, .. }, value)| {
                    document_type
                        .serialize_value_for_key(name, value, platform_version)
                        .map_err(Error::from)
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

impl VotePollPaths for ContestedDocumentResourceVotePollWithContractInfoAllowBorrowed<'_> {
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
        let index = self.index()?;
        if index.properties.len() != self.index_values.len() {
            return Err(Error::Query(QuerySyntaxError::InvalidParameter(format!(
                "contested index expects {} values, received {}",
                index.properties.len(),
                self.index_values.len()
            ))));
        }
        root.append(
            &mut index
                .properties
                .iter()
                .zip(self.index_values.iter())
                .map(|(IndexProperty { name, .. }, value)| {
                    document_type
                        .serialize_value_for_key(name, value, platform_version)
                        .map_err(Error::from)
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::drive::votes::resolved::vote_polls::contested_document_resource_vote_poll::ContestedDocumentResourceVotePollWithContractInfo;
    use crate::util::object_size_info::DataContractOwnedResolvedInfo;
    use dpp::tests::fixtures::get_dpns_data_contract_fixture;
    use platform_version::version::PlatformVersion;

    const VOTES_BYTE: u8 = RootTree::Votes as u8;

    #[test]
    fn contenders_path_rejects_missing_index_values() {
        let platform_version = PlatformVersion::latest();
        let contract = get_dpns_data_contract_fixture(None, 0, platform_version.protocol_version)
            .data_contract_owned();
        let vote_poll = ContestedDocumentResourceVotePollWithContractInfo {
            contract: DataContractOwnedResolvedInfo::OwnedDataContract(contract),
            document_type_name: "domain".to_string(),
            index_name: "parentNameAndLabel".to_string(),
            index_values: vec![],
        };

        let error = vote_poll
            .contenders_path(platform_version)
            .expect_err("partial contested paths must be rejected");

        assert!(matches!(
            error,
            Error::Query(QuerySyntaxError::InvalidParameter(_))
        ));
    }

    // ---------------------------------------------------------------
    // vote_root_path / vote_root_path_vec
    // ---------------------------------------------------------------

    #[test]
    fn test_vote_root_path_has_single_element() {
        let path = vote_root_path();
        assert_eq!(path.len(), 1);
        assert_eq!(path[0], &[VOTES_BYTE]);
    }

    #[test]
    fn test_vote_root_path_vec_matches_slice_form() {
        let path_vec = vote_root_path_vec();
        let path = vote_root_path();
        assert_eq!(path_vec.len(), path.len());
        assert_eq!(path_vec[0], path[0]);
    }

    // ---------------------------------------------------------------
    // vote_decisions_tree_path / _vec
    // ---------------------------------------------------------------

    #[test]
    fn test_vote_decisions_tree_path_structure() {
        let path = vote_decisions_tree_path();
        assert_eq!(path.len(), 2);
        assert_eq!(path[0], &[VOTES_BYTE]);
        assert_eq!(path[1], &[VOTE_DECISIONS_TREE_KEY as u8]);
    }

    #[test]
    fn test_vote_decisions_tree_path_vec_matches_slice_form() {
        let path_vec = vote_decisions_tree_path_vec();
        let path = vote_decisions_tree_path();
        assert_eq!(path_vec.len(), path.len());
        for (vec_elem, slice_elem) in path_vec.iter().zip(path.iter()) {
            assert_eq!(vec_elem.as_slice(), *slice_elem);
        }
    }

    // ---------------------------------------------------------------
    // vote_contested_resource_tree_path / _vec
    // ---------------------------------------------------------------

    #[test]
    fn test_vote_contested_resource_tree_path_structure() {
        let path = vote_contested_resource_tree_path();
        assert_eq!(path.len(), 2);
        assert_eq!(path[0], &[VOTES_BYTE]);
        assert_eq!(path[1], &[CONTESTED_RESOURCE_TREE_KEY as u8]);
    }

    #[test]
    fn test_vote_contested_resource_tree_path_vec_matches_slice_form() {
        let path_vec = vote_contested_resource_tree_path_vec();
        let path = vote_contested_resource_tree_path();
        assert_eq!(path_vec.len(), path.len());
        for (vec_elem, slice_elem) in path_vec.iter().zip(path.iter()) {
            assert_eq!(vec_elem.as_slice(), *slice_elem);
        }
    }

    // ---------------------------------------------------------------
    // vote_end_date_queries_tree_path / _vec
    // ---------------------------------------------------------------

    #[test]
    fn test_vote_end_date_queries_tree_path_structure() {
        let path = vote_end_date_queries_tree_path();
        assert_eq!(path.len(), 2);
        assert_eq!(path[0], &[VOTES_BYTE]);
        assert_eq!(path[1], &[END_DATE_QUERIES_TREE_KEY as u8]);
    }

    #[test]
    fn test_vote_end_date_queries_tree_path_vec_matches_slice_form() {
        let path_vec = vote_end_date_queries_tree_path_vec();
        let path = vote_end_date_queries_tree_path();
        assert_eq!(path_vec.len(), path.len());
        for (vec_elem, slice_elem) in path_vec.iter().zip(path.iter()) {
            assert_eq!(vec_elem.as_slice(), *slice_elem);
        }
    }

    // ---------------------------------------------------------------
    // vote_contested_resource_active_polls_tree_path / _vec
    // ---------------------------------------------------------------

    #[test]
    fn test_vote_contested_resource_active_polls_tree_path_structure() {
        let path = vote_contested_resource_active_polls_tree_path();
        assert_eq!(path.len(), 3);
        assert_eq!(path[0], &[VOTES_BYTE]);
        assert_eq!(path[1], &[CONTESTED_RESOURCE_TREE_KEY as u8]);
        assert_eq!(path[2], &[ACTIVE_POLLS_TREE_KEY as u8]);
    }

    #[test]
    fn test_vote_contested_resource_active_polls_tree_path_vec_matches_slice_form() {
        let path_vec = vote_contested_resource_active_polls_tree_path_vec();
        let path = vote_contested_resource_active_polls_tree_path();
        assert_eq!(path_vec.len(), path.len());
        for (vec_elem, slice_elem) in path_vec.iter().zip(path.iter()) {
            assert_eq!(vec_elem.as_slice(), *slice_elem);
        }
    }

    // ---------------------------------------------------------------
    // vote_contested_resource_active_polls_contract_tree_path / _vec
    // ---------------------------------------------------------------

    #[test]
    fn test_active_polls_contract_tree_path_includes_contract_id() {
        let contract_id: [u8; 32] = [42u8; 32];
        let path = vote_contested_resource_active_polls_contract_tree_path(&contract_id);
        assert_eq!(path.len(), 4);
        assert_eq!(path[0], &[VOTES_BYTE]);
        assert_eq!(path[1], &[CONTESTED_RESOURCE_TREE_KEY as u8]);
        assert_eq!(path[2], &[ACTIVE_POLLS_TREE_KEY as u8]);
        assert_eq!(path[3], &contract_id);
    }

    #[test]
    fn test_active_polls_contract_tree_path_vec_matches_slice_form() {
        let contract_id: [u8; 32] = [7u8; 32];
        let path_vec = vote_contested_resource_active_polls_contract_tree_path_vec(&contract_id);
        let path = vote_contested_resource_active_polls_contract_tree_path(&contract_id);
        assert_eq!(path_vec.len(), path.len());
        for (vec_elem, slice_elem) in path_vec.iter().zip(path.iter()) {
            assert_eq!(vec_elem.as_slice(), *slice_elem);
        }
    }

    #[test]
    fn test_active_polls_contract_tree_path_different_ids_differ() {
        let id_a: [u8; 32] = [1u8; 32];
        let id_b: [u8; 32] = [2u8; 32];
        let path_a = vote_contested_resource_active_polls_contract_tree_path(&id_a);
        let path_b = vote_contested_resource_active_polls_contract_tree_path(&id_b);
        // First three path elements should be the same
        assert_eq!(path_a[0], path_b[0]);
        assert_eq!(path_a[1], path_b[1]);
        assert_eq!(path_a[2], path_b[2]);
        // Contract ID (4th element) should differ
        assert_ne!(path_a[3], path_b[3]);
    }

    // ---------------------------------------------------------------
    // vote_contested_resource_active_polls_contract_document_tree_path / _vec
    // ---------------------------------------------------------------

    #[test]
    fn test_active_polls_contract_document_tree_path_structure() {
        let contract_id: [u8; 32] = [10u8; 32];
        let doc_type_name = "domain";
        let path = vote_contested_resource_active_polls_contract_document_tree_path(
            &contract_id,
            doc_type_name,
        );
        assert_eq!(path.len(), 5);
        assert_eq!(path[0], &[VOTES_BYTE]);
        assert_eq!(path[1], &[CONTESTED_RESOURCE_TREE_KEY as u8]);
        assert_eq!(path[2], &[ACTIVE_POLLS_TREE_KEY as u8]);
        assert_eq!(path[3], &contract_id);
        assert_eq!(path[4], doc_type_name.as_bytes());
    }

    #[test]
    fn test_active_polls_contract_document_tree_path_vec_matches_slice_form() {
        let contract_id: [u8; 32] = [99u8; 32];
        let doc_type_name = "preorder";
        let path_vec = vote_contested_resource_active_polls_contract_document_tree_path_vec(
            &contract_id,
            doc_type_name,
        );
        let path = vote_contested_resource_active_polls_contract_document_tree_path(
            &contract_id,
            doc_type_name,
        );
        assert_eq!(path_vec.len(), path.len());
        for (vec_elem, slice_elem) in path_vec.iter().zip(path.iter()) {
            assert_eq!(vec_elem.as_slice(), *slice_elem);
        }
    }

    #[test]
    fn test_active_polls_contract_document_tree_path_different_doc_types() {
        let contract_id: [u8; 32] = [5u8; 32];
        let path_a =
            vote_contested_resource_active_polls_contract_document_tree_path(&contract_id, "alpha");
        let path_b =
            vote_contested_resource_active_polls_contract_document_tree_path(&contract_id, "beta");
        // Same prefix
        assert_eq!(path_a[..4], path_b[..4]);
        // Different document type name
        assert_ne!(path_a[4], path_b[4]);
    }

    // ---------------------------------------------------------------
    // vote_contested_resource_contract_documents_storage_path / _vec
    // ---------------------------------------------------------------

    #[test]
    fn test_contract_documents_storage_path_has_storage_key() {
        let contract_id: [u8; 32] = [11u8; 32];
        let doc_type_name = "note";
        let path =
            vote_contested_resource_contract_documents_storage_path(&contract_id, doc_type_name);
        assert_eq!(path.len(), 6);
        assert_eq!(path[5], &[CONTESTED_DOCUMENT_STORAGE_TREE_KEY]);
    }

    #[test]
    fn test_contract_documents_storage_path_vec_matches_slice_form() {
        let contract_id: [u8; 32] = [11u8; 32];
        let doc_type_name = "note";
        let path_vec = vote_contested_resource_contract_documents_storage_path_vec(
            &contract_id,
            doc_type_name,
        );
        let path =
            vote_contested_resource_contract_documents_storage_path(&contract_id, doc_type_name);
        assert_eq!(path_vec.len(), path.len());
        for (vec_elem, slice_elem) in path_vec.iter().zip(path.iter()) {
            assert_eq!(vec_elem.as_slice(), *slice_elem);
        }
    }

    // ---------------------------------------------------------------
    // vote_contested_resource_contract_documents_indexes_path / _vec
    // ---------------------------------------------------------------

    #[test]
    fn test_contract_documents_indexes_path_has_indexes_key() {
        let contract_id: [u8; 32] = [22u8; 32];
        let doc_type_name = "profile";
        let path =
            vote_contested_resource_contract_documents_indexes_path(&contract_id, doc_type_name);
        assert_eq!(path.len(), 6);
        assert_eq!(path[5], &[CONTESTED_DOCUMENT_INDEXES_TREE_KEY]);
    }

    #[test]
    fn test_contract_documents_indexes_path_vec_matches_slice_form() {
        let contract_id: [u8; 32] = [22u8; 32];
        let doc_type_name = "profile";
        let path_vec = vote_contested_resource_contract_documents_indexes_path_vec(
            &contract_id,
            doc_type_name,
        );
        let path =
            vote_contested_resource_contract_documents_indexes_path(&contract_id, doc_type_name);
        assert_eq!(path_vec.len(), path.len());
        for (vec_elem, slice_elem) in path_vec.iter().zip(path.iter()) {
            assert_eq!(vec_elem.as_slice(), *slice_elem);
        }
    }

    #[test]
    fn test_storage_and_indexes_paths_differ_only_in_last_element() {
        let contract_id: [u8; 32] = [33u8; 32];
        let doc_type_name = "record";
        let storage_path =
            vote_contested_resource_contract_documents_storage_path(&contract_id, doc_type_name);
        let indexes_path =
            vote_contested_resource_contract_documents_indexes_path(&contract_id, doc_type_name);
        // The first five elements should be identical
        assert_eq!(storage_path[..5], indexes_path[..5]);
        // The 6th element (tree key) should differ
        assert_ne!(storage_path[5], indexes_path[5]);
        assert_eq!(storage_path[5], &[CONTESTED_DOCUMENT_STORAGE_TREE_KEY]);
        assert_eq!(indexes_path[5], &[CONTESTED_DOCUMENT_INDEXES_TREE_KEY]);
    }

    // ---------------------------------------------------------------
    // vote_contested_resource_end_date_queries_at_time_tree_path_vec
    // ---------------------------------------------------------------

    #[test]
    fn test_end_date_queries_at_time_tree_path_vec_structure() {
        let time: TimestampMillis = 1_700_000_000_000;
        let path = vote_contested_resource_end_date_queries_at_time_tree_path_vec(time);
        assert_eq!(path.len(), 3);
        assert_eq!(path[0], vec![VOTES_BYTE]);
        assert_eq!(path[1], vec![END_DATE_QUERIES_TREE_KEY as u8]);
        assert_eq!(path[2], encode_u64(time));
    }

    #[test]
    fn test_end_date_queries_different_times_produce_different_paths() {
        let path_a = vote_contested_resource_end_date_queries_at_time_tree_path_vec(1_000_000);
        let path_b = vote_contested_resource_end_date_queries_at_time_tree_path_vec(2_000_000);
        // First two elements should be identical
        assert_eq!(path_a[0], path_b[0]);
        assert_eq!(path_a[1], path_b[1]);
        // Time-encoded element should differ
        assert_ne!(path_a[2], path_b[2]);
    }

    #[test]
    fn test_end_date_queries_at_time_zero() {
        let path = vote_contested_resource_end_date_queries_at_time_tree_path_vec(0);
        assert_eq!(path.len(), 3);
        assert_eq!(path[2], encode_u64(0));
    }

    #[test]
    fn test_end_date_queries_at_time_max() {
        let path = vote_contested_resource_end_date_queries_at_time_tree_path_vec(u64::MAX);
        assert_eq!(path.len(), 3);
        assert_eq!(path[2], encode_u64(u64::MAX));
    }

    // ---------------------------------------------------------------
    // vote_contested_resource_identity_votes_tree_path / _vec
    // ---------------------------------------------------------------

    #[test]
    fn test_identity_votes_tree_path_structure() {
        let path = vote_contested_resource_identity_votes_tree_path();
        assert_eq!(path.len(), 3);
        assert_eq!(path[0], &[VOTES_BYTE]);
        assert_eq!(path[1], &[CONTESTED_RESOURCE_TREE_KEY as u8]);
        assert_eq!(path[2], &[IDENTITY_VOTES_TREE_KEY as u8]);
    }

    #[test]
    fn test_identity_votes_tree_path_vec_matches_slice_form() {
        let path_vec = vote_contested_resource_identity_votes_tree_path_vec();
        let path = vote_contested_resource_identity_votes_tree_path();
        assert_eq!(path_vec.len(), path.len());
        for (vec_elem, slice_elem) in path_vec.iter().zip(path.iter()) {
            assert_eq!(vec_elem.as_slice(), *slice_elem);
        }
    }

    // ---------------------------------------------------------------
    // vote_contested_resource_identity_votes_tree_path_for_identity / _vec
    // ---------------------------------------------------------------

    #[test]
    fn test_identity_votes_tree_path_for_identity_includes_id() {
        let identity_id: [u8; 32] = [55u8; 32];
        let path = vote_contested_resource_identity_votes_tree_path_for_identity(&identity_id);
        assert_eq!(path.len(), 4);
        assert_eq!(path[0], &[VOTES_BYTE]);
        assert_eq!(path[1], &[CONTESTED_RESOURCE_TREE_KEY as u8]);
        assert_eq!(path[2], &[IDENTITY_VOTES_TREE_KEY as u8]);
        assert_eq!(path[3], &identity_id);
    }

    #[test]
    fn test_identity_votes_tree_path_for_identity_vec_matches_slice_form() {
        let identity_id: [u8; 32] = [88u8; 32];
        let path_vec =
            vote_contested_resource_identity_votes_tree_path_for_identity_vec(&identity_id);
        let path = vote_contested_resource_identity_votes_tree_path_for_identity(&identity_id);
        assert_eq!(path_vec.len(), path.len());
        for (vec_elem, slice_elem) in path_vec.iter().zip(path.iter()) {
            assert_eq!(vec_elem.as_slice(), *slice_elem);
        }
    }

    #[test]
    fn test_identity_votes_tree_path_for_identity_different_ids_differ() {
        let id_a: [u8; 32] = [0u8; 32];
        let id_b: [u8; 32] = [255u8; 32];
        let path_a = vote_contested_resource_identity_votes_tree_path_for_identity(&id_a);
        let path_b = vote_contested_resource_identity_votes_tree_path_for_identity(&id_b);
        // Prefix should be the same
        assert_eq!(path_a[..3], path_b[..3]);
        // Identity ID should differ
        assert_ne!(path_a[3], path_b[3]);
    }

    // ---------------------------------------------------------------
    // Constant key values
    // ---------------------------------------------------------------

    #[test]
    fn test_resource_stored_info_key_is_all_zeroes() {
        assert_eq!(RESOURCE_STORED_INFO_KEY_U8_32, [0u8; 32]);
    }

    #[test]
    fn test_resource_abstain_vote_key_is_one() {
        let mut expected = [0u8; 32];
        expected[31] = 1;
        assert_eq!(RESOURCE_ABSTAIN_VOTE_TREE_KEY_U8_32, expected);
    }

    #[test]
    fn test_resource_lock_vote_key_is_two() {
        let mut expected = [0u8; 32];
        expected[31] = 2;
        assert_eq!(RESOURCE_LOCK_VOTE_TREE_KEY_U8_32, expected);
    }

    #[test]
    fn test_tree_key_constants_are_distinct() {
        assert_ne!(VOTE_DECISIONS_TREE_KEY, CONTESTED_RESOURCE_TREE_KEY);
        assert_ne!(VOTE_DECISIONS_TREE_KEY, END_DATE_QUERIES_TREE_KEY);
        assert_ne!(CONTESTED_RESOURCE_TREE_KEY, END_DATE_QUERIES_TREE_KEY);
        assert_ne!(ACTIVE_POLLS_TREE_KEY, IDENTITY_VOTES_TREE_KEY);
    }

    #[test]
    fn test_document_tree_keys_are_distinct() {
        assert_ne!(
            CONTESTED_DOCUMENT_STORAGE_TREE_KEY,
            CONTESTED_DOCUMENT_INDEXES_TREE_KEY
        );
    }

    // ---------------------------------------------------------------
    // Paths share a common prefix where expected
    // ---------------------------------------------------------------

    #[test]
    fn test_active_polls_and_identity_votes_share_contested_resource_prefix() {
        let active = vote_contested_resource_active_polls_tree_path();
        let identity = vote_contested_resource_identity_votes_tree_path();
        // Both share votes root + contested resource key
        assert_eq!(active[0], identity[0]);
        assert_eq!(active[1], identity[1]);
        // But diverge at third element
        assert_ne!(active[2], identity[2]);
    }

    #[test]
    fn test_active_polls_path_is_prefix_of_contract_path() {
        let polls_path = vote_contested_resource_active_polls_tree_path();
        let contract_id: [u8; 32] = [99u8; 32];
        let contract_path = vote_contested_resource_active_polls_contract_tree_path(&contract_id);
        // Contract path starts with the same elements as polls path
        for (i, &elem) in polls_path.iter().enumerate() {
            assert_eq!(contract_path[i], elem);
        }
    }
}
