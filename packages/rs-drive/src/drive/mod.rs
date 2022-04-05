pub mod defaults;
pub mod object_size_info;

use crate::contract::{Contract, Document, DocumentType};
use crate::drive::defaults::{CONTRACT_DOCUMENTS_PATH_HEIGHT, DEFAULT_HASH_SIZE};
use crate::drive::object_size_info::DocumentInfo::{DocumentAndSerialization, DocumentSize};
use crate::drive::object_size_info::KeyElementInfo::{KeyElement, KeyElementSize};
use crate::drive::object_size_info::KeyInfo::{Key, KeyRef, KeySize};
use crate::drive::object_size_info::PathInfo::PathSize;
use crate::drive::object_size_info::PathKeyElementInfo::{
    PathFixedSizeKeyElement, PathKeyElement, PathKeyElementSize,
};
use crate::drive::object_size_info::PathKeyInfo::{PathFixedSizeKeyRef, PathKeyRef, PathKeySize};
use crate::drive::object_size_info::{
    DocumentAndContractInfo, DocumentInfo, KeyElementInfo, KeyInfo, PathInfo, PathKeyElementInfo,
    PathKeyInfo,
};
use crate::fee::calculate_fee;
use crate::fee::op::{BaseOp, DeleteOperation, InsertOperation, QueryOperation};
use crate::query::DriveQuery;
use enum_map::EnumMap;
use grovedb::{Element, Error, GroveDb, TransactionArg};
use std::collections::HashMap;
use std::path::Path;

pub struct Drive {
    pub grove: GroveDb,
}

#[repr(u8)]
pub enum RootTree {
    // Input data errors
    Identities = 0,
    ContractDocuments = 1,
    PublicKeyHashesToIdentities = 2,
    Misc = 3,
}

pub const STORAGE_COST: i32 = 50;

impl From<RootTree> for u8 {
    fn from(root_tree: RootTree) -> Self {
        root_tree as u8
    }
}

impl From<RootTree> for [u8; 1] {
    fn from(root_tree: RootTree) -> Self {
        [root_tree as u8]
    }
}

impl From<RootTree> for &'static [u8; 1] {
    fn from(root_tree: RootTree) -> Self {
        match root_tree {
            RootTree::Identities => &[0],
            RootTree::ContractDocuments => &[1],
            RootTree::PublicKeyHashesToIdentities => &[2],
            RootTree::Misc => &[3],
        }
    }
}

fn contract_root_path(contract_id: &[u8]) -> [&[u8]; 2] {
    [
        Into::<&[u8; 1]>::into(RootTree::ContractDocuments),
        contract_id,
    ]
}

fn contract_keeping_history_storage_path(contract_id: &[u8]) -> [&[u8]; 3] {
    [
        Into::<&[u8; 1]>::into(RootTree::ContractDocuments),
        contract_id,
        &[0],
    ]
}

fn contract_keeping_history_storage_time_reference_path(
    contract_id: &[u8],
    encoded_time: Vec<u8>,
) -> Vec<Vec<u8>> {
    vec![
        Into::<&[u8; 1]>::into(RootTree::ContractDocuments).to_vec(),
        contract_id.to_vec(),
        vec![0],
        encoded_time,
    ]
}

fn contract_documents_path(contract_id: &[u8]) -> [&[u8]; 3] {
    [
        Into::<&[u8; 1]>::into(RootTree::ContractDocuments),
        contract_id,
        &[1],
    ]
}

fn contract_document_type_path<'a>(
    contract_id: &'a [u8],
    document_type_name: &'a str,
) -> [&'a [u8]; 4] {
    [
        Into::<&[u8; 1]>::into(RootTree::ContractDocuments),
        contract_id,
        &[1],
        document_type_name.as_bytes(),
    ]
}

fn contract_documents_primary_key_path<'a>(
    contract_id: &'a [u8],
    document_type_name: &'a str,
) -> [&'a [u8]; 5] {
    [
        Into::<&[u8; 1]>::into(RootTree::ContractDocuments), // 1
        contract_id,                                         // 32
        &[1],                                                // 1
        document_type_name.as_bytes(),
        &[0], // 1
    ]
}

fn contract_documents_keeping_history_primary_key_path_for_document_id<'a>(
    contract_id: &'a [u8],
    document_type_name: &'a str,
    document_id: &'a [u8],
) -> [&'a [u8]; 6] {
    [
        Into::<&[u8; 1]>::into(RootTree::ContractDocuments),
        contract_id,
        &[1],
        document_type_name.as_bytes(),
        &[0],
        document_id,
    ]
}

fn contract_documents_keeping_history_primary_key_path_for_document_id_size(
    document_type_name_len: usize,
) -> usize {
    crate::drive::defaults::BASE_CONTRACT_DOCUMENTS_KEEPING_HISTORY_PRIMARY_KEY_PATH_FOR_DOCUMENT_ID_SIZE + document_type_name_len
}

fn contract_documents_keeping_history_storage_time_reference_path(
    contract_id: &[u8],
    document_type_name: &str,
    document_id: &[u8],
    encoded_time: Vec<u8>,
) -> Vec<Vec<u8>> {
    vec![
        Into::<&[u8; 1]>::into(RootTree::ContractDocuments).to_vec(), // 1 byte
        contract_id.to_vec(),                                         // 32 bytes
        vec![1],                                                      // 1
        document_type_name.as_bytes().to_vec(),
        vec![0],              // 1
        document_id.to_vec(), // 32 bytes
        encoded_time,         // 8 bytes
    ]
}

fn contract_documents_keeping_history_storage_time_reference_path_size(
    document_type_name_len: usize,
) -> usize {
    crate::drive::defaults::BASE_CONTRACT_DOCUMENTS_KEEPING_HISTORY_STORAGE_TIME_REFERENCE_PATH
        + document_type_name_len
}

impl Drive {
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self, Error> {
        match GroveDb::open(path) {
            Ok(grove) => Ok(Drive { grove }),
            Err(e) => Err(e),
        }
    }

    pub const fn check_protocol_version(_version: u32) -> bool {
        // Temporary disabled due protocol version is dynamic and goes from consensus params
        true
    }

    pub fn check_protocol_version_bytes(version_bytes: &[u8]) -> bool {
        if version_bytes.len() != 4 {
            false
        } else {
            let version_set_bytes: [u8; 4] = version_bytes
                .try_into()
                .expect("slice with incorrect length");
            let version = u32::from_be_bytes(version_set_bytes);
            Drive::check_protocol_version(version)
        }
    }

    pub fn create_root_tree(&self, transaction: TransactionArg) -> Result<(), Error> {
        self.grove.insert(
            [],
            Into::<&[u8; 1]>::into(RootTree::Identities),
            Element::empty_tree(),
            transaction,
        )?;
        self.grove.insert(
            [],
            Into::<&[u8; 1]>::into(RootTree::ContractDocuments),
            Element::empty_tree(),
            transaction,
        )?;
        self.grove.insert(
            [],
            Into::<&[u8; 1]>::into(RootTree::PublicKeyHashesToIdentities),
            Element::empty_tree(),
            transaction,
        )?;
        self.grove.insert(
            [],
            Into::<&[u8; 1]>::into(RootTree::Misc),
            Element::empty_tree(),
            transaction,
        )?;
        Ok(())
    }

    fn grove_insert_empty_tree<'a, 'c, P>(
        &'a self,
        path: P,
        key_info: KeyInfo<'c>,
        transaction: TransactionArg,
        insert_operations: &mut Vec<InsertOperation>,
    ) -> Result<(), Error>
    where
        P: IntoIterator<Item = &'c [u8]>,
        <P as IntoIterator>::IntoIter: ExactSizeIterator + DoubleEndedIterator + Clone,
    {
        match key_info {
            KeyInfo::KeyRef(key) => {
                insert_operations.push(InsertOperation::for_empty_tree(key.len()));
                self.grove
                    .insert(path, key, Element::empty_tree(), transaction)
            }
            KeyInfo::KeySize(key_max_length) => {
                insert_operations.push(InsertOperation::for_empty_tree(key_max_length));
                Ok(())
            }
            KeyInfo::Key(key) => Err(Error::InternalError(
                "only a key ref can be inserted into groveDB",
            )),
        }
    }

    fn grove_insert_empty_tree_if_not_exists<'a, 'c, const N: usize>(
        &'a self,
        path_key_info: PathKeyInfo<'c, N>,
        transaction: TransactionArg,
        query_operations: &mut Vec<QueryOperation>,
        insert_operations: &mut Vec<InsertOperation>,
    ) -> Result<bool, Error> {
        match path_key_info {
            PathKeyInfo::PathKeyRef((path, key)) => {
                let path = path.iter().map(|x| x.as_slice());
                let inserted = self.grove.insert_if_not_exists(
                    path.clone(),
                    key,
                    Element::empty_tree(),
                    transaction,
                )?;
                if inserted {
                    insert_operations.push(InsertOperation::for_empty_tree(key.len()));
                }
                query_operations.push(QueryOperation::for_key_check_in_path(key.len(), path));
                Ok(inserted)
            }
            PathKeyInfo::PathKeySize((path_max_length, key_max_length)) => {
                insert_operations.push(InsertOperation::for_empty_tree(key_max_length));
                query_operations.push(QueryOperation::for_key_check_with_path_length(
                    key_max_length,
                    path_max_length,
                ));
                Ok(true)
            }
            PathKeyInfo::PathKey((path, key)) => {
                let path = path.iter().map(|x| x.as_slice());
                let inserted = self.grove.insert_if_not_exists(
                    path.clone(),
                    key.as_slice(),
                    Element::empty_tree(),
                    transaction,
                )?;
                if inserted {
                    insert_operations.push(InsertOperation::for_empty_tree(key.len()));
                }
                query_operations.push(QueryOperation::for_key_check_in_path(key.len(), path));
                Ok(inserted)
            }
            PathKeyInfo::PathFixedSizeKey((path, key)) => {
                let path = path.into_iter();
                let inserted = self.grove.insert_if_not_exists(
                    path.clone(),
                    key.as_slice(),
                    Element::empty_tree(),
                    transaction,
                )?;
                if inserted {
                    insert_operations.push(InsertOperation::for_empty_tree(key.len()));
                }
                query_operations.push(QueryOperation::for_key_check_in_path(key.len(), path));
                Ok(inserted)
            }
            PathKeyInfo::PathFixedSizeKeyRef((path, key)) => {
                let path = path.into_iter();
                let inserted = self.grove.insert_if_not_exists(
                    path.clone(),
                    key,
                    Element::empty_tree(),
                    transaction,
                )?;
                if inserted {
                    insert_operations.push(InsertOperation::for_empty_tree(key.len()));
                }
                query_operations.push(QueryOperation::for_key_check_in_path(key.len(), path));
                Ok(inserted)
            }
        }
    }

    fn grove_insert<'a, 'c, const N: usize>(
        &'a self,
        path_key_element_info: PathKeyElementInfo<'c, N>,
        transaction: TransactionArg,
        insert_operations: &mut Vec<InsertOperation>,
    ) -> Result<(), Error> {
        match path_key_element_info {
            PathKeyElement((path, key, element)) => {
                let path = path.iter().map(|x| x.as_slice());
                insert_operations.push(InsertOperation::for_key_value(key.len(), &element));
                self.grove.insert(path, key, element, transaction)
            }
            PathKeyElementSize((path_max_length, key_max_length, element_max_size)) => {
                insert_operations.push(InsertOperation::for_key_value_size(
                    key_max_length,
                    element_max_size,
                ));
                Ok(())
            }
            PathFixedSizeKeyElement((path, key, element)) => {
                insert_operations.push(InsertOperation::for_key_value(key.len(), &element));
                self.grove.insert(path, key, element, transaction)
            }
        }
    }

    fn grove_insert_if_not_exists<'a, 'c, const N: usize>(
        &'a self,
        path_key_element_info: PathKeyElementInfo<'c, N>,
        transaction: TransactionArg,
        query_operations: &mut Vec<QueryOperation>,
        insert_operations: &mut Vec<InsertOperation>,
    ) -> Result<bool, Error> {
        match path_key_element_info {
            PathKeyElement((path, key, element)) => {
                let path_iter = path.iter().map(|x| x.as_slice());
                let insert_operation = InsertOperation::for_key_value(key.len(), &element);
                let query_operation =
                    QueryOperation::for_key_check_in_path(key.len(), path_iter.clone());
                let inserted =
                    self.grove
                        .insert_if_not_exists(path_iter, key, element, transaction)?;
                if inserted {
                    insert_operations.push(insert_operation);
                }
                query_operations.push(query_operation);
                Ok(inserted)
            }
            PathKeyElementSize((path_size, key_max_length, element_max_size)) => {
                let insert_operation =
                    InsertOperation::for_key_value_size(key_max_length, element_max_size);
                let query_operation =
                    QueryOperation::for_key_check_with_path_length(key_max_length, path_size);
                insert_operations.push(insert_operation);
                query_operations.push(query_operation);
                Ok(true)
            }
            PathKeyElementInfo::PathFixedSizeKeyElement((path, key, element)) => {
                let path_iter = path.into_iter();
                let insert_operation = InsertOperation::for_key_value(key.len(), &element);
                let query_operation =
                    QueryOperation::for_key_check_in_path(key.len(), path_iter.clone());
                let inserted =
                    self.grove
                        .insert_if_not_exists(path_iter, key, element, transaction)?;
                if inserted {
                    insert_operations.push(insert_operation);
                }
                query_operations.push(query_operation);
                Ok(inserted)
            }
        }
    }

    fn grove_get<'a, 'c, P>(
        &'a self,
        path: P,
        key_info: KeyInfo<'c>,
        transaction: TransactionArg,
        query_operations: &mut Vec<QueryOperation>,
    ) -> Result<Option<Element>, Error>
    where
        P: IntoIterator<Item = &'c [u8]>,
        <P as IntoIterator>::IntoIter: ExactSizeIterator + DoubleEndedIterator + Clone,
    {
        let path_iter = path.into_iter();
        query_operations.push(QueryOperation::for_key_check_in_path(
            key_info.len(),
            path_iter.clone(),
        ));
        if let KeyRef(key) = key_info {
            Ok(Some(self.grove.get(path_iter, key, transaction)?))
        } else {
            Ok(None)
        }
    }

    fn add_contract_to_storage(
        &self,
        contract_bytes: Element,
        contract: &Contract,
        block_time: f64,
        transaction: TransactionArg,
        insert_operations: &mut Vec<InsertOperation>,
    ) -> Result<(), Error> {
        let contract_root_path = contract_root_path(&contract.id);
        if contract.keeps_history {
            self.grove_insert_empty_tree(
                contract_root_path,
                KeyRef(&[0]),
                transaction,
                insert_operations,
            )?;
            let encoded_time = crate::contract::types::encode_float(block_time)?;
            let contract_keeping_history_storage_path =
                contract_keeping_history_storage_path(&contract.id);
            self.grove_insert(
                PathFixedSizeKeyElement((
                    contract_keeping_history_storage_path,
                    encoded_time.as_slice(),
                    contract_bytes,
                )),
                transaction,
                insert_operations,
            )?;

            // we should also insert a reference at 0 to the current value
            let contract_storage_path =
                contract_keeping_history_storage_time_reference_path(&contract.id, encoded_time);
            self.grove_insert(
                PathFixedSizeKeyElement((
                    contract_keeping_history_storage_path,
                    &[0],
                    Element::Reference(contract_storage_path),
                )),
                transaction,
                insert_operations,
            )?;
        } else {
            // the contract is just stored at key 0
            self.grove_insert(
                PathFixedSizeKeyElement((contract_root_path, &[0], contract_bytes)),
                transaction,
                insert_operations,
            )?;
        }
        Ok(())
    }

    fn insert_contract(
        &self,
        contract_bytes: Element,
        contract: &Contract,
        block_time: f64,
        transaction: TransactionArg,
        insert_operations: &mut Vec<InsertOperation>,
    ) -> Result<(), Error> {
        self.grove_insert_empty_tree(
            [Into::<&[u8; 1]>::into(RootTree::ContractDocuments).as_slice()],
            KeyRef(contract.id.as_slice()),
            transaction,
            insert_operations,
        )?;

        self.add_contract_to_storage(
            contract_bytes,
            contract,
            block_time,
            transaction,
            insert_operations,
        )?;

        // the documents
        let contract_root_path = contract_root_path(&contract.id);
        self.grove_insert_empty_tree(
            contract_root_path,
            KeyRef(&[1]),
            transaction,
            insert_operations,
        )?;

        // next we should store each document type
        // right now we are referring them by name
        // toDo: change this to be a reference by index
        let contract_documents_path = contract_documents_path(&contract.id);

        for (type_key, document_type) in &contract.document_types {
            self.grove_insert_empty_tree(
                contract_documents_path,
                KeyRef(type_key.as_bytes()),
                transaction,
                insert_operations,
            )?;

            let type_path = [
                contract_documents_path[0],
                contract_documents_path[1],
                contract_documents_path[2],
                type_key.as_bytes(),
            ];

            // primary key tree
            self.grove_insert_empty_tree(type_path, KeyRef(&[0]), transaction, insert_operations)?;

            // for each type we should insert the indices that are top level
            for index in document_type.top_level_indices()? {
                // toDo: change this to be a reference by index
                self.grove_insert_empty_tree(
                    type_path,
                    KeyRef(index.name.as_bytes()),
                    transaction,
                    insert_operations,
                )?;
            }
        }

        Ok(())
    }

