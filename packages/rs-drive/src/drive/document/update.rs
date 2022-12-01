// MIT LICENSE
//
// Copyright (c) 2021 Dash Core Group
//
// Permission is hereby granted, free of charge, to any
// person obtaining a copy of this software and associated
// documentation files (the "Software"), to deal in the
// Software without restriction, including without
// limitation the rights to use, copy, modify, merge,
// publish, distribute, sublicense, and/or sell copies of
// the Software, and to permit persons to whom the Software
// is furnished to do so, subject to the following
// conditions:
//
// The above copyright notice and this permission notice
// shall be included in all copies or substantial portions
// of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF
// ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED
// TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A
// PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT
// SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY
// CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION
// OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR
// IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
// DEALINGS IN THE SOFTWARE.
//

//! Update Documents.
//!
//! This modules implements functions in Drive relevant to updating Documents.
//!

use std::collections::HashSet;

use grovedb::{Element, TransactionArg};

use crate::contract::document::Document;
use crate::contract::Contract;
use crate::drive::defaults::{CONTRACT_DOCUMENTS_PATH_HEIGHT, SOME_OPTIMIZED_DOCUMENT_REFERENCE};
use crate::drive::document::{
    contract_document_type_path,
    contract_documents_keeping_history_primary_key_path_for_document_id,
    contract_documents_primary_key_path, make_document_reference,
};
use crate::drive::flags::StorageFlags;
use crate::drive::object_size_info::DocumentInfo::{
    DocumentRefAndSerialization, DocumentSize, DocumentWithoutSerialization,
};
use crate::drive::object_size_info::KeyValueInfo::KeyRefRequest;
use crate::drive::object_size_info::PathKeyElementInfo::PathKeyElement;
use crate::drive::object_size_info::{DocumentAndContractInfo, DriveKeyInfo, PathKeyInfo};
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fee::op::DriveOperation;
use crate::fee::{calculate_fee, FeeResult};

use crate::drive::block_info::BlockInfo;
use crate::error::document::DocumentError;
use dpp::data_contract::extra::DriveContractExt;

impl Drive {
    /// Updates a serialized document given a contract CBOR and returns the associated fee.
    pub fn update_document_for_contract_cbor(
        &self,
        serialized_document: &[u8],
        contract_cbor: &[u8],
        document_type: &str,
        owner_id: Option<[u8; 32]>,
        block_info: BlockInfo,
        apply: bool,
        storage_flags: Option<&StorageFlags>,
        transaction: TransactionArg,
    ) -> Result<FeeResult, Error> {
        let contract = <Contract as DriveContractExt>::from_cbor(contract_cbor, None)?;

        let document = Document::from_cbor(serialized_document, None, owner_id)?;

        self.update_document_for_contract(
            &document,
            serialized_document,
            &contract,
            document_type,
            owner_id,
            block_info,
            apply,
            storage_flags,
            transaction,
        )
    }

    /// Updates a serialized document given a contract id and returns the associated fee.
    pub fn update_document_for_contract_id(
        &self,
        serialized_document: &[u8],
        contract_id: [u8; 32],
        document_type: &str,
        owner_id: Option<[u8; 32]>,
        block_info: BlockInfo,
        apply: bool,
        storage_flags: Option<&StorageFlags>,
        transaction: TransactionArg,
    ) -> Result<FeeResult, Error> {
        let mut drive_operations: Vec<DriveOperation> = vec![];

        let contract_fetch_info = self
            .get_contract_with_fetch_info_and_add_to_operations(
                contract_id,
                Some(&block_info.epoch),
                transaction,
                &mut drive_operations,
            )?
            .ok_or(Error::Document(DocumentError::ContractNotFound()))?;

        let contract = &contract_fetch_info.contract;

        let document = Document::from_cbor(serialized_document, None, owner_id)?;

        let document_info =
            DocumentRefAndSerialization((&document, serialized_document, storage_flags));

        let document_type = contract.document_type_for_name(document_type)?;

        self.update_document_for_contract_apply_and_add_to_operations(
            DocumentAndContractInfo {
                document_info,
                contract,
                document_type,
                owner_id,
            },
            &block_info,
            apply,
            transaction,
            &mut drive_operations,
        )?;

        let fees = calculate_fee(None, Some(drive_operations), &block_info.epoch)?;

        Ok(fees)
    }

    /// Updates a serialized document and returns the associated fee.
    pub fn update_serialized_document_for_contract(
        &self,
        serialized_document: &[u8],
        contract: &Contract,
        document_type: &str,
        owner_id: Option<[u8; 32]>,
        block_info: BlockInfo,
        apply: bool,
        storage_flags: Option<&StorageFlags>,
        transaction: TransactionArg,
    ) -> Result<FeeResult, Error> {
        let document = Document::from_cbor(serialized_document, None, owner_id)?;

        self.update_document_for_contract(
            &document,
            serialized_document,
            contract,
            document_type,
            owner_id,
            block_info,
            apply,
            storage_flags,
            transaction,
        )
    }

    /// Updates a document and returns the associated fee.
    pub fn update_document_for_contract(
        &self,
        document: &Document,
        serialized_document: &[u8],
        contract: &Contract,
        document_type_name: &str,
        owner_id: Option<[u8; 32]>,
        block_info: BlockInfo,
        apply: bool,
        storage_flags: Option<&StorageFlags>,
        transaction: TransactionArg,
    ) -> Result<FeeResult, Error> {
        let mut drive_operations: Vec<DriveOperation> = vec![];

        let document_type = contract.document_type_for_name(document_type_name)?;

        let document_info =
            DocumentRefAndSerialization((document, serialized_document, storage_flags));

        self.update_document_for_contract_apply_and_add_to_operations(
            DocumentAndContractInfo {
                document_info,
                contract,
                document_type,
                owner_id,
            },
            &block_info,
            apply,
            transaction,
            &mut drive_operations,
        )?;
        let fees = calculate_fee(None, Some(drive_operations), &block_info.epoch)?;
        Ok(fees)
    }

    /// Updates a document.
    pub(crate) fn update_document_for_contract_apply_and_add_to_operations(
        &self,
        document_and_contract_info: DocumentAndContractInfo,
        block_info: &BlockInfo,
        apply: bool,
        transaction: TransactionArg,
        drive_operations: &mut Vec<DriveOperation>,
    ) -> Result<(), Error> {
        let batch_operations = self.update_document_for_contract_operations(
            document_and_contract_info,
            block_info,
            apply,
            transaction,
        )?;
        self.apply_batch_drive_operations(apply, transaction, batch_operations, drive_operations)
    }

