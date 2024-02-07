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

//! Delete Documents.
//!
//! This module implements functions in Drive for deleting documents.
//!

// Module: delete_document_for_contract
// This module contains functionality for deleting a document associated with a given contract
mod delete_document_for_contract;
pub use delete_document_for_contract::*;

// Module: delete_document_for_contract_id
// This module contains functionality for deleting a document associated with a given contract id
mod delete_document_for_contract_id;
pub use delete_document_for_contract_id::*;

// Module: delete_document_for_contract_apply_and_add_to_operations
// This module contains functionality to apply a delete operation and add to the operations of a contract
mod delete_document_for_contract_apply_and_add_to_operations;
pub use delete_document_for_contract_apply_and_add_to_operations::*;

// Module: remove_document_from_primary_storage
// This module contains functionality to remove a document from primary storage
mod remove_document_from_primary_storage;
pub use remove_document_from_primary_storage::*;

// Module: remove_reference_for_index_level_for_contract_operations
// This module contains functionality to remove a reference for an index level for contract operations
mod remove_reference_for_index_level_for_contract_operations;
pub use remove_reference_for_index_level_for_contract_operations::*;

// Module: remove_indices_for_index_level_for_contract_operations
// This module contains functionality to remove indices for an index level for contract operations
mod remove_indices_for_index_level_for_contract_operations;
pub use remove_indices_for_index_level_for_contract_operations::*;

// Module: remove_indices_for_top_index_level_for_contract_operations
// This module contains functionality to remove indices for the top index level for contract operations
mod remove_indices_for_top_index_level_for_contract_operations;
pub use remove_indices_for_top_index_level_for_contract_operations::*;

// Module: delete_document_for_contract_id_with_named_type_operations
// This module contains functionality to delete a document for a contract id with named type operations
mod delete_document_for_contract_id_with_named_type_operations;
pub use delete_document_for_contract_id_with_named_type_operations::*;

// Module: delete_document_for_contract_with_named_type_operations
// This module contains functionality to delete a document for a contract with named type operations
mod delete_document_for_contract_with_named_type_operations;
pub use delete_document_for_contract_with_named_type_operations::*;

// Module: delete_document_for_contract_operations
// This module contains functionality to delete a document for contract operations
mod delete_document_for_contract_operations;
pub use delete_document_for_contract_operations::*;

mod internal;

#[cfg(feature = "full")]
#[cfg(test)]
mod tests {
    use dpp::balances::credits::Creditable;
    use dpp::block::block_info::BlockInfo;
    use rand::Rng;

    use std::borrow::Cow;
    use std::option::Option::None;
    use tempfile::TempDir;

    use crate::common::setup_contract;
    use crate::drive::config::DriveConfig;
    use crate::drive::document::tests::setup_dashpay;
    use crate::drive::flags::StorageFlags;
    use crate::drive::object_size_info::DocumentInfo::DocumentRefInfo;
    use crate::drive::object_size_info::{DocumentAndContractInfo, OwnedDocumentInfo};
    use crate::drive::Drive;

    use crate::query::DriveQuery;
    use dpp::block::epoch::Epoch;
    use dpp::data_contract::accessors::v0::DataContractV0Getters;
    use dpp::document::serialization_traits::DocumentPlatformConversionMethodsV0;
    use dpp::document::Document;
    use dpp::fee::default_costs::EpochCosts;
    use dpp::fee::default_costs::KnownCostItem::StorageDiskUsageCreditPerByte;
    use dpp::tests::json_document::{json_document_to_contract, json_document_to_document};

    use crate::tests::helpers::setup::setup_drive_with_initial_state_structure;
    use dpp::version::PlatformVersion;

