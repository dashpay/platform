use grovedb::{Element, TransactionArg};

use crate::contract::document::Document;
use crate::contract::Contract;
use crate::drive::defaults::CONTRACT_DOCUMENTS_PATH_HEIGHT;
use crate::drive::document::{contract_document_type_path, contract_documents_primary_key_path};
use crate::drive::object_size_info::KeyValueInfo::KeyRefRequest;
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fee::calculate_fee;
use crate::fee::op::DriveOperation;

impl Drive {
    pub fn delete_document_for_contract(
        &self,
        document_id: &[u8],
        contract: &Contract,
        document_type_name: &str,
        owner_id: Option<&[u8]>,
        apply: bool,
        transaction: TransactionArg,
    ) -> Result<(i64, u64), Error> {
        let mut drive_operations: Vec<DriveOperation> = vec![];
        self.delete_document_for_contract_operations(
            document_id,
            contract,
            document_type_name,
            owner_id,
            apply,
            transaction,
            &mut drive_operations,
        )?;
        let fees = calculate_fee(None, Some(drive_operations))?;
        Ok(fees)
    }

    pub fn delete_document_for_contract_cbor(
        &self,
        document_id: &[u8],
        contract_cbor: &[u8],
        document_type_name: &str,
        owner_id: Option<&[u8]>,
        apply: bool,
        transaction: TransactionArg,
    ) -> Result<(i64, u64), Error> {
        let contract = Contract::from_cbor(contract_cbor, None)?;
        self.delete_document_for_contract(
            document_id,
            &contract,
            document_type_name,
            owner_id,
            apply,
            transaction,
        )
    }

    pub fn delete_document_for_contract_operations(
        &self,
        document_id: &[u8],
        contract: &Contract,
        document_type_name: &str,
        owner_id: Option<&[u8]>,
        apply: bool,
        transaction: TransactionArg,
        drive_operations: &mut Vec<DriveOperation>,
    ) -> Result<(), Error> {
        let mut batch_operations: Vec<DriveOperation> = vec![];
        let document_type = contract.document_type_for_name(document_type_name)?;

        if !document_type.documents_mutable {
            return Err(Error::Drive(DriveError::UpdatingReadOnlyImmutableDocument(
                "documents for this contract are not mutable",
            )));
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
            KeyRefRequest(document_id),
            transaction,
            &mut batch_operations,
        )?;

        if document_element.is_none() {
            return Err(Error::Drive(DriveError::DeletingDocumentThatDoesNotExist(
                "document being deleted does not exist",
            )));
        }

        let document_bytes: Vec<u8> = match document_element.unwrap() {
            Element::Item(data, _) => data,
            _ => todo!(), // TODO: how should this be handled, possibility that document might not be in storage
        };

        let document = Document::from_cbor(document_bytes.as_slice(), None, owner_id)?;

        // third we need to delete the document for it's primary key
        self.batch_delete(
            contract_documents_primary_key_path,
            document_id,
            true, // not a tree, irrelevant
            transaction,
            &mut batch_operations,
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
            let top_index_property = index.properties.get(0).ok_or(Error::Drive(
                DriveError::CorruptedContractIndexes("invalid contract indices"),
            ))?;
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
                let index_property = index.properties.get(i).ok_or(Error::Drive(
                    DriveError::CorruptedContractIndexes("invalid contract indices"),
                ))?;

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
                self.batch_delete_up_tree_while_empty(
                    index_path_slices,
                    document_id,
                    Some(CONTRACT_DOCUMENTS_PATH_HEIGHT),
                    transaction,
                    &mut batch_operations,
                )?;
            } else {
                let index_path_slices: Vec<&[u8]> =
                    index_path.iter().map(|x| x.as_slice()).collect();

                // here we should return an error if the element already exists
                self.batch_delete_up_tree_while_empty(
                    index_path_slices,
                    &[0],
                    Some(CONTRACT_DOCUMENTS_PATH_HEIGHT),
                    transaction,
                    &mut batch_operations,
                )?;
            }
        }
        self.apply_batch(apply, transaction, batch_operations, drive_operations)
    }
}

#[cfg(test)]
mod tests {
    use rand::Rng;
    use serde_json::json;
    use std::option::Option::None;
    use tempfile::TempDir;