    fn update_contract(
        &self,
        contract_bytes: Element,
        contract: &Contract,
        original_contract: &Contract,
        block_time: f64,
        transaction: TransactionArg,
        query_operations: &mut Vec<QueryOperation>,
        insert_operations: &mut Vec<InsertOperation>,
    ) -> Result<(), Error> {
        if original_contract.readonly {
            return Err(Error::InternalError("contract is readonly"));
        }

        if contract.readonly {
            return Err(Error::InternalError(
                "contract can not be changed to readonly",
            ));
        }

        if contract.keeps_history ^ original_contract.keeps_history {
            return Err(Error::InternalError(
                "contract can not change whether it keeps history",
            ));
        }

        if contract.documents_keep_history_contract_default
            ^ original_contract.documents_keep_history_contract_default
        {
            return Err(Error::InternalError(
                "contract can not change the default of whether documents keeps history",
            ));
        }

        if contract.documents_mutable_contract_default
            ^ original_contract.documents_mutable_contract_default
        {
            return Err(Error::InternalError(
                "contract can not change the default of whether documents are mutable",
            ));
        }

        // this will override the previous contract if we do not keep history
        self.add_contract_to_storage(
            contract_bytes,
            contract,
            block_time,
            transaction,
            insert_operations,
        )?;

        let contract_documents_path = contract_documents_path(&contract.id);
        for (type_key, document_type) in &contract.document_types {
            let original_document_type = &original_contract.document_types.get(type_key);
            if let Some(original_document_type) = original_document_type {
                if original_document_type.documents_mutable ^ document_type.documents_mutable {
                    return Err(Error::InternalError(
                        "contract can not change whether a specific document type is mutable",
                    ));
                }
                if original_document_type.documents_keep_history
                    ^ document_type.documents_keep_history
                {
                    return Err(Error::InternalError(
                        "contract can not change whether a specific document type keeps history",
                    ));
                }

                let type_path = [
                    contract_documents_path[0],
                    contract_documents_path[1],
                    contract_documents_path[2],
                    type_key.as_bytes(),
                ];

                // for each type we should insert the indices that are top level
                for index in document_type.top_level_indices()? {
                    // toDo: we can save a little by only inserting on new indexes
                    self.grove_insert_empty_tree_if_not_exists(
                        PathFixedSizeKeyRef((type_path, index.name.as_bytes())),
                        transaction,
                        query_operations,
                        insert_operations,
                    )?;
                }
            } else {
                // We can just insert this directly because the original document type already exists
                self.grove_insert_empty_tree(
                    contract_documents_path,
                    KeyRef(type_key.as_bytes()),
                    transaction,
                    insert_operations,
                )?;

                let type_path = [
                    contract_documents_path[0],
                    contract_documents_path[1],
                    contract_documents_path[2],
                    type_key.as_bytes(),
                ];

                // primary key tree
                self.grove_insert_empty_tree(
                    type_path,
                    KeyRef(&[0]),
                    transaction,
                    insert_operations,
                )?;

                // for each type we should insert the indices that are top level
                for index in document_type.top_level_indices()? {
                    // toDo: change this to be a reference by index
                    self.grove_insert_empty_tree(
                        type_path,
                        KeyRef(index.name.as_bytes()),
                        transaction,
                        insert_operations,
                    )?;
                }
            }
        }

        Ok(())
    }

    pub fn apply_contract_cbor(
        &self,
        contract_cbor: Vec<u8>,
        contract_id: Option<[u8; 32]>,
        block_time: f64,
        transaction: TransactionArg,
    ) -> Result<(i64, u64), Error> {
        // first we need to deserialize the contract
        let contract = Contract::from_cbor(&contract_cbor, contract_id)?;
        self.apply_contract(&contract, contract_cbor, block_time, transaction)
    }

    pub fn apply_contract(
        &self,
        contract: &Contract,
        contract_serialization: Vec<u8>,
        block_time: f64,
        transaction: TransactionArg,
    ) -> Result<(i64, u64), Error> {
        let mut query_operations: Vec<QueryOperation> = vec![];
        let mut insert_operations: Vec<InsertOperation> = vec![];

        // overlying structure
        let mut already_exists = false;
        let mut original_contract_stored_data = vec![];

        if let Ok(Some(stored_element)) = self.grove_get(
            contract_root_path(&contract.id),
            KeyRef(&[0]),
            transaction,
            &mut query_operations,
        ) {
            already_exists = true;
            match stored_element {
                Element::Item(stored_contract_bytes) => {
                    if contract_serialization != stored_contract_bytes {
                        original_contract_stored_data = stored_contract_bytes;
                    }
                }
                _ => {
                    already_exists = false;
                }
            }
        };

        let contract_element = Element::Item(contract_serialization);

        if already_exists {
            if !original_contract_stored_data.is_empty() {
                let original_contract = Contract::from_cbor(&original_contract_stored_data, None)?;
                // if the contract is not mutable update_contract will return an error
                self.update_contract(
                    contract_element,
                    contract,
                    &original_contract,
                    block_time,
                    transaction,
                    &mut query_operations,
                    &mut insert_operations,
                )?;
            }
        } else {
            self.insert_contract(
                contract_element,
                contract,
                block_time,
                transaction,
                &mut insert_operations,
            )?;
        }
        let fees = calculate_fee(None, Some(query_operations), Some(insert_operations), None)?;
        Ok(fees)
    }

    // If a document isn't sent to this function then we are just calling to know the query and
    // insert operations
    fn add_document_to_primary_storage(
        &self,
        document_and_contract_info: &DocumentAndContractInfo,
        block_time: f64,
        insert_without_check: bool,
        transaction: TransactionArg,
        query_operations: &mut Vec<QueryOperation>,
        insert_operations: &mut Vec<InsertOperation>,
    ) -> Result<(), Error> {
        //let mut base_operations : EnumMap<Op, u64> = EnumMap::default();
        let contract = document_and_contract_info.contract;
        let document_type = document_and_contract_info.document_type;
        let primary_key_path =
            contract_documents_primary_key_path(&contract.id, document_type.name.as_str());
        if document_type.documents_keep_history {
            let path_key_info = if let DocumentAndSerialization((document, _)) =
                document_and_contract_info.document_info
            {
                PathFixedSizeKeyRef((primary_key_path, document.id.as_slice()))
            } else {
                PathKeySize((
                    crate::drive::defaults::BASE_CONTRACT_DOCUMENTS_PRIMARY_KEY_PATH
                        + document_type.name.len(),
                    DEFAULT_HASH_SIZE,
                ))
            };
            // we first insert an empty tree if the document is new
            self.grove_insert_empty_tree_if_not_exists(
                path_key_info,
                transaction,
                query_operations,
                insert_operations,
            )?;
            let encoded_time = crate::contract::types::encode_float(block_time)?;
            let path_key_element_info = match document_and_contract_info.document_info {
                DocumentAndSerialization((document, document_cbor)) => {
                    let document_id_in_primary_path =
                        contract_documents_keeping_history_primary_key_path_for_document_id(
                            &contract.id,
                            document_type.name.as_str(),
                            document.id.as_slice(),
                        );
                    PathFixedSizeKeyElement((
                        document_id_in_primary_path,
                        encoded_time.as_slice(),
                        Element::Item(Vec::from(document_cbor)),
                    ))
                }
                DocumentSize(max_size) => {
                    let path_max_length =
                        contract_documents_keeping_history_primary_key_path_for_document_id_size(
                            document_type.name.len(),
                        );
                    PathKeyElementSize((path_max_length, 8_usize, max_size))
                }
            };
            self.grove_insert(path_key_element_info, transaction, insert_operations)?;

            let path_key_element_info = if let DocumentAndSerialization((document, _)) =
                document_and_contract_info.document_info
            {
                // we should also insert a reference at 0 to the current value
                // todo: we could construct this only once
                let document_id_in_primary_path =
                    contract_documents_keeping_history_primary_key_path_for_document_id(
                        &contract.id,
                        document_type.name.as_str(),
                        document.id.as_slice(),
                    );
                let contract_storage_path =
                    contract_documents_keeping_history_storage_time_reference_path(
                        &contract.id,
                        document_type.name.as_str(),
                        document.id.as_slice(),
                        encoded_time,
                    );
                PathFixedSizeKeyElement((
                    document_id_in_primary_path,
                    &[0],
                    Element::Reference(contract_storage_path),
                ))
            } else {
                let path_max_length =
                    contract_documents_keeping_history_primary_key_path_for_document_id_size(
                        document_type.name.len(),
                    );
                let reference_max_size =
                    contract_documents_keeping_history_storage_time_reference_path_size(
                        document_type.name.len(),
                    );
                PathKeyElementSize((path_max_length, 1, reference_max_size))
            };

            self.grove_insert(path_key_element_info, transaction, insert_operations)?;
        } else if insert_without_check {
            let path_key_element_info = match document_and_contract_info.document_info {
                DocumentAndSerialization((document, document_cbor)) => PathFixedSizeKeyElement((
                    primary_key_path,
                    document.id.as_slice(),
                    Element::Item(Vec::from(document_cbor)),
                )),
                DocumentSize(max_size) => PathKeyElementSize((
                    crate::drive::defaults::BASE_CONTRACT_DOCUMENTS_PRIMARY_KEY_PATH
                        + document_type.name.len(),
                    DEFAULT_HASH_SIZE,
                    max_size,
                )),
            };
            self.grove_insert(path_key_element_info, transaction, insert_operations)?;
        } else {
            let path_key_element_info = match document_and_contract_info.document_info {
                DocumentAndSerialization((document, document_cbor)) => PathFixedSizeKeyElement((
                    primary_key_path,
                    document.id.as_slice(),
                    Element::Item(Vec::from(document_cbor)),
                )),
                DocumentSize(max_size) => PathKeyElementSize((
                    crate::drive::defaults::BASE_CONTRACT_DOCUMENTS_PRIMARY_KEY_PATH
                        + document_type.name.len(),
                    DEFAULT_HASH_SIZE,
                    max_size,
                )),
            };
            let inserted = self.grove_insert_if_not_exists(
                path_key_element_info,
                transaction,
                query_operations,
                insert_operations,
            )?;
            if !inserted {
                return Err(Error::CorruptedData(String::from("item already exists")));
            }
        }
        Ok(())
    }

    pub fn add_document(&self, _document_cbor: &[u8]) -> Result<(), Error> {
        todo!()
    }

    pub fn add_document_for_contract_cbor(
        &self,
        document_cbor: &[u8],
        contract_cbor: &[u8],
        document_type_name: &str,
        owner_id: Option<&[u8]>,
        override_document: bool,
        block_time: f64,
        transaction: TransactionArg,
    ) -> Result<(i64, u64), Error> {
        let contract = Contract::from_cbor(contract_cbor, None)?;

        let document = Document::from_cbor(document_cbor, None, owner_id)?;

        let document_info = DocumentAndSerialization((&document, document_cbor));

        let document_type = contract.document_type_for_name(document_type_name)?;

        self.add_document_for_contract(
            DocumentAndContractInfo {
                document_info,
                contract: &contract,
                document_type,
                owner_id,
            },
            override_document,
            block_time,
            transaction,
        )
    }

    pub fn add_document_cbor_for_contract(
        &self,
        document_cbor: &[u8],
        contract: &Contract,
        document_type_name: &str,
        owner_id: Option<&[u8]>,
        override_document: bool,
        block_time: f64,
        transaction: TransactionArg,
    ) -> Result<(i64, u64), Error> {
        let document = Document::from_cbor(document_cbor, None, owner_id)?;

        let document_info = DocumentAndSerialization((&document, document_cbor));

        let document_type = contract.document_type_for_name(document_type_name)?;

        self.add_document_for_contract(
            DocumentAndContractInfo {
                document_info,
                contract,
                document_type,
                owner_id,
            },
            override_document,
            block_time,
            transaction,
        )
    }

    pub fn add_document_for_contract(
        &self,
        document_and_contract_info: DocumentAndContractInfo,
        override_document: bool,
        block_time: f64,
        transaction: TransactionArg,
    ) -> Result<(i64, u64), Error> {
        let mut query_operations: Vec<QueryOperation> = vec![];
        let mut insert_operations: Vec<InsertOperation> = vec![];
        self.add_document_for_contract_operations(
            document_and_contract_info,
            override_document,
            block_time,
            transaction,
            &mut query_operations,
            &mut insert_operations,
        )?;
        let fees = calculate_fee(None, Some(query_operations), Some(insert_operations), None)?;
        Ok(fees)
    }

    fn add_document_for_contract_operations(
        &self,
        document_and_contract_info: DocumentAndContractInfo,
        override_document: bool,
        block_time: f64,
        transaction: TransactionArg,
        query_operations: &mut Vec<QueryOperation>,
        insert_operations: &mut Vec<InsertOperation>,
    ) -> Result<(), Error> {
        // second we need to construct the path for documents on the contract
        // the path is
        //  * Document and Contract root tree
        //  * Contract ID recovered from document
        //  * 0 to signify Documents and not Contract
        let contract_document_type_path = contract_document_type_path(
            &document_and_contract_info.contract.id,
            document_and_contract_info.document_type.name.as_str(),
        );

        let primary_key_path = contract_documents_primary_key_path(
            &document_and_contract_info.contract.id,
            document_and_contract_info.document_type.name.as_str(),
        );
        if override_document
            && self
                .grove_get(
                    primary_key_path,
                    document_and_contract_info.document_info.id_key_info(),
                    transaction,
                    query_operations,
                )
                .is_ok()
            && document_and_contract_info
                .document_info
                .is_document_and_serialization()
        {
            self.update_document_for_contract_operations(
                document_and_contract_info,
                block_time,
                transaction,
                query_operations,
                insert_operations,
            )?;
            return Ok(());
        } else {
            // if we have override_document set that means we already checked if it exists
            self.add_document_to_primary_storage(
                &document_and_contract_info,
                block_time,
                override_document,
                transaction,
                query_operations,
                insert_operations,
            )?;
        }
        // fourth we need to store a reference to the document for each index
        for index in &document_and_contract_info.document_type.indices {
            // at this point the contract path is to the contract documents
            // for each index the top index component will already have been added
            // when the contract itself was created
            let mut index_path: Vec<Vec<u8>> = contract_document_type_path
                .iter()
                .map(|&x| Vec::from(x))
                .collect();
            let top_index_property = index
                .properties
                .get(0)
                .ok_or_else(|| Error::CorruptedData(String::from("invalid contract indices")))?;
            index_path.push(Vec::from(top_index_property.name.as_bytes()));

            // with the example of the dashpay contract's first index
            // the index path is now something like Contracts/ContractID/Documents(1)/$ownerId
            let document_top_field = document_and_contract_info
                .document_info
                .get_raw_for_document_type(
                    &top_index_property.name,
                    document_and_contract_info.document_type,
                    document_and_contract_info.owner_id,
                )?
                .unwrap_or_default();

            // The zero will not matter here, because the PathKeyInfo is variable
            let path_key_info = document_top_field.clone().add_path::<0>(index_path.clone());

            // here we are inserting an empty tree that will have a subtree of all other index properties
            self.grove_insert_empty_tree_if_not_exists(
                path_key_info,
                transaction,
                query_operations,
                insert_operations,
            )?;

            let mut all_fields_null = document_top_field.is_empty();

            let mut index_path_info = if document_and_contract_info
                .document_info
                .is_document_and_serialization()
            {
                PathInfo::PathIterator::<0>(index_path)
            } else {
                PathInfo::PathSize(index_path.iter().map(|x| x.len()).sum())
            };

            // we push the actual value of the index path
            index_path_info.push(document_top_field)?;
            // the index path is now something like Contracts/ContractID/Documents(1)/$ownerId/<ownerId>

            for i in 1..index.properties.len() {
                let index_property = index.properties.get(i).ok_or_else(|| {
                    Error::CorruptedData(String::from("invalid contract indices"))
                })?;

                let index_property_key = KeyRef(index_property.name.as_bytes());

                let document_index_field = document_and_contract_info
                    .document_info
                    .get_raw_for_document_type(
                        &index_property.name,
                        document_and_contract_info.document_type,
                        document_and_contract_info.owner_id,
                    )?
                    .unwrap_or_default();

                let path_key_info = index_property_key
                    .clone()
                    .add_path_info(index_path_info.clone());

                // here we are inserting an empty tree that will have a subtree of all other index properties
                self.grove_insert_empty_tree_if_not_exists(
                    path_key_info,
                    transaction,
                    query_operations,
                    insert_operations,
                )?;

                index_path_info.push(index_property_key)?;

                // Iteration 1. the index path is now something like Contracts/ContractID/Documents(1)/$ownerId/<ownerId>/toUserId
                // Iteration 2. the index path is now something like Contracts/ContractID/Documents(1)/$ownerId/<ownerId>/toUserId/<ToUserId>/accountReference

                let path_key_info = document_index_field
                    .clone()
                    .add_path_info(index_path_info.clone());

                // here we are inserting an empty tree that will have a subtree of all other index properties
                self.grove_insert_empty_tree_if_not_exists(
                    path_key_info,
                    transaction,
                    query_operations,
                    insert_operations,
                )?;

                all_fields_null &= document_index_field.is_empty();

                // we push the actual value of the index path
                index_path_info.push(document_index_field)?;
                // Iteration 1. the index path is now something like Contracts/ContractID/Documents(1)/$ownerId/<ownerId>/toUserId/<ToUserId>/
                // Iteration 2. the index path is now something like Contracts/ContractID/Documents(1)/$ownerId/<ownerId>/toUserId/<ToUserId>/accountReference/<accountReference>
            }

            fn make_document_reference(
                primary_key_path: [&[u8]; 5],
                document: &Document,
                document_type: &DocumentType,
            ) -> Element {
                // we need to construct the reference to the original document
                let mut reference_path = primary_key_path
                    .iter()
                    .map(|x| x.to_vec())
                    .collect::<Vec<Vec<u8>>>();
                reference_path.push(Vec::from(document.id));
                if document_type.documents_keep_history {
                    reference_path.push(vec![0]);
                }
                Element::Reference(reference_path)
            }

            // unique indexes will be stored under key "0"
            // non unique indices should have a tree at key "0" that has all elements based off of primary key
            if !index.unique || all_fields_null {
                let key_path_info = KeyRef(&[0]);

                let path_key_info = key_path_info.add_path_info(index_path_info.clone());
                // here we are inserting an empty tree that will have a subtree of all other index properties
                self.grove_insert_empty_tree_if_not_exists(
                    path_key_info,
                    transaction,
                    query_operations,
                    insert_operations,
                )?;

                index_path_info.push(Key(vec![0]))?;

                let key_element_info = match &document_and_contract_info.document_info {
                    DocumentAndSerialization((document, _)) => {
                        let document_reference = make_document_reference(
                            primary_key_path,
                            document,
                            document_and_contract_info.document_type,
                        );
                        KeyElement((document.id.as_slice(), document_reference))
                    }
                    DocumentSize(max_size) => KeyElementSize((8_usize, *max_size)),
                };

                let path_key_element_info = PathKeyElementInfo::from_path_info_and_key_element(
                    index_path_info,
                    key_element_info,
                )?;

                // here we should return an error if the element already exists
                self.grove_insert(path_key_element_info, transaction, insert_operations)?;
            } else {
                let key_element_info = match &document_and_contract_info.document_info {
                    DocumentAndSerialization((document, _)) => {
                        let document_reference = make_document_reference(
                            primary_key_path,
                            document,
                            document_and_contract_info.document_type,
                        );
                        KeyElement((&[0], document_reference))
                    }
                    DocumentSize(max_size) => KeyElementSize((1, *max_size)),
                };

                let path_key_element_info = PathKeyElementInfo::from_path_info_and_key_element(
                    index_path_info,
                    key_element_info,
                )?;

                // here we should return an error if the element already exists
                let inserted = self.grove_insert_if_not_exists(
                    path_key_element_info,
                    transaction,
                    query_operations,
                    insert_operations,
                )?;
                if !inserted {
                    return Err(Error::CorruptedData(String::from("index already exists")));
                }
            }
        }
        Ok(())
    }