    #[test]
    fn test_add_and_remove_family_one_document_no_transaction() {
        let tmp_dir = TempDir::new().unwrap();

        let (drive, _) = Drive::open(tmp_dir, None).expect("expected to open Drive successfully");

        let platform_version = PlatformVersion::latest();

        drive
            .create_initial_state_structure(None, platform_version)
            .expect("expected to create root tree successfully");

        let contract = setup_contract(
            &drive,
            "tests/supporting_files/contract/family/family-contract-reduced.json",
            None,
            None,
        );

        let document_type = contract
            .document_type_for_name("person")
            .expect("expected to get document type");

        let random_owner_id0 = rand::thread_rng().gen::<[u8; 32]>();

        let person_document0 = json_document_to_document(
            "tests/supporting_files/contract/family/person0.json",
            Some(random_owner_id0.into()),
            document_type,
            platform_version,
        )
        .expect("expected to get document");

        let storage_flags = Some(Cow::Owned(StorageFlags::SingleEpoch(0)));

        drive
            .add_document_for_contract(
                DocumentAndContractInfo {
                    owned_document_info: OwnedDocumentInfo {
                        document_info: DocumentRefInfo((&person_document0, storage_flags)),
                        owner_id: None,
                    },
                    contract: &contract,
                    document_type,
                },
                false,
                BlockInfo::default(),
                true,
                None,
                platform_version,
            )
            .expect("expected to insert a document successfully");

        let sql_string =
            "select * from person where firstName = 'Samuel' order by firstName asc limit 100";
        let query = DriveQuery::from_sql_expr(sql_string, &contract, Some(&DriveConfig::default()))
            .expect("should build query");

        let (results_no_transaction, _, _) = query
            .execute_raw_results_no_proof(&drive, None, None, platform_version)
            .expect("expected to execute query");

        assert_eq!(results_no_transaction.len(), 1);

        let (results_on_transaction, _, _) = query
            .execute_raw_results_no_proof(&drive, None, None, platform_version)
            .expect("expected to execute query");

        assert_eq!(results_on_transaction.len(), 1);
        let document_id = bs58::decode("AYjYxDqLy2hvGQADqE6FAkBnQEpJSzNd3CRw1tpS6vZ7")
            .into_vec()
            .expect("should decode")
            .as_slice()
            .try_into()
            .expect("this be 32 bytes");

        drive
            .delete_document_for_contract(
                document_id,
                &contract,
                "person",
                BlockInfo::default(),
                true,
                None,
                platform_version,
            )
            .expect("expected to be able to delete the document");

        let (results_on_transaction, _, _) = query
            .execute_raw_results_no_proof(&drive, None, None, platform_version)
            .expect("expected to execute query");

        assert_eq!(results_on_transaction.len(), 0);
    }

    #[test]
    fn test_add_and_remove_family_one_document() {
        let drive = setup_drive_with_initial_state_structure();

        let db_transaction = drive.grove.start_transaction();

        let platform_version = PlatformVersion::latest();

        let contract = setup_contract(
            &drive,
            "tests/supporting_files/contract/family/family-contract-reduced.json",
            None,
            Some(&db_transaction),
        );

        let document_type = contract
            .document_type_for_name("person")
            .expect("expected to get a document type");

        let random_owner_id0 = rand::thread_rng().gen::<[u8; 32]>();

        let person_document0 = json_document_to_document(
            "tests/supporting_files/contract/family/person0.json",
            Some(random_owner_id0.into()),
            document_type,
            platform_version,
        )
        .expect("expected to get document");

        let storage_flags = Some(Cow::Owned(StorageFlags::SingleEpoch(0)));

        drive
            .add_document_for_contract(
                DocumentAndContractInfo {
                    owned_document_info: OwnedDocumentInfo {
                        document_info: DocumentRefInfo((&person_document0, storage_flags)),
                        owner_id: None,
                    },
                    contract: &contract,
                    document_type,
                },
                false,
                BlockInfo::default(),
                true,
                Some(&db_transaction),
                platform_version,
            )
            .expect("expected to insert a document successfully");

        drive
            .grove
            .commit_transaction(db_transaction)
            .unwrap()
            .expect("unable to commit transaction");

        let sql_string =
            "select * from person where firstName = 'Samuel' order by firstName asc limit 100";
        let query = DriveQuery::from_sql_expr(sql_string, &contract, Some(&DriveConfig::default()))
            .expect("should build query");

        let (results_no_transaction, _, _) = query
            .execute_raw_results_no_proof(&drive, None, None, platform_version)
            .expect("expected to execute query");

        assert_eq!(results_no_transaction.len(), 1);

        let db_transaction = drive.grove.start_transaction();

        let (results_on_transaction, _, _) = query
            .execute_raw_results_no_proof(&drive, None, Some(&db_transaction), platform_version)
            .expect("expected to execute query");

        assert_eq!(results_on_transaction.len(), 1);
        let document_id = bs58::decode("AYjYxDqLy2hvGQADqE6FAkBnQEpJSzNd3CRw1tpS6vZ7")
            .into_vec()
            .expect("should decode")
            .as_slice()
            .try_into()
            .expect("this be 32 bytes");

        drive
            .delete_document_for_contract(
                document_id,
                &contract,
                "person",
                BlockInfo::default(),
                true,
                Some(&db_transaction),
                platform_version,
            )
            .expect("expected to be able to delete the document");

        drive
            .grove
            .commit_transaction(db_transaction)
            .unwrap()
            .expect("unable to commit transaction");

        let db_transaction = drive.grove.start_transaction();

        let (results_on_transaction, _, _) = query
            .execute_raw_results_no_proof(&drive, None, Some(&db_transaction), platform_version)
            .expect("expected to execute query");

        assert_eq!(results_on_transaction.len(), 0);
    }

