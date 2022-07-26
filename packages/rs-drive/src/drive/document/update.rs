use std::collections::HashSet;
use std::option::Option::None;

use grovedb::{Element, TransactionArg};

use crate::contract::document::Document;
use crate::contract::Contract;
use crate::drive::defaults::CONTRACT_DOCUMENTS_PATH_HEIGHT;
use crate::drive::document::{
    contract_document_type_path,
    contract_documents_keeping_history_primary_key_path_for_document_id,
    contract_documents_primary_key_path,
};
use crate::drive::flags::StorageFlags;
use crate::drive::object_size_info::DocumentInfo::{DocumentAndSerialization, DocumentSize};
use crate::drive::object_size_info::KeyValueInfo::KeyRefRequest;
use crate::drive::object_size_info::PathKeyElementInfo::PathKeyElement;
use crate::drive::object_size_info::{DocumentAndContractInfo, PathKeyInfo};
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fee::calculate_fee;
use crate::fee::op::DriveOperation;

use dpp::data_contract::extra::DriveContractExt;

impl Drive {
    pub fn update_document_for_contract_cbor(
        &self,
        serialized_document: &[u8],
        contract_cbor: &[u8],
        document_type: &str,
        owner_id: Option<&[u8]>,
        block_time: f64,
        apply: bool,
        storage_flags: StorageFlags,
        transaction: TransactionArg,
    ) -> Result<(i64, u64), Error> {
        let contract = <Contract as DriveContractExt>::from_cbor(contract_cbor, None)?;

        let document = Document::from_cbor(serialized_document, None, owner_id)?;

        self.update_document_for_contract(
            &document,
            serialized_document,
            &contract,
            document_type,
            owner_id,
            block_time,
            apply,
            storage_flags,
            transaction,
        )
    }

    pub fn update_serialized_document_for_contract(
        &self,
        serialized_document: &[u8],
        contract: &Contract,
        document_type: &str,
        owner_id: Option<&[u8]>,
        block_time: f64,
        apply: bool,
        storage_flags: StorageFlags,
        transaction: TransactionArg,
    ) -> Result<(i64, u64), Error> {
        let document = Document::from_cbor(serialized_document, None, owner_id)?;

        self.update_document_for_contract(
            &document,
            serialized_document,
            contract,
            document_type,
            owner_id,
            block_time,
            apply,
            storage_flags,
            transaction,
        )
    }

    pub fn update_document_for_contract(
        &self,
        document: &Document,
        serialized_document: &[u8],
        contract: &Contract,
        document_type_name: &str,
        owner_id: Option<&[u8]>,
        block_time: f64,
        apply: bool,
        storage_flags: StorageFlags,
        transaction: TransactionArg,
    ) -> Result<(i64, u64), Error> {
        let mut drive_operations: Vec<DriveOperation> = vec![];

        let document_type = contract.document_type_for_name(document_type_name)?;

        let document_info = if apply {
            DocumentAndSerialization((document, serialized_document, &storage_flags))
        } else {
            let element_size = Element::Item(
                serialized_document.to_vec(),
                StorageFlags::to_element_flags(&storage_flags),
            )
            .serialized_byte_size();

            DocumentSize(element_size)
        };

        self.update_document_for_contract_operations(
            DocumentAndContractInfo {
                document_info,
                contract,
                document_type,
                owner_id,
            },
            block_time,
            apply,
            transaction,
            &mut drive_operations,
        )?;
        let fees = calculate_fee(None, Some(drive_operations))?;
        Ok(fees)
    }

    pub(crate) fn update_document_for_contract_operations(
        &self,
        document_and_contract_info: DocumentAndContractInfo,
        block_time: f64,
        apply: bool,
        transaction: TransactionArg,
        drive_operations: &mut Vec<DriveOperation>,
    ) -> Result<(), Error> {
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
            self.add_document_for_contract_operations(
                document_and_contract_info,
                false,
                block_time,
                apply,
                transaction,
                &mut batch_operations,
            )?;
            return Ok(());
        }

        let contract = document_and_contract_info.contract;
        let document_type = document_and_contract_info.document_type;
        let owner_id = document_and_contract_info.owner_id;

        if let DocumentAndSerialization((document, _serialized_document, storage_flags)) =
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
            let document_reference =
                Element::Reference(reference_path, storage_flags.to_element_flags());