    pub fn update_document_for_contract_cbor(
        &self,
        document_cbor: &[u8],
        contract_cbor: &[u8],
        document_type: &str,
        owner_id: Option<&[u8]>,
        block_time: f64,
        transaction: TransactionArg,
    ) -> Result<(i64, u64), Error> {
        let contract = Contract::from_cbor(contract_cbor, None)?;

        let document = Document::from_cbor(document_cbor, None, owner_id)?;

        self.update_document_for_contract(
            &document,
            document_cbor,
            &contract,
            document_type,
            owner_id,
            block_time,
            transaction,
        )
    }

    pub fn update_document_cbor_for_contract(
        &self,
        document_cbor: &[u8],
        contract: &Contract,
        document_type: &str,
        owner_id: Option<&[u8]>,
        block_time: f64,
        transaction: TransactionArg,
    ) -> Result<(i64, u64), Error> {
        let document = Document::from_cbor(document_cbor, None, owner_id)?;

        self.update_document_for_contract(
            &document,
            document_cbor,
            contract,
            document_type,
            owner_id,
            block_time,
            transaction,
        )
    }

    pub fn update_document_for_contract(
        &self,
        document: &Document,
        document_cbor: &[u8],
        contract: &Contract,
        document_type_name: &str,
        owner_id: Option<&[u8]>,
        block_time: f64,
        transaction: TransactionArg,
    ) -> Result<(i64, u64), Error> {
        let mut query_operations: Vec<QueryOperation> = vec![];
        let mut insert_operations: Vec<InsertOperation> = vec![];

        let document_type = contract.document_type_for_name(document_type_name)?;

        self.update_document_for_contract_operations(
            DocumentAndContractInfo {
                document_info: DocumentInfo::DocumentAndSerialization((document, document_cbor)),
                contract,
                document_type,
                owner_id,
            },
            block_time,
            transaction,
            &mut query_operations,
            &mut insert_operations,
        )?;
        let fees = calculate_fee(None, Some(query_operations), Some(insert_operations), None)?;
        Ok(fees)
    }

    fn update_document_for_contract_operations(
        &self,
        document_and_contract_info: DocumentAndContractInfo,
        block_time: f64,
        transaction: TransactionArg,
        query_operations: &mut Vec<QueryOperation>,
        insert_operations: &mut Vec<InsertOperation>,
    ) -> Result<(), Error> {
        if !document_and_contract_info.document_type.documents_mutable {
            return Err(Error::InternalError(
                "documents for this contract are not mutable",
            ));
        }

        if !document_and_contract_info
            .document_info
            .is_document_and_serialization()
        {
            // todo: right now let's say the worst case scenario for an update is that all the data must be added again
            self.add_document_for_contract_operations(
                document_and_contract_info,
                false,
                block_time,
                transaction,
                query_operations,
                insert_operations,
            )?;
            return Ok(());
        }

        let contract = document_and_contract_info.contract;
        let document_type = document_and_contract_info.document_type;
        let owner_id = document_and_contract_info.owner_id;

        if let DocumentAndSerialization((document, document_cbor)) =
            document_and_contract_info.document_info
        {
            // we need to construct the path for documents on the contract
            // the path is
            //  * Document and Contract root tree
            //  * Contract ID recovered from document
            //  * 0 to signify Documents and not Contract
            let contract_document_type_path =
                contract_document_type_path(&contract.id, document_type.name.as_str());

            let contract_documents_primary_key_path =
                contract_documents_primary_key_path(&contract.id, document_type.name.as_str());

            // we need to construct the reference to the original document
            let mut reference_path = contract_documents_primary_key_path
                .iter()
                .map(|x| x.to_vec())
                .collect::<Vec<Vec<u8>>>();
            reference_path.push(Vec::from(document.id));
            if document_type.documents_keep_history {
                // if the document keeps history the value will at 0 will always point to the most recent version
                reference_path.push(vec![0]);
            }
            let document_reference = Element::Reference(reference_path);

            // next we need to get the old document from storage
            let old_document_element: Element = if document_type.documents_keep_history {
                let contract_documents_keeping_history_primary_key_path_for_document_id =
                    contract_documents_keeping_history_primary_key_path_for_document_id(
                        &contract.id,
                        document_type.name.as_str(),
                        document.id.as_slice(),
                    );
                self.grove_get(
                    contract_documents_keeping_history_primary_key_path_for_document_id,
                    KeyRef(&[0]),
                    transaction,
                    query_operations,
                )?
            } else {
                self.grove_get(
                    contract_documents_primary_key_path,
                    KeyRef(document.id.as_slice()),
                    transaction,
                    query_operations,
                )?
            }
            .unwrap();

            // we need to store the document for it's primary key
            // we should be overriding if the document_type does not have history enabled
            self.add_document_to_primary_storage(
                &document_and_contract_info,
                block_time,
                true,
                transaction,
                query_operations,
                insert_operations,
            )?;

            let old_document = if let Element::Item(old_document_cbor) = old_document_element {
                Ok(Document::from_cbor(
                    old_document_cbor.as_slice(),
                    None,
                    owner_id,
                )?)
            } else {
                Err(Error::CorruptedData(String::from(
                    "old document is not an item",
                )))
            }?;

            // fourth we need to store a reference to the document for each index
            for index in &document_type.indices {
                // at this point the contract path is to the contract documents
                // for each index the top index component will already have been added
                // when the contract itself was created
                let mut index_path: Vec<Vec<u8>> = contract_document_type_path
                    .iter()
                    .map(|&x| Vec::from(x))
                    .collect();
                let top_index_property = index.properties.get(0).ok_or_else(|| {
                    Error::CorruptedData(String::from("invalid contract indices"))
                })?;
                index_path.push(Vec::from(top_index_property.name.as_bytes()));

                // with the example of the dashpay contract's first index
                // the index path is now something like Contracts/ContractID/Documents(1)/$ownerId
                let document_top_field = document
                    .get_raw_for_contract(
                        &top_index_property.name,
                        document_type.name.as_str(),
                        contract,
                        owner_id,
                    )?
                    .unwrap_or_default();

                let old_document_top_field = old_document
                    .get_raw_for_contract(
                        &top_index_property.name,
                        document_type.name.as_str(),
                        contract,
                        owner_id,
                    )?
                    .unwrap_or_default();

                let mut change_occured_on_index = document_top_field != old_document_top_field;

                if change_occured_on_index {
                    // here we are inserting an empty tree that will have a subtree of all other index properties
                    self.grove_insert_empty_tree_if_not_exists(
                        PathKeyInfo::PathKeyRef::<0>((
                            index_path.clone(),
                            document_top_field.as_slice(),
                        )),
                        transaction,
                        query_operations,
                        insert_operations,
                    )?;
                }

                let mut all_fields_null = document_top_field.is_empty();

                let mut old_index_path = index_path.clone();
                // we push the actual value of the index path
                index_path.push(document_top_field);
                // the index path is now something like Contracts/ContractID/Documents(1)/$ownerId/<ownerId>

                old_index_path.push(old_document_top_field);

                for i in 1..index.properties.len() {
                    let index_property = index.properties.get(i).ok_or_else(|| {
                        Error::CorruptedData(String::from("invalid contract indices"))
                    })?;

                    let document_index_field = document
                        .get_raw_for_contract(
                            &index_property.name,
                            document_type.name.as_str(),
                            contract,
                            owner_id,
                        )?
                        .unwrap_or_default();

                    let old_document_index_field = old_document
                        .get_raw_for_contract(
                            &index_property.name,
                            document_type.name.as_str(),
                            contract,
                            owner_id,
                        )?
                        .unwrap_or_default();

                    change_occured_on_index |= document_index_field != old_document_index_field;

                    if change_occured_on_index {
                        // here we are inserting an empty tree that will have a subtree of all other index properties
                        self.grove_insert_empty_tree_if_not_exists(
                            PathKeyInfo::PathKeyRef::<0>((
                                index_path.clone(),
                                index_property.name.as_bytes(),
                            )),
                            transaction,
                            query_operations,
                            insert_operations,
                        )?;
                    }

                    index_path.push(Vec::from(index_property.name.as_bytes()));
                    old_index_path.push(Vec::from(index_property.name.as_bytes()));

                    // Iteration 1. the index path is now something like Contracts/ContractID/Documents(1)/$ownerId/<ownerId>/toUserId
                    // Iteration 2. the index path is now something like Contracts/ContractID/Documents(1)/$ownerId/<ownerId>/toUserId/<ToUserId>/accountReference

                    if change_occured_on_index {
                        // here we are inserting an empty tree that will have a subtree of all other index properties
                        self.grove_insert_empty_tree_if_not_exists(
                            PathKeyInfo::PathKeyRef::<0>((
                                index_path.clone(),
                                document_index_field.as_slice(),
                            )),
                            transaction,
                            query_operations,
                            insert_operations,
                        )?;
                    }

                    all_fields_null &= document_index_field.is_empty();

                    // we push the actual value of the index path, both for the new and the old
                    index_path.push(document_index_field);
                    old_index_path.push(old_document_index_field);
                    // Iteration 1. the index path is now something like Contracts/ContractID/Documents(1)/$ownerId/<ownerId>/toUserId/<ToUserId>/
                    // Iteration 2. the index path is now something like Contracts/ContractID/Documents(1)/$ownerId/<ownerId>/toUserId/<ToUserId>/accountReference/<accountReference>
                }

                if change_occured_on_index {
                    // we first need to delete the old values
                    // unique indexes will be stored under key "0"
                    // non unique indices should have a tree at key "0" that has all elements based off of primary key
                    if !index.unique {
                        old_index_path.push(vec![0]);

                        let old_index_path_slices: Vec<&[u8]> =
                            old_index_path.iter().map(|x| x.as_slice()).collect();

                        // here we should return an error if the element already exists
                        self.grove.delete_up_tree_while_empty(
                            old_index_path_slices,
                            document.id.as_slice(),
                            Some(CONTRACT_DOCUMENTS_PATH_HEIGHT),
                            transaction,
                        )?;
                    } else {
                        let old_index_path_slices: Vec<&[u8]> =
                            old_index_path.iter().map(|x| x.as_slice()).collect();

                        // here we should return an error if the element already exists
                        self.grove.delete_up_tree_while_empty(
                            old_index_path_slices,
                            &[0],
                            Some(CONTRACT_DOCUMENTS_PATH_HEIGHT),
                            transaction,
                        )?;
                    }

                    // unique indexes will be stored under key "0"
                    // non unique indices should have a tree at key "0" that has all elements based off of primary key
                    if !index.unique || all_fields_null {
                        // here we are inserting an empty tree that will have a subtree of all other index properties
                        self.grove_insert_empty_tree_if_not_exists(
                            PathKeyInfo::PathKeyRef::<0>((index_path.clone(), &[0])),
                            transaction,
                            query_operations,
                            insert_operations,
                        )?;
                        index_path.push(vec![0]);

                        let index_path_slices: Vec<&[u8]> =
                            index_path.iter().map(|x| x.as_slice()).collect();

                        // here we should return an error if the element already exists
                        self.grove_insert(
                            PathKeyElement::<0>((
                                index_path,
                                document.id.as_slice(),
                                document_reference.clone(),
                            )),
                            transaction,
                            insert_operations,
                        )?;
                    } else {
                        let index_path_slices: Vec<&[u8]> =
                            index_path.iter().map(|x| x.as_slice()).collect();

                        // here we should return an error if the element already exists
                        let inserted = self.grove_insert_if_not_exists(
                            PathKeyElement::<0>((index_path, &[0], document_reference.clone())),
                            transaction,
                            query_operations,
                            insert_operations,
                        )?;
                        if !inserted {
                            return Err(Error::CorruptedData(String::from("index already exists")));
                        }
                    }
                }
            }
        }
        Ok(())
    }

    pub fn delete_document_for_contract(
        &self,
        document_id: &[u8],
        contract: &Contract,
        document_type_name: &str,
        owner_id: Option<&[u8]>,
        transaction: TransactionArg,
    ) -> Result<(i64, u64), Error> {
        let mut query_operations: Vec<QueryOperation> = vec![];
        let mut delete_operations: Vec<DeleteOperation> = vec![];
        self.delete_document_for_contract_operations(
            document_id,
            contract,
            document_type_name,
            owner_id,
            transaction,
            &mut query_operations,
            &mut delete_operations,
        )?;
        let fees = calculate_fee(None, Some(query_operations), None, Some(delete_operations))?;
        Ok(fees)
    }

    pub fn delete_document_for_contract_cbor(
        &self,
        document_id: &[u8],
        contract_cbor: &[u8],
        document_type_name: &str,
        owner_id: Option<&[u8]>,
        transaction: TransactionArg,
    ) -> Result<(i64, u64), Error> {
        let contract = Contract::from_cbor(contract_cbor, None)?;
        self.delete_document_for_contract(
            document_id,
            &contract,
            document_type_name,
            owner_id,
            transaction,
        )
    }

    pub fn delete_document_for_contract_operations(
        &self,
        document_id: &[u8],
        contract: &Contract,
        document_type_name: &str,
        owner_id: Option<&[u8]>,
        transaction: TransactionArg,
        query_operations: &mut Vec<QueryOperation>,
        delete_operations: &mut Vec<DeleteOperation>,
    ) -> Result<(), Error> {
        let document_type = contract.document_type_for_name(document_type_name)?;

        if !document_type.documents_mutable {
            return Err(Error::InternalError(
                "documents for this contract are not mutable",
            ));
        }

        // first we need to construct the path for documents on the contract
        // the path is
        //  * Document and Contract root tree
        //  * Contract ID recovered from document
        //  * 0 to signify Documents and not Contract
        let contract_documents_primary_key_path =
            contract_documents_primary_key_path(&contract.id, document_type_name);

        // next we need to get the document from storage
        let document_element: Option<Element> = self.grove_get(
            contract_documents_primary_key_path,
            KeyRef(document_id),
            transaction,
            query_operations,
        )?;

        if document_element.is_none() {
            return Err(Error::InternalError(
                "document being deleted does not exist",
            ));
        }

        let document_bytes: Vec<u8> = match document_element.unwrap() {
            Element::Item(data) => data,
            _ => todo!(), // TODO: how should this be handled, possibility that document might not be in storage
        };

        let document = Document::from_cbor(document_bytes.as_slice(), None, owner_id)?;

        // third we need to delete the document for it's primary key
        self.grove.delete(
            contract_documents_primary_key_path,
            document_id,
            transaction,
        )?;

        let contract_document_type_path =
            contract_document_type_path(&contract.id, document_type_name);

        // fourth we need delete all references to the document
        // to do this we need to go through each index
        for index in &document_type.indices {
            // at this point the contract path is to the contract documents
            // for each index the top index component will already have been added
            // when the contract itself was created
            let mut index_path: Vec<Vec<u8>> = contract_document_type_path
                .iter()
                .map(|&x| Vec::from(x))
                .collect();
            let top_index_property = index
                .properties
                .get(0)
                .ok_or_else(|| Error::CorruptedData(String::from("invalid contract indices")))?;
            index_path.push(Vec::from(top_index_property.name.as_bytes()));

            // with the example of the dashpay contract's first index
            // the index path is now something like Contracts/ContractID/Documents(1)/$ownerId
            let document_top_field: Vec<u8> = document
                .get_raw_for_contract(
                    &top_index_property.name,
                    document_type_name,
                    contract,
                    owner_id,
                )?
                .unwrap_or_default();

            // we push the actual value of the index path
            index_path.push(document_top_field);
            // the index path is now something like Contracts/ContractID/Documents(1)/$ownerId/<ownerId>

            for i in 1..index.properties.len() {
                let index_property = index.properties.get(i).ok_or_else(|| {
                    Error::CorruptedData(String::from("invalid contract indices"))
                })?;

                index_path.push(Vec::from(index_property.name.as_bytes()));
                // Iteration 1. the index path is now something like Contracts/ContractID/Documents(1)/$ownerId/<ownerId>/toUserId
                // Iteration 2. the index path is now something like Contracts/ContractID/Documents(1)/$ownerId/<ownerId>/toUserId/<ToUserId>/accountReference

                let document_top_field: Vec<u8> = document
                    .get_raw_for_contract(
                        &index_property.name,
                        document_type_name,
                        contract,
                        owner_id,
                    )?
                    .unwrap_or_default();

                // we push the actual value of the index path
                index_path.push(document_top_field);
                // Iteration 1. the index path is now something like Contracts/ContractID/Documents(1)/$ownerId/<ownerId>/toUserId/<ToUserId>/
                // Iteration 2. the index path is now something like Contracts/ContractID/Documents(1)/$ownerId/<ownerId>/toUserId/<ToUserId>/accountReference/<accountReference>
            }

            // unique indexes will be stored under key "0"
            // non unique indices should have a tree at key "0" that has all elements based off of primary key
            if !index.unique {
                index_path.push(vec![0]);

                let index_path_slices: Vec<&[u8]> =
                    index_path.iter().map(|x| x.as_slice()).collect();

                // here we should return an error if the element already exists
                self.grove.delete_up_tree_while_empty(
                    index_path_slices,
                    document_id,
                    Some(CONTRACT_DOCUMENTS_PATH_HEIGHT),
                    transaction,
                )?;
            } else {
                let index_path_slices: Vec<&[u8]> =
                    index_path.iter().map(|x| x.as_slice()).collect();

                // here we should return an error if the element already exists
                self.grove.delete_up_tree_while_empty(
                    index_path_slices,
                    &[0],
                    Some(CONTRACT_DOCUMENTS_PATH_HEIGHT),
                    transaction,
                )?;
            }
        }
        Ok(())
    }