    #[test]
    fn serialize_deserialize_document() {
        let platform_version = PlatformVersion::latest();
        let contract = json_document_to_contract(
            "tests/supporting_files/contract/family/family-contract-reduced.json",
            false,
            platform_version,
        )
        .expect("expected to get cbor contract");

        let document_type = contract
            .document_type_for_name("person")
            .expect("expected to get a document type");

        let random_owner_id0 = rand::thread_rng().gen::<[u8; 32]>();

        let person_document0 = json_document_to_document(
            "tests/supporting_files/contract/family/person0.json",
            Some(random_owner_id0.into()),
            document_type,
            platform_version,
        )
        .expect("expected to get document");

        let serialized = person_document0
            .serialize(document_type, platform_version)
            .expect("expected to serialize");
        let _deserialized = Document::from_bytes(&serialized, document_type, platform_version)
            .expect("expected to deserialize");
    }

    #[test]
    fn test_add_and_remove_family_documents() {
        let drive = setup_drive_with_initial_state_structure();

        let db_transaction = drive.grove.start_transaction();

        let platform_version = PlatformVersion::latest();

        let contract = setup_contract(
            &drive,
            "tests/supporting_files/contract/family/family-contract-reduced.json",
            None,
            Some(&db_transaction),
        );

        let document_type = contract
            .document_type_for_name("person")
            .expect("expected to get a document type");

        let random_owner_id0 = rand::thread_rng().gen::<[u8; 32]>();

        let person_document0 = json_document_to_document(
            "tests/supporting_files/contract/family/person0.json",
            Some(random_owner_id0.into()),
            document_type,
            platform_version,
        )
        .expect("expected to get document");

        let storage_flags = Some(Cow::Owned(StorageFlags::SingleEpoch(0)));

        drive
            .add_document_for_contract(
                DocumentAndContractInfo {
                    owned_document_info: OwnedDocumentInfo {
                        document_info: DocumentRefInfo((&person_document0, storage_flags)),
                        owner_id: None,
                    },
                    contract: &contract,
                    document_type,
                },
                false,
                BlockInfo::default(),
                true,
                Some(&db_transaction),
                platform_version,
            )
            .expect("expected to insert a document successfully");

        let document_type = contract
            .document_type_for_name("person")
            .expect("expected to get a document type");

        let _random_owner_id1 = rand::thread_rng().gen::<[u8; 32]>();

        let person_document1 = json_document_to_document(
            "tests/supporting_files/contract/family/person1.json",
            Some(random_owner_id0.into()),
            document_type,
            platform_version,
        )
        .expect("expected to get document");

        let storage_flags = Some(Cow::Owned(StorageFlags::SingleEpoch(0)));

        drive
            .add_document_for_contract(
                DocumentAndContractInfo {
                    owned_document_info: OwnedDocumentInfo {
                        document_info: DocumentRefInfo((&person_document1, storage_flags)),
                        owner_id: None,
                    },
                    contract: &contract,
                    document_type,
                },
                false,
                BlockInfo::default(),
                true,
                Some(&db_transaction),
                platform_version,
            )
            .expect("expected to insert a document successfully");

        drive
            .grove
            .commit_transaction(db_transaction)
            .unwrap()
            .expect("unable to commit transaction");

        let sql_string =
            "select * from person where firstName > 'A' order by firstName asc limit 5";
        let query = DriveQuery::from_sql_expr(sql_string, &contract, Some(&DriveConfig::default()))
            .expect("should build query");

        let (results_no_transaction, _, _) = query
            .execute_raw_results_no_proof(&drive, None, None, platform_version)
            .expect("expected to execute query");

        assert_eq!(results_no_transaction.len(), 2);

        let document_id = bs58::decode("8wjx2TC1vj2grssQvdwWnksNLwpi4xKraYy1TbProgd4")
            .into_vec()
            .expect("should decode")
            .as_slice()
            .try_into()
            .expect("this be 32 bytes");

        let db_transaction = drive.grove.start_transaction();

        drive
            .delete_document_for_contract(
                document_id,
                &contract,
                "person",
                BlockInfo::default(),
                true,
                Some(&db_transaction),
                platform_version,
            )
            .expect("expected to be able to delete the document");

        drive
            .grove
            .commit_transaction(db_transaction)
            .unwrap()
            .expect("unable to commit transaction");

        let sql_string =
            "select * from person where firstName > 'A' order by firstName asc limit 5";
        let query = DriveQuery::from_sql_expr(sql_string, &contract, Some(&DriveConfig::default()))
            .expect("should build query");

        let (results_no_transaction, _, _) = query
            .execute_raw_results_no_proof(&drive, None, None, platform_version)
            .expect("expected to execute query");

        assert_eq!(results_no_transaction.len(), 1);

        let document_id = bs58::decode("AYjYxDqLy2hvGQADqE6FAkBnQEpJSzNd3CRw1tpS6vZ7")
            .into_vec()
            .expect("should decode")
            .as_slice()
            .try_into()
            .expect("this be 32 bytes");

        let db_transaction = drive.grove.start_transaction();

        drive
            .delete_document_for_contract(
                document_id,
                &contract,
                "person",
                BlockInfo::default(),
                true,
                Some(&db_transaction),
                platform_version,
            )
            .expect("expected to be able to delete the document");

        drive
            .grove
            .commit_transaction(db_transaction)
            .unwrap()
            .expect("unable to commit transaction");

        let sql_string =
            "select * from person where firstName > 'A' order by firstName asc limit 5";
        let query = DriveQuery::from_sql_expr(sql_string, &contract, Some(&DriveConfig::default()))
            .expect("should build query");

        let (results_no_transaction, _, _) = query
            .execute_raw_results_no_proof(&drive, None, None, platform_version)
            .expect("expected to execute query");

        assert_eq!(results_no_transaction.len(), 0);
    }