            // next we need to get the old document from storage
            let old_document_element: Element = if document_type.documents_keep_history {
                let contract_documents_keeping_history_primary_key_path_for_document_id =
                    contract_documents_keeping_history_primary_key_path_for_document_id(
                        contract.id.as_bytes(),
                        document_type.name.as_str(),
                        document.id.as_slice(),
                    );
                self.grove_get(
                    contract_documents_keeping_history_primary_key_path_for_document_id,
                    KeyRefRequest(&[0]),
                    transaction,
                    &mut batch_operations,
                )?
            } else {
                self.grove_get(
                    contract_documents_primary_key_path,
                    KeyRefRequest(document.id.as_slice()),
                    transaction,
                    &mut batch_operations,
                )?
            }
            .unwrap();

            // we need to store the document for it's primary key
            // we should be overriding if the document_type does not have history enabled
            self.add_document_to_primary_storage(
                &document_and_contract_info,
                block_time,
                true,
                apply,
                transaction,
                &mut batch_operations,
            )?;

            let old_document =
                if let Element::Item(old_serialized_document, _) = old_document_element {
                    Ok(Document::from_cbor(
                        old_serialized_document.as_slice(),
                        None,
                        owner_id,
                    )?)
                } else {
                    Err(Error::Drive(DriveError::CorruptedDocumentNotItem(
                        "old document is not an item",
                    )))
                }?;

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

                let mut change_occurred_on_index = document_top_field != old_document_top_field;

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

                let mut old_index_path = index_path.clone();
                // we push the actual value of the index path
                index_path.push(document_top_field);
                // the index path is now something like Contracts/ContractID/Documents(1)/$ownerId/<ownerId>

                old_index_path.push(old_document_top_field);

                for i in 1..index.properties.len() {
                    let index_property = index.properties.get(i).ok_or(Error::Drive(
                        DriveError::CorruptedContractIndexes("invalid contract indices"),
                    ))?;

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

                    change_occurred_on_index |= document_index_field != old_document_index_field;

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
                    old_index_path.push(Vec::from(index_property.name.as_bytes()));

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
                        old_index_path.push(vec![0]);

                        let old_index_path_slices: Vec<&[u8]> =
                            old_index_path.iter().map(|x| x.as_slice()).collect();

                        // here we should return an error if the element already exists
                        self.batch_delete_up_tree_while_empty(
                            old_index_path_slices,
                            document.id.as_slice(),
                            Some(CONTRACT_DOCUMENTS_PATH_HEIGHT),
                            transaction,
                            &mut batch_operations,
                        )?;
                    } else {
                        let old_index_path_slices: Vec<&[u8]> =
                            old_index_path.iter().map(|x| x.as_slice()).collect();

                        // here we should return an error if the element already exists
                        self.batch_delete_up_tree_while_empty(
                            old_index_path_slices,
                            &[0],
                            Some(CONTRACT_DOCUMENTS_PATH_HEIGHT),
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
                            apply,
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
        self.apply_batch_drive_operations(apply, transaction, batch_operations, drive_operations)
    }
}

#[cfg(test)]
mod tests {
    use grovedb::TransactionArg;
    use std::option::Option::None;

    use rand::Rng;
    use serde::{Deserialize, Serialize};
    use serde_json::json;
    use tempfile::TempDir;

    use super::*;
    use crate::common::{json_document_to_cbor, setup_contract, value_to_cbor};
    use crate::contract::{document::Document, Contract};
    use crate::drive::config::{DriveConfig, DriveEncoding};
    use crate::drive::flags::StorageFlags;
    use crate::drive::object_size_info::DocumentAndContractInfo;
    use crate::drive::object_size_info::DocumentInfo::DocumentAndSerialization;
    use crate::drive::{defaults, Drive};
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
                0f64,
                true,
                StorageFlags::default(),
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
                0f64,
                true,
                StorageFlags::default(),
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
                true,
                StorageFlags::default(),
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
                0f64,
                true,
                StorageFlags::default(),
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

        let storage_flags = StorageFlags { epoch: 0 };

        drive
            .add_document_for_contract(
                DocumentAndContractInfo {
                    document_info: DocumentAndSerialization((
                        &alice_profile,
                        alice_profile_cbor.as_slice(),
                        &storage_flags,
                    )),
                    contract: &contract,
                    document_type,
                    owner_id: None,
                },
                true,
                0f64,
                true,
                None,
            )
            .expect("should create alice profile");

        let sql_string = "select * from profile";
        let query = DriveQuery::from_sql_expr(sql_string, &contract).expect("should build query");

