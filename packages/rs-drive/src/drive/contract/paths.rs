use crate::drive::RootTree;
use dpp::data_contract::accessors::v0::DataContractV0Getters;

use dpp::data_contract::DataContract;

/// The various GroveDB paths underneath a contract
pub trait DataContractPaths {
    /// The root path, under this there should be the documents area and the contract itself
    fn root_path(&self) -> [&[u8]; 2];
    /// The documents path, under this you should have the various document types
    fn documents_path(&self) -> [&[u8]; 3];
    /// The document type path, this is based on the document type name
    fn document_type_path<'a>(&'a self, document_type_name: &'a str) -> [&'a [u8]; 4];
    /// The document primary key path, this is under the document type
    fn documents_primary_key_path<'a>(&'a self, document_type_name: &'a str) -> [&'a [u8]; 5];
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

    fn documents_primary_key_path<'a>(&'a self, document_type_name: &'a str) -> [&'a [u8]; 5] {
        [
            Into::<&[u8; 1]>::into(RootTree::DataContractDocuments),
            self.id_ref().as_bytes(),
            &[1],
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