    use crate::common::{
        cbor_from_hex, json_document_to_cbor, setup_contract, setup_contract_from_hex,
        value_to_cbor,
    };
    use crate::contract::document::Document;
    use crate::drive::document::tests::setup_dashpay;
    use crate::drive::flags::StorageFlags;
    use crate::drive::object_size_info::DocumentAndContractInfo;
    use crate::drive::object_size_info::DocumentInfo::DocumentAndSerialization;
    use crate::drive::Drive;
    use crate::query::DriveQuery;

    #[test]
    fn test_add_and_remove_family_one_document_no_transaction() {
        let tmp_dir = TempDir::new().unwrap();
        let drive: Drive = Drive::open(tmp_dir, None).expect("expected to open Drive successfully");

        drive
            .create_root_tree(None)
            .expect("expected to create root tree successfully");

        let contract = setup_contract(
            &drive,
            "tests/supporting_files/contract/family/family-contract-reduced.json",
            None,
            None,
        );

        let person_serialized_document = json_document_to_cbor(
            "tests/supporting_files/contract/family/person0.json",
            Some(1),
        );

        let random_owner_id = rand::thread_rng().gen::<[u8; 32]>();

        let document =
            Document::from_cbor(&person_serialized_document, None, Some(&random_owner_id))
                .expect("expected to deserialize the document");

        let document_type = contract
            .document_type_for_name("person")
            .expect("expected to get a document type");

        let storage_flags = StorageFlags { epoch: 0 };

        drive
            .add_document_for_contract(
                DocumentAndContractInfo {
                    document_info: DocumentAndSerialization((
                        &document,
                        &person_serialized_document,
                        &storage_flags,
                    )),
                    contract: &contract,
                    document_type,
                    owner_id: None,
                },
                false,
                0f64,
                true,
                None,
            )
            .expect("expected to insert a document successfully");

        let sql_string =
            "select * from person where firstName = 'Samuel' order by firstName asc limit 100";
        let query = DriveQuery::from_sql_expr(sql_string, &contract).expect("should build query");

        let (results_no_transaction, _, _) = query
            .execute_no_proof(&drive, None)
            .expect("expected to execute query");

        assert_eq!(results_no_transaction.len(), 1);

        let (results_on_transaction, _, _) = query
            .execute_no_proof(&drive, None)
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
                true,
                None,
            )
            .expect("expected to be able to delete the document");

        let (results_on_transaction, _, _) = query
            .execute_no_proof(&drive, None)
            .expect("expected to execute query");