        let (results_no_transaction, _, _) = query
            .execute_no_proof(&drive, None)
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
                true,
                StorageFlags::default(),
                None,
            )
            .expect("should update alice profile");

        let (results_no_transaction, _, _) = query
            .execute_no_proof(&drive, None)
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
                0f64,
                true,
                StorageFlags::default(),
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

        let storage_flags = StorageFlags { epoch: 0 };

        drive
            .add_document_for_contract(
                DocumentAndContractInfo {
                    document_info: DocumentAndSerialization((
                        &alice_profile,
                        alice_profile_cbor.as_slice(),
                        &storage_flags,
                    )),
                    contract: &contract,
                    document_type,
                    owner_id: None,
                },
                true,
                0f64,
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
            .execute_no_proof(&drive, None)
            .expect("expected to execute query");

        assert_eq!(results_no_transaction.len(), 1);

        let db_transaction = drive.grove.start_transaction();

        let (results_on_transaction, _, _) = query
            .execute_no_proof(&drive, Some(&db_transaction))
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
                true,
                StorageFlags::default(),
                Some(&db_transaction),
            )
            .expect("should update alice profile");

        let (results_on_transaction, _, _) = query
            .execute_no_proof(&drive, Some(&db_transaction))
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
                0f64,
                true,
                StorageFlags::default(),
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

        let storage_flags = StorageFlags { epoch: 0 };

        drive
            .add_document_for_contract(
                DocumentAndContractInfo {
                    document_info: DocumentAndSerialization((
                        &alice_profile,
                        alice_profile_cbor.as_slice(),
                        &storage_flags,
                    )),
                    contract: &contract,
                    document_type,
                    owner_id: None,
                },
                true,
                0f64,
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
            .execute_no_proof(&drive, None)
            .expect("expected to execute query");

        assert_eq!(results_no_transaction.len(), 1);

        let db_transaction = drive.grove.start_transaction();

        let (results_on_transaction, _, _) = query
            .execute_no_proof(&drive, Some(&db_transaction))
            .expect("expected to execute query");

        assert_eq!(results_on_transaction.len(), 1);

        drive
            .delete_document_for_contract(
                &alice_profile.id,
                &contract,
                "profile",
                None,
                true,
                Some(&db_transaction),
            )
            .expect("expected to delete document");

        let (results_on_transaction, _, _) = query
            .execute_no_proof(&drive, Some(&db_transaction))
            .expect("expected to execute query");

        assert_eq!(results_on_transaction.len(), 0);

        drive
            .grove
            .rollback_transaction(&db_transaction)
            .expect("expected to rollback transaction");

        let (results_on_transaction, _, _) = query
            .execute_no_proof(&drive, Some(&db_transaction))
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
                true,
                StorageFlags::default(),
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
                0f64,
                true,
                StorageFlags::default(),
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
                0f64,
                true,
                StorageFlags::default(),
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
                0f64,
                true,
                StorageFlags::default(),
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
                Some(&random_owner_id),
                false,
                0f64,
                true,
                StorageFlags::default(),
                Some(&db_transaction),
            )
            .expect("expected to insert a document successfully");

        drive
            .update_serialized_document_for_contract(
                &dashpay_cr_serialized_document,
                &contract,
                "contactRequest",
                Some(&random_owner_id),
                0f64,
                true,
                StorageFlags::default(),
                Some(&db_transaction),
            )
            .expect_err("expected not to be able to update a non mutable document");

        drive
            .add_serialized_document_for_contract(
                &dashpay_cr_serialized_document,
                &contract,
                "contactRequest",
                Some(&random_owner_id),
                true,
                0f64,
                true,
                StorageFlags::default(),
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
                Some(&random_owner_id),
                false,
                0f64,
                true,
                StorageFlags::default(),
                Some(&db_transaction),
            )
            .expect("expected to insert a document successfully");