    #[test]
    fn test_add_and_remove_family_documents_with_empty_fields() {
        let drive = setup_drive_with_initial_state_structure();

        let db_transaction = drive.grove.start_transaction();

        let platform_version = PlatformVersion::latest();

        let contract = setup_contract(
            &drive,
            "tests/supporting_files/contract/family/family-contract-reduced.json",
            None,
            Some(&db_transaction),
        );

        let document_type = contract
            .document_type_for_name("person")
            .expect("expected to get document type");

        let random_owner_id0 = rand::thread_rng().gen::<[u8; 32]>();

        let person_document0 = json_document_to_document(
            "tests/supporting_files/contract/family/person0.json",
            Some(random_owner_id0.into()),
            document_type,
            platform_version,
        )
        .expect("expected to get document");

        let storage_flags = Some(Cow::Owned(StorageFlags::SingleEpoch(0)));

        drive
            .add_document_for_contract(
                DocumentAndContractInfo {
                    owned_document_info: OwnedDocumentInfo {
                        document_info: DocumentRefInfo((&person_document0, storage_flags)),
                        owner_id: None,
                    },
                    contract: &contract,
                    document_type,
                },
                false,
                BlockInfo::default(),
                true,
                Some(&db_transaction),
                platform_version,
            )
            .expect("expected to insert a document successfully");

        let random_owner_id0 = rand::thread_rng().gen::<[u8; 32]>();

        let person_document1 = json_document_to_document(
            "tests/supporting_files/contract/family/person2-no-middle-name.json",
            Some(random_owner_id0.into()),
            document_type,
            platform_version,
        )
        .expect("expected to get document");

        let storage_flags = Some(Cow::Owned(StorageFlags::SingleEpoch(0)));

        drive
            .add_document_for_contract(
                DocumentAndContractInfo {
                    owned_document_info: OwnedDocumentInfo {
                        document_info: DocumentRefInfo((&person_document1, storage_flags)),
                        owner_id: None,
                    },
                    contract: &contract,
                    document_type,
                },
                false,
                BlockInfo::default(),
                true,
                Some(&db_transaction),
                platform_version,
            )
            .expect("expected to insert a document successfully");

        drive
            .grove
            .commit_transaction(db_transaction)
            .unwrap()
            .expect("unable to commit transaction");

        let sql_string =
            "select * from person where firstName > 'A' order by firstName asc limit 5";
        let query = DriveQuery::from_sql_expr(sql_string, &contract, Some(&DriveConfig::default()))
            .expect("should build query");

        let (results_no_transaction, _, _) = query
            .execute_raw_results_no_proof(&drive, None, None, platform_version)
            .expect("expected to execute query");

        assert_eq!(results_no_transaction.len(), 2);

        let document_id = bs58::decode("BZjYxDqLy2hvGQADqE6FAkBnQEpJSzNd3CRw1tpS6vZ7")
            .into_vec()
            .expect("should decode")
            .as_slice()
            .try_into()
            .expect("this be 32 bytes");

        let db_transaction = drive.grove.start_transaction();

        drive
            .delete_document_for_contract(
                document_id,
                &contract,
                "person",
                BlockInfo::default(),
                true,
                Some(&db_transaction),
                platform_version,
            )
            .expect("expected to be able to delete the document");

        drive
            .grove
            .commit_transaction(db_transaction)
            .unwrap()
            .expect("unable to commit transaction");

        // Let's try adding the document back after it was deleted

        let db_transaction = drive.grove.start_transaction();

        let storage_flags = Some(Cow::Owned(StorageFlags::SingleEpoch(0)));

        drive
            .add_document_for_contract(
                DocumentAndContractInfo {
                    owned_document_info: OwnedDocumentInfo {
                        document_info: DocumentRefInfo((&person_document1, storage_flags)),
                        owner_id: None,
                    },
                    contract: &contract,
                    document_type,
                },
                false,
                BlockInfo::default(),
                true,
                Some(&db_transaction),
                platform_version,
            )
            .expect("expected to insert a document successfully");

        drive
            .grove
            .commit_transaction(db_transaction)
            .unwrap()
            .expect("unable to commit transaction");

        // Let's try removing all documents now

        let db_transaction = drive.grove.start_transaction();

        drive
            .delete_document_for_contract(
                document_id,
                &contract,
                "person",
                BlockInfo::default(),
                true,
                Some(&db_transaction),
                platform_version,
            )
            .expect("expected to be able to delete the document");

        let document_id = bs58::decode("AYjYxDqLy2hvGQADqE6FAkBnQEpJSzNd3CRw1tpS6vZ7")
            .into_vec()
            .expect("should decode")
            .as_slice()
            .try_into()
            .expect("this be 32 bytes");

        drive
            .delete_document_for_contract(
                document_id,
                &contract,
                "person",
                BlockInfo::default(),
                true,
                Some(&db_transaction),
                platform_version,
            )
            .expect("expected to be able to delete the document");

        drive
            .grove
            .commit_transaction(db_transaction)
            .unwrap()
            .expect("unable to commit transaction");

        let sql_string =
            "select * from person where firstName > 'A' order by firstName asc limit 5";
        let query = DriveQuery::from_sql_expr(sql_string, &contract, Some(&DriveConfig::default()))
            .expect("should build query");

        let (results_no_transaction, _, _) = query
            .execute_raw_results_no_proof(&drive, None, None, platform_version)
            .expect("expected to execute query");

        assert_eq!(results_no_transaction.len(), 0);
    }