    pub fn query_documents_from_contract_cbor(
        &self,
        contract_cbor: &[u8],
        document_type_name: String,
        query_cbor: &[u8],
        transaction: TransactionArg,
    ) -> Result<(Vec<Vec<u8>>, u16), Error> {
        let contract = Contract::from_cbor(contract_cbor, None)?;

        let document_type = contract.document_type_for_name(document_type_name.as_str())?;

        self.query_documents_from_contract(&contract, document_type, query_cbor, transaction)
    }

    pub fn query_documents_from_contract(
        &self,
        contract: &Contract,
        document_type: &DocumentType,
        query_cbor: &[u8],
        transaction: TransactionArg,
    ) -> Result<(Vec<Vec<u8>>, u16), Error> {
        let query = DriveQuery::from_cbor(query_cbor, contract, document_type)?;

        query.execute_no_proof(&self.grove, transaction)
    }

    pub fn worst_case_fee_for_document_type_with_name(
        &self,
        contract: &Contract,
        document_type_name: &str,
    ) -> Result<(i64, u64), Error> {
        let document_type = contract.document_type_for_name(document_type_name)?;
        self.add_document_for_contract(
            DocumentAndContractInfo {
                document_info: DocumentSize(document_type.max_size()),
                contract,
                document_type,
                owner_id: None,
            },
            false,
            0.0,
            None,
        )
    }
}

#[cfg(test)]
mod tests {
    use crate::common::{
        cbor_from_hex, json_document_to_cbor, setup_contract, setup_contract_from_hex,
        value_to_cbor,
    };
    use crate::contract::{Contract, Document};
    use crate::drive::object_size_info::{DocumentAndContractInfo, DocumentInfo};
    use crate::drive::{defaults, Drive};
    use crate::query::DriveQuery;
    use rand::Rng;
    use serde::de::Unexpected::Option;
    use serde_json::json;
    use std::collections::HashMap;
    use std::option::Option::None;
    use tempfile::TempDir;

    fn setup_dashpay(_prefix: &str, mutable_contact_requests: bool) -> (Drive, Vec<u8>) {
        // Todo: make TempDir based on _prefix
        let tmp_dir = TempDir::new().unwrap();
        let drive: Drive = Drive::open(tmp_dir).expect("expected to open Drive successfully");

        drive
            .create_root_tree(None)
            .expect("expected to create root tree successfully");

        let dashpay_path = if mutable_contact_requests {
            "tests/supporting_files/contract/dashpay/dashpay-contract-all-mutable.json"
        } else {
            "tests/supporting_files/contract/dashpay/dashpay-contract.json"
        };

        // let's construct the grovedb structure for the dashpay data contract
        let dashpay_cbor = json_document_to_cbor(dashpay_path, Some(1));
        drive
            .apply_contract_cbor(dashpay_cbor.clone(), None, 0f64, None)
            .expect("expected to apply contract successfully");

        (drive, dashpay_cbor)
    }

    fn setup_deep_nested_contract() -> (Drive, Contract, Vec<u8>) {
        // Todo: make TempDir based on _prefix
        let tmp_dir = TempDir::new().unwrap();
        let drive: Drive = Drive::open(tmp_dir).expect("expected to open Drive successfully");

        drive
            .create_root_tree(None)
            .expect("expected to create root tree successfully");

        let contract_path = "tests/supporting_files/contract/deepNested/deep-nested50.json";
        // let's construct the grovedb structure for the dashpay data contract
        let contract_cbor = json_document_to_cbor(contract_path, Some(1));
        let contract = Contract::from_cbor(&contract_cbor, None)
            .expect("expected to deserialize the contract");
        drive
            .apply_contract(&contract, contract_cbor.clone(), 0f64, None)
            .expect("expected to apply contract successfully");

        (drive, contract, contract_cbor)
    }

    fn setup_reference_contract() -> (Drive, Contract, Vec<u8>) {
        let tmp_dir = TempDir::new().unwrap();
        let drive: Drive = Drive::open(tmp_dir).expect("expected to open Drive successfully");

        drive
            .create_root_tree(None)
            .expect("expected to create root tree successfully");

        let contract_path = "tests/supporting_files/contract/references/references.json";

        // let's construct the grovedb structure for the dashpay data contract
        let contract_cbor = json_document_to_cbor(contract_path, Some(1));
        let contract = Contract::from_cbor(&contract_cbor, None)
            .expect("expected to deserialize the contract");
        drive
            .apply_contract(&contract, contract_cbor.clone(), 0f64, None)
            .expect("expected to apply contract successfully");

        (drive, contract, contract_cbor)
    }

    #[test]
    fn test_add_dashpay_documents_no_transaction() {
        let (drive, dashpay_cbor) = setup_dashpay("add", true);

        let dashpay_cr_document_cbor = json_document_to_cbor(
            "tests/supporting_files/contract/dashpay/contact-request0.json",
            Some(1),
        );

        let random_owner_id = rand::thread_rng().gen::<[u8; 32]>();
        drive
            .add_document_for_contract_cbor(
                &dashpay_cr_document_cbor,
                &dashpay_cbor,
                "contactRequest",
                Some(&random_owner_id),
                false,
                0f64,
                None,
            )
            .expect("expected to insert a document successfully");

        drive
            .add_document_for_contract_cbor(
                &dashpay_cr_document_cbor,
                &dashpay_cbor,
                "contactRequest",
                Some(&random_owner_id),
                false,
                0f64,
                None,
            )
            .expect_err("expected not to be able to insert same document twice");

        drive
            .add_document_for_contract_cbor(
                &dashpay_cr_document_cbor,
                &dashpay_cbor,
                "contactRequest",
                Some(&random_owner_id),
                true,
                0f64,
                None,
            )
            .expect("expected to override a document successfully");
    }

    #[test]
    fn test_add_dashpay_documents() {
        let tmp_dir = TempDir::new().unwrap();
        let drive: Drive = Drive::open(tmp_dir).expect("expected to open Drive successfully");

        let db_transaction = drive.grove.start_transaction();

        drive
            .create_root_tree(Some(&db_transaction))
            .expect("expected to create root tree successfully");

        let contract = setup_contract(
            &drive,
            "tests/supporting_files/contract/dashpay/dashpay-contract-all-mutable.json",
            None,
            Some(&db_transaction),
        );

        let dashpay_cr_document_cbor = json_document_to_cbor(
            "tests/supporting_files/contract/dashpay/contact-request0.json",
            Some(1),
        );

        let random_owner_id = rand::thread_rng().gen::<[u8; 32]>();
        drive
            .add_document_cbor_for_contract(
                &dashpay_cr_document_cbor,
                &contract,
                "contactRequest",
                Some(&random_owner_id),
                false,
                0f64,
                Some(&db_transaction),
            )
            .expect("expected to insert a document successfully");

        drive
            .add_document_cbor_for_contract(
                &dashpay_cr_document_cbor,
                &contract,
                "contactRequest",
                Some(&random_owner_id),
                false,
                0f64,
                Some(&db_transaction),
            )
            .expect_err("expected not to be able to insert same document twice");

        drive
            .add_document_cbor_for_contract(
                &dashpay_cr_document_cbor,
                &contract,
                "contactRequest",
                Some(&random_owner_id),
                true,
                0f64,
                Some(&db_transaction),
            )
            .expect("expected to override a document successfully");
    }

    #[test]
    fn test_modify_dashpay_contact_request() {
        let tmp_dir = TempDir::new().unwrap();
        let drive: Drive = Drive::open(tmp_dir).expect("expected to open Drive successfully");

        let db_transaction = drive.grove.start_transaction();

        drive
            .create_root_tree(Some(&db_transaction))
            .expect("expected to create root tree successfully");

        let contract = setup_contract(
            &drive,
            "tests/supporting_files/contract/dashpay/dashpay-contract.json",
            None,
            Some(&db_transaction),
        );

        let dashpay_cr_document_cbor = json_document_to_cbor(
            "tests/supporting_files/contract/dashpay/contact-request0.json",
            Some(1),
        );

        let random_owner_id = rand::thread_rng().gen::<[u8; 32]>();
        drive
            .add_document_cbor_for_contract(
                &dashpay_cr_document_cbor,
                &contract,
                "contactRequest",
                Some(&random_owner_id),
                false,
                0f64,
                Some(&db_transaction),
            )
            .expect("expected to insert a document successfully");

        drive
            .update_document_cbor_for_contract(
                &dashpay_cr_document_cbor,
                &contract,
                "contactRequest",
                Some(&random_owner_id),
                0f64,
                Some(&db_transaction),
            )
            .expect_err("expected not to be able to update a non mutable document");

        drive
            .add_document_cbor_for_contract(
                &dashpay_cr_document_cbor,
                &contract,
                "contactRequest",
                Some(&random_owner_id),
                true,
                0f64,
                Some(&db_transaction),
            )
            .expect_err("expected not to be able to override a non mutable document");
    }

    #[test]
    fn test_update_dashpay_profile_with_history() {
        let tmp_dir = TempDir::new().unwrap();
        let drive: Drive = Drive::open(tmp_dir).expect("expected to open Drive successfully");

        let db_transaction = drive.grove.start_transaction();

        drive
            .create_root_tree(Some(&db_transaction))
            .expect("expected to create root tree successfully");

        let contract = setup_contract(
            &drive,
            "tests/supporting_files/contract/dashpay/dashpay-contract-with-profile-history.json",
            None,
            Some(&db_transaction),
        );

        let dashpay_profile_document_cbor = json_document_to_cbor(
            "tests/supporting_files/contract/dashpay/profile0.json",
            Some(1),
        );

        let dashpay_profile_updated_public_message_document_cbor = json_document_to_cbor(
            "tests/supporting_files/contract/dashpay/profile0-updated-public-message.json",
            Some(1),
        );

        let random_owner_id = rand::thread_rng().gen::<[u8; 32]>();
        drive
            .add_document_cbor_for_contract(
                &dashpay_profile_document_cbor,
                &contract,
                "profile",
                Some(&random_owner_id),
                false,
                0f64,
                Some(&db_transaction),
            )
            .expect("expected to insert a document successfully");

        drive
            .update_document_cbor_for_contract(
                &dashpay_profile_updated_public_message_document_cbor,
                &contract,
                "profile",
                Some(&random_owner_id),
                0f64,
                Some(&db_transaction),
            )
            .expect("expected to update a document with history successfully");
    }

    #[test]
    fn test_delete_dashpay_documents_no_transaction() {
        let (drive, dashpay_cbor) = setup_dashpay("delete", false);

        let dashpay_profile_document_cbor = json_document_to_cbor(
            "tests/supporting_files/contract/dashpay/profile0.json",
            Some(1),
        );

        let random_owner_id = rand::thread_rng().gen::<[u8; 32]>();
        drive
            .add_document_for_contract_cbor(
                &dashpay_profile_document_cbor,
                &dashpay_cbor,
                "profile",
                Some(&random_owner_id),
                false,
                0f64,
                None,
            )
            .expect("expected to insert a document successfully");

        let document_id = bs58::decode("AM47xnyLfTAC9f61ZQPGfMK5Datk2FeYZwgYvcAnzqFY")
            .into_vec()
            .expect("this should decode");

        drive
            .delete_document_for_contract_cbor(
                &document_id,
                &dashpay_cbor,
                "profile",
                Some(&random_owner_id),
                None,
            )
            .expect("expected to be able to delete the document");
    }

    #[test]
    fn test_delete_dashpay_documents() {
        let tmp_dir = TempDir::new().unwrap();
        let drive: Drive = Drive::open(tmp_dir).expect("expected to open Drive successfully");

        let db_transaction = drive.grove.start_transaction();

        drive
            .create_root_tree(Some(&db_transaction))
            .expect("expected to create root tree successfully");

        let contract = setup_contract(
            &drive,
            "tests/supporting_files/contract/dashpay/dashpay-contract.json",
            None,
            Some(&db_transaction),
        );

        let dashpay_profile_document_cbor = json_document_to_cbor(
            "tests/supporting_files/contract/dashpay/profile0.json",
            Some(1),
        );

        let random_owner_id = rand::thread_rng().gen::<[u8; 32]>();
        drive
            .add_document_cbor_for_contract(
                &dashpay_profile_document_cbor,
                &contract,
                "profile",
                Some(&random_owner_id),
                false,
                0f64,
                Some(&db_transaction),
            )
            .expect("expected to insert a document successfully");

        let document_id = bs58::decode("AM47xnyLfTAC9f61ZQPGfMK5Datk2FeYZwgYvcAnzqFY")
            .into_vec()
            .expect("this should decode");

        drive
            .delete_document_for_contract(
                &document_id,
                &contract,
                "profile",
                Some(&random_owner_id),
                Some(&db_transaction),
            )
            .expect("expected to be able to delete the document");
    }

    #[test]
    fn test_add_dpns_documents() {
        let tmp_dir = TempDir::new().unwrap();
        let drive: Drive = Drive::open(tmp_dir).expect("expected to open Drive successfully");

        let db_transaction = drive.grove.start_transaction();

        drive
            .create_root_tree(Some(&db_transaction))
            .expect("expected to create root tree successfully");

        let contract = setup_contract(
            &drive,
            "tests/supporting_files/contract/dpns/dpns-contract.json",
            None,
            Some(&db_transaction),
        );

        let dpns_domain_document_cbor =
            json_document_to_cbor("tests/supporting_files/contract/dpns/domain0.json", Some(1));

        let random_owner_id = rand::thread_rng().gen::<[u8; 32]>();

        let document =
            Document::from_cbor(&dpns_domain_document_cbor, None, Some(&random_owner_id))
                .expect("expected to deserialize the document");

        let document_type = contract
            .document_type_for_name("domain")
            .expect("expected to get a document type");

        drive
            .add_document_for_contract(
                DocumentAndContractInfo {
                    document_info: DocumentInfo::DocumentAndSerialization((
                        &document,
                        &dpns_domain_document_cbor,
                    )),
                    contract: &contract,
                    document_type,
                    owner_id: None,
                },
                false,
                0f64,
                Some(&db_transaction),
            )
            .expect("expected to insert a document successfully");

        drive
            .grove
            .commit_transaction(db_transaction)
            .expect("unable to commit transaction");
    }

