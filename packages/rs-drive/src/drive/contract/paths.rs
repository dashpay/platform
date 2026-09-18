use crate::drive::RootTree;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::config::moderation::ContractModerationList;

use crate::drive::votes::paths::{ACTIVE_POLLS_TREE_KEY, CONTESTED_RESOURCE_TREE_KEY};
use dpp::data_contract::DataContract;

/// The various GroveDB paths underneath a contract
pub trait DataContractPaths {
    /// The root path, under this there should be the documents area and the contract itself
    fn root_path(&self) -> [&[u8]; 2];
    /// The documents path, under this you should have the various document types
    fn documents_path(&self) -> [&[u8]; 3];
    /// The document type path, this is based on the document type name
    fn document_type_path<'a>(&'a self, document_type_name: &'a str) -> [&'a [u8]; 4];
    /// The contested document type path, this is based on the document type name
    fn contested_document_type_path<'a>(&'a self, document_type_name: &'a str) -> [&'a [u8]; 5];
    /// The document primary key path, this is under the document type
    fn documents_primary_key_path<'a>(&'a self, document_type_name: &'a str) -> [&'a [u8]; 5];
    /// The contested document primary key path, this is under the document type
    fn contested_documents_primary_key_path<'a>(
        &'a self,
        document_type_name: &'a str,
    ) -> [&'a [u8]; 6];
    /// The underlying storage for documents that keep history
    fn documents_with_history_primary_key_path<'a>(
        &'a self,
        document_type_name: &'a str,
        id: &'a [u8],
    ) -> [&'a [u8]; 6];
}

impl DataContractPaths for DataContract {
    fn root_path(&self) -> [&[u8]; 2] {
        [
            Into::<&[u8; 1]>::into(RootTree::DataContractDocuments),
            self.id_ref().as_bytes(),
        ]
    }

    fn documents_path(&self) -> [&[u8]; 3] {
        [
            Into::<&[u8; 1]>::into(RootTree::DataContractDocuments),
            self.id_ref().as_bytes(),
            &[1],
        ]
    }

    fn document_type_path<'a>(&'a self, document_type_name: &'a str) -> [&'a [u8]; 4] {
        [
            Into::<&[u8; 1]>::into(RootTree::DataContractDocuments),
            self.id_ref().as_bytes(),
            &[1],
            document_type_name.as_bytes(),
        ]
    }

    fn contested_document_type_path<'a>(&'a self, document_type_name: &'a str) -> [&'a [u8]; 5] {
        [
            Into::<&[u8; 1]>::into(RootTree::Votes), // 1
            &[CONTESTED_RESOURCE_TREE_KEY as u8],    // 1
            &[ACTIVE_POLLS_TREE_KEY as u8],          // 1
            self.id_ref().as_bytes(),                // 32
            document_type_name.as_bytes(),
        ]
    }

    fn documents_primary_key_path<'a>(&'a self, document_type_name: &'a str) -> [&'a [u8]; 5] {
        [
            Into::<&[u8; 1]>::into(RootTree::DataContractDocuments),
            self.id_ref().as_bytes(),
            &[1],
            document_type_name.as_bytes(),
            &[0],
        ]
    }

    fn contested_documents_primary_key_path<'a>(
        &'a self,
        document_type_name: &'a str,
    ) -> [&'a [u8]; 6] {
        [
            Into::<&[u8; 1]>::into(RootTree::Votes), // 1
            &[CONTESTED_RESOURCE_TREE_KEY as u8],    // 1
            &[ACTIVE_POLLS_TREE_KEY as u8],          // 1
            self.id_ref().as_bytes(),                // 32
            document_type_name.as_bytes(),
            &[0],
        ]
    }

    fn documents_with_history_primary_key_path<'a>(
        &'a self,
        document_type_name: &'a str,
        id: &'a [u8],
    ) -> [&'a [u8]; 6] {
        [
            Into::<&[u8; 1]>::into(RootTree::DataContractDocuments),
            self.id_ref().as_bytes(),
            &[1],
            document_type_name.as_bytes(),
            &[0],
            id,
        ]
    }
}

/// The global root path for all contracts
pub fn all_contracts_global_root_path() -> [&'static [u8]; 1] {
    [Into::<&[u8; 1]>::into(RootTree::DataContractDocuments)]
}