    /// Gathers operations for updating a document.
    pub(crate) fn update_document_for_contract_operations(
        &self,
        document_and_contract_info: DocumentAndContractInfo,
        block_info: &BlockInfo,
        apply: bool,
        transaction: TransactionArg,
    ) -> Result<Vec<DriveOperation>, Error> {
        let mut batch_operations: Vec<DriveOperation> = vec![];

        if !document_and_contract_info.document_type.documents_mutable {
            return Err(Error::Drive(DriveError::UpdatingReadOnlyImmutableDocument(
                "documents for this contract are not mutable",
            )));
        }

        if !document_and_contract_info
            .document_info
            .is_document_and_serialization()
        {
            // todo: right now let's say the worst case scenario for an update is that all the data must be added again
            return self.add_document_for_contract_operations(
                document_and_contract_info,
                false,
                block_info,
                apply,
                transaction,
            );
        }

        let contract = document_and_contract_info.contract;
        let document_type = document_and_contract_info.document_type;
        let owner_id = document_and_contract_info.owner_id;

        if let DocumentRefAndSerialization((document, _serialized_document, storage_flags)) =
            document_and_contract_info.document_info
        {
            // we need to construct the path for documents on the contract
            // the path is
            //  * Document and Contract root tree
            //  * Contract ID recovered from document
            //  * 0 to signify Documents and not Contract
            let contract_document_type_path =
                contract_document_type_path(contract.id.as_bytes(), document_type.name.as_str());

            let contract_documents_primary_key_path = contract_documents_primary_key_path(
                contract.id.as_bytes(),
                document_type.name.as_str(),
            );

            let document_reference = make_document_reference(
                document,
                document_and_contract_info.document_type,
                storage_flags,
            );
            let query_stateless_max_value_size = if apply {
                None
            } else {
                Some(document_type.max_size())
            };

            // next we need to get the old document from storage
            let old_document_element = if document_type.documents_keep_history {
                let contract_documents_keeping_history_primary_key_path_for_document_id =
                    contract_documents_keeping_history_primary_key_path_for_document_id(
                        contract.id.as_bytes(),
                        document_type.name.as_str(),
                        document.id.as_slice(),
                    );
                // When keeping document history the 0 is a reference that points to the current value
                // O is just on one byte, so we have at most one hop of size 1 (1 byte)
                let query_stateless_with_max_value_size_and_max_reference_sizes =
                    query_stateless_max_value_size.map(|vs| (vs, vec![1]));
                self.grove_get(
                    contract_documents_keeping_history_primary_key_path_for_document_id,
                    KeyRefRequest(&[0]),
                    query_stateless_with_max_value_size_and_max_reference_sizes,
                    transaction,
                    &mut batch_operations,
                )?
            } else {
                self.grove_get_direct(
                    contract_documents_primary_key_path,
                    KeyRefRequest(document.id.as_slice()),
                    query_stateless_max_value_size,
                    transaction,
                    &mut batch_operations,
                )?
            };

            // we need to store the document for it's primary key
            // we should be overriding if the document_type does not have history enabled
            self.add_document_to_primary_storage(
                &document_and_contract_info,
                block_info,
                true,
                apply,
                transaction,
                &mut batch_operations,
            )?;

            let old_document_info = if let Some(old_document_element) = old_document_element {
                if let Element::Item(old_serialized_document, element_flags) = old_document_element
                {
                    let document =
                        Document::from_cbor(old_serialized_document.as_slice(), None, owner_id)?;
                    Ok(DocumentWithoutSerialization((
                        document,
                        StorageFlags::from_some_element_flags_ref(&element_flags)?,
                    )))
                } else {
                    Err(Error::Drive(DriveError::CorruptedDocumentNotItem(
                        "old document is not an item",
                    )))
                }?
            } else if let Some(max_value_size) = query_stateless_max_value_size {
                DocumentSize(max_value_size as u32)
            } else {
                return Err(Error::Drive(DriveError::UpdatingDocumentThatDoesNotExist(
                    "document being updated does not exist",
                )));
            };

            let mut batch_insertion_cache: HashSet<Vec<Vec<u8>>> = HashSet::new();
            // fourth we need to store a reference to the document for each index
            for index in &document_type.indices {
                // at this point the contract path is to the contract documents
                // for each index the top index component will already have been added
                // when the contract itself was created
                let mut index_path: Vec<Vec<u8>> = contract_document_type_path
                    .iter()
                    .map(|&x| Vec::from(x))
                    .collect();
                let top_index_property = index.properties.get(0).ok_or(Error::Drive(
                    DriveError::CorruptedContractIndexes("invalid contract indices"),
                ))?;
                index_path.push(Vec::from(top_index_property.name.as_bytes()));

                // with the example of the dashpay contract's first index
                // the index path is now something like Contracts/ContractID/Documents(1)/$ownerId
                let document_top_field = document
                    .get_raw_for_document_type(&top_index_property.name, document_type, owner_id)?
                    .unwrap_or_default();

                let old_document_top_field = old_document_info
                    .get_raw_for_document_type(&top_index_property.name, document_type, owner_id)?
                    .unwrap_or_default();

                // if we are not applying that means we are trying to get worst case costs
                // which would entail a change on every index
                let mut change_occurred_on_index = match &old_document_top_field {
                    DriveKeyInfo::Key(k) => &document_top_field != k,
                    DriveKeyInfo::KeyRef(k) => document_top_field.as_slice() != *k,
                    DriveKeyInfo::KeySize(_) => {
                        // we should assume true in this worst case cost scenario
                        true
                    }
                };

                if change_occurred_on_index {
                    // here we are inserting an empty tree that will have a subtree of all other index properties
                    let mut qualified_path = index_path.clone();
                    qualified_path.push(document_top_field.clone());

                    if !batch_insertion_cache.contains(&qualified_path) {
                        let inserted = self.batch_insert_empty_tree_if_not_exists(
                            PathKeyInfo::PathKeyRef::<0>((
                                index_path.clone(),
                                document_top_field.as_slice(),
                            )),
                            storage_flags,
                            apply,
                            transaction,
                            &mut batch_operations,
                        )?;
                        if inserted {
                            batch_insertion_cache.insert(qualified_path);
                        }
                    }
                }

                let mut all_fields_null = document_top_field.is_empty();

                let mut old_index_path: Vec<DriveKeyInfo> = index_path
                    .iter()
                    .map(|path_item| DriveKeyInfo::Key(path_item.clone()))
                    .collect();
                // we push the actual value of the index path
                index_path.push(document_top_field);
                // the index path is now something like Contracts/ContractID/Documents(1)/$ownerId/<ownerId>

                old_index_path.push(old_document_top_field);

                for i in 1..index.properties.len() {
                    let index_property = index.properties.get(i).ok_or(Error::Drive(
                        DriveError::CorruptedContractIndexes("invalid contract indices"),
                    ))?;

                    let document_index_field = document
                        .get_raw_for_document_type(&index_property.name, document_type, owner_id)?
                        .unwrap_or_default();

                    let old_document_index_field = old_document_info
                        .get_raw_for_document_type(&index_property.name, document_type, owner_id)?
                        .unwrap_or_default();

                    // if we are not applying that means we are trying to get worst case costs
                    // which would entail a change on every index
                    change_occurred_on_index |= match &old_document_index_field {
                        DriveKeyInfo::Key(k) => &document_index_field != k,
                        DriveKeyInfo::KeyRef(k) => document_index_field != *k,
                        DriveKeyInfo::KeySize(_) => {
                            // we should assume true in this worst case cost scenario
                            true
                        }
                    };

                    if change_occurred_on_index {
                        // here we are inserting an empty tree that will have a subtree of all other index properties

                        let mut qualified_path = index_path.clone();
                        qualified_path.push(index_property.name.as_bytes().to_vec());

                        if !batch_insertion_cache.contains(&qualified_path) {
                            let inserted = self.batch_insert_empty_tree_if_not_exists(
                                PathKeyInfo::PathKeyRef::<0>((
                                    index_path.clone(),
                                    index_property.name.as_bytes(),
                                )),
                                storage_flags,
                                apply,
                                transaction,
                                &mut batch_operations,
                            )?;
                            if inserted {
                                batch_insertion_cache.insert(qualified_path);
                            }
                        }
                    }

                    index_path.push(Vec::from(index_property.name.as_bytes()));
                    old_index_path
                        .push(DriveKeyInfo::Key(Vec::from(index_property.name.as_bytes())));

                    // Iteration 1. the index path is now something like Contracts/ContractID/Documents(1)/$ownerId/<ownerId>/toUserId
                    // Iteration 2. the index path is now something like Contracts/ContractID/Documents(1)/$ownerId/<ownerId>/toUserId/<ToUserId>/accountReference

                    if change_occurred_on_index {
                        // here we are inserting an empty tree that will have a subtree of all other index properties

                        let mut qualified_path = index_path.clone();
                        qualified_path.push(document_index_field.clone());

                        if !batch_insertion_cache.contains(&qualified_path) {
                            let inserted = self.batch_insert_empty_tree_if_not_exists(
                                PathKeyInfo::PathKeyRef::<0>((
                                    index_path.clone(),
                                    document_index_field.as_slice(),
                                )),
                                storage_flags,
                                apply,
                                transaction,
                                &mut batch_operations,
                            )?;
                            if inserted {
                                batch_insertion_cache.insert(qualified_path);
                            }
                        }
                    }

                    all_fields_null &= document_index_field.is_empty();

                    // we push the actual value of the index path, both for the new and the old
                    index_path.push(document_index_field);
                    old_index_path.push(old_document_index_field);
                    // Iteration 1. the index path is now something like Contracts/ContractID/Documents(1)/$ownerId/<ownerId>/toUserId/<ToUserId>/
                    // Iteration 2. the index path is now something like Contracts/ContractID/Documents(1)/$ownerId/<ownerId>/toUserId/<ToUserId>/accountReference/<accountReference>
                }

                if change_occurred_on_index {
                    // we first need to delete the old values
                    // unique indexes will be stored under key "0"
                    // non unique indices should have a tree at key "0" that has all elements based off of primary key
                    if !index.unique {
                        old_index_path.push(DriveKeyInfo::Key(vec![0]));

                        // here we should return an error if the element already exists
                        self.batch_delete_up_tree_while_empty(
                            old_index_path,
                            document.id.as_slice(),
                            Some(CONTRACT_DOCUMENTS_PATH_HEIGHT),
                            apply,
                            transaction,
                            &mut batch_operations,
                        )?;
                    } else {
                        // here we should return an error if the element already exists
                        self.batch_delete_up_tree_while_empty(
                            old_index_path,
                            &[0],
                            Some(CONTRACT_DOCUMENTS_PATH_HEIGHT),
                            apply,
                            transaction,
                            &mut batch_operations,
                        )?;
                    }

                    // unique indexes will be stored under key "0"
                    // non unique indices should have a tree at key "0" that has all elements based off of primary key
                    if !index.unique || all_fields_null {
                        // here we are inserting an empty tree that will have a subtree of all other index properties
                        self.batch_insert_empty_tree_if_not_exists(
                            PathKeyInfo::PathKeyRef::<0>((index_path.clone(), &[0])),
                            storage_flags,
                            apply,
                            transaction,
                            &mut batch_operations,
                        )?;
                        index_path.push(vec![0]);

                        // here we should return an error if the element already exists
                        self.batch_insert(
                            PathKeyElement::<0>((
                                index_path,
                                document.id.as_slice(),
                                document_reference.clone(),
                            )),
                            &mut batch_operations,
                        )?;
                    } else {
                        // in one update you can't insert an element twice, so need to check the cache
                        // here we should return an error if the element already exists
                        let inserted = self.batch_insert_if_not_exists(
                            PathKeyElement::<0>((index_path, &[0], document_reference.clone())),
                            if apply {
                                None
                            } else {
                                SOME_OPTIMIZED_DOCUMENT_REFERENCE
                            },
                            transaction,
                            &mut batch_operations,
                        )?;
                        if !inserted {
                            return Err(Error::Drive(DriveError::CorruptedContractIndexes(
                                "index already exists",
                            )));
                        }
                    }
                }
            }
        }
        Ok(batch_operations)
    }
}