        assert_eq!(results_on_transaction.len(), 0);
    }

    #[test]
    fn test_add_and_remove_family_one_document() {
        let tmp_dir = TempDir::new().unwrap();
        let drive: Drive = Drive::open(tmp_dir, None).expect("expected to open Drive successfully");

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

        let person_serialized_document = json_document_to_cbor(
            "tests/supporting_files/contract/family/person0.json",
            Some(1),
        );

        let random_owner_id = rand::thread_rng().gen::<[u8; 32]>();

        let document =
            Document::from_cbor(&person_serialized_document, None, Some(&random_owner_id))
                .expect("expected to deserialize the document");

        let document_type = contract
            .document_type_for_name("person")
            .expect("expected to get a document type");

        let storage_flags = StorageFlags { epoch: 0 };

        drive
            .add_document_for_contract(
                DocumentAndContractInfo {
                    document_info: DocumentAndSerialization((
                        &document,
                        &person_serialized_document,
                        &storage_flags,
                    )),
                    contract: &contract,
                    document_type,
                    owner_id: None,
                },
                false,
                0f64,
                true,
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

        let (results_no_transaction, _, _) = query
            .execute_no_proof(&drive, None)
            .expect("expected to execute query");

        assert_eq!(results_no_transaction.len(), 1);

        let db_transaction = drive.grove.start_transaction();

        let (results_on_transaction, _, _) = query
            .execute_no_proof(&drive, Some(&db_transaction))
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
                true,
                Some(&db_transaction),
            )
            .expect("expected to be able to delete the document");

        drive
            .grove
            .commit_transaction(db_transaction)
            .expect("unable to commit transaction");

        let db_transaction = drive.grove.start_transaction();

        let (results_on_transaction, _, _) = query
            .execute_no_proof(&drive, Some(&db_transaction))
            .expect("expected to execute query");

        assert_eq!(results_on_transaction.len(), 0);
    }

    #[test]
    fn test_add_and_remove_family_documents() {
        let tmp_dir = TempDir::new().unwrap();
        let drive: Drive = Drive::open(tmp_dir, None).expect("expected to open Drive successfully");

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

        let person_serialized_document = json_document_to_cbor(
            "tests/supporting_files/contract/family/person0.json",
            Some(1),
        );

        let random_owner_id = rand::thread_rng().gen::<[u8; 32]>();

        let document =
            Document::from_cbor(&person_serialized_document, None, Some(&random_owner_id))
                .expect("expected to deserialize the document");

        let document_type = contract
            .document_type_for_name("person")
            .expect("expected to get a document type");

        let storage_flags = StorageFlags { epoch: 0 };

        drive
            .add_document_for_contract(
                DocumentAndContractInfo {
                    document_info: DocumentAndSerialization((
                        &document,
                        &person_serialized_document,
                        &storage_flags,
                    )),
                    contract: &contract,
                    document_type,
                    owner_id: None,
                },
                false,
                0f64,
                true,
                Some(&db_transaction),
            )
            .expect("expected to insert a document successfully");

        let person_serialized_document = json_document_to_cbor(
            "tests/supporting_files/contract/family/person1.json",
            Some(1),
        );

        let random_owner_id = rand::thread_rng().gen::<[u8; 32]>();

        let document =
            Document::from_cbor(&person_serialized_document, None, Some(&random_owner_id))
                .expect("expected to deserialize the document");

        let document_type = contract
            .document_type_for_name("person")
            .expect("expected to get a document type");

        let storage_flags = StorageFlags { epoch: 0 };

        drive
            .add_document_for_contract(
                DocumentAndContractInfo {
                    document_info: DocumentAndSerialization((
                        &document,
                        &person_serialized_document,
                        &storage_flags,
                    )),
                    contract: &contract,
                    document_type,
                    owner_id: None,
                },
                false,
                0f64,
                true,
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

        let (results_no_transaction, _, _) = query
            .execute_no_proof(&drive, None)
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
                true,
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

        let (results_no_transaction, _, _) = query
            .execute_no_proof(&drive, None)
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
                true,
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

        let (results_no_transaction, _, _) = query
            .execute_no_proof(&drive, None)
            .expect("expected to execute query");

        assert_eq!(results_no_transaction.len(), 0);
    }

    #[test]
    fn test_add_and_remove_family_documents_with_empty_fields() {
        let tmp_dir = TempDir::new().unwrap();
        let drive: Drive = Drive::open(tmp_dir, None).expect("expected to open Drive successfully");

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

        let person_serialized_document = json_document_to_cbor(
            "tests/supporting_files/contract/family/person0.json",
            Some(1),
        );

        let random_owner_id = rand::thread_rng().gen::<[u8; 32]>();

        let document =
            Document::from_cbor(&person_serialized_document, None, Some(&random_owner_id))
                .expect("expected to deserialize the document");

        let document_type = contract
            .document_type_for_name("person")
            .expect("expected to get a document type");

        let storage_flags = StorageFlags { epoch: 0 };

        drive
            .add_document_for_contract(
                DocumentAndContractInfo {
                    document_info: DocumentAndSerialization((
                        &document,
                        &person_serialized_document,
                        &storage_flags,
                    )),
                    contract: &contract,
                    document_type,
                    owner_id: None,
                },
                false,
                0f64,
                true,
                Some(&db_transaction),
            )
            .expect("expected to insert a document successfully");

        let person_serialized_document = json_document_to_cbor(
            "tests/supporting_files/contract/family/person2-no-middle-name.json",
            Some(1),
        );

        let random_owner_id = rand::thread_rng().gen::<[u8; 32]>();

        let document =
            Document::from_cbor(&person_serialized_document, None, Some(&random_owner_id))
                .expect("expected to deserialize the document");

        let document_type = contract
            .document_type_for_name("person")
            .expect("expected to get a document type");

        let storage_flags = StorageFlags { epoch: 0 };

        drive
            .add_document_for_contract(
                DocumentAndContractInfo {
                    document_info: DocumentAndSerialization((
                        &document,
                        &person_serialized_document,
                        &storage_flags,
                    )),
                    contract: &contract,
                    document_type,
                    owner_id: None,
                },
                false,
                0f64,
                true,
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

        let (results_no_transaction, _, _) = query
            .execute_no_proof(&drive, None)
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
                true,
                Some(&db_transaction),
            )
            .expect("expected to be able to delete the document");

        drive
            .grove
            .commit_transaction(db_transaction)
            .expect("unable to commit transaction");

        // Let's try adding the document back after it was deleted

        let db_transaction = drive.grove.start_transaction();

        let document =
            Document::from_cbor(&person_serialized_document, None, Some(&random_owner_id))
                .expect("expected to deserialize the document");

        let storage_flags = StorageFlags { epoch: 0 };

        drive
            .add_document_for_contract(
                DocumentAndContractInfo {
                    document_info: DocumentAndSerialization((
                        &document,
                        &person_serialized_document,
                        &storage_flags,
                    )),
                    contract: &contract,
                    document_type,
                    owner_id: None,
                },
                false,
                0f64,
                true,
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
                true,
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
                true,
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

        let (results_no_transaction, _, _) = query
            .execute_no_proof(&drive, None)
            .expect("expected to execute query");

        assert_eq!(results_no_transaction.len(), 0);
    }

    #[test]
    fn test_delete_dashpay_documents_no_transaction() {
        let (drive, dashpay_cbor) = setup_dashpay("delete", false);

        let dashpay_profile_serialized_document = json_document_to_cbor(
            "tests/supporting_files/contract/dashpay/profile0.json",
            Some(1),
        );

        let random_owner_id = rand::thread_rng().gen::<[u8; 32]>();
        drive
            .add_serialized_document_for_serialized_contract(
                &dashpay_profile_serialized_document,
                &dashpay_cbor,
                "profile",
                Some(&random_owner_id),
                false,
                0f64,
                true,
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
                true,
                None,
            )
            .expect("expected to be able to delete the document");
    }

    #[test]
    fn test_delete_dashpay_documents() {
        let tmp_dir = TempDir::new().unwrap();
        let drive: Drive = Drive::open(tmp_dir, None).expect("expected to open Drive successfully");

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

        let dashpay_profile_serialized_document = json_document_to_cbor(
            "tests/supporting_files/contract/dashpay/profile0.json",
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
                true,
                Some(&db_transaction),
            )
            .expect("expected to be able to delete the document");
    }

    #[test]
    fn test_deletion_real_data() {
        let tmp_dir = TempDir::new().unwrap();
        let drive: Drive = Drive::open(tmp_dir, None).expect("expected to open Drive successfully");

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

        let storage_flags = StorageFlags { epoch: 0 };

        let documents: Vec<Document> = document_hexes
            .iter()
            .map(|document_hex| {
                let serialized_document = cbor_from_hex(document_hex.to_string());

                let document = Document::from_cbor(&serialized_document, None, None)
                    .expect("expected to deserialize the document");

                let document_type = contract
                    .document_type_for_name("niceDocument")
                    .expect("expected to get a document type");

                drive
                    .add_document_for_contract(
                        DocumentAndContractInfo {
                            document_info: DocumentAndSerialization((
                                &document,
                                &serialized_document,
                                &storage_flags,
                            )),
                            contract: &contract,
                            document_type,
                            owner_id: None,
                        },
                        false,
                        0f64,
                        true,
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

        let (results, _, _) = drive
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
                true,
                Some(&db_transaction),
            )
            .expect("expected to be able to delete the document");

        let query_json = json!({
            "where": [
                ["$id", "==", String::from(document_id)]
            ],
        });

        let query_cbor = value_to_cbor(query_json, None);

        let (results, _, _) = drive
            .query_documents_from_contract(
                &contract,
                contract.document_types.get("niceDocument").unwrap(),
                query_cbor.as_slice(),
                Some(&db_transaction),
            )
            .expect("expected to execute query");

        assert_eq!(results.len(), 0);
    }
}