    #[test]
    fn test_deletion_real_data() {
        let tmp_dir = TempDir::new().unwrap();
        let drive: Drive = Drive::open(tmp_dir).expect("expected to open Drive successfully");

        let db_transaction = drive.grove.start_transaction();

        drive
            .create_root_tree(Some(&db_transaction))
            .expect("expected to create root tree successfully");

        let contract = setup_contract_from_hex(
            &drive,
            "01000000a5632469645820e8f72680f2e3910c95e1497a2b0029d9f7374891ac1f39ab1cfe3ae63336b9a96724736368656d61783468747470733a2f2f736368656d612e646173682e6f72672f6470702d302d342d302f6d6574612f646174612d636f6e7472616374676f776e6572496458209e412570bf3b7ce068b9bce81c569ce701e43edaea80b62a2773be7d21038b266776657273696f6e0169646f63756d656e7473a76b756e697175654461746573a56474797065666f626a65637467696e646963657382a3646e616d6566696e6465783166756e69717565f56a70726f7065727469657382a16a2463726561746564417463617363a16a2475706461746564417463617363a2646e616d6566696e646578326a70726f7065727469657381a16a2475706461746564417463617363687265717569726564836966697273744e616d656a246372656174656441746a247570646174656441746a70726f70657274696573a2686c6173744e616d65a1647479706566737472696e676966697273744e616d65a1647479706566737472696e67746164646974696f6e616c50726f70657274696573f46c6e696365446f63756d656e74a56474797065666f626a65637467696e646963657384a2646e616d6566696e646578316a70726f7065727469657381a1646e616d6563617363a2646e616d6566696e646578336a70726f7065727469657381a1656f7264657263617363a2646e616d6566696e646578346a70726f7065727469657381a1686c6173744e616d6563617363a2646e616d6567696e64657831306a70726f7065727469657381a168246f776e657249646464657363687265717569726564816a246372656174656441746a70726f70657274696573a3646e616d65a1647479706566737472696e67656f72646572a16474797065666e756d626572686c6173744e616d65a1647479706566737472696e67746164646974696f6e616c50726f70657274696573f46e6e6f54696d65446f63756d656e74a36474797065666f626a6563746a70726f70657274696573a1646e616d65a1647479706566737472696e67746164646974696f6e616c50726f70657274696573f46e707265747479446f63756d656e74a46474797065666f626a65637468726571756972656482686c6173744e616d656a247570646174656441746a70726f70657274696573a1686c6173744e616d65a1647479706566737472696e67746164646974696f6e616c50726f70657274696573f46e7769746842797465417272617973a56474797065666f626a65637467696e646963657381a2646e616d6566696e646578316a70726f7065727469657381a16e6279746541727261794669656c6463617363687265717569726564816e6279746541727261794669656c646a70726f70657274696573a26e6279746541727261794669656c64a36474797065656172726179686d61784974656d731069627974654172726179f56f6964656e7469666965724669656c64a56474797065656172726179686d61784974656d731820686d696e4974656d73182069627974654172726179f570636f6e74656e744d656469615479706578216170706c69636174696f6e2f782e646173682e6470702e6964656e746966696572746164646974696f6e616c50726f70657274696573f46f696e6465786564446f63756d656e74a56474797065666f626a65637467696e646963657386a3646e616d6566696e6465783166756e69717565f56a70726f7065727469657382a168246f776e6572496463617363a16966697273744e616d656464657363a3646e616d6566696e6465783266756e69717565f56a70726f7065727469657382a168246f776e6572496463617363a1686c6173744e616d656464657363a2646e616d6566696e646578336a70726f7065727469657381a1686c6173744e616d6563617363a2646e616d6566696e646578346a70726f7065727469657382a16a2463726561746564417463617363a16a2475706461746564417463617363a2646e616d6566696e646578356a70726f7065727469657381a16a2475706461746564417463617363a2646e616d6566696e646578366a70726f7065727469657381a16a2463726561746564417463617363687265717569726564846966697273744e616d656a246372656174656441746a24757064617465644174686c6173744e616d656a70726f70657274696573a2686c6173744e616d65a2647479706566737472696e67696d61784c656e677468183f6966697273744e616d65a2647479706566737472696e67696d61784c656e677468183f746164646974696f6e616c50726f70657274696573f4781d6f7074696f6e616c556e69717565496e6465786564446f63756d656e74a56474797065666f626a65637467696e646963657383a3646e616d6566696e6465783166756e69717565f56a70726f7065727469657381a16966697273744e616d656464657363a3646e616d6566696e6465783266756e69717565f56a70726f7065727469657383a168246f776e6572496463617363a16966697273744e616d6563617363a1686c6173744e616d6563617363a3646e616d6566696e6465783366756e69717565f56a70726f7065727469657382a167636f756e74727963617363a1646369747963617363687265717569726564826966697273744e616d65686c6173744e616d656a70726f70657274696573a46463697479a2647479706566737472696e67696d61784c656e677468183f67636f756e747279a2647479706566737472696e67696d61784c656e677468183f686c6173744e616d65a2647479706566737472696e67696d61784c656e677468183f6966697273744e616d65a2647479706566737472696e67696d61784c656e677468183f746164646974696f6e616c50726f70657274696573f4".to_string(),
            Some(&db_transaction),
        );

        let document_hexes = [
            "01000000a86324696458208fcfbce88a219c6e6f4cca4aa55c1ba08303d62985d94084a28d3c298753b8a6646e616d656543757469656524747970656c6e696365446f63756d656e74656f726465720068246f776e657249645820cac675648b485d2606a53fca9942cb7bfdf34e08cee1ebe6e0e74e8502ac6c8069247265766973696f6e016a246372656174656441741b0000017f9334371f6f2464617461436f6e747261637449645820e8f72680f2e3910c95e1497a2b0029d9f7374891ac1f39ab1cfe3ae63336b9a9",
            "01000000a863246964582067a18898a8bfdd139353359d907d487b45d62ab4694a63ad1fe34a34cd8c42116524747970656c6e696365446f63756d656e74656f726465720168246f776e657249645820cac675648b485d2606a53fca9942cb7bfdf34e08cee1ebe6e0e74e8502ac6c80686c6173744e616d65655368696e7969247265766973696f6e016a247570646174656441741b0000017f9334371f6f2464617461436f6e747261637449645820e8f72680f2e3910c95e1497a2b0029d9f7374891ac1f39ab1cfe3ae63336b9a9",
            "01000000a863246964582091bf487b6041e26d7e22a4a10d544fb733daba7b60ef8ed557bb21fd722bdd036524747970656c6e696365446f63756d656e74656f726465720268246f776e657249645820cac675648b485d2606a53fca9942cb7bfdf34e08cee1ebe6e0e74e8502ac6c80686c6173744e616d656653776565747969247265766973696f6e016a247570646174656441741b0000017f9334371f6f2464617461436f6e747261637449645820e8f72680f2e3910c95e1497a2b0029d9f7374891ac1f39ab1cfe3ae63336b9a9",
            "01000000aa632469645820a2869e44207381542b144f22a65b961e5ddf489d68d7a720144bee223a0555956524747970656c6e696365446f63756d656e74656f726465720368246f776e657249645820cac675648b485d2606a53fca9942cb7bfdf34e08cee1ebe6e0e74e8502ac6c80686c6173744e616d65664269726b696e69247265766973696f6e016966697273744e616d656757696c6c69616d6a246372656174656441741b0000017f933437206a247570646174656441741b0000017f933437206f2464617461436f6e747261637449645820e8f72680f2e3910c95e1497a2b0029d9f7374891ac1f39ab1cfe3ae63336b9a9",
            "01000000aa6324696458208d2a661748268018725cf0dc612c74cf1e8621dc86c5e9cc64d2bbe17a2f855a6524747970656c6e696365446f63756d656e74656f726465720468246f776e657249645820cac675648b485d2606a53fca9942cb7bfdf34e08cee1ebe6e0e74e8502ac6c80686c6173744e616d65674b656e6e65647969247265766973696f6e016966697273744e616d65644c656f6e6a246372656174656441741b0000017f933437206a247570646174656441741b0000017f933437206f2464617461436f6e747261637449645820e8f72680f2e3910c95e1497a2b0029d9f7374891ac1f39ab1cfe3ae63336b9a9"
        ];

        let documents: Vec<Document> = document_hexes
            .iter()
            .map(|document_hex| {
                let document_cbor = cbor_from_hex(document_hex.to_string());

                let document = Document::from_cbor(&document_cbor, None, None)
                    .expect("expected to deserialize the document");

                let document_type = contract
                    .document_type_for_name("niceDocument")
                    .expect("expected to get a document type");

                drive
                    .add_document_for_contract(
                        DocumentAndContractInfo {
                            document_info: DocumentInfo::DocumentAndSerialization((
                                &document,
                                &document_cbor,
                            )),
                            contract: &contract,
                            document_type,
                            owner_id: None,
                        },
                        false,
                        0f64,
                        Some(&db_transaction),
                    )
                    .expect("expected to insert a document successfully");

                document
            })
            .collect();

        let document_id = "AgP2Tx2ayfobSQ6xZCEVLzfmmLD4YR3CNAJcfgZfBcY5";

        let query_json = json!({
            "where": [
                ["$id", "==", String::from(document_id)]
            ],
        });

        let query_cbor = value_to_cbor(query_json, None);

        drive
            .grove
            .commit_transaction(db_transaction)
            .expect("unable to commit transaction");

        let (results, _) = drive
            .query_documents_from_contract(
                &contract,
                contract.document_types.get("niceDocument").unwrap(),
                query_cbor.as_slice(),
                None,
            )
            .expect("expected to execute query");

        assert_eq!(results.len(), 1);

        let db_transaction = drive.grove.start_transaction();

        drive
            .delete_document_for_contract(
                &documents.get(0).unwrap().id,
                &contract,
                "niceDocument",
                Some(&documents.get(0).unwrap().owner_id),
                Some(&db_transaction),
            )
            .expect("expected to be able to delete the document");

        let query_json = json!({
            "where": [
                ["$id", "==", String::from(document_id)]
            ],
        });

        let query_cbor = value_to_cbor(query_json, None);

        let (results, _) = drive
            .query_documents_from_contract(
                &contract,
                contract.document_types.get("niceDocument").unwrap(),
                query_cbor.as_slice(),
                Some(&db_transaction),
            )
            .expect("expected to execute query");

        assert_eq!(results.len(), 0);
    }

    #[test]
    fn test_add_and_remove_family_one_document() {
        let tmp_dir = TempDir::new().unwrap();
        let drive: Drive = Drive::open(tmp_dir).expect("expected to open Drive successfully");

        let db_transaction = drive.grove.start_transaction();

        drive
            .create_root_tree(Some(&db_transaction))
            .expect("expected to create root tree successfully");

        let contract = setup_contract(
            &drive,
            "tests/supporting_files/contract/family/family-contract-reduced.json",
            None,
            Some(&db_transaction),
        );

        let person_document_cbor = json_document_to_cbor(
            "tests/supporting_files/contract/family/person0.json",
            Some(1),
        );

        let random_owner_id = rand::thread_rng().gen::<[u8; 32]>();

        let document = Document::from_cbor(&person_document_cbor, None, Some(&random_owner_id))
            .expect("expected to deserialize the document");

        let document_type = contract
            .document_type_for_name("person")
            .expect("expected to get a document type");

        drive
            .add_document_for_contract(
                DocumentAndContractInfo {
                    document_info: DocumentInfo::DocumentAndSerialization((
                        &document,
                        &person_document_cbor,
                    )),
                    contract: &contract,
                    document_type,
                    owner_id: None,
                },
                false,
                0f64,
                Some(&db_transaction),
            )
            .expect("expected to insert a document successfully");

        drive
            .grove
            .commit_transaction(db_transaction)
            .expect("unable to commit transaction");

        let sql_string =
            "select * from person where firstName = 'Samuel' order by firstName asc limit 100";
        let query = DriveQuery::from_sql_expr(sql_string, &contract).expect("should build query");

        let (results_no_transaction, _) = query
            .execute_no_proof(&drive.grove, None)
            .expect("expected to execute query");

        assert_eq!(results_no_transaction.len(), 1);

        let db_transaction = drive.grove.start_transaction();

        let (results_on_transaction, _) = query
            .execute_no_proof(&drive.grove, Some(&db_transaction))
            .expect("expected to execute query");

        assert_eq!(results_on_transaction.len(), 1);
        let document_id = bs58::decode("AYjYxDqLy2hvGQADqE6FAkBnQEpJSzNd3CRw1tpS6vZ7")
            .into_vec()
            .expect("this should decode");

        drive
            .delete_document_for_contract(
                &document_id,
                &contract,
                "person",
                Some(&random_owner_id),
                Some(&db_transaction),
            )
            .expect("expected to be able to delete the document");

        drive
            .grove
            .commit_transaction(db_transaction)
            .expect("unable to commit transaction");

        let db_transaction = drive.grove.start_transaction();

        let (results_on_transaction, _) = query
            .execute_no_proof(&drive.grove, Some(&db_transaction))
            .expect("expected to execute query");

        assert_eq!(results_on_transaction.len(), 0);
    }

    #[test]
    fn test_add_and_remove_family_documents() {
        let tmp_dir = TempDir::new().unwrap();
        let drive: Drive = Drive::open(tmp_dir).expect("expected to open Drive successfully");

        let db_transaction = drive.grove.start_transaction();

        drive
            .create_root_tree(Some(&db_transaction))
            .expect("expected to create root tree successfully");

        let contract = setup_contract(
            &drive,
            "tests/supporting_files/contract/family/family-contract-reduced.json",
            None,
            Some(&db_transaction),
        );

        let person_document_cbor = json_document_to_cbor(
            "tests/supporting_files/contract/family/person0.json",
            Some(1),
        );

        let random_owner_id = rand::thread_rng().gen::<[u8; 32]>();

        let document = Document::from_cbor(&person_document_cbor, None, Some(&random_owner_id))
            .expect("expected to deserialize the document");

        let document_type = contract
            .document_type_for_name("person")
            .expect("expected to get a document type");

        drive
            .add_document_for_contract(
                DocumentAndContractInfo {
                    document_info: DocumentInfo::DocumentAndSerialization((
                        &document,
                        &person_document_cbor,
                    )),
                    contract: &contract,
                    document_type,
                    owner_id: None,
                },
                false,
                0f64,
                Some(&db_transaction),
            )
            .expect("expected to insert a document successfully");

        let person_document_cbor = json_document_to_cbor(
            "tests/supporting_files/contract/family/person1.json",
            Some(1),
        );

        let random_owner_id = rand::thread_rng().gen::<[u8; 32]>();

        let document = Document::from_cbor(&person_document_cbor, None, Some(&random_owner_id))
            .expect("expected to deserialize the document");

        let document_type = contract
            .document_type_for_name("person")
            .expect("expected to get a document type");

        drive
            .add_document_for_contract(
                DocumentAndContractInfo {
                    document_info: DocumentInfo::DocumentAndSerialization((
                        &document,
                        &person_document_cbor,
                    )),
                    contract: &contract,
                    document_type,
                    owner_id: None,
                },
                false,
                0f64,
                Some(&db_transaction),
            )
            .expect("expected to insert a document successfully");

        drive
            .grove
            .commit_transaction(db_transaction)
            .expect("unable to commit transaction");

        let sql_string =
            "select * from person where firstName > 'A' order by firstName asc limit 5";
        let query = DriveQuery::from_sql_expr(sql_string, &contract).expect("should build query");

        let (results_no_transaction, _) = query
            .execute_no_proof(&drive.grove, None)
            .expect("expected to execute query");

        assert_eq!(results_no_transaction.len(), 2);

        let document_id = bs58::decode("8wjx2TC1vj2grssQvdwWnksNLwpi4xKraYy1TbProgd4")
            .into_vec()
            .expect("this should decode");

        let db_transaction = drive.grove.start_transaction();

        drive
            .delete_document_for_contract(
                &document_id,
                &contract,
                "person",
                Some(&random_owner_id),
                Some(&db_transaction),
            )
            .expect("expected to be able to delete the document");

        drive
            .grove
            .commit_transaction(db_transaction)
            .expect("unable to commit transaction");

        let sql_string =
            "select * from person where firstName > 'A' order by firstName asc limit 5";
        let query = DriveQuery::from_sql_expr(sql_string, &contract).expect("should build query");

        let (results_no_transaction, _) = query
            .execute_no_proof(&drive.grove, None)
            .expect("expected to execute query");

        assert_eq!(results_no_transaction.len(), 1);

        let document_id = bs58::decode("AYjYxDqLy2hvGQADqE6FAkBnQEpJSzNd3CRw1tpS6vZ7")
            .into_vec()
            .expect("this should decode");

        let db_transaction = drive.grove.start_transaction();

        drive
            .delete_document_for_contract(
                &document_id,
                &contract,
                "person",
                Some(&random_owner_id),
                Some(&db_transaction),
            )
            .expect("expected to be able to delete the document");

        drive
            .grove
            .commit_transaction(db_transaction)
            .expect("unable to commit transaction");

        let sql_string =
            "select * from person where firstName > 'A' order by firstName asc limit 5";
        let query = DriveQuery::from_sql_expr(sql_string, &contract).expect("should build query");

        let (results_no_transaction, _) = query
            .execute_no_proof(&drive.grove, None)
            .expect("expected to execute query");

        assert_eq!(results_no_transaction.len(), 0);
    }