#[cfg(test)]
mod tests {
    use grovedb::TransactionArg;
    use std::default::Default;
    use std::option::Option::None;
    use std::sync::Arc;

    use dpp::data_contract::validation::data_contract_validator::DataContractValidator;
    use dpp::data_contract::DataContractFactory;
    use dpp::document::document_factory::DocumentFactory;
    use dpp::document::document_validator::DocumentValidator;
    use dpp::mocks;
    use dpp::prelude::DataContract;
    use dpp::version::{ProtocolVersionValidator, COMPATIBILITY_MAP, LATEST_VERSION};
    use rand::Rng;
    use serde::{Deserialize, Serialize};
    use serde_json::{json, Value};
    use tempfile::TempDir;

    use super::*;
    use crate::common::{json_document_to_cbor, setup_contract, value_to_cbor};
    use crate::contract::{document::Document, Contract};
    use crate::drive::config::{DriveConfig, DriveEncoding};
    use crate::drive::flags::StorageFlags;
    use crate::drive::object_size_info::DocumentAndContractInfo;
    use crate::drive::object_size_info::DocumentInfo::DocumentRefAndSerialization;
    use crate::drive::{defaults, Drive};
    use crate::fee::default_costs::STORAGE_DISK_USAGE_CREDIT_PER_BYTE;
    use crate::fee_pools::epochs::Epoch;
    use crate::query::DriveQuery;

    #[test]
    fn test_create_and_update_document_same_transaction() {
        let tmp_dir = TempDir::new().unwrap();
        let drive: Drive = Drive::open(tmp_dir, None).expect("expected to open Drive successfully");

        let db_transaction = drive.grove.start_transaction();

        drive
            .create_initial_state_structure(Some(&db_transaction))
            .expect("expected to create root tree successfully");

        let contract_cbor = hex::decode("01000000a5632469645820b0248cd9a27f86d05badf475dd9ff574d63219cd60c52e2be1e540c2fdd713336724736368656d61783468747470733a2f2f736368656d612e646173682e6f72672f6470702d302d342d302f6d6574612f646174612d636f6e7472616374676f776e6572496458204c9bf0db6ae315c85465e9ef26e6a006de9673731d08d14881945ddef1b5c5f26776657273696f6e0169646f63756d656e7473a267636f6e74616374a56474797065666f626a65637467696e646963657381a3646e616d656f6f6e7765724964546f55736572496466756e69717565f56a70726f7065727469657382a168246f776e6572496463617363a168746f557365724964636173636872657175697265648268746f557365724964697075626c69634b65796a70726f70657274696573a268746f557365724964a56474797065656172726179686d61784974656d731820686d696e4974656d73182069627974654172726179f570636f6e74656e744d656469615479706578216170706c69636174696f6e2f782e646173682e6470702e6964656e746966696572697075626c69634b6579a36474797065656172726179686d61784974656d73182169627974654172726179f5746164646974696f6e616c50726f70657274696573f46770726f66696c65a56474797065666f626a65637467696e646963657381a3646e616d65676f776e6572496466756e69717565f56a70726f7065727469657381a168246f776e6572496463617363687265717569726564826961766174617255726c6561626f75746a70726f70657274696573a26561626f7574a2647479706566737472696e67696d61784c656e67746818ff6961766174617255726ca3647479706566737472696e6766666f726d61746375726c696d61784c656e67746818ff746164646974696f6e616c50726f70657274696573f4").unwrap();

        drive
            .apply_contract_cbor(
                contract_cbor.clone(),
                None,
                BlockInfo::default(),
                true,
                StorageFlags::optional_default_as_ref(),
                Some(&db_transaction),
            )
            .expect("expected to apply contract successfully");

        // Create Alice profile

        let alice_profile_cbor = hex::decode("01000000a763246964582035edfec54aea574df968990abb47b39c206abe5c43a6157885f62958a1f1230c6524747970656770726f66696c656561626f75746a4920616d20416c69636568246f776e65724964582041d52f93f6f7c5af79ce994381c90df73cce2863d3850b9c05ef586ff0fe795f69247265766973696f6e016961766174617255726c7819687474703a2f2f746573742e636f6d2f616c6963652e6a70676f2464617461436f6e747261637449645820b0248cd9a27f86d05badf475dd9ff574d63219cd60c52e2be1e540c2fdd71333").unwrap();

        drive
            .add_serialized_document_for_serialized_contract(
                alice_profile_cbor.as_slice(),
                contract_cbor.as_slice(),
                "profile",
                None,
                true,
                BlockInfo::default(),
                true,
                StorageFlags::optional_default_as_ref(),
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
                BlockInfo::default(),
                true,
                StorageFlags::optional_default_as_ref(),
                Some(&db_transaction),
            )
            .expect("should update alice profile");
    }

    #[test]
    fn test_create_and_update_document_no_transactions() {
        let tmp_dir = TempDir::new().unwrap();
        let drive: Drive = Drive::open(tmp_dir, None).expect("expected to open Drive successfully");

        drive
            .create_initial_state_structure(None)
            .expect("expected to create root tree successfully");

        let contract_cbor = hex::decode("01000000a5632469645820b0248cd9a27f86d05badf475dd9ff574d63219cd60c52e2be1e540c2fdd713336724736368656d61783468747470733a2f2f736368656d612e646173682e6f72672f6470702d302d342d302f6d6574612f646174612d636f6e7472616374676f776e6572496458204c9bf0db6ae315c85465e9ef26e6a006de9673731d08d14881945ddef1b5c5f26776657273696f6e0169646f63756d656e7473a267636f6e74616374a56474797065666f626a65637467696e646963657381a3646e616d656f6f6e7765724964546f55736572496466756e69717565f56a70726f7065727469657382a168246f776e6572496463617363a168746f557365724964636173636872657175697265648268746f557365724964697075626c69634b65796a70726f70657274696573a268746f557365724964a56474797065656172726179686d61784974656d731820686d696e4974656d73182069627974654172726179f570636f6e74656e744d656469615479706578216170706c69636174696f6e2f782e646173682e6470702e6964656e746966696572697075626c69634b6579a36474797065656172726179686d61784974656d73182169627974654172726179f5746164646974696f6e616c50726f70657274696573f46770726f66696c65a56474797065666f626a65637467696e646963657381a3646e616d65676f776e6572496466756e69717565f56a70726f7065727469657381a168246f776e6572496463617363687265717569726564826961766174617255726c6561626f75746a70726f70657274696573a26561626f7574a2647479706566737472696e67696d61784c656e67746818ff6961766174617255726ca3647479706566737472696e6766666f726d61746375726c696d61784c656e67746818ff746164646974696f6e616c50726f70657274696573f4").unwrap();

        let contract = <Contract as DriveContractExt>::from_cbor(contract_cbor.as_slice(), None)
            .expect("expected to create contract");
        drive
            .apply_contract_cbor(
                contract_cbor.clone(),
                None,
                BlockInfo::default(),
                true,
                StorageFlags::optional_default_as_ref(),
                None,
            )
            .expect("expected to apply contract successfully");

        // Create Alice profile

        let alice_profile_cbor = hex::decode("01000000a763246964582035edfec54aea574df968990abb47b39c206abe5c43a6157885f62958a1f1230c6524747970656770726f66696c656561626f75746a4920616d20416c69636568246f776e65724964582041d52f93f6f7c5af79ce994381c90df73cce2863d3850b9c05ef586ff0fe795f69247265766973696f6e016961766174617255726c7819687474703a2f2f746573742e636f6d2f616c6963652e6a70676f2464617461436f6e747261637449645820b0248cd9a27f86d05badf475dd9ff574d63219cd60c52e2be1e540c2fdd71333").unwrap();

        let alice_profile = Document::from_cbor(alice_profile_cbor.as_slice(), None, None)
            .expect("expected to get a document");

        let document_type = contract
            .document_type_for_name("profile")
            .expect("expected to get a document type");

        let storage_flags = Some(StorageFlags::SingleEpoch(0));

        drive
            .add_document_for_contract(
                DocumentAndContractInfo {
                    document_info: DocumentRefAndSerialization((
                        &alice_profile,
                        alice_profile_cbor.as_slice(),
                        storage_flags.as_ref(),
                    )),
                    contract: &contract,
                    document_type,
                    owner_id: None,
                },
                true,
                BlockInfo::default(),
                true,
                None,
            )
            .expect("should create alice profile");

        let sql_string = "select * from profile";
        let query = DriveQuery::from_sql_expr(sql_string, &contract).expect("should build query");

        let (results_no_transaction, _, _) = query
            .execute_no_proof(&drive, None, None)
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
                BlockInfo::default(),
                true,
                StorageFlags::optional_default_as_ref(),
                None,
            )
            .expect("should update alice profile");