        drive
            .update_serialized_document_for_contract(
                &dashpay_profile_updated_public_message_serialized_document,
                &contract,
                "profile",
                Some(&random_owner_id),
                0f64,
                true,
                StorageFlags::default(),
                Some(&db_transaction),
            )
            .expect("expected to update a document with history successfully");
    }

    #[derive(Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Person {
        #[serde(rename = "$id")]
        id: Vec<u8>,
        #[serde(rename = "$ownerId")]
        owner_id: Vec<u8>,
        first_name: String,
        middle_name: String,
        last_name: String,
        message: Option<String>,
        age: u8,
    }

    fn apply_person(
        drive: &Drive,
        contract: &Contract,
        block_time: u64,
        person: &Person,
        transaction: TransactionArg,
    ) {
        let value = serde_json::to_value(person).expect("serialized person");
        let document_cbor = value_to_cbor(value, Some(defaults::PROTOCOL_VERSION));
        let document = Document::from_cbor(document_cbor.as_slice(), None, None)
            .expect("document should be properly deserialized");
        let document_type = contract
            .document_type_for_name("person")
            .expect("expected to get document type");

        let storage_flags = StorageFlags { epoch: 0 };

        drive
            .add_document_for_contract(
                DocumentAndContractInfo {
                    document_info: DocumentAndSerialization((
                        &document,
                        &document_cbor,
                        &storage_flags,
                    )),
                    contract: &contract,
                    document_type,
                    owner_id: None,
                },
                true,
                block_time as f64,
                true,
                transaction,
            )
            .expect("expected to add document");
    }

    fn test_update_complex_person_with_history(
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

        // setup code
        let contract = setup_contract(
            &drive,
            "tests/supporting_files/contract/family/family-contract-with-history-only-message-index.json",
            None,
            transaction.as_ref(),
        );

        let person_0_original = Person {
            id: [0u8; 32].to_vec(),
            owner_id: [0u8; 32].to_vec(),
            first_name: "Samuel".to_string(),
            middle_name: "Abraham".to_string(),
            last_name: "Westrich".to_string(),
            message: Some("My apples are safe".to_string()),
            age: 33,
        };

        let person_0_updated = Person {
            id: [0u8; 32].to_vec(),
            owner_id: [0u8; 32].to_vec(),
            first_name: "Samuel".to_string(),
            middle_name: "Abraham".to_string(),
            last_name: "Westrich".to_string(),
            message: Some("Lemons are now my thing".to_string()),
            age: 35,
        };

        let person_1_original = Person {
            id: [1u8; 32].to_vec(),
            owner_id: [1u8; 32].to_vec(),
            first_name: "Wisdom".to_string(),
            middle_name: "Madabuchukwu".to_string(),
            last_name: "Ogwu".to_string(),
            message: Some("Cantaloupe is the best fruit".to_string()),
            age: 20,
        };

        let person_1_updated = Person {
            id: [1u8; 32].to_vec(),
            owner_id: [1u8; 32].to_vec(),
            first_name: "Wisdom".to_string(),
            middle_name: "Madabuchukwu".to_string(),
            last_name: "Ogwu".to_string(),
            message: Some("My apples are safe".to_string()),
            age: 22,
        };

        apply_person(
            &drive,
            &contract,
            0,
            &person_0_original,
            transaction.as_ref(),
        );
        apply_person(
            &drive,
            &contract,
            0,
            &person_1_original,
            transaction.as_ref(),
        );
        apply_person(
            &drive,
            &contract,
            100,
            &person_0_updated,
            transaction.as_ref(),
        );
        apply_person(
            &drive,
            &contract,
            100,
            &person_1_updated,
            transaction.as_ref(),
        );
    }

    #[test]
    fn test_update_complex_person_with_history_no_transaction_using_batches_and_has_raw() {
        test_update_complex_person_with_history(false, true, true)
    }

    #[test]
    fn test_update_complex_person_with_history_no_transaction_using_batches_and_get_raw() {
        test_update_complex_person_with_history(false, true, false)
    }

    #[test]
    fn test_update_complex_person_with_history_with_transaction_using_batches_and_has_raw() {
        test_update_complex_person_with_history(true, true, true)
    }

    #[test]
    fn test_update_complex_person_with_history_with_transaction_using_batches_and_get_raw() {
        test_update_complex_person_with_history(true, true, false)
    }

    #[test]
    fn test_update_complex_person_with_history_no_transaction_no_batches_and_has_raw() {
        test_update_complex_person_with_history(false, false, true)
    }

    #[test]
    fn test_update_complex_person_with_history_no_transaction_no_batches_and_get_raw() {
        test_update_complex_person_with_history(false, false, false)
    }

    #[test]
    fn test_update_complex_person_with_history_with_transaction_no_batches_and_has_raw() {
        test_update_complex_person_with_history(true, false, true)
    }

    #[test]
    fn test_update_complex_person_with_history_with_transaction_no_batches_and_get_raw() {
        test_update_complex_person_with_history(true, false, false)
    }
}