    #[test]
    fn test_add_and_remove_family_documents_with_empty_fields() {
        let tmp_dir = TempDir::new().unwrap();
        let drive: Drive = Drive::open(tmp_dir).expect("expected to open Drive successfully");

        let db_transaction = drive.grove.start_transaction();

        drive
            .create_root_tree(Some(&db_transaction))
            .expect("expected to create root tree successfully");

        let contract = setup_contract(
            &drive,
            "tests/supporting_files/contract/family/family-contract-reduced.json",
            None,
            Some(&db_transaction),
        );

        let person_document_cbor = json_document_to_cbor(
            "tests/supporting_files/contract/family/person0.json",
            Some(1),
        );

        let random_owner_id = rand::thread_rng().gen::<[u8; 32]>();

        let document = Document::from_cbor(&person_document_cbor, None, Some(&random_owner_id))
            .expect("expected to deserialize the document");

        let document_type = contract
            .document_type_for_name("person")
            .expect("expected to get a document type");

        drive
            .add_document_for_contract(
                DocumentAndContractInfo {
                    document_info: DocumentInfo::DocumentAndSerialization((
                        &document,
                        &person_document_cbor,
                    )),
                    contract: &contract,
                    document_type,
                    owner_id: None,
                },
                false,
                0f64,
                Some(&db_transaction),
            )
            .expect("expected to insert a document successfully");

        let person_document_cbor = json_document_to_cbor(
            "tests/supporting_files/contract/family/person2-no-middle-name.json",
            Some(1),
        );

        let random_owner_id = rand::thread_rng().gen::<[u8; 32]>();

        let document = Document::from_cbor(&person_document_cbor, None, Some(&random_owner_id))
            .expect("expected to deserialize the document");

        let document_type = contract
            .document_type_for_name("person")
            .expect("expected to get a document type");

        drive
            .add_document_for_contract(
                DocumentAndContractInfo {
                    document_info: DocumentInfo::DocumentAndSerialization((
                        &document,
                        &person_document_cbor,
                    )),
                    contract: &contract,
                    document_type,
                    owner_id: None,
                },
                false,
                0f64,
                Some(&db_transaction),
            )
            .expect("expected to insert a document successfully");

        drive
            .grove
            .commit_transaction(db_transaction)
            .expect("unable to commit transaction");

        let sql_string =
            "select * from person where firstName > 'A' order by firstName asc limit 5";
        let query = DriveQuery::from_sql_expr(sql_string, &contract).expect("should build query");

        let (results_no_transaction, _) = query
            .execute_no_proof(&drive.grove, None)
            .expect("expected to execute query");

        assert_eq!(results_no_transaction.len(), 2);

        let document_id = bs58::decode("BZjYxDqLy2hvGQADqE6FAkBnQEpJSzNd3CRw1tpS6vZ7")
            .into_vec()
            .expect("this should decode");

        let db_transaction = drive.grove.start_transaction();

        drive
            .delete_document_for_contract(
                &document_id,
                &contract,
                "person",
                Some(&random_owner_id),
                Some(&db_transaction),
            )
            .expect("expected to be able to delete the document");

        drive
            .grove
            .commit_transaction(db_transaction)
            .expect("unable to commit transaction");

        // Let's try adding the document back after it was deleted

        let db_transaction = drive.grove.start_transaction();

        let document = Document::from_cbor(&person_document_cbor, None, Some(&random_owner_id))
            .expect("expected to deserialize the document");

        drive
            .add_document_for_contract(
                DocumentAndContractInfo {
                    document_info: DocumentInfo::DocumentAndSerialization((
                        &document,
                        &person_document_cbor,
                    )),
                    contract: &contract,
                    document_type,
                    owner_id: None,
                },
                false,
                0f64,
                Some(&db_transaction),
            )
            .expect("expected to insert a document successfully");

        drive
            .grove
            .commit_transaction(db_transaction)
            .expect("unable to commit transaction");

        // Let's try removing all documents now

        let db_transaction = drive.grove.start_transaction();

        drive
            .delete_document_for_contract(
                &document_id,
                &contract,
                "person",
                Some(&random_owner_id),
                Some(&db_transaction),
            )
            .expect("expected to be able to delete the document");

        let document_id = bs58::decode("AYjYxDqLy2hvGQADqE6FAkBnQEpJSzNd3CRw1tpS6vZ7")
            .into_vec()
            .expect("this should decode");

        drive
            .delete_document_for_contract(
                &document_id,
                &contract,
                "person",
                Some(&random_owner_id),
                Some(&db_transaction),
            )
            .expect("expected to be able to delete the document");

        drive
            .grove
            .commit_transaction(db_transaction)
            .expect("unable to commit transaction");

        let sql_string =
            "select * from person where firstName > 'A' order by firstName asc limit 5";
        let query = DriveQuery::from_sql_expr(sql_string, &contract).expect("should build query");

        let (results_no_transaction, _) = query
            .execute_no_proof(&drive.grove, None)
            .expect("expected to execute query");

        assert_eq!(results_no_transaction.len(), 0);
    }

    #[test]
    fn test_add_dashpay_many_non_conflicting_documents() {
        let (drive, dashpay_cbor) = setup_dashpay("add_no_conflict", true);

        let dashpay_cr_document_cbor_0 = json_document_to_cbor(
            "tests/supporting_files/contract/dashpay/contact-request0.json",
            Some(1),
        );

        let dashpay_cr_document_cbor_1 = json_document_to_cbor(
            "tests/supporting_files/contract/dashpay/contact-request1.json",
            Some(1),
        );

        let dashpay_cr_document_cbor_2 = json_document_to_cbor(
            "tests/supporting_files/contract/dashpay/contact-request2.json",
            Some(1),
        );

        let random_owner_id = rand::thread_rng().gen::<[u8; 32]>();
        drive
            .add_document_for_contract_cbor(
                &dashpay_cr_document_cbor_0,
                &dashpay_cbor,
                "contactRequest",
                Some(&random_owner_id),
                false,
                0f64,
                None,
            )
            .expect("expected to insert a document successfully");
        drive
            .add_document_for_contract_cbor(
                &dashpay_cr_document_cbor_1,
                &dashpay_cbor,
                "contactRequest",
                Some(&random_owner_id),
                false,
                0f64,
                None,
            )
            .expect("expected to insert a document successfully");
        drive
            .add_document_for_contract_cbor(
                &dashpay_cr_document_cbor_2,
                &dashpay_cbor,
                "contactRequest",
                Some(&random_owner_id),
                false,
                0f64,
                None,
            )
            .expect("expected to insert a document successfully");
    }

    #[test]
    fn test_add_dashpay_conflicting_unique_index_documents() {
        let (drive, dashpay_cbor) = setup_dashpay("add_conflict", true);

        let dashpay_cr_document_cbor_0 = json_document_to_cbor(
            "tests/supporting_files/contract/dashpay/contact-request0.json",
            Some(1),
        );

        let dashpay_cr_document_cbor_0_dup = json_document_to_cbor(
            "tests/supporting_files/contract/dashpay/contact-request0-dup-unique-index.json",
            Some(1),
        );

        let random_owner_id = rand::thread_rng().gen::<[u8; 32]>();
        drive
            .add_document_for_contract_cbor(
                &dashpay_cr_document_cbor_0,
                &dashpay_cbor,
                "contactRequest",
                Some(&random_owner_id),
                false,
                0f64,
                None,
            )
            .expect("expected to insert a document successfully");
        drive
            .add_document_for_contract_cbor(
                &dashpay_cr_document_cbor_0_dup,
                &dashpay_cbor,
                "contactRequest",
                Some(&random_owner_id),
                false,
                0f64,
                None,
            )
            .expect_err(
                "expected not to be able to insert document with already existing unique index",
            );
    }

    #[test]
    fn test_create_and_update_document_same_transaction() {
        let tmp_dir = TempDir::new().unwrap();
        let drive: Drive = Drive::open(tmp_dir).expect("expected to open Drive successfully");

        let db_transaction = drive.grove.start_transaction();

        drive
            .create_root_tree(Some(&db_transaction))
            .expect("expected to create root tree successfully");

        let contract_cbor = hex::decode("01000000a5632469645820b0248cd9a27f86d05badf475dd9ff574d63219cd60c52e2be1e540c2fdd713336724736368656d61783468747470733a2f2f736368656d612e646173682e6f72672f6470702d302d342d302f6d6574612f646174612d636f6e7472616374676f776e6572496458204c9bf0db6ae315c85465e9ef26e6a006de9673731d08d14881945ddef1b5c5f26776657273696f6e0169646f63756d656e7473a267636f6e74616374a56474797065666f626a65637467696e646963657381a3646e616d656f6f6e7765724964546f55736572496466756e69717565f56a70726f7065727469657382a168246f776e6572496463617363a168746f557365724964636173636872657175697265648268746f557365724964697075626c69634b65796a70726f70657274696573a268746f557365724964a56474797065656172726179686d61784974656d731820686d696e4974656d73182069627974654172726179f570636f6e74656e744d656469615479706578216170706c69636174696f6e2f782e646173682e6470702e6964656e746966696572697075626c69634b6579a36474797065656172726179686d61784974656d73182169627974654172726179f5746164646974696f6e616c50726f70657274696573f46770726f66696c65a56474797065666f626a65637467696e646963657381a3646e616d65676f776e6572496466756e69717565f56a70726f7065727469657381a168246f776e6572496463617363687265717569726564826961766174617255726c6561626f75746a70726f70657274696573a26561626f7574a2647479706566737472696e67696d61784c656e67746818ff6961766174617255726ca3647479706566737472696e6766666f726d61746375726c696d61784c656e67746818ff746164646974696f6e616c50726f70657274696573f4").unwrap();

        drive
            .apply_contract_cbor(contract_cbor.clone(), None, 0f64, Some(&db_transaction))
            .expect("expected to apply contract successfully");

        // Create Alice profile

        let alice_profile_cbor = hex::decode("01000000a763246964582035edfec54aea574df968990abb47b39c206abe5c43a6157885f62958a1f1230c6524747970656770726f66696c656561626f75746a4920616d20416c69636568246f776e65724964582041d52f93f6f7c5af79ce994381c90df73cce2863d3850b9c05ef586ff0fe795f69247265766973696f6e016961766174617255726c7819687474703a2f2f746573742e636f6d2f616c6963652e6a70676f2464617461436f6e747261637449645820b0248cd9a27f86d05badf475dd9ff574d63219cd60c52e2be1e540c2fdd71333").unwrap();

        drive
            .add_document_for_contract_cbor(
                alice_profile_cbor.as_slice(),
                contract_cbor.as_slice(),
                "profile",
                None,
                true,
                0f64,
                Some(&db_transaction),
            )
            .expect("should create alice profile");

        // Update Alice profile

        let updated_alice_profile_cbor = hex::decode("01000000a763246964582035edfec54aea574df968990abb47b39c206abe5c43a6157885f62958a1f1230c6524747970656770726f66696c656561626f75746a4920616d20416c69636568246f776e65724964582041d52f93f6f7c5af79ce994381c90df73cce2863d3850b9c05ef586ff0fe795f69247265766973696f6e026961766174617255726c781a687474703a2f2f746573742e636f6d2f616c696365322e6a70676f2464617461436f6e747261637449645820b0248cd9a27f86d05badf475dd9ff574d63219cd60c52e2be1e540c2fdd71333").unwrap();

        drive
            .update_document_for_contract_cbor(
                updated_alice_profile_cbor.as_slice(),
                contract_cbor.as_slice(),
                "profile",
                None,
                0f64,
                Some(&db_transaction),
            )
            .expect("should update alice profile");
    }

    #[test]
    fn test_create_and_update_document_no_transactions() {
        let tmp_dir = TempDir::new().unwrap();
        let drive: Drive = Drive::open(tmp_dir).expect("expected to open Drive successfully");

        drive
            .create_root_tree(None)
            .expect("expected to create root tree successfully");

        let contract_cbor = hex::decode("01000000a5632469645820b0248cd9a27f86d05badf475dd9ff574d63219cd60c52e2be1e540c2fdd713336724736368656d61783468747470733a2f2f736368656d612e646173682e6f72672f6470702d302d342d302f6d6574612f646174612d636f6e7472616374676f776e6572496458204c9bf0db6ae315c85465e9ef26e6a006de9673731d08d14881945ddef1b5c5f26776657273696f6e0169646f63756d656e7473a267636f6e74616374a56474797065666f626a65637467696e646963657381a3646e616d656f6f6e7765724964546f55736572496466756e69717565f56a70726f7065727469657382a168246f776e6572496463617363a168746f557365724964636173636872657175697265648268746f557365724964697075626c69634b65796a70726f70657274696573a268746f557365724964a56474797065656172726179686d61784974656d731820686d696e4974656d73182069627974654172726179f570636f6e74656e744d656469615479706578216170706c69636174696f6e2f782e646173682e6470702e6964656e746966696572697075626c69634b6579a36474797065656172726179686d61784974656d73182169627974654172726179f5746164646974696f6e616c50726f70657274696573f46770726f66696c65a56474797065666f626a65637467696e646963657381a3646e616d65676f776e6572496466756e69717565f56a70726f7065727469657381a168246f776e6572496463617363687265717569726564826961766174617255726c6561626f75746a70726f70657274696573a26561626f7574a2647479706566737472696e67696d61784c656e67746818ff6961766174617255726ca3647479706566737472696e6766666f726d61746375726c696d61784c656e67746818ff746164646974696f6e616c50726f70657274696573f4").unwrap();

        let contract = Contract::from_cbor(contract_cbor.as_slice(), None)
            .expect("expected to create contract");
        drive
            .apply_contract_cbor(contract_cbor.clone(), None, 0f64, None)
            .expect("expected to apply contract successfully");

        // Create Alice profile

        let alice_profile_cbor = hex::decode("01000000a763246964582035edfec54aea574df968990abb47b39c206abe5c43a6157885f62958a1f1230c6524747970656770726f66696c656561626f75746a4920616d20416c69636568246f776e65724964582041d52f93f6f7c5af79ce994381c90df73cce2863d3850b9c05ef586ff0fe795f69247265766973696f6e016961766174617255726c7819687474703a2f2f746573742e636f6d2f616c6963652e6a70676f2464617461436f6e747261637449645820b0248cd9a27f86d05badf475dd9ff574d63219cd60c52e2be1e540c2fdd71333").unwrap();

        let alice_profile = Document::from_cbor(alice_profile_cbor.as_slice(), None, None)
            .expect("expected to get a document");

        let document_type = contract
            .document_type_for_name("profile")
            .expect("expected to get a document type");

        drive
            .add_document_for_contract(
                DocumentAndContractInfo {
                    document_info: DocumentInfo::DocumentAndSerialization((
                        &alice_profile,
                        alice_profile_cbor.as_slice(),
                    )),
                    contract: &contract,
                    document_type,
                    owner_id: None,
                },
                true,
                0f64,
                None,
            )
            .expect("should create alice profile");

        let sql_string = "select * from profile";
        let query = DriveQuery::from_sql_expr(sql_string, &contract).expect("should build query");

        let (results_no_transaction, _) = query
            .execute_no_proof(&drive.grove, None)
            .expect("expected to execute query");

        assert_eq!(results_no_transaction.len(), 1);

        // Update Alice profile

        let updated_alice_profile_cbor = hex::decode("01000000a763246964582035edfec54aea574df968990abb47b39c206abe5c43a6157885f62958a1f1230c6524747970656770726f66696c656561626f75746a4920616d20416c69636568246f776e65724964582041d52f93f6f7c5af79ce994381c90df73cce2863d3850b9c05ef586ff0fe795f69247265766973696f6e026961766174617255726c781a687474703a2f2f746573742e636f6d2f616c696365322e6a70676f2464617461436f6e747261637449645820b0248cd9a27f86d05badf475dd9ff574d63219cd60c52e2be1e540c2fdd71333").unwrap();

        drive
            .update_document_for_contract_cbor(
                updated_alice_profile_cbor.as_slice(),
                contract_cbor.as_slice(),
                "profile",
                None,
                0f64,
                None,
            )
            .expect("should update alice profile");

        let (results_no_transaction, _) = query
            .execute_no_proof(&drive.grove, None)
            .expect("expected to execute query");

        assert_eq!(results_no_transaction.len(), 1);
    }

    #[test]
    fn test_create_and_update_document_in_different_transactions() {
        let tmp_dir = TempDir::new().unwrap();
        let drive: Drive = Drive::open(tmp_dir).expect("expected to open Drive successfully");

        let db_transaction = drive.grove.start_transaction();

        drive
            .create_root_tree(Some(&db_transaction))
            .expect("expected to create root tree successfully");

        let contract_cbor = hex::decode("01000000a5632469645820b0248cd9a27f86d05badf475dd9ff574d63219cd60c52e2be1e540c2fdd713336724736368656d61783468747470733a2f2f736368656d612e646173682e6f72672f6470702d302d342d302f6d6574612f646174612d636f6e7472616374676f776e6572496458204c9bf0db6ae315c85465e9ef26e6a006de9673731d08d14881945ddef1b5c5f26776657273696f6e0169646f63756d656e7473a267636f6e74616374a56474797065666f626a65637467696e646963657381a3646e616d656f6f6e7765724964546f55736572496466756e69717565f56a70726f7065727469657382a168246f776e6572496463617363a168746f557365724964636173636872657175697265648268746f557365724964697075626c69634b65796a70726f70657274696573a268746f557365724964a56474797065656172726179686d61784974656d731820686d696e4974656d73182069627974654172726179f570636f6e74656e744d656469615479706578216170706c69636174696f6e2f782e646173682e6470702e6964656e746966696572697075626c69634b6579a36474797065656172726179686d61784974656d73182169627974654172726179f5746164646974696f6e616c50726f70657274696573f46770726f66696c65a56474797065666f626a65637467696e646963657381a3646e616d65676f776e6572496466756e69717565f56a70726f7065727469657381a168246f776e6572496463617363687265717569726564826961766174617255726c6561626f75746a70726f70657274696573a26561626f7574a2647479706566737472696e67696d61784c656e67746818ff6961766174617255726ca3647479706566737472696e6766666f726d61746375726c696d61784c656e67746818ff746164646974696f6e616c50726f70657274696573f4").unwrap();

        let contract = Contract::from_cbor(contract_cbor.as_slice(), None)
            .expect("expected to create contract");
        drive
            .apply_contract_cbor(contract_cbor.clone(), None, 0f64, Some(&db_transaction))
            .expect("expected to apply contract successfully");

        // Create Alice profile

        let alice_profile_cbor = hex::decode("01000000a763246964582035edfec54aea574df968990abb47b39c206abe5c43a6157885f62958a1f1230c6524747970656770726f66696c656561626f75746a4920616d20416c69636568246f776e65724964582041d52f93f6f7c5af79ce994381c90df73cce2863d3850b9c05ef586ff0fe795f69247265766973696f6e016961766174617255726c7819687474703a2f2f746573742e636f6d2f616c6963652e6a70676f2464617461436f6e747261637449645820b0248cd9a27f86d05badf475dd9ff574d63219cd60c52e2be1e540c2fdd71333").unwrap();

        let alice_profile = Document::from_cbor(alice_profile_cbor.as_slice(), None, None)
            .expect("expected to get a document");

        let document_type = contract
            .document_type_for_name("profile")
            .expect("expected to get a document type");

        drive
            .add_document_for_contract(
                DocumentAndContractInfo {
                    document_info: DocumentInfo::DocumentAndSerialization((
                        &alice_profile,
                        alice_profile_cbor.as_slice(),
                    )),
                    contract: &contract,
                    document_type,
                    owner_id: None,
                },
                true,
                0f64,
                Some(&db_transaction),
            )
            .expect("should create alice profile");

        drive
            .grove
            .commit_transaction(db_transaction)
            .expect("should commit transaction");

        let sql_string = "select * from profile";
        let query = DriveQuery::from_sql_expr(sql_string, &contract).expect("should build query");

        let (results_no_transaction, _) = query
            .execute_no_proof(&drive.grove, None)
            .expect("expected to execute query");

        assert_eq!(results_no_transaction.len(), 1);

        let db_transaction = drive.grove.start_transaction();

        let (results_on_transaction, _) = query
            .execute_no_proof(&drive.grove, Some(&db_transaction))
            .expect("expected to execute query");

        assert_eq!(results_on_transaction.len(), 1);

        // Update Alice profile

        let updated_alice_profile_cbor = hex::decode("01000000a763246964582035edfec54aea574df968990abb47b39c206abe5c43a6157885f62958a1f1230c6524747970656770726f66696c656561626f75746a4920616d20416c69636568246f776e65724964582041d52f93f6f7c5af79ce994381c90df73cce2863d3850b9c05ef586ff0fe795f69247265766973696f6e026961766174617255726c781a687474703a2f2f746573742e636f6d2f616c696365322e6a70676f2464617461436f6e747261637449645820b0248cd9a27f86d05badf475dd9ff574d63219cd60c52e2be1e540c2fdd71333").unwrap();

        drive
            .update_document_for_contract_cbor(
                updated_alice_profile_cbor.as_slice(),
                contract_cbor.as_slice(),
                "profile",
                None,
                0f64,
                Some(&db_transaction),
            )
            .expect("should update alice profile");

        let (results_on_transaction, _) = query
            .execute_no_proof(&drive.grove, Some(&db_transaction))
            .expect("expected to execute query");

        assert_eq!(results_on_transaction.len(), 1);

        drive
            .grove
            .commit_transaction(db_transaction)
            .expect("should commit transaction");
    }