        let (results_no_transaction, _, _) = query
            .execute_no_proof(&drive, None, None)
            .expect("expected to execute query");

        assert_eq!(results_no_transaction.len(), 1);
    }

    #[test]
    fn test_create_and_update_document_in_different_transactions() {
        let tmp_dir = TempDir::new().unwrap();
        let drive: Drive = Drive::open(tmp_dir, None).expect("expected to open Drive successfully");

        let db_transaction = drive.grove.start_transaction();

        drive
            .create_initial_state_structure(Some(&db_transaction))
            .expect("expected to create root tree successfully");

        let contract_cbor = hex::decode("01000000a5632469645820b0248cd9a27f86d05badf475dd9ff574d63219cd60c52e2be1e540c2fdd713336724736368656d61783468747470733a2f2f736368656d612e646173682e6f72672f6470702d302d342d302f6d6574612f646174612d636f6e7472616374676f776e6572496458204c9bf0db6ae315c85465e9ef26e6a006de9673731d08d14881945ddef1b5c5f26776657273696f6e0169646f63756d656e7473a267636f6e74616374a56474797065666f626a65637467696e646963657381a3646e616d656f6f6e7765724964546f55736572496466756e69717565f56a70726f7065727469657382a168246f776e6572496463617363a168746f557365724964636173636872657175697265648268746f557365724964697075626c69634b65796a70726f70657274696573a268746f557365724964a56474797065656172726179686d61784974656d731820686d696e4974656d73182069627974654172726179f570636f6e74656e744d656469615479706578216170706c69636174696f6e2f782e646173682e6470702e6964656e746966696572697075626c69634b6579a36474797065656172726179686d61784974656d73182169627974654172726179f5746164646974696f6e616c50726f70657274696573f46770726f66696c65a56474797065666f626a65637467696e646963657381a3646e616d65676f776e6572496466756e69717565f56a70726f7065727469657381a168246f776e6572496463617363687265717569726564826961766174617255726c6561626f75746a70726f70657274696573a26561626f7574a2647479706566737472696e67696d61784c656e67746818ff6961766174617255726ca3647479706566737472696e6766666f726d61746375726c696d61784c656e67746818ff746164646974696f6e616c50726f70657274696573f4").unwrap();

        let contract = <Contract as DriveContractExt>::from_cbor(contract_cbor.as_slice(), None)
            .expect("expected to create contract");
        drive
            .apply_contract_cbor(
                contract_cbor.clone(),
                None,
                BlockInfo::default(),
                true,
                StorageFlags::optional_default_as_ref(),
                Some(&db_transaction),
            )
            .expect("expected to apply contract successfully");

        // Create Alice profile

        let alice_profile_cbor = hex::decode("01000000a763246964582035edfec54aea574df968990abb47b39c206abe5c43a6157885f62958a1f1230c6524747970656770726f66696c656561626f75746a4920616d20416c69636568246f776e65724964582041d52f93f6f7c5af79ce994381c90df73cce2863d3850b9c05ef586ff0fe795f69247265766973696f6e016961766174617255726c7819687474703a2f2f746573742e636f6d2f616c6963652e6a70676f2464617461436f6e747261637449645820b0248cd9a27f86d05badf475dd9ff574d63219cd60c52e2be1e540c2fdd71333").unwrap();

        let alice_profile = Document::from_cbor(alice_profile_cbor.as_slice(), None, None)
            .expect("expected to get a document");

        let document_type = contract
            .document_type_for_name("profile")
            .expect("expected to get a document type");

        let storage_flags = Some(StorageFlags::SingleEpoch(0));

        drive
            .add_document_for_contract(
                DocumentAndContractInfo {
                    document_info: DocumentRefAndSerialization((
                        &alice_profile,
                        alice_profile_cbor.as_slice(),
                        storage_flags.as_ref(),
                    )),
                    contract: &contract,
                    document_type,
                    owner_id: None,
                },
                true,
                BlockInfo::default(),
                true,
                Some(&db_transaction),
            )
            .expect("should create alice profile");

        drive
            .grove
            .commit_transaction(db_transaction)
            .unwrap()
            .expect("should commit transaction");

        let sql_string = "select * from profile";
        let query = DriveQuery::from_sql_expr(sql_string, &contract).expect("should build query");

        let (results_no_transaction, _, _) = query
            .execute_no_proof(&drive, None, None)
            .expect("expected to execute query");

        assert_eq!(results_no_transaction.len(), 1);

        let db_transaction = drive.grove.start_transaction();

        let (results_on_transaction, _, _) = query
            .execute_no_proof(&drive, None, Some(&db_transaction))
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
                BlockInfo::default(),
                true,
                StorageFlags::optional_default_as_ref(),
                Some(&db_transaction),
            )
            .expect("should update alice profile");

        let (results_on_transaction, _, _) = query
            .execute_no_proof(&drive, None, Some(&db_transaction))
            .expect("expected to execute query");

        assert_eq!(results_on_transaction.len(), 1);

        drive
            .grove
            .commit_transaction(db_transaction)
            .unwrap()
            .expect("should commit transaction");
    }

    #[test]
    fn test_create_and_update_document_in_different_transactions_with_delete_rollback() {
        let tmp_dir = TempDir::new().unwrap();
        let drive: Drive = Drive::open(tmp_dir, None).expect("expected to open Drive successfully");

        let db_transaction = drive.grove.start_transaction();

        drive
            .create_initial_state_structure(Some(&db_transaction))
            .expect("expected to create root tree successfully");

        let contract_cbor = hex::decode("01000000a5632469645820b0248cd9a27f86d05badf475dd9ff574d63219cd60c52e2be1e540c2fdd713336724736368656d61783468747470733a2f2f736368656d612e646173682e6f72672f6470702d302d342d302f6d6574612f646174612d636f6e7472616374676f776e6572496458204c9bf0db6ae315c85465e9ef26e6a006de9673731d08d14881945ddef1b5c5f26776657273696f6e0169646f63756d656e7473a267636f6e74616374a56474797065666f626a65637467696e646963657381a3646e616d656f6f6e7765724964546f55736572496466756e69717565f56a70726f7065727469657382a168246f776e6572496463617363a168746f557365724964636173636872657175697265648268746f557365724964697075626c69634b65796a70726f70657274696573a268746f557365724964a56474797065656172726179686d61784974656d731820686d696e4974656d73182069627974654172726179f570636f6e74656e744d656469615479706578216170706c69636174696f6e2f782e646173682e6470702e6964656e746966696572697075626c69634b6579a36474797065656172726179686d61784974656d73182169627974654172726179f5746164646974696f6e616c50726f70657274696573f46770726f66696c65a56474797065666f626a65637467696e646963657381a3646e616d65676f776e6572496466756e69717565f56a70726f7065727469657381a168246f776e6572496463617363687265717569726564826961766174617255726c6561626f75746a70726f70657274696573a26561626f7574a2647479706566737472696e67696d61784c656e67746818ff6961766174617255726ca3647479706566737472696e6766666f726d61746375726c696d61784c656e67746818ff746164646974696f6e616c50726f70657274696573f4").unwrap();

        let contract = <Contract as DriveContractExt>::from_cbor(contract_cbor.as_slice(), None)
            .expect("expected to create contract");
        drive
            .apply_contract_cbor(
                contract_cbor.clone(),
                None,
                BlockInfo::default(),
                true,
                StorageFlags::optional_default_as_ref(),
                Some(&db_transaction),
            )
            .expect("expected to apply contract successfully");

        // Create Alice profile

        let alice_profile_cbor = hex::decode("01000000a763246964582035edfec54aea574df968990abb47b39c206abe5c43a6157885f62958a1f1230c6524747970656770726f66696c656561626f75746a4920616d20416c69636568246f776e65724964582041d52f93f6f7c5af79ce994381c90df73cce2863d3850b9c05ef586ff0fe795f69247265766973696f6e016961766174617255726c7819687474703a2f2f746573742e636f6d2f616c6963652e6a70676f2464617461436f6e747261637449645820b0248cd9a27f86d05badf475dd9ff574d63219cd60c52e2be1e540c2fdd71333").unwrap();

        let alice_profile = Document::from_cbor(alice_profile_cbor.as_slice(), None, None)
            .expect("expected to get a document");

        let document_type = contract
            .document_type_for_name("profile")
            .expect("expected to get a document type");

        let storage_flags = Some(StorageFlags::SingleEpoch(0));

        drive
            .add_document_for_contract(
                DocumentAndContractInfo {
                    document_info: DocumentRefAndSerialization((
                        &alice_profile,
                        alice_profile_cbor.as_slice(),
                        storage_flags.as_ref(),
                    )),
                    contract: &contract,
                    document_type,
                    owner_id: None,
                },
                true,
                BlockInfo::default(),
                true,
                Some(&db_transaction),
            )
            .expect("should create alice profile");

        drive
            .grove
            .commit_transaction(db_transaction)
            .unwrap()
            .expect("should commit transaction");

        let sql_string = "select * from profile";
        let query = DriveQuery::from_sql_expr(sql_string, &contract).expect("should build query");

        let (results_no_transaction, _, _) = query
            .execute_no_proof(&drive, None, None)
            .expect("expected to execute query");

        assert_eq!(results_no_transaction.len(), 1);

        let db_transaction = drive.grove.start_transaction();

        let (results_on_transaction, _, _) = query
            .execute_no_proof(&drive, None, Some(&db_transaction))
            .expect("expected to execute query");

        assert_eq!(results_on_transaction.len(), 1);

        drive
            .delete_document_for_contract(
                alice_profile.id,
                &contract,
                "profile",
                None,
                BlockInfo::default(),
                true,
                Some(&db_transaction),
            )
            .expect("expected to delete document");

        let (results_on_transaction, _, _) = query
            .execute_no_proof(&drive, None, Some(&db_transaction))
            .expect("expected to execute query");

        assert_eq!(results_on_transaction.len(), 0);

        drive
            .grove
            .rollback_transaction(&db_transaction)
            .expect("expected to rollback transaction");

        let (results_on_transaction, _, _) = query
            .execute_no_proof(&drive, None, Some(&db_transaction))
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
                BlockInfo::default(),
                true,
                StorageFlags::optional_default_as_ref(),
                Some(&db_transaction),
            )
            .expect("should update alice profile");
    }

    #[test]
    fn test_create_update_and_delete_document() {
        let tmp_dir = TempDir::new().unwrap();
        let drive: Drive = Drive::open(tmp_dir, None).expect("expected to open Drive successfully");

        drive
            .create_initial_state_structure(None)
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
            .apply_contract_cbor(
                contract.clone(),
                None,
                BlockInfo::default(),
                true,
                StorageFlags::optional_default_as_ref(),
                None,
            )
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

        let serialized_document = value_to_cbor(document, Some(defaults::PROTOCOL_VERSION));

        drive
            .add_serialized_document_for_serialized_contract(
                serialized_document.as_slice(),
                &contract.as_slice(),
                "indexedDocument",
                None,
                true,
                BlockInfo::default(),
                true,
                StorageFlags::optional_default_as_ref(),
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

        let serialized_document = value_to_cbor(document, Some(defaults::PROTOCOL_VERSION));

        drive
            .update_document_for_contract_cbor(
                serialized_document.as_slice(),
                &contract.as_slice(),
                "indexedDocument",
                None,
                BlockInfo::default(),
                true,
                StorageFlags::optional_default_as_ref(),
                None,
            )
            .expect("should update document");

        let document_id = bs58::decode("DLRWw2eRbLAW5zDU2c7wwsSFQypTSZPhFYzpY48tnaXN")
            .into_vec()
            .expect("should decode")
            .as_slice()
            .try_into()
            .expect("this be 32 bytes");

        // Delete document

        drive
            .delete_document_for_contract_cbor(
                document_id,
                &contract,
                "indexedDocument",
                None,
                BlockInfo::default(),
                true,
                None,
            )
            .expect("should delete document");
    }

    #[test]
    fn test_modify_dashpay_contact_request() {
        let tmp_dir = TempDir::new().unwrap();
        let drive: Drive = Drive::open(tmp_dir, None).expect("expected to open Drive successfully");

        let db_transaction = drive.grove.start_transaction();

        drive
            .create_initial_state_structure(Some(&db_transaction))
            .expect("expected to create root tree successfully");

        let contract = setup_contract(
            &drive,
            "tests/supporting_files/contract/dashpay/dashpay-contract.json",
            None,
            Some(&db_transaction),
        );

        let dashpay_cr_serialized_document = json_document_to_cbor(
            "tests/supporting_files/contract/dashpay/contact-request0.json",
            Some(1),
        );

        let random_owner_id = rand::thread_rng().gen::<[u8; 32]>();
        drive
            .add_serialized_document_for_contract(
                &dashpay_cr_serialized_document,
                &contract,
                "contactRequest",
                Some(random_owner_id),
                false,
                BlockInfo::default(),
                true,
                StorageFlags::optional_default_as_ref(),
                Some(&db_transaction),
            )
            .expect("expected to insert a document successfully");

        drive
            .update_serialized_document_for_contract(
                &dashpay_cr_serialized_document,
                &contract,
                "contactRequest",
                Some(random_owner_id),
                BlockInfo::default(),
                true,
                StorageFlags::optional_default_as_ref(),
                Some(&db_transaction),
            )
            .expect_err("expected not to be able to update a non mutable document");

        drive
            .add_serialized_document_for_contract(
                &dashpay_cr_serialized_document,
                &contract,
                "contactRequest",
                Some(random_owner_id),
                true,
                BlockInfo::default(),
                true,
                StorageFlags::optional_default_as_ref(),
                Some(&db_transaction),
            )
            .expect_err("expected not to be able to override a non mutable document");
    }

    #[test]
    fn test_update_dashpay_profile_with_history() {
        let tmp_dir = TempDir::new().unwrap();
        let drive: Drive = Drive::open(tmp_dir, None).expect("expected to open Drive successfully");

        let db_transaction = drive.grove.start_transaction();

        drive
            .create_initial_state_structure(Some(&db_transaction))
            .expect("expected to create root tree successfully");

        let contract = setup_contract(
            &drive,
            "tests/supporting_files/contract/dashpay/dashpay-contract-with-profile-history.json",
            None,
            Some(&db_transaction),
        );

        let dashpay_profile_serialized_document = json_document_to_cbor(
            "tests/supporting_files/contract/dashpay/profile0.json",
            Some(1),
        );

        let dashpay_profile_updated_public_message_serialized_document = json_document_to_cbor(
            "tests/supporting_files/contract/dashpay/profile0-updated-public-message.json",
            Some(1),
        );

        let random_owner_id = rand::thread_rng().gen::<[u8; 32]>();
        drive
            .add_serialized_document_for_contract(
                &dashpay_profile_serialized_document,
                &contract,
                "profile",
                Some(random_owner_id),
                false,
                BlockInfo::default(),
                true,
                StorageFlags::optional_default_as_ref(),
                Some(&db_transaction),
            )
            .expect("expected to insert a document successfully");

        drive
            .update_serialized_document_for_contract(
                &dashpay_profile_updated_public_message_serialized_document,
                &contract,
                "profile",
                Some(random_owner_id),
                BlockInfo::default(),
                true,
                StorageFlags::optional_default_as_ref(),
                Some(&db_transaction),
            )
            .expect("expected to update a document with history successfully");
    }

    fn test_fees_for_update_document(using_history: bool, using_transaction: bool) {
        let config = DriveConfig {
            batching_enabled: true,
            batching_consistency_verification: true,
            has_raw_enabled: true,
            default_genesis_time: Some(0),
            encoding: DriveEncoding::DriveCbor,
            ..Default::default()
        };
        let tmp_dir = TempDir::new().unwrap();

        let drive: Drive =
            Drive::open(&tmp_dir, Some(config)).expect("expected to open Drive successfully");

        let transaction = if using_transaction {
            Some(drive.grove.start_transaction())
        } else {
            None
        };

        drive
            .create_initial_state_structure(transaction.as_ref())
            .expect("expected to create root tree successfully");

        let path = if using_history {
            "tests/supporting_files/contract/family/family-contract-with-history-only-message-index.json"
        } else {
            "tests/supporting_files/contract/family/family-contract-only-message-index.json"
        };

        // setup code
        let contract = setup_contract(&drive, path, None, transaction.as_ref());

        let id = [1u8; 32];
        let owner_id = [2u8; 32];
        let person_0_original = Person {
            id,
            owner_id,
            first_name: "Samuel".to_string(),
            middle_name: "Abraham".to_string(),
            last_name: "Westrich".to_string(),
            message: Some("My apples are safe".to_string()),
            age: 33,
        };

        let person_0_updated = Person {
            id,
            owner_id,
            first_name: "Samuel".to_string(),
            middle_name: "Abraham".to_string(),
            last_name: "Westrich2".to_string(),
            message: Some("My apples are safe".to_string()),
            age: 35,
        };

        let original_fees = apply_person(
            &drive,
            &contract,
            BlockInfo::default(),
            &person_0_original,
            true,
            transaction.as_ref(),
        );
        let original_bytes = original_fees.storage_fee / STORAGE_DISK_USAGE_CREDIT_PER_BYTE;
        let expected_added_bytes = if using_history {
            //Explanation for 1350

            //todo
            1350
        } else {
            //Explanation for 1049

            // Document Storage

            //// Item
            // = 410 Bytes

            // Explanation for 410 storage_written_bytes

            // Key -> 65 bytes
            // 32 bytes for the key prefix
            // 32 bytes for the unique id
            // 1 byte for key_size (required space for 64)

            // Value -> 278
            //   1 for the flag option with flags
            //   1 for the flags size
            //   35 for flags 32 + 1 + 2
            //   1 for the enum type
            //   1 for item
            //   173 for item serialized bytes
            // 32 for node hash
            // 32 for value hash
            // 2 byte for the value_size (required space for above 128)

            // Parent Hook -> 67
            // Key Bytes 32
            // Hash Size 32
            // Key Length 1
            // Child Heights 2

            // Total 65 + 278 + 67 = 410

            //// Tree 1 / <Person Contract> / 1 / person / message
            // Key: My apples are safe
            // = 177 Bytes

            // Explanation for 177 storage_written_bytes

            // Key -> 51 bytes
            // 32 bytes for the key prefix
            // 18 bytes for the key "My apples are safe" 18 characters
            // 1 byte for key_size (required space for 50)

            // Value -> 73
            //   1 for the flag option with flags
            //   1 for the flags size
            //   35 for flags
            //   1 for the enum type
            //   1 for empty tree value
            // 32 for node hash
            // 0 for value hash
            // 2 byte for the value_size (required space for 73 + up to 256 for child key)

            // Parent Hook -> 53
            // Key Bytes 18
            // Hash Size 32
            // Key Length 1
            // Child Heights 2

            // Total 51 + 73 + 53 = 177

            //// Tree 1 / <Person Contract> / 1 / person / message / My apples are safe
            // Key: 0
            // = 143 Bytes

            // Explanation for 143 storage_written_bytes

            // Key -> 34 bytes
            // 32 bytes for the key prefix
            // 1 bytes for the key "My apples are safe" 18 characters
            // 1 byte for key_size (required space for 33)

            // Value -> 73
            //   1 for the flag option with flags
            //   1 for the flags size
            //   35 for flags
            //   1 for the enum type
            //   1 for empty tree value
            // 32 for node hash
            // 0 for value hash
            // 2 byte for the value_size (required space for 73 + up to 256 for child key)

            // Parent Hook -> 36
            // Key Bytes 1
            // Hash Size 32
            // Key Length 1
            // Child Heights 2

            // Total 34 + 73 + 36 = 143

            //// Ref 1 / <Person Contract> / 1 / person / message / My apples are safe
            // Reference to Serialized Item
            // = 319 Bytes

            // Explanation for 276 storage_written_bytes

            // Key -> 65 bytes
            // 32 bytes for the key prefix
            // 32 bytes for the unique id
            // 1 byte for key_size (required space for 64)

            // Value -> 144
            //   1 for the flag option with flags
            //   1 for the flags size
            //   35 for flags 32 + 1 + 2
            //   1 for the element type as reference
            //   1 for reference type as upstream root reference
            //   1 for reference root height
            //   36 for the reference path bytes ( 1 + 1 + 32 + 1 + 1)
            //   2 for the max reference hop
            // 32 for node hash
            // 32 for value hash
            // 2 byte for the value_size (required space for above 128)

            // Parent Hook -> 67
            // Key Bytes 32
            // Hash Size 32
            // Key Length 1
            // Child Heights 2

            // Total 65 + 144 + 67 = 276

            1006
        };
        assert_eq!(original_bytes, expected_added_bytes);

        if !using_history {
            // let's delete it, just to make sure everything is working.
            // we can delete items that use history though
            let deletion_fees = delete_person(
                &drive,
                &contract,
                BlockInfo::default(),
                &person_0_original,
                transaction.as_ref(),
            );
            let removed_bytes = deletion_fees
                .removed_bytes_from_identities
                .get(&owner_id)
                .unwrap()
                .get(0)
                .unwrap();
            assert_eq!(original_bytes, *removed_bytes as u64);
            // let's re-add it again
            let original_fees = apply_person(
                &drive,
                &contract,
                BlockInfo::default(),
                &person_0_original,
                true,
                transaction.as_ref(),
            );
            let original_bytes = original_fees.storage_fee / STORAGE_DISK_USAGE_CREDIT_PER_BYTE;
            assert_eq!(original_bytes, expected_added_bytes);
        }

        // now let's update it 1 second later
        let update_fees = apply_person(
            &drive,
            &contract,
            BlockInfo::default_with_time(1000),
            &person_0_updated,
            true,
            transaction.as_ref(),
        );
        // we both add and remove bytes
        // this is because trees are added because of indexes, and also removed
        let added_bytes = update_fees.storage_fee / STORAGE_DISK_USAGE_CREDIT_PER_BYTE;

        let expected_added_bytes = if using_history { 363 } else { 1 };
        assert_eq!(added_bytes, expected_added_bytes);
    }

    fn test_fees_for_update_document_on_index(using_history: bool, using_transaction: bool) {
        let config = DriveConfig {
            batching_enabled: true,
            batching_consistency_verification: true,
            has_raw_enabled: true,
            default_genesis_time: Some(0),
            encoding: DriveEncoding::DriveCbor,
            ..Default::default()
        };
        let tmp_dir = TempDir::new().unwrap();

        let drive: Drive =
            Drive::open(&tmp_dir, Some(config)).expect("expected to open Drive successfully");

        let transaction = if using_transaction {
            Some(drive.grove.start_transaction())
        } else {
            None
        };

        drive
            .create_initial_state_structure(transaction.as_ref())
            .expect("expected to create root tree successfully");

        let path = if using_history {
            "tests/supporting_files/contract/family/family-contract-with-history-only-message-index.json"
        } else {
            "tests/supporting_files/contract/family/family-contract-only-message-index.json"
        };

        // setup code
        let contract = setup_contract(&drive, path, None, transaction.as_ref());

        let id = [1u8; 32];
        let owner_id = [2u8; 32];
        let person_0_original = Person {
            id,
            owner_id,
            first_name: "Samuel".to_string(),
            middle_name: "Abraham".to_string(),
            last_name: "Westrich".to_string(),
            message: Some("My apples are safe".to_string()),
            age: 33,
        };

        let person_0_updated = Person {
            id,
            owner_id,
            first_name: "Samuel".to_string(),
            middle_name: "Abraham".to_string(),
            last_name: "Westrich".to_string(),
            message: Some("My apples are safer".to_string()),
            age: 35,
        };

        let original_fees = apply_person(
            &drive,
            &contract,
            BlockInfo::default(),
            &person_0_original,
            true,
            transaction.as_ref(),
        );
        let original_bytes = original_fees.storage_fee / STORAGE_DISK_USAGE_CREDIT_PER_BYTE;
        let expected_added_bytes = if using_history { 1350 } else { 1006 };
        assert_eq!(original_bytes, expected_added_bytes);
        if !using_history {
            // let's delete it, just to make sure everything is working.
            let deletion_fees = delete_person(
                &drive,
                &contract,
                BlockInfo::default(),
                &person_0_original,
                transaction.as_ref(),
            );
            let removed_bytes = deletion_fees
                .removed_bytes_from_identities
                .get(&owner_id)
                .unwrap()
                .get(0)
                .unwrap();
            assert_eq!(original_bytes, *removed_bytes as u64);
            // let's re-add it again
            let original_fees = apply_person(
                &drive,
                &contract,
                BlockInfo::default(),
                &person_0_original,
                true,
                transaction.as_ref(),
            );
            let original_bytes = original_fees.storage_fee / STORAGE_DISK_USAGE_CREDIT_PER_BYTE;
            assert_eq!(original_bytes, expected_added_bytes);
        }
        // now let's update it

        let update_fees = apply_person(
            &drive,
            &contract,
            BlockInfo::default(),
            &person_0_updated,
            true,
            transaction.as_ref(),
        );
        // we both add and remove bytes
        // this is because trees are added because of indexes, and also removed
        let added_bytes = update_fees.storage_fee / STORAGE_DISK_USAGE_CREDIT_PER_BYTE;
        let removed_bytes = update_fees
            .removed_bytes_from_identities
            .get(&owner_id)
            .unwrap()
            .get(0)
            .unwrap();

        // We added one byte, and since it is an index, and keys are doubled it's 2 extra bytes
        let expected_added_bytes = if using_history { 601 } else { 599 };
        assert_eq!(added_bytes, expected_added_bytes);

        let expected_removed_bytes = if using_history { 598 } else { 596 };

        assert_eq!(*removed_bytes, expected_removed_bytes);
    }

    #[test]
    fn test_fees_for_update_document_no_history_using_transaction() {
        test_fees_for_update_document(false, true)
    }

    #[test]
    fn test_fees_for_update_document_no_history_no_transaction() {
        test_fees_for_update_document(false, false)
    }

    #[test]
    fn test_fees_for_update_document_with_history_using_transaction() {
        test_fees_for_update_document(true, true)
    }

    #[test]
    fn test_fees_for_update_document_with_history_no_transaction() {
        test_fees_for_update_document(true, false)
    }

    #[test]
    fn test_fees_for_update_document_on_index_no_history_using_transaction() {
        test_fees_for_update_document_on_index(false, true)
    }

    #[test]
    fn test_fees_for_update_document_on_index_no_history_no_transaction() {
        test_fees_for_update_document_on_index(false, false)
    }

    #[test]
    fn test_fees_for_update_document_on_index_with_history_using_transaction() {
        test_fees_for_update_document_on_index(true, true)
    }

    #[test]
    fn test_fees_for_update_document_on_index_with_history_no_transaction() {
        test_fees_for_update_document_on_index(true, false)
    }

    fn test_worst_case_fees_for_update_document(using_history: bool, using_transaction: bool) {
        let config = DriveConfig {
            batching_enabled: true,
            batching_consistency_verification: true,
            has_raw_enabled: true,
            default_genesis_time: Some(0),
            encoding: DriveEncoding::DriveCbor,
            ..Default::default()
        };
        let tmp_dir = TempDir::new().unwrap();

        let drive: Drive =
            Drive::open(&tmp_dir, Some(config)).expect("expected to open Drive successfully");

        let transaction = if using_transaction {
            Some(drive.grove.start_transaction())
        } else {
            None
        };

        drive
            .create_initial_state_structure(transaction.as_ref())
            .expect("expected to create root tree successfully");

        let path = if using_history {
            "tests/supporting_files/contract/family/family-contract-with-history-only-message-index.json"
        } else {
            "tests/supporting_files/contract/family/family-contract-only-message-index.json"
        };

        // setup code
        let contract = setup_contract(&drive, path, None, transaction.as_ref());

        let id = [1u8; 32];
        let owner_id = [2u8; 32];
        let person_0_original = Person {
            id,
            owner_id,
            first_name: "Samuel".to_string(),
            middle_name: "Abraham".to_string(),
            last_name: "Westrich".to_string(),
            message: Some("My apples are safe".to_string()),
            age: 33,
        };

        let person_0_updated = Person {
            id: id.clone(),
            owner_id: owner_id.clone(),
            first_name: "Samuel".to_string(),
            middle_name: "Abraham".to_string(),
            last_name: "Westrich2".to_string(),
            message: Some("My apples are safe".to_string()),
            age: 35,
        };

        let original_fees = apply_person(
            &drive,
            &contract,
            BlockInfo::default(),
            &person_0_original,
            false,
            transaction.as_ref(),
        );
        let original_bytes = original_fees.storage_fee / STORAGE_DISK_USAGE_CREDIT_PER_BYTE;
        let expected_added_bytes = if using_history {
            //Explanation for 1350

            //todo
            1350
        } else {
            //Explanation for 1049

            // Document Storage

            //// Item
            // = 410 Bytes

            // Explanation for 410 storage_written_bytes

            // Key -> 65 bytes
            // 32 bytes for the key prefix
            // 32 bytes for the unique id
            // 1 byte for key_size (required space for 64)

            // Value -> 278
            //   1 for the flag option with flags
            //   1 for the flags size
            //   35 for flags 32 + 1 + 2
            //   1 for the enum type
            //   1 for item
            //   173 for item serialized bytes
            // 32 for node hash
            // 32 for value hash
            // 2 byte for the value_size (required space for above 128)

            // Parent Hook -> 67
            // Key Bytes 32
            // Hash Size 32
            // Key Length 1
            // Child Heights 2

            // Total 65 + 278 + 67 = 410

            //// Tree 1 / <Person Contract> / 1 / person / message
            // Key: My apples are safe
            // = 177 Bytes

            // Explanation for 177 storage_written_bytes

            // Key -> 51 bytes
            // 32 bytes for the key prefix
            // 18 bytes for the key "My apples are safe" 18 characters
            // 1 byte for key_size (required space for 50)

            // Value -> 73
            //   1 for the flag option with flags
            //   1 for the flags size
            //   35 for flags
            //   1 for the enum type
            //   1 for empty tree value
            // 32 for node hash
            // 0 for value hash
            // 2 byte for the value_size (required space for 73 + up to 256 for child key)

            // Parent Hook -> 53
            // Key Bytes 18
            // Hash Size 32
            // Key Length 1
            // Child Heights 2

            // Total 51 + 73 + 53 = 177

            //// Tree 1 / <Person Contract> / 1 / person / message / My apples are safe
            // Key: 0
            // = 143 Bytes

            // Explanation for 143 storage_written_bytes

            // Key -> 34 bytes
            // 32 bytes for the key prefix
            // 1 bytes for the key "My apples are safe" 18 characters
            // 1 byte for key_size (required space for 33)

            // Value -> 73
            //   1 for the flag option with flags
            //   1 for the flags size
            //   35 for flags
            //   1 for the enum type
            //   1 for empty tree value
            // 32 for node hash
            // 0 for value hash
            // 2 byte for the value_size (required space for 73 + up to 256 for child key)

            // Parent Hook -> 36
            // Key Bytes 1
            // Hash Size 32
            // Key Length 1
            // Child Heights 2

            // Total 34 + 73 + 36 = 143

            //// Ref 1 / <Person Contract> / 1 / person / message / My apples are safe
            // Reference to Serialized Item
            // = 319 Bytes

            // Explanation for 276 storage_written_bytes

            // Key -> 65 bytes
            // 32 bytes for the key prefix
            // 32 bytes for the unique id
            // 1 byte for key_size (required space for 64)

            // Value -> 144
            //   1 for the flag option with flags
            //   1 for the flags size
            //   35 for flags 32 + 1 + 2
            //   1 for the element type as reference
            //   1 for reference type as upstream root reference
            //   1 for reference root height
            //   36 for the reference path bytes ( 1 + 1 + 32 + 1 + 1)
            //   2 for the max reference hop
            // 32 for node hash
            // 32 for value hash
            // 2 byte for the value_size (required space for above 128)

            // Parent Hook -> 67
            // Key Bytes 32
            // Hash Size 32
            // Key Length 1
            // Child Heights 2

            // Total 65 + 144 + 67 = 276

            1006
        };
        assert_eq!(original_bytes, expected_added_bytes);

        // now let's update it 1 second later
        let update_fees = apply_person(
            &drive,
            &contract,
            BlockInfo::default_with_time(1000),
            &person_0_updated,
            false,
            transaction.as_ref(),
        );
        // we both add and remove bytes
        // this is because trees are added because of indexes, and also removed
        let added_bytes = update_fees.storage_fee / STORAGE_DISK_USAGE_CREDIT_PER_BYTE;

        let expected_added_bytes = if using_history { 1351 } else { 1007 };
        assert_eq!(added_bytes, expected_added_bytes);
    }

    #[test]
    fn test_worst_case_fees_for_update_document_no_history_using_transaction() {
        test_worst_case_fees_for_update_document(false, true)
    }

    #[test]
    fn test_worst_case_fees_for_update_document_no_history_no_transaction() {
        test_worst_case_fees_for_update_document(false, false)
    }

    #[test]
    fn test_worst_case_fees_for_update_document_with_history_using_transaction() {
        test_worst_case_fees_for_update_document(true, true)
    }

    #[test]
    fn test_worst_case_fees_for_update_document_with_history_no_transaction() {
        test_worst_case_fees_for_update_document(true, false)
    }

    #[derive(Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Person {
        #[serde(rename = "$id")]
        id: [u8; 32],
        #[serde(rename = "$ownerId")]
        owner_id: [u8; 32],
        first_name: String,
        middle_name: String,
        last_name: String,
        message: Option<String>,
        age: u8,
    }

    fn apply_person(
        drive: &Drive,
        contract: &Contract,
        block_info: BlockInfo,
        person: &Person,
        apply: bool,
        transaction: TransactionArg,
    ) -> FeeResult {
        let value = serde_json::to_value(person).expect("serialized person");
        let document_cbor = value_to_cbor(value, Some(defaults::PROTOCOL_VERSION));
        let document = Document::from_cbor(document_cbor.as_slice(), None, None)
            .expect("document should be properly deserialized");
        let document_type = contract
            .document_type_for_name("person")
            .expect("expected to get document type");
        let storage_flags = Some(StorageFlags::SingleEpochOwned(
            0,
            person
                .owner_id
                .clone()
                .try_into()
                .expect("expected to get owner_id"),
        ));

        drive
            .add_document_for_contract(
                DocumentAndContractInfo {
                    document_info: DocumentRefAndSerialization((
                        &document,
                        &document_cbor,
                        storage_flags.as_ref(),
                    )),
                    contract: &contract,
                    document_type,
                    owner_id: None,
                },
                true,
                block_info,
                apply,
                transaction,
            )
            .expect("expected to add document")
    }

    fn delete_person(
        drive: &Drive,
        contract: &Contract,
        block_info: BlockInfo,
        person: &Person,
        transaction: TransactionArg,
    ) -> FeeResult {
        let value = serde_json::to_value(person).expect("serialized person");
        let document_cbor = value_to_cbor(value, Some(defaults::PROTOCOL_VERSION));
        let document = Document::from_cbor(document_cbor.as_slice(), None, None)
            .expect("document should be properly deserialized");
        let document_type = contract
            .document_type_for_name("person")
            .expect("expected to get document type");

        let storage_flags = Some(StorageFlags::SingleEpochOwned(
            0,
            person
                .owner_id
                .clone()
                .try_into()
                .expect("expected to get owner_id"),
        ));

        drive
            .delete_document_for_contract(
                person.id,
                &contract,
                "person",
                Some(person.owner_id),
                block_info,
                true,
                transaction,
            )
            .expect("expected to remove person")
    }

    fn test_update_complex_person(
        using_history: bool,
        using_transaction: bool,
        using_batches: bool,
        using_has_raw: bool,
    ) {
        let config = DriveConfig {
            batching_enabled: using_batches,
            batching_consistency_verification: true,
            has_raw_enabled: using_has_raw,
            default_genesis_time: Some(0),
            encoding: DriveEncoding::DriveCbor,
            ..Default::default()
        };
        let tmp_dir = TempDir::new().unwrap();

        let drive: Drive =
            Drive::open(&tmp_dir, Some(config)).expect("expected to open Drive successfully");

        let transaction = if using_transaction {
            Some(drive.grove.start_transaction())
        } else {
            None
        };

        drive
            .create_initial_state_structure(transaction.as_ref())
            .expect("expected to create root tree successfully");

        let path = if using_history {
            "tests/supporting_files/contract/family/family-contract-with-history-only-message-index.json"
        } else {
            "tests/supporting_files/contract/family/family-contract-only-message-index.json"
        };

        // setup code
        let contract = setup_contract(&drive, path, None, transaction.as_ref());

        let person_0_original = Person {
            id: [0u8; 32],
            owner_id: [0u8; 32],
            first_name: "Samuel".to_string(),
            middle_name: "Abraham".to_string(),
            last_name: "Westrich".to_string(),
            message: Some("My apples are safe".to_string()),
            age: 33,
        };

        let person_0_updated = Person {
            id: [0u8; 32],
            owner_id: [0u8; 32],
            first_name: "Samuel".to_string(),
            middle_name: "Abraham".to_string(),
            last_name: "Westrich".to_string(),
            message: Some("Lemons are now my thing too".to_string()),
            age: 35,
        };

        let person_1_original = Person {
            id: [1u8; 32],
            owner_id: [1u8; 32],
            first_name: "Wisdom".to_string(),
            middle_name: "Madabuchukwu".to_string(),
            last_name: "Ogwu".to_string(),
            message: Some("Cantaloupe is the best fruit under the sun".to_string()),
            age: 20,
        };

        let person_1_updated = Person {
            id: [1u8; 32],
            owner_id: [1u8; 32],
            first_name: "Wisdom".to_string(),
            middle_name: "Madabuchukwu".to_string(),
            last_name: "Ogwu".to_string(),
            message: Some("My apples are safe".to_string()),
            age: 22,
        };

        apply_person(
            &drive,
            &contract,
            BlockInfo::default(),
            &person_0_original,
            true,
            transaction.as_ref(),
        );
        apply_person(
            &drive,
            &contract,
            BlockInfo::default(),
            &person_1_original,
            true,
            transaction.as_ref(),
        );
        apply_person(
            &drive,
            &contract,
            BlockInfo::default_with_time(100),
            &person_0_updated,
            true,
            transaction.as_ref(),
        );
        apply_person(
            &drive,
            &contract,
            BlockInfo::default_with_time(100),
            &person_1_updated,
            true,
            transaction.as_ref(),
        );
    }

    #[test]
    fn test_update_complex_person_with_history_no_transaction_using_batches_and_has_raw() {
        test_update_complex_person(true, false, true, true)
    }

    #[test]
    fn test_update_complex_person_with_history_no_transaction_using_batches_and_get_raw() {
        test_update_complex_person(true, false, true, false)
    }

    #[test]
    fn test_update_complex_person_with_history_with_transaction_using_batches_and_has_raw() {
        test_update_complex_person(true, true, true, true)
    }

    #[test]
    fn test_update_complex_person_with_history_with_transaction_using_batches_and_get_raw() {
        test_update_complex_person(true, true, true, false)
    }

    #[test]
    fn test_update_complex_person_with_history_no_transaction_no_batches_and_has_raw() {
        test_update_complex_person(true, false, false, true)
    }

    #[test]
    fn test_update_complex_person_with_history_no_transaction_no_batches_and_get_raw() {
        test_update_complex_person(true, false, false, false)
    }

    #[test]
    fn test_update_complex_person_with_history_with_transaction_no_batches_and_has_raw() {
        test_update_complex_person(true, true, false, true)
    }

    #[test]
    fn test_update_complex_person_with_history_with_transaction_no_batches_and_get_raw() {
        test_update_complex_person(true, true, false, false)
    }

    #[test]
    fn test_update_complex_person_no_history_no_transaction_using_batches_and_has_raw() {
        test_update_complex_person(false, false, true, true)
    }

    #[test]
    fn test_update_complex_person_no_history_no_transaction_using_batches_and_get_raw() {
        test_update_complex_person(false, false, true, false)
    }

    #[test]
    fn test_update_complex_person_no_history_with_transaction_using_batches_and_has_raw() {
        test_update_complex_person(false, true, true, true)
    }

    #[test]
    fn test_update_complex_person_no_history_with_transaction_using_batches_and_get_raw() {
        test_update_complex_person(false, true, true, false)
    }

    #[test]
    fn test_update_complex_person_no_history_no_transaction_no_batches_and_has_raw() {
        test_update_complex_person(false, false, false, true)
    }

    #[test]
    fn test_update_complex_person_no_history_no_transaction_no_batches_and_get_raw() {
        test_update_complex_person(false, false, false, false)
    }

    #[test]
    fn test_update_complex_person_no_history_with_transaction_no_batches_and_has_raw() {
        test_update_complex_person(false, true, false, true)
    }

    #[test]
    fn test_update_complex_person_no_history_with_transaction_no_batches_and_get_raw() {
        test_update_complex_person(false, true, false, false)
    }

    #[test]
    fn test_update_document_without_apply_should_calculate_storage_fees() {
        let tmp_dir = TempDir::new().unwrap();

        let drive: Drive =
            Drive::open(&tmp_dir, None).expect("expected to open Drive successfully");

        drive
            .create_initial_state_structure(None)
            .expect("expected to create root tree successfully");

        // Create a contract

        let block_info = BlockInfo::default();
        let owner_id = dpp::identifier::Identifier::new([2u8; 32]);

        let documents = json!({
            "niceDocument": {
                "type": "object",
                "properties": {
                    "name": {
                        "type": "string"
                    }
                },
                "required": [
                    "$createdAt"
                ],
                "additionalProperties": false
            }
        });

        let protocol_version_validator = ProtocolVersionValidator::new(
            LATEST_VERSION,
            LATEST_VERSION,
            COMPATIBILITY_MAP.clone(),
        );

        let data_contract_validator =
            DataContractValidator::new(Arc::new(protocol_version_validator));
        let factory = DataContractFactory::new(1, data_contract_validator);

        let contract = factory
            .create(owner_id.clone(), documents)
            .expect("data in fixture should be correct");

        let contract_cbor = contract.to_cbor().expect("should encode contract to cbor");

        // TODO: Create method doesn't initiate document_types. It must be fixed
        let contract = DataContract::from_cbor(contract_cbor.clone())
            .expect("should create decode contract from cbor");

        drive
            .apply_contract(
                &contract,
                contract_cbor.clone(),
                block_info.clone(),
                true,
                StorageFlags::optional_default_as_ref(),
                None,
            )
            .expect("should apply contract");

        // Create a document factory

        let protocol_version_validator = Arc::new(ProtocolVersionValidator::new(
            LATEST_VERSION,
            LATEST_VERSION,
            COMPATIBILITY_MAP.clone(),
        ));

        let document_validator = DocumentValidator::new(protocol_version_validator);

        let document_factory = DocumentFactory::new(
            1,
            document_validator,
            mocks::FetchAndValidateDataContract {},
        );

        // Create a document

        let document_type = "niceDocument".to_string();

        let mut document = document_factory
            .create(
                contract.clone(),
                owner_id.clone(),
                document_type.clone(),
                json!({ "name": "Ivan" }),
            )
            .expect("should create a document");

        let document_cbor = document.to_cbor().expect("should encode to cbor");

        let storage_flags = StorageFlags::SingleEpochOwned(0, owner_id.to_buffer());

        let create_fees = drive
            .add_serialized_document_for_contract(
                &document_cbor,
                &contract,
                &document_type,
                Some(owner_id.to_buffer()),
                false,
                block_info.clone(),
                true,
                Some(&storage_flags),
                None,
            )
            .expect("should create document");

        assert_ne!(create_fees.storage_fee, 0);

        // Update the document in a second

        document
            .set("name", Value::String("Ivaaaaaaaaaan!".to_string()))
            .expect("should change name");

        let document_cbor = document.to_cbor().expect("should encode to cbor");

        let block_info = BlockInfo::default_with_time(10000);

        let update_fees = drive
            .update_document_for_contract_cbor(
                &document_cbor,
                &contract_cbor,
                &document_type,
                Some(owner_id.to_buffer()),
                block_info,
                false,
                Some(&storage_flags),
                None,
            )
            .expect("should update document");

        assert_ne!(update_fees.storage_fee, 0);
    }
}