/// Takes a contract ID and returns the contract's root path.
pub fn contract_root_path(contract_id: &[u8]) -> [&[u8]; 2] {
    [
        Into::<&[u8; 1]>::into(RootTree::DataContractDocuments),
        contract_id,
    ]
}

/// Takes a contract ID and returns the contract's root path.
pub fn contract_root_path_vec(contract_id: &[u8]) -> Vec<Vec<u8>> {
    vec![
        Into::<&[u8; 1]>::into(RootTree::DataContractDocuments).to_vec(),
        contract_id.to_vec(),
    ]
}

/// Takes a contract ID and returns the contract's storage path (where it is stored).
pub fn contract_storage_path_vec(contract_id: &[u8]) -> Vec<Vec<u8>> {
    vec![
        Into::<&[u8; 1]>::into(RootTree::DataContractDocuments).to_vec(),
        contract_id.to_vec(),
        vec![0],
    ]
}

/// Takes a contract ID and returns the contract's storage history path.
pub fn contract_keeping_history_root_path_vec(contract_id: &[u8]) -> Vec<Vec<u8>> {
    vec![
        Into::<&[u8; 1]>::into(RootTree::DataContractDocuments).to_vec(),
        contract_id.to_vec(),
        vec![0],
    ]
}

/// Takes a contract ID and returns the contract's storage history path.
pub fn contract_keeping_history_root_path(contract_id: &[u8]) -> [&[u8]; 3] {
    [
        Into::<&[u8; 1]>::into(RootTree::DataContractDocuments),
        contract_id,
        &[0],
    ]
}

/// Takes a contract ID and an encoded timestamp and returns the contract's storage history path
/// for that timestamp.
pub fn contract_keeping_history_storage_time_reference_path(
    contract_id: &[u8],
    encoded_time: Vec<u8>,
) -> Vec<Vec<u8>> {
    vec![
        Into::<&[u8; 1]>::into(RootTree::DataContractDocuments).to_vec(),
        contract_id.to_vec(),
        vec![0],
        encoded_time,
    ]
}

/// The key under a contract's root subtree (`[64, id]`) that holds the contract's version
/// number as a four-byte big-endian item, written beside the contract from protocol
/// version 14. Keys `0` (the contract, or its history subtree) and `1` (the documents) are
/// the other children of that subtree, and a moderated contract also has `3` (its banlist)
/// and `4` (its suspension list).
pub const CONTRACT_VERSION_KEY: u8 = 2;

/// The key under a contract's root subtree (`[64, id]`) of the banlist a moderated contract
/// keeps (protocol version 14): `identity id -> Item([])`. Present only when the contract's
/// config declares a banlist.
pub const CONTRACT_BANLIST_KEY: u8 = 3;

/// The key under a contract's root subtree (`[64, id]`) of the suspension list a moderated
/// contract keeps (protocol version 14): `identity id -> Item(until, u64 big-endian
/// milliseconds)`. Present only when the contract's config declares a suspension list.
pub const CONTRACT_SUSPENSIONS_KEY: u8 = 4;

/// The tree key of a moderation list.
pub fn contract_moderation_list_key(list: ContractModerationList) -> &'static [u8; 1] {
    match list {
        ContractModerationList::Banlist => &[CONTRACT_BANLIST_KEY],
        ContractModerationList::Suspensions => &[CONTRACT_SUSPENSIONS_KEY],
    }
}

/// `[64, contract id, 3]` or `[64, contract id, 4]`: the tree of one moderation list.
pub fn contract_moderation_list_path(
    contract_id: &[u8],
    list: ContractModerationList,
) -> [&[u8]; 3] {
    [
        Into::<&[u8; 1]>::into(RootTree::DataContractDocuments),
        contract_id,
        contract_moderation_list_key(list),
    ]
}

/// `[64, contract id, 3]` or `[64, contract id, 4]`: the tree of one moderation list.
pub fn contract_moderation_list_path_vec(
    contract_id: &[u8],
    list: ContractModerationList,
) -> Vec<Vec<u8>> {
    vec![
        Into::<&[u8; 1]>::into(RootTree::DataContractDocuments).to_vec(),
        contract_id.to_vec(),
        contract_moderation_list_key(list).to_vec(),
    ]
}