    #[test]
    fn test_delete_dashpay_documents_no_transaction() {
        let (drive, dashpay) = setup_dashpay("delete", false);

        let document_type = dashpay
            .document_type_for_name("profile")
            .expect("expected to get profile document type");
        let random_owner_id = rand::thread_rng().gen::<[u8; 32]>();

        let platform_version = PlatformVersion::first();
        let dashpay_profile_document = json_document_to_document(
            "tests/supporting_files/contract/dashpay/profile0.json",
            Some(random_owner_id.into()),
            document_type,
            platform_version,
        )
        .expect("expected to get cbor document");

        drive
            .add_document_for_contract(
                DocumentAndContractInfo {
                    owned_document_info: OwnedDocumentInfo {
                        document_info: DocumentRefInfo((
                            &dashpay_profile_document,
                            StorageFlags::optional_default_as_cow(),
                        )),
                        owner_id: Some(random_owner_id),
                    },
                    contract: &dashpay,
                    document_type,
                },
                false,
                BlockInfo::default(),
                true,
                None,
                platform_version,
            )
            .expect("expected to insert a document successfully");

        let document_id = bs58::decode("AM47xnyLfTAC9f61ZQPGfMK5Datk2FeYZwgYvcAnzqFY")
            .into_vec()
            .expect("should decode")
            .as_slice()
            .try_into()
            .expect("this be 32 bytes");

        drive
            .delete_document_for_contract(
                document_id,
                &dashpay,
                "profile",
                BlockInfo::default(),
                true,
                None,
                platform_version,
            )
            .expect("expected to be able to delete the document");
    }