    #[test]
    fn test_create_and_update_document_in_different_transactions_with_delete_rollback() {
        let tmp_dir = TempDir::new().unwrap();
        let drive: Drive = Drive::open(tmp_dir).expect("expected to open Drive successfully");

        let db_transaction = drive.grove.start_transaction();

        drive
            .create_root_tree(Some(&db_transaction))
            .expect("expected to create root tree successfully");

        let contract_cbor = hex::decode("01000000a5632469645820b0248cd9a27f86d05badf475dd9ff574d63219cd60c52e2be1e540c2fdd713336724736368656d61783468747470733a2f2f736368656d612e646173682e6f72672f6470702d302d342d302f6d6574612f646174612d636f6e7472616374676f776e6572496458204c9bf0db6ae315c85465e9ef26e6a006de9673731d08d14881945ddef1b5c5f26776657273696f6e0169646f63756d656e7473a267636f6e74616374a56474797065666f626a65637467696e646963657381a3646e616d656f6f6e7765724964546f55736572496466756e69717565f56a70726f7065727469657382a168246f776e6572496463617363a168746f557365724964636173636872657175697265648268746f557365724964697075626c69634b65796a70726f70657274696573a268746f557365724964a56474797065656172726179686d61784974656d731820686d696e4974656d73182069627974654172726179f570636f6e74656e744d656469615479706578216170706c69636174696f6e2f782e646173682e6470702e6964656e746966696572697075626c69634b6579a36474797065656172726179686d61784974656d73182169627974654172726179f5746164646974696f6e616c50726f70657274696573f46770726f66696c65a56474797065666f626a65637467696e646963657381a3646e616d65676f776e6572496466756e69717565f56a70726f7065727469657381a168246f776e6572496463617363687265717569726564826961766174617255726c6561626f75746a70726f70657274696573a26561626f7574a2647479706566737472696e67696d61784c656e67746818ff6961766174617255726ca3647479706566737472696e6766666f726d61746375726c696d61784c656e67746818ff746164646974696f6e616c50726f70657274696573f4").unwrap();

        let contract = Contract::from_cbor(contract_cbor.as_slice(), None)
            .expect("expected to create contract");
        drive
            .apply_contract_cbor(contract_cbor.clone(), None, 0f64, Some(&db_transaction))
            .expect("expected to apply contract successfully");

        // Create Alice profile

        let alice_profile_cbor = hex::decode("01000000a763246964582035edfec54aea574df968990abb47b39c206abe5c43a6157885f62958a1f1230c6524747970656770726f66696c656561626f75746a4920616d20416c69636568246f776e65724964582041d52f93f6f7c5af79ce994381c90df73cce2863d3850b9c05ef586ff0fe795f69247265766973696f6e016961766174617255726c7819687474703a2f2f746573742e636f6d2f616c6963652e6a70676f2464617461436f6e747261637449645820b0248cd9a27f86d05badf475dd9ff574d63219cd60c52e2be1e540c2fdd71333").unwrap();

        let alice_profile = Document::from_cbor(alice_profile_cbor.as_slice(), None, None)
            .expect("expected to get a document");

        let document_type = contract
            .document_type_for_name("profile")
            .expect("expected to get a document type");

        drive
            .add_document_for_contract(
                DocumentAndContractInfo {
                    document_info: DocumentInfo::DocumentAndSerialization((
                        &alice_profile,
                        alice_profile_cbor.as_slice(),
                    )),
                    contract: &contract,
                    document_type,
                    owner_id: None,
                },
                true,
                0f64,
                Some(&db_transaction),
            )
            .expect("should create alice profile");

        drive
            .grove
            .commit_transaction(db_transaction)
            .expect("should commit transaction");

        let sql_string = "select * from profile";
        let query = DriveQuery::from_sql_expr(sql_string, &contract).expect("should build query");

        let (results_no_transaction, _) = query
            .execute_no_proof(&drive.grove, None)
            .expect("expected to execute query");

        assert_eq!(results_no_transaction.len(), 1);

        let db_transaction = drive.grove.start_transaction();

        let (results_on_transaction, _) = query
            .execute_no_proof(&drive.grove, Some(&db_transaction))
            .expect("expected to execute query");

        assert_eq!(results_on_transaction.len(), 1);

        drive
            .delete_document_for_contract(
                &alice_profile.id,
                &contract,
                "profile",
                None,
                Some(&db_transaction),
            )
            .expect("expected to delete document");

        let (results_on_transaction, _) = query
            .execute_no_proof(&drive.grove, Some(&db_transaction))
            .expect("expected to execute query");

        assert_eq!(results_on_transaction.len(), 0);

        drive
            .grove
            .rollback_transaction(&db_transaction)
            .expect("expected to rollback transaction");

        let (results_on_transaction, _) = query
            .execute_no_proof(&drive.grove, Some(&db_transaction))
            .expect("expected to execute query");

        assert_eq!(results_on_transaction.len(), 1);

        // Update Alice profile

        let updated_alice_profile_cbor = hex::decode("01000000a763246964582035edfec54aea574df968990abb47b39c206abe5c43a6157885f62958a1f1230c6524747970656770726f66696c656561626f75746a4920616d20416c69636568246f776e65724964582041d52f93f6f7c5af79ce994381c90df73cce2863d3850b9c05ef586ff0fe795f69247265766973696f6e026961766174617255726c781a687474703a2f2f746573742e636f6d2f616c696365322e6a70676f2464617461436f6e747261637449645820b0248cd9a27f86d05badf475dd9ff574d63219cd60c52e2be1e540c2fdd71333").unwrap();

        drive
            .update_document_for_contract_cbor(
                updated_alice_profile_cbor.as_slice(),
                contract_cbor.as_slice(),
                "profile",
                None,
                0f64,
                Some(&db_transaction),
            )
            .expect("should update alice profile");
    }

    #[test]
    fn test_create_two_documents_with_the_same_index_in_different_transactions() {
        let tmp_dir = TempDir::new().unwrap();
        let drive: Drive = Drive::open(tmp_dir).expect("expected to open Drive successfully");

        let db_transaction = drive.grove.start_transaction();

        drive
            .create_root_tree(Some(&db_transaction))
            .expect("expected to create root tree successfully");

        let contract_cbor = hex::decode("01000000a5632469645820e668c659af66aee1e72c186dde7b5b7e0a1d712a09c40d5721f622bf53c531556724736368656d61783468747470733a2f2f736368656d612e646173682e6f72672f6470702d302d342d302f6d6574612f646174612d636f6e7472616374676f776e6572496458203012c19b98ec0033addb36cd64b7f510670f2a351a4304b5f6994144286efdac6776657273696f6e0169646f63756d656e7473a266646f6d61696ea66474797065666f626a65637467696e646963657383a3646e616d6572706172656e744e616d65416e644c6162656c66756e69717565f56a70726f7065727469657382a1781a6e6f726d616c697a6564506172656e74446f6d61696e4e616d6563617363a16f6e6f726d616c697a65644c6162656c63617363a3646e616d656e646173684964656e74697479496466756e69717565f56a70726f7065727469657381a1781c7265636f7264732e64617368556e697175654964656e74697479496463617363a2646e616d656964617368416c6961736a70726f7065727469657381a1781b7265636f7264732e64617368416c6961734964656e746974794964636173636824636f6d6d656e74790137496e206f7264657220746f207265676973746572206120646f6d61696e20796f75206e65656420746f206372656174652061207072656f726465722e20546865207072656f726465722073746570206973206e656564656420746f2070726576656e74206d616e2d696e2d7468652d6d6964646c652061747461636b732e206e6f726d616c697a65644c6162656c202b20272e27202b206e6f726d616c697a6564506172656e74446f6d61696e206d757374206e6f74206265206c6f6e676572207468616e20323533206368617273206c656e67746820617320646566696e65642062792052464320313033352e20446f6d61696e20646f63756d656e74732061726520696d6d757461626c653a206d6f64696669636174696f6e20616e642064656c6574696f6e20617265207265737472696374656468726571756972656486656c6162656c6f6e6f726d616c697a65644c6162656c781a6e6f726d616c697a6564506172656e74446f6d61696e4e616d656c7072656f7264657253616c74677265636f7264736e737562646f6d61696e52756c65736a70726f70657274696573a6656c6162656ca5647479706566737472696e67677061747465726e782a5e5b612d7a412d5a302d395d5b612d7a412d5a302d392d5d7b302c36317d5b612d7a412d5a302d395d24696d61784c656e677468183f696d696e4c656e677468036b6465736372697074696f6e7819446f6d61696e206c6162656c2e20652e672e2027426f62272e677265636f726473a66474797065666f626a6563746824636f6d6d656e747890436f6e73747261696e742077697468206d617820616e64206d696e2070726f7065727469657320656e737572652074686174206f6e6c79206f6e65206964656e74697479207265636f72642069732075736564202d206569746865722061206064617368556e697175654964656e74697479496460206f722061206064617368416c6961734964656e746974794964606a70726f70657274696573a27364617368416c6961734964656e746974794964a764747970656561727261796824636f6d6d656e7478234d75737420626520657175616c20746f2074686520646f63756d656e74206f776e6572686d61784974656d731820686d696e4974656d73182069627974654172726179f56b6465736372697074696f6e783d4964656e7469747920494420746f206265207573656420746f2063726561746520616c696173206e616d657320666f7220746865204964656e7469747970636f6e74656e744d656469615479706578216170706c69636174696f6e2f782e646173682e6470702e6964656e7469666965727464617368556e697175654964656e746974794964a764747970656561727261796824636f6d6d656e7478234d75737420626520657175616c20746f2074686520646f63756d656e74206f776e6572686d61784974656d731820686d696e4974656d73182069627974654172726179f56b6465736372697074696f6e783e4964656e7469747920494420746f206265207573656420746f2063726561746520746865207072696d617279206e616d6520746865204964656e7469747970636f6e74656e744d656469615479706578216170706c69636174696f6e2f782e646173682e6470702e6964656e7469666965726d6d617850726f70657274696573016d6d696e50726f7065727469657301746164646974696f6e616c50726f70657274696573f46c7072656f7264657253616c74a56474797065656172726179686d61784974656d731820686d696e4974656d73182069627974654172726179f56b6465736372697074696f6e782253616c74207573656420696e20746865207072656f7264657220646f63756d656e746e737562646f6d61696e52756c6573a56474797065666f626a656374687265717569726564816f616c6c6f77537562646f6d61696e736a70726f70657274696573a16f616c6c6f77537562646f6d61696e73a3647479706567626f6f6c65616e6824636f6d6d656e74784f4f6e6c792074686520646f6d61696e206f776e657220697320616c6c6f77656420746f2063726561746520737562646f6d61696e7320666f72206e6f6e20746f702d6c6576656c20646f6d61696e736b6465736372697074696f6e785b54686973206f7074696f6e20646566696e65732077686f2063616e2063726561746520737562646f6d61696e733a2074727565202d20616e796f6e653b2066616c7365202d206f6e6c792074686520646f6d61696e206f776e65726b6465736372697074696f6e7842537562646f6d61696e2072756c657320616c6c6f7720646f6d61696e206f776e65727320746f20646566696e652072756c657320666f7220737562646f6d61696e73746164646974696f6e616c50726f70657274696573f46f6e6f726d616c697a65644c6162656ca5647479706566737472696e67677061747465726e78215e5b612d7a302d395d5b612d7a302d392d5d7b302c36317d5b612d7a302d395d246824636f6d6d656e7478694d75737420626520657175616c20746f20746865206c6162656c20696e206c6f776572636173652e20546869732070726f70657274792077696c6c20626520646570726563617465642064756520746f206361736520696e73656e73697469766520696e6469636573696d61784c656e677468183f6b6465736372697074696f6e7850446f6d61696e206c6162656c20696e206c6f7765726361736520666f7220636173652d696e73656e73697469766520756e697175656e6573732076616c69646174696f6e2e20652e672e2027626f6227781a6e6f726d616c697a6564506172656e74446f6d61696e4e616d65a6647479706566737472696e67677061747465726e78285e247c5e5b5b612d7a302d395d5b612d7a302d392d5c2e5d7b302c3138387d5b612d7a302d395d246824636f6d6d656e74788c4d7573742065697468657220626520657175616c20746f20616e206578697374696e6720646f6d61696e206f7220656d70747920746f20637265617465206120746f70206c6576656c20646f6d61696e2e204f6e6c7920746865206461746120636f6e7472616374206f776e65722063616e2063726561746520746f70206c6576656c20646f6d61696e732e696d61784c656e67746818be696d696e4c656e677468006b6465736372697074696f6e785e412066756c6c20706172656e7420646f6d61696e206e616d6520696e206c6f7765726361736520666f7220636173652d696e73656e73697469766520756e697175656e6573732076616c69646174696f6e2e20652e672e20276461736827746164646974696f6e616c50726f70657274696573f4687072656f72646572a66474797065666f626a65637467696e646963657381a3646e616d656a73616c7465644861736866756e69717565f56a70726f7065727469657381a17073616c746564446f6d61696e48617368636173636824636f6d6d656e74784a5072656f7264657220646f63756d656e74732061726520696d6d757461626c653a206d6f64696669636174696f6e20616e642064656c6574696f6e206172652072657374726963746564687265717569726564817073616c746564446f6d61696e486173686a70726f70657274696573a17073616c746564446f6d61696e48617368a56474797065656172726179686d61784974656d731820686d696e4974656d73182069627974654172726179f56b6465736372697074696f6e7859446f75626c65207368612d323536206f662074686520636f6e636174656e6174696f6e206f66206120333220627974652072616e646f6d2073616c7420616e642061206e6f726d616c697a656420646f6d61696e206e616d65746164646974696f6e616c50726f70657274696573f4").unwrap();

        drive
            .apply_contract_cbor(contract_cbor.clone(), None, 0f64, Some(&db_transaction))
            .expect("expected to apply contract successfully");

        // Create dash TLD

        let dash_tld_cbor = hex::decode("01000000ac632469645820d7f2c53f46a917ab6e5b39a2d7bc260b649289453744d1e0d4f26a8d8eff37cf65247479706566646f6d61696e656c6162656c6464617368677265636f726473a17364617368416c6961734964656e74697479496458203012c19b98ec0033addb36cd64b7f510670f2a351a4304b5f6994144286efdac68246f776e6572496458203012c19b98ec0033addb36cd64b7f510670f2a351a4304b5f6994144286efdac69247265766973696f6e016a246372656174656441741b0000017f07c861586c7072656f7264657253616c745820e0b508c5a36825a206693a1f414aa13edbecf43c41e3c799ea9e737b4f9aa2266e737562646f6d61696e52756c6573a16f616c6c6f77537562646f6d61696e73f56f2464617461436f6e747261637449645820e668c659af66aee1e72c186dde7b5b7e0a1d712a09c40d5721f622bf53c531556f6e6f726d616c697a65644c6162656c6464617368781a6e6f726d616c697a6564506172656e74446f6d61696e4e616d6560").unwrap();

        drive
            .add_document_for_contract_cbor(
                dash_tld_cbor.as_slice(),
                contract_cbor.as_slice(),
                "domain",
                None,
                true,
                0f64,
                Some(&db_transaction),
            )
            .expect("should create dash tld");

        drive
            .grove
            .commit_transaction(db_transaction)
            .expect("should commit transaction");

        let db_transaction = drive.grove.start_transaction();

        // add random TLD

        let random_tld_cbor = hex::decode("01000000ab632469645820655c9b5606f4ad53daea90de9c540aad656ed5fbe5fb14b40700f6f56dc793ac65247479706566646f6d61696e656c6162656c746433653966343532373963343865306261363561677265636f726473a17364617368416c6961734964656e74697479496458203012c19b98ec0033addb36cd64b7f510670f2a351a4304b5f6994144286efdac68246f776e6572496458203012c19b98ec0033addb36cd64b7f510670f2a351a4304b5f6994144286efdac69247265766973696f6e016c7072656f7264657253616c745820219353a923a29cd02c521b141f326ac0d12c362a84f1979a5de89b8dba12891b6e737562646f6d61696e52756c6573a16f616c6c6f77537562646f6d61696e73f56f2464617461436f6e747261637449645820e668c659af66aee1e72c186dde7b5b7e0a1d712a09c40d5721f622bf53c531556f6e6f726d616c697a65644c6162656c746433653966343532373963343865306261363561781a6e6f726d616c697a6564506172656e74446f6d61696e4e616d6560").unwrap();

        drive
            .add_document_for_contract_cbor(
                random_tld_cbor.as_slice(),
                contract_cbor.as_slice(),
                "domain",
                None,
                true,
                0f64,
                Some(&db_transaction),
            )
            .expect("should add random tld");
    }