    #[test]
    fn test_delete_dashpay_documents() {
        let drive = setup_drive_with_initial_state_structure();

        let db_transaction = drive.grove.start_transaction();

        let platform_version = PlatformVersion::latest();

        let contract = setup_contract(
            &drive,
            "tests/supporting_files/contract/dashpay/dashpay-contract.json",
            None,
            Some(&db_transaction),
        );

        let random_owner_id = rand::thread_rng().gen::<[u8; 32]>();

        let document_type = contract
            .document_type_for_name("profile")
            .expect("expected to get profile document type");

        let dashpay_profile_document = json_document_to_document(
            "tests/supporting_files/contract/dashpay/profile0.json",
            Some(random_owner_id.into()),
            document_type,
            platform_version,
        )
        .expect("expected to get cbor document");

        let random_owner_id = rand::thread_rng().gen::<[u8; 32]>();
        let storage_flags = Some(Cow::Owned(StorageFlags::SingleEpochOwned(
            0,
            random_owner_id,
        )));
        let fee_result = drive
            .add_document_for_contract(
                DocumentAndContractInfo {
                    owned_document_info: OwnedDocumentInfo {
                        document_info: DocumentRefInfo((&dashpay_profile_document, storage_flags)),
                        owner_id: None,
                    },
                    contract: &contract,
                    document_type,
                },
                false,
                BlockInfo::default(),
                true,
                Some(&db_transaction),
                platform_version,
            )
            .expect("expected to insert a document successfully");

        let added_bytes = fee_result.storage_fee
            / Epoch::new(0)
                .unwrap()
                .cost_for_known_cost_item(StorageDiskUsageCreditPerByte);
        // We added 1557 bytes
        assert_eq!(added_bytes, 1557);

        let document_id = bs58::decode("AM47xnyLfTAC9f61ZQPGfMK5Datk2FeYZwgYvcAnzqFY")
            .into_vec()
            .expect("should decode")
            .as_slice()
            .try_into()
            .expect("this be 32 bytes");

        // Let's delete the document at the third epoch
        let fee_result = drive
            .delete_document_for_contract(
                document_id,
                &contract,
                "profile",
                BlockInfo::default_with_epoch(Epoch::new(3).unwrap()),
                true,
                Some(&db_transaction),
                platform_version,
            )
            .expect("expected to be able to delete the document");

        let removed_credits = fee_result
            .fee_refunds
            .get(&random_owner_id)
            .unwrap()
            .get(&0)
            .unwrap();

        assert_eq!(*removed_credits, 41827688);
        let refund_equivalent_bytes = removed_credits.to_unsigned()
            / Epoch::new(0)
                .unwrap()
                .cost_for_known_cost_item(StorageDiskUsageCreditPerByte);

        assert!(added_bytes > refund_equivalent_bytes);
        assert_eq!(refund_equivalent_bytes, 1549); // we refunded 1549 instead of 1556
    }