    #[test]
    fn test_create_and_update_contract() {
        let tmp_dir = TempDir::new().unwrap();
        let drive: Drive = Drive::open(tmp_dir).expect("expected to open Drive successfully");

        drive
            .create_root_tree(None)
            .expect("expected to create root tree successfully");

        let initial_contract_cbor = hex::decode("01000000a66324696458209c2b800c5ea525d032a9fda4dda22a896f1e763af5f0e15ae7f93882b7439d77652464656673a1686c6173744e616d65a1647479706566737472696e676724736368656d61783468747470733a2f2f736368656d612e646173682e6f72672f6470702d302d342d302f6d6574612f646174612d636f6e7472616374676f776e657249645820636d3188dfffe62efb10e20347ec6c41b3e49fa31cb757ef4bad6cd8f1c7f4b66776657273696f6e0169646f63756d656e7473a76b756e697175654461746573a56474797065666f626a65637467696e646963657382a3646e616d6566696e6465783166756e69717565f56a70726f7065727469657382a16a2463726561746564417463617363a16a2475706461746564417463617363a2646e616d6566696e646578326a70726f7065727469657381a16a2475706461746564417463617363687265717569726564836966697273744e616d656a246372656174656441746a247570646174656441746a70726f70657274696573a2686c6173744e616d65a1647479706566737472696e676966697273744e616d65a1647479706566737472696e67746164646974696f6e616c50726f70657274696573f46c6e696365446f63756d656e74a46474797065666f626a656374687265717569726564816a246372656174656441746a70726f70657274696573a1646e616d65a1647479706566737472696e67746164646974696f6e616c50726f70657274696573f46e6e6f54696d65446f63756d656e74a36474797065666f626a6563746a70726f70657274696573a1646e616d65a1647479706566737472696e67746164646974696f6e616c50726f70657274696573f46e707265747479446f63756d656e74a46474797065666f626a65637468726571756972656482686c6173744e616d656a247570646174656441746a70726f70657274696573a1686c6173744e616d65a1642472656670232f24646566732f6c6173744e616d65746164646974696f6e616c50726f70657274696573f46e7769746842797465417272617973a56474797065666f626a65637467696e646963657381a2646e616d6566696e646578316a70726f7065727469657381a16e6279746541727261794669656c6463617363687265717569726564816e6279746541727261794669656c646a70726f70657274696573a26e6279746541727261794669656c64a36474797065656172726179686d61784974656d731069627974654172726179f56f6964656e7469666965724669656c64a56474797065656172726179686d61784974656d731820686d696e4974656d73182069627974654172726179f570636f6e74656e744d656469615479706578216170706c69636174696f6e2f782e646173682e6470702e6964656e746966696572746164646974696f6e616c50726f70657274696573f46f696e6465786564446f63756d656e74a56474797065666f626a65637467696e646963657386a3646e616d6566696e6465783166756e69717565f56a70726f7065727469657382a168246f776e6572496463617363a16966697273744e616d656464657363a3646e616d6566696e6465783266756e69717565f56a70726f7065727469657382a168246f776e6572496463617363a1686c6173744e616d656464657363a2646e616d6566696e646578336a70726f7065727469657381a1686c6173744e616d6563617363a2646e616d6566696e646578346a70726f7065727469657382a16a2463726561746564417463617363a16a2475706461746564417463617363a2646e616d6566696e646578356a70726f7065727469657381a16a2475706461746564417463617363a2646e616d6566696e646578366a70726f7065727469657381a16a2463726561746564417463617363687265717569726564846966697273744e616d656a246372656174656441746a24757064617465644174686c6173744e616d656a70726f70657274696573a2686c6173744e616d65a2647479706566737472696e67696d61784c656e6774681901006966697273744e616d65a2647479706566737472696e67696d61784c656e677468190100746164646974696f6e616c50726f70657274696573f4781d6f7074696f6e616c556e69717565496e6465786564446f63756d656e74a56474797065666f626a65637467696e646963657383a3646e616d6566696e6465783166756e69717565f56a70726f7065727469657381a16966697273744e616d656464657363a3646e616d6566696e6465783266756e69717565f56a70726f7065727469657383a168246f776e6572496463617363a16966697273744e616d6563617363a1686c6173744e616d6563617363a3646e616d6566696e6465783366756e69717565f56a70726f7065727469657382a167636f756e74727963617363a1646369747963617363687265717569726564826966697273744e616d65686c6173744e616d656a70726f70657274696573a46463697479a2647479706566737472696e67696d61784c656e67746819010067636f756e747279a2647479706566737472696e67696d61784c656e677468190100686c6173744e616d65a2647479706566737472696e67696d61784c656e6774681901006966697273744e616d65a2647479706566737472696e67696d61784c656e677468190100746164646974696f6e616c50726f70657274696573f4").unwrap();

        drive
            .apply_contract_cbor(initial_contract_cbor, None, 0f64, None)
            .expect("expected to apply contract successfully");

        let updated_contract_cbor = hex::decode("01000000a66324696458209c2b800c5ea525d032a9fda4dda22a896f1e763af5f0e15ae7f93882b7439d77652464656673a1686c6173744e616d65a1647479706566737472696e676724736368656d61783468747470733a2f2f736368656d612e646173682e6f72672f6470702d302d342d302f6d6574612f646174612d636f6e7472616374676f776e657249645820636d3188dfffe62efb10e20347ec6c41b3e49fa31cb757ef4bad6cd8f1c7f4b66776657273696f6e0269646f63756d656e7473a86b756e697175654461746573a56474797065666f626a65637467696e646963657382a3646e616d6566696e6465783166756e69717565f56a70726f7065727469657382a16a2463726561746564417463617363a16a2475706461746564417463617363a2646e616d6566696e646578326a70726f7065727469657381a16a2475706461746564417463617363687265717569726564836966697273744e616d656a246372656174656441746a247570646174656441746a70726f70657274696573a2686c6173744e616d65a1647479706566737472696e676966697273744e616d65a1647479706566737472696e67746164646974696f6e616c50726f70657274696573f46c6e696365446f63756d656e74a46474797065666f626a656374687265717569726564816a246372656174656441746a70726f70657274696573a1646e616d65a1647479706566737472696e67746164646974696f6e616c50726f70657274696573f46e6e6f54696d65446f63756d656e74a36474797065666f626a6563746a70726f70657274696573a1646e616d65a1647479706566737472696e67746164646974696f6e616c50726f70657274696573f46e707265747479446f63756d656e74a46474797065666f626a65637468726571756972656482686c6173744e616d656a247570646174656441746a70726f70657274696573a1686c6173744e616d65a1642472656670232f24646566732f6c6173744e616d65746164646974696f6e616c50726f70657274696573f46e7769746842797465417272617973a56474797065666f626a65637467696e646963657381a2646e616d6566696e646578316a70726f7065727469657381a16e6279746541727261794669656c6463617363687265717569726564816e6279746541727261794669656c646a70726f70657274696573a26e6279746541727261794669656c64a36474797065656172726179686d61784974656d731069627974654172726179f56f6964656e7469666965724669656c64a56474797065656172726179686d61784974656d731820686d696e4974656d73182069627974654172726179f570636f6e74656e744d656469615479706578216170706c69636174696f6e2f782e646173682e6470702e6964656e746966696572746164646974696f6e616c50726f70657274696573f46f696e6465786564446f63756d656e74a56474797065666f626a65637467696e646963657386a3646e616d6566696e6465783166756e69717565f56a70726f7065727469657382a168246f776e6572496463617363a16966697273744e616d656464657363a3646e616d6566696e6465783266756e69717565f56a70726f7065727469657382a168246f776e6572496463617363a1686c6173744e616d656464657363a2646e616d6566696e646578336a70726f7065727469657381a1686c6173744e616d6563617363a2646e616d6566696e646578346a70726f7065727469657382a16a2463726561746564417463617363a16a2475706461746564417463617363a2646e616d6566696e646578356a70726f7065727469657381a16a2475706461746564417463617363a2646e616d6566696e646578366a70726f7065727469657381a16a2463726561746564417463617363687265717569726564846966697273744e616d656a246372656174656441746a24757064617465644174686c6173744e616d656a70726f70657274696573a2686c6173744e616d65a2647479706566737472696e67696d61784c656e6774681901006966697273744e616d65a2647479706566737472696e67696d61784c656e677468190100746164646974696f6e616c50726f70657274696573f4716d79417765736f6d65446f63756d656e74a56474797065666f626a65637467696e646963657382a3646e616d656966697273744e616d6566756e69717565f56a70726f7065727469657381a16966697273744e616d6563617363a3646e616d657166697273744e616d654c6173744e616d6566756e69717565f56a70726f7065727469657382a16966697273744e616d6563617363a1686c6173744e616d6563617363687265717569726564846966697273744e616d656a246372656174656441746a24757064617465644174686c6173744e616d656a70726f70657274696573a2686c6173744e616d65a2647479706566737472696e67696d61784c656e6774681901006966697273744e616d65a2647479706566737472696e67696d61784c656e677468190100746164646974696f6e616c50726f70657274696573f4781d6f7074696f6e616c556e69717565496e6465786564446f63756d656e74a56474797065666f626a65637467696e646963657383a3646e616d6566696e6465783166756e69717565f56a70726f7065727469657381a16966697273744e616d656464657363a3646e616d6566696e6465783266756e69717565f56a70726f7065727469657383a168246f776e6572496463617363a16966697273744e616d6563617363a1686c6173744e616d6563617363a3646e616d6566696e6465783366756e69717565f56a70726f7065727469657382a167636f756e74727963617363a1646369747963617363687265717569726564826966697273744e616d65686c6173744e616d656a70726f70657274696573a46463697479a2647479706566737472696e67696d61784c656e67746819010067636f756e747279a2647479706566737472696e67696d61784c656e677468190100686c6173744e616d65a2647479706566737472696e67696d61784c656e6774681901006966697273744e616d65a2647479706566737472696e67696d61784c656e677468190100746164646974696f6e616c50726f70657274696573f4").unwrap();

        drive
            .apply_contract_cbor(updated_contract_cbor, None, 0f64, None)
            .expect("should update initial contract");
    }

    #[test]
    fn test_create_deep_nested_contract_50() {
        let (drive, contract, contract_cbor) = setup_deep_nested_contract();

        let document_type = contract
            .document_type_for_name("nest")
            .expect("expected to get document type");

        let document = document_type.random_document(Some(5));

        let nested_value = document.properties.get("abc0.abc0.abc0.abc0.abc0.abc0.abc0.abc0.abc0.abc0.abc0.abc0.abc0.abc0.abc0.abc0.abc0.abc0.abc0.abc0.abc0.abc0.abc0.abc0.abc0.abc0.abc0.abc0.abc0.abc0.abc0.abc0.abc0.abc0.abc0.abc0.abc0.abc0.abc0.abc0.abc0.abc0.abc0.abc0.abc0.abc0.abc0.abc0.abc0.abc0");

        assert!(nested_value.is_some());

        let random_owner_id = rand::thread_rng().gen::<[u8; 32]>();
        drive
            .add_document_for_contract(
                DocumentAndContractInfo {
                    document_info: DocumentInfo::DocumentAndSerialization((
                        &document,
                        document.to_cbor().as_slice(),
                    )),
                    contract: &contract,
                    document_type,
                    owner_id: Some(&random_owner_id),
                },
                false,
                0f64,
                None,
            )
            .expect("expected to insert a document successfully");
    }

    #[test]
    fn test_create_reference_contract() {
        let (drive, contract, contract_cbor) = setup_reference_contract();

        let document_type = contract
            .document_type_for_name("note")
            .expect("expected to get document type");

        let document = document_type.random_document(Some(5));

        let ref_value = document.properties.get("abc17");

        assert!(ref_value.is_some());

        let random_owner_id = rand::thread_rng().gen::<[u8; 32]>();
        drive
            .add_document_for_contract(
                DocumentAndContractInfo {
                    document_info: DocumentInfo::DocumentAndSerialization((
                        &document,
                        document.to_cbor().as_slice(),
                    )),
                    contract: &contract,
                    document_type,
                    owner_id: Some(&random_owner_id),
                },
                false,
                0f64,
                None,
            )
            .expect("expected to insert a document successfully");
    }

    #[test]
    fn test_create_update_and_delete_document() {
        let tmp_dir = TempDir::new().unwrap();
        let drive: Drive = Drive::open(tmp_dir).expect("expected to open Drive successfully");

        drive
            .create_root_tree(None)
            .expect("should create root tree");

        let contract = json!({
            "protocolVersion": 1,
            "$id": "BZUodcFoFL6KvnonehrnMVggTvCe8W5MiRnZuqLb6M54",
            "$schema": "https://schema.dash.org/dpp-0-4-0/meta/data-contract",
            "version": 1,
            "ownerId": "GZVdTnLFAN2yE9rLeCHBDBCr7YQgmXJuoExkY347j7Z5",
            "documents": {
                "indexedDocument": {
                    "type": "object",
                    "indices": [
                        {"name":"index1", "properties": [{"$ownerId":"asc"}, {"firstName":"desc"}], "unique":true},
                        {"name":"index2", "properties": [{"$ownerId":"asc"}, {"lastName":"desc"}], "unique":true},
                        {"name":"index3", "properties": [{"lastName":"asc"}]},
                        {"name":"index4", "properties": [{"$createdAt":"asc"}, {"$updatedAt":"asc"}]},
                        {"name":"index5", "properties": [{"$updatedAt":"asc"}]},
                        {"name":"index6", "properties": [{"$createdAt":"asc"}]}
                    ],
                    "properties":{
                        "firstName": {
                            "type": "string",
                            "maxLength": 63,
                        },
                        "lastName": {
                            "type": "string",
                            "maxLength": 63,
                        }
                    },
                    "required": ["firstName", "$createdAt", "$updatedAt", "lastName"],
                    "additionalProperties": false,
                },
            },
        });

        let contract = value_to_cbor(contract, Some(defaults::PROTOCOL_VERSION));

        drive
            .apply_contract_cbor(contract.clone(), None, 0f64, None)
            .expect("should create a contract");

        // Create document

        let document = json!({
           "$protocolVersion": 1,
           "$id": "DLRWw2eRbLAW5zDU2c7wwsSFQypTSZPhFYzpY48tnaXN",
           "$type": "indexedDocument",
           "$dataContractId": "BZUodcFoFL6KvnonehrnMVggTvCe8W5MiRnZuqLb6M54",
           "$ownerId": "GZVdTnLFAN2yE9rLeCHBDBCr7YQgmXJuoExkY347j7Z5",
           "$revision": 1,
           "firstName": "myName",
           "lastName": "lastName",
           "$createdAt":1647535750329 as u64,
           "$updatedAt":1647535750329 as u64,
        });

        let document_cbor = value_to_cbor(document, Some(defaults::PROTOCOL_VERSION));

        drive
            .add_document_for_contract_cbor(
                document_cbor.as_slice(),
                &contract.as_slice(),
                "indexedDocument",
                None,
                true,
                0f64,
                None,
            )
            .expect("should add document");

        // Update document

        let document = json!({
           "$protocolVersion": 1,
           "$id": "DLRWw2eRbLAW5zDU2c7wwsSFQypTSZPhFYzpY48tnaXN",
           "$type": "indexedDocument",
           "$dataContractId": "BZUodcFoFL6KvnonehrnMVggTvCe8W5MiRnZuqLb6M54",
           "$ownerId": "GZVdTnLFAN2yE9rLeCHBDBCr7YQgmXJuoExkY347j7Z5",
           "$revision": 2,
           "firstName": "updatedName",
           "lastName": "lastName",
           "$createdAt":1647535750329 as u64,
           "$updatedAt":1647535754556 as u64,
        });

        let document_cbor = value_to_cbor(document, Some(defaults::PROTOCOL_VERSION));

        drive
            .update_document_for_contract_cbor(
                document_cbor.as_slice(),
                &contract.as_slice(),
                "indexedDocument",
                None,
                0f64,
                None,
            )
            .expect("should update document");

        let document_id = bs58::decode("DLRWw2eRbLAW5zDU2c7wwsSFQypTSZPhFYzpY48tnaXN")
            .into_vec()
            .expect("should decode base58");

        // Delete document

        drive
            .delete_document_for_contract_cbor(
                document_id.as_slice(),
                &contract,
                "indexedDocument",
                None,
                None,
            )
            .expect("should delete document");
    }

    #[test]
    fn store_document_1() {
        let tmp_dir = TempDir::new().unwrap();
        let _drive = Drive::open(tmp_dir);
    }

    #[test]
    fn test_cbor_deserialization() {
        let document_cbor = json_document_to_cbor("simple.json", Some(1));
        let (version, read_document_cbor) = document_cbor.split_at(4);
        assert!(Drive::check_protocol_version_bytes(version));
        let document: HashMap<String, ciborium::value::Value> =
            ciborium::de::from_reader(read_document_cbor).expect("cannot deserialize cbor");
        assert!(document.get("a").is_some());
        let tmp_dir = TempDir::new().unwrap();
        let _drive = Drive::open(tmp_dir);
    }
}