    #[test]
    fn test_delete_dashpay_documents_without_apply() {
        let drive = setup_drive_with_initial_state_structure();

        let db_transaction = drive.grove.start_transaction();

        let platform_version = PlatformVersion::latest();

        let contract = setup_contract(
            &drive,
            "tests/supporting_files/contract/dashpay/dashpay-contract.json",
            None,
            Some(&db_transaction),
        );

        let document_type = contract
            .document_type_for_name("profile")
            .expect("expected to get profile document type");

        let random_owner_id = rand::thread_rng().gen::<[u8; 32]>();

        let dashpay_profile_document = json_document_to_document(
            "tests/supporting_files/contract/dashpay/profile0.json",
            Some(random_owner_id.into()),
            document_type,
            platform_version,
        )
        .expect("expected to get cbor document");

        let random_owner_id = rand::thread_rng().gen::<[u8; 32]>();
        let storage_flags = Some(Cow::Owned(StorageFlags::SingleEpochOwned(
            0,
            random_owner_id,
        )));
        let fee_result = drive
            .add_document_for_contract(
                DocumentAndContractInfo {
                    owned_document_info: OwnedDocumentInfo {
                        document_info: DocumentRefInfo((&dashpay_profile_document, storage_flags)),
                        owner_id: None,
                    },
                    contract: &contract,
                    document_type,
                },
                false,
                BlockInfo::default(),
                true,
                Some(&db_transaction),
                platform_version,
            )
            .expect("expected to insert a document successfully");

        let added_bytes = fee_result.storage_fee
            / Epoch::new(0)
                .unwrap()
                .cost_for_known_cost_item(StorageDiskUsageCreditPerByte);
        // We added 1553 bytes
        assert_eq!(added_bytes, 1557);

        let document_id = bs58::decode("AM47xnyLfTAC9f61ZQPGfMK5Datk2FeYZwgYvcAnzqFY")
            .into_vec()
            .expect("should decode")
            .as_slice()
            .try_into()
            .expect("this be 32 bytes");

        // Let's delete the document at the third epoch
        let fee_result = drive
            .delete_document_for_contract(
                document_id,
                &contract,
                "profile",
                BlockInfo::default_with_epoch(Epoch::new(3).unwrap()),
                false,
                Some(&db_transaction),
                platform_version,
            )
            .expect("expected to be able to delete the document");

        assert!(fee_result.fee_refunds.0.is_empty());
        assert_eq!(fee_result.storage_fee, 0);
        assert_eq!(fee_result.processing_fee, 145470580);
    }
}
