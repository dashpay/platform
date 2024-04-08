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

// Module: add_update_multiple_documents_operations
// This module contains functionality for adding operations to update multiple documents
#[cfg(feature = "full")]
mod add_update_multiple_documents_operations;

// Module: update_document_for_contract
// This module contains functionality for updating a document for a given contract
#[cfg(feature = "full")]
mod update_document_for_contract;

// Module: update_document_for_contract_id
// This module contains functionality for updating a document associated with a given contract id
#[cfg(feature = "full")]
mod update_document_for_contract_id;

// Module: update_document_with_serialization_for_contract
// This module contains functionality for updating a document (with serialization) for a contract
mod internal;
mod update_document_with_serialization_for_contract;

#[cfg(test)]
mod tests {
    use grovedb::TransactionArg;
    use std::borrow::Cow;
    use std::default::Default;
    use std::option::Option::None;

    use dpp::data_contract::{DataContract, DataContractFactory};

    use dpp::platform_value::{platform_value, Identifier, Value};

    use dpp::block::block_info::BlockInfo;

    use dpp::balances::credits::Creditable;
    use rand::{random, Rng};
    use serde::{Deserialize, Serialize};
    use serde_json::json;

    use crate::drive::config::DriveConfig;
    use crate::drive::flags::StorageFlags;
    use crate::drive::object_size_info::DocumentInfo::{DocumentOwnedInfo, DocumentRefInfo};
    use crate::drive::object_size_info::{DocumentAndContractInfo, OwnedDocumentInfo};
    use crate::drive::Drive;

    use crate::common::setup_contract;
    use crate::drive::document::tests::setup_dashpay;
    use crate::query::DriveQuery;
    use crate::tests::helpers::setup::{setup_drive, setup_drive_with_initial_state_structure};
    use dpp::block::epoch::Epoch;
    use dpp::data_contract::accessors::v0::DataContractV0Getters;
    use dpp::data_contract::conversion::value::v0::DataContractValueConversionMethodsV0;
    use dpp::data_contract::document_type::methods::DocumentTypeV0Methods;
    use dpp::document::serialization_traits::{
        DocumentPlatformConversionMethodsV0, DocumentPlatformValueMethodsV0,
    };
    use dpp::document::specialized_document_factory::SpecializedDocumentFactory;
    use dpp::document::{Document, DocumentV0Getters, DocumentV0Setters};
    use dpp::fee::default_costs::EpochCosts;
    use dpp::fee::default_costs::KnownCostItem::StorageDiskUsageCreditPerByte;
    use dpp::fee::fee_result::FeeResult;
    use dpp::platform_value;
    use dpp::tests::json_document::json_document_to_document;
    use platform_version::version::PlatformVersion;

    #[test]
    fn test_create_and_update_document_same_transaction() {
        let (drive, contract) = setup_dashpay("", true);

        let platform_version = PlatformVersion::latest();

        let db_transaction = drive.grove.start_transaction();

        let mut document = contract
            .document_type_for_name("profile")
            .expect("profile document exists")
            .create_document_from_data(
                platform_value!({"displayName": "Alice"}),
                Identifier::random(),
                random(),
                random(),
                random(),
                platform_version,
            )
            .expect("should create document");

        // Create Alice profile

        let info = DocumentAndContractInfo {
            owned_document_info: OwnedDocumentInfo {
                document_info: DocumentRefInfo((
                    &document,
                    StorageFlags::optional_default_as_cow(),
                )),
                owner_id: None,
            },
            contract: &contract,
            document_type: contract
                .document_type_for_name("profile")
                .expect("profile document exists"),
        };

        drive
            .add_document_for_contract(
                info,
                true,
                BlockInfo::default(),
                true,
                Some(&db_transaction),
                platform_version,
            )
            .expect("should create alice profile");

        // Update Alice profile

        document.set("displayName", "alice2".into());

        let info = DocumentAndContractInfo {
            owned_document_info: OwnedDocumentInfo {
                document_info: DocumentRefInfo((
                    &document,
                    StorageFlags::optional_default_as_cow(),
                )),
                owner_id: None,
            },
            contract: &contract,
            document_type: contract
                .document_type_for_name("profile")
                .expect("profile document exists"),
        };

        drive
            .add_document_for_contract(
                info,
                true,
                BlockInfo::default(),
                true,
                Some(&db_transaction),
                platform_version,
            )
            .expect("should update alice profile");
    }

    #[test]
    fn test_create_and_update_document_no_transactions() {
        let (drive, contract) = setup_dashpay("", true);

        let platform_version = PlatformVersion::latest();

        let mut document = contract
            .document_type_for_name("profile")
            .expect("profile document exists")
            .create_document_from_data(
                platform_value!({"displayName": "Alice"}),
                Identifier::random(),
                random(),
                random(),
                random(),
                platform_version,
            )
            .expect("should create document");

        // Create Alice profile

        let info = DocumentAndContractInfo {
            owned_document_info: OwnedDocumentInfo {
                document_info: DocumentRefInfo((
                    &document,
                    StorageFlags::optional_default_as_cow(),
                )),
                owner_id: None,
            },
            contract: &contract,
            document_type: contract
                .document_type_for_name("profile")
                .expect("profile document exists"),
        };

        drive
            .add_document_for_contract(
                info,
                true,
                BlockInfo::default(),
                true,
                None,
                platform_version,
            )
            .expect("should create alice profile");

        // Check Alice profile

        let sql_string = "select * from profile";
        let query = DriveQuery::from_sql_expr(sql_string, &contract, Some(&DriveConfig::default()))
            .expect("should build query");

        let (results_no_transaction, _, _) = query
            .execute_raw_results_no_proof(&drive, None, None, platform_version)
            .expect("expected to execute query");

        assert_eq!(results_no_transaction.len(), 1);

        // Update Alice profile

        document.set("displayName", "alice2".into());

        let info = DocumentAndContractInfo {
            owned_document_info: OwnedDocumentInfo {
                document_info: DocumentRefInfo((
                    &document,
                    StorageFlags::optional_default_as_cow(),
                )),
                owner_id: None,
            },
            contract: &contract,
            document_type: contract
                .document_type_for_name("profile")
                .expect("profile document exists"),
        };

        drive
            .add_document_for_contract(
                info,
                true,
                BlockInfo::default(),
                true,
                None,
                platform_version,
            )
            .expect("should update alice profile");

        let (results_no_transaction, _, _) = query
            .execute_raw_results_no_proof(&drive, None, None, platform_version)
            .expect("expected to execute query");

        assert_eq!(results_no_transaction.len(), 1);
    }

    #[test]
    fn test_create_and_update_document_in_different_transactions() {
        let (drive, contract) = setup_dashpay("", true);

        let platform_version = PlatformVersion::latest();

        let db_transaction = drive.grove.start_transaction();

        let mut document = contract
            .document_type_for_name("profile")
            .expect("profile document exists")
            .create_document_from_data(
                platform_value!({"displayName": "Alice"}),
                Identifier::random(),
                random(),
                random(),
                random(),
                platform_version,
            )
            .expect("should create document");

        // Create Alice profile

        let info = DocumentAndContractInfo {
            owned_document_info: OwnedDocumentInfo {
                document_info: DocumentRefInfo((
                    &document,
                    StorageFlags::optional_default_as_cow(),
                )),
                owner_id: None,
            },
            contract: &contract,
            document_type: contract
                .document_type_for_name("profile")
                .expect("profile document exists"),
        };

        drive
            .add_document_for_contract(
                info,
                true,
                BlockInfo::default(),
                true,
                Some(&db_transaction),
                platform_version,
            )
            .expect("should create alice profile");

        drive
            .grove
            .commit_transaction(db_transaction)
            .unwrap()
            .expect("should commit transaction");

        // Check Alice profile

        let sql_string = "select * from profile";
        let query = DriveQuery::from_sql_expr(sql_string, &contract, Some(&DriveConfig::default()))
            .expect("should build query");

        let (results_no_transaction, _, _) = query
            .execute_raw_results_no_proof(&drive, None, None, platform_version)
            .expect("expected to execute query");

        assert_eq!(results_no_transaction.len(), 1);

        // Update Alice profile

        let db_transaction = drive.grove.start_transaction();

        document.set("displayName", "alice2".into());

        let info = DocumentAndContractInfo {
            owned_document_info: OwnedDocumentInfo {
                document_info: DocumentRefInfo((
                    &document,
                    StorageFlags::optional_default_as_cow(),
                )),
                owner_id: None,
            },
            contract: &contract,
            document_type: contract
                .document_type_for_name("profile")
                .expect("profile document exists"),
        };

        drive
            .add_document_for_contract(
                info,
                true,
                BlockInfo::default(),
                true,
                Some(&db_transaction),
                platform_version,
            )
            .expect("should update alice profile");

        drive
            .grove
            .commit_transaction(db_transaction)
            .unwrap()
            .expect("should commit transaction");

        let (results_no_transaction, _, _) = query
            .execute_raw_results_no_proof(&drive, None, None, platform_version)
            .expect("expected to execute query");

        assert_eq!(results_no_transaction.len(), 1);
    }

    #[test]
    fn test_create_and_update_document_in_different_transactions_with_delete_rollback() {
        let (drive, contract) = setup_dashpay("", true);

        let platform_version = PlatformVersion::latest();

        let db_transaction = drive.grove.start_transaction();

        let mut document = contract
            .document_type_for_name("profile")
            .expect("profile document exists")
            .create_document_from_data(
                platform_value!({"displayName": "Alice"}),
                Identifier::random(),
                random(),
                random(),
                random(),
                platform_version,
            )
            .expect("should create document");

        // Create Alice profile

        let info = DocumentAndContractInfo {
            owned_document_info: OwnedDocumentInfo {
                document_info: DocumentRefInfo((
                    &document,
                    StorageFlags::optional_default_as_cow(),
                )),
                owner_id: None,
            },
            contract: &contract,
            document_type: contract
                .document_type_for_name("profile")
                .expect("profile document exists"),
        };

        drive
            .add_document_for_contract(
                info,
                true,
                BlockInfo::default(),
                true,
                Some(&db_transaction),
                platform_version,
            )
            .expect("should create alice profile");

        drive
            .grove
            .commit_transaction(db_transaction)
            .unwrap()
            .expect("should commit transaction");

        // Check Alice profile

        let sql_string = "select * from profile";
        let query = DriveQuery::from_sql_expr(sql_string, &contract, Some(&DriveConfig::default()))
            .expect("should build query");

        let (results_no_transaction, _, _) = query
            .execute_raw_results_no_proof(&drive, None, None, platform_version)
            .expect("expected to execute query");

        assert_eq!(results_no_transaction.len(), 1);

        // Delete and then rollback the deletion of Alice profile

        let db_transaction = drive.grove.start_transaction();

        let (results_on_transaction, _, _) = query
            .execute_raw_results_no_proof(&drive, None, Some(&db_transaction), platform_version)
            .expect("expected to execute query");

        assert_eq!(results_on_transaction.len(), 1);

        drive
            .delete_document_for_contract(
                document.id().to_buffer(),
                &contract,
                "profile",
                BlockInfo::default(),
                true,
                Some(&db_transaction),
                platform_version,
            )
            .expect("expected to delete document");

        let (results_on_transaction, _, _) = query
            .execute_raw_results_no_proof(&drive, None, Some(&db_transaction), platform_version)
            .expect("expected to execute query");

        assert_eq!(results_on_transaction.len(), 0);

        drive
            .grove
            .rollback_transaction(&db_transaction)
            .expect("expected to rollback transaction");

        let (results_on_transaction, _, _) = query
            .execute_raw_results_no_proof(&drive, None, Some(&db_transaction), platform_version)
            .expect("expected to execute query");

        assert_eq!(results_on_transaction.len(), 1);

        // Update Alice profile

        document.set("displayName", "alice2".into());

        let info = DocumentAndContractInfo {
            owned_document_info: OwnedDocumentInfo {
                document_info: DocumentRefInfo((
                    &document,
                    StorageFlags::optional_default_as_cow(),
                )),
                owner_id: None,
            },
            contract: &contract,
            document_type: contract
                .document_type_for_name("profile")
                .expect("profile document exists"),
        };

        drive
            .add_document_for_contract(
                info,
                true,
                BlockInfo::default(),
                true,
                Some(&db_transaction),
                platform_version,
            )
            .expect("should update alice profile");

        drive
            .grove
            .commit_transaction(db_transaction)
            .unwrap()
            .expect("should commit transaction");

        let (results_no_transaction, _, _) = query
            .execute_raw_results_no_proof(&drive, None, None, platform_version)
            .expect("expected to execute query");

        assert_eq!(results_no_transaction.len(), 1);
    }

    #[test]
    fn test_create_update_and_delete_document() {
        let drive = setup_drive_with_initial_state_structure();

        let platform_version = PlatformVersion::latest();

        let contract = platform_value!({
            "$format_version": "0",
            "id": "BZUodcFoFL6KvnonehrnMVggTvCe8W5MiRnZuqLb6M54",
            "schema": "https://schema.dash.org/dpp-0-4-0/meta/data-contract",
            "version": 1,
            "ownerId": "GZVdTnLFAN2yE9rLeCHBDBCr7YQgmXJuoExkY347j7Z5",
            "documentSchemas": {
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
                            "position": 0,
                        },
                        "lastName": {
                            "type": "string",
                            "maxLength": 63,
                            "position": 1,
                        }
                    },
                    "required": ["firstName", "$createdAt", "$updatedAt", "lastName"],
                    "additionalProperties": false,
                },
            },
        });

        // first we need to deserialize the contract
        let contract = DataContract::from_value(contract, false, platform_version)
            .expect("expected data contract");

        drive
            .apply_contract(
                &contract,
                BlockInfo::default(),
                true,
                StorageFlags::optional_default_as_cow(),
                None,
                platform_version,
            )
            .expect("should create a contract");

        // Create document

        let document_values = platform_value!({
           "$id": Identifier::new(bs58::decode("DLRWw2eRbLAW5zDU2c7wwsSFQypTSZPhFYzpY48tnaXN").into_vec()
                        .unwrap().try_into().unwrap()),
           "$type": "indexedDocument",
           "$dataContractId": Identifier::new(bs58::decode("BZUodcFoFL6KvnonehrnMVggTvCe8W5MiRnZuqLb6M54").into_vec()
                        .unwrap().try_into().unwrap()),
           "$ownerId": Identifier::new(bs58::decode("GZVdTnLFAN2yE9rLeCHBDBCr7YQgmXJuoExkY347j7Z5").into_vec()
                        .unwrap().try_into().unwrap()),
           "$revision": 1,
           "firstName": "myName",
           "lastName": "lastName",
           "$createdAt": 1647535750329_u64,
           "$updatedAt": 1647535750329_u64,
        });

        let document = Document::from_platform_value(document_values, platform_version)
            .expect("expected to make document");

        let document_type = contract
            .document_type_for_name("indexedDocument")
            .expect("expected to get a document type");
        drive
            .add_document_for_contract(
                DocumentAndContractInfo {
                    owned_document_info: OwnedDocumentInfo {
                        document_info: DocumentOwnedInfo((
                            document,
                            StorageFlags::optional_default_as_cow(),
                        )),
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
            .expect("should add document");

        // Update document

        let document_values = platform_value!({
           "$id": Identifier::new(bs58::decode("DLRWw2eRbLAW5zDU2c7wwsSFQypTSZPhFYzpY48tnaXN").into_vec()
                        .unwrap().try_into().unwrap()),
           "$type": "indexedDocument",
           "$dataContractId": Identifier::new(bs58::decode("BZUodcFoFL6KvnonehrnMVggTvCe8W5MiRnZuqLb6M54").into_vec()
                        .unwrap().try_into().unwrap()),
           "$ownerId": Identifier::new(bs58::decode("GZVdTnLFAN2yE9rLeCHBDBCr7YQgmXJuoExkY347j7Z5").into_vec()
                        .unwrap().try_into().unwrap()),
           "$revision": 2,
           "firstName": "updatedName",
           "lastName": "lastName",
           "$createdAt":1647535750329_u64,
           "$updatedAt":1647535754556_u64,
        });

        let document = Document::from_platform_value(document_values, platform_version)
            .expect("expected to make document");

        drive
            .update_document_for_contract(
                &document,
                &contract,
                document_type,
                None,
                BlockInfo::default(),
                true,
                StorageFlags::optional_default_as_cow(),
                None,
                platform_version,
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
            .delete_document_for_contract(
                document_id,
                &contract,
                "indexedDocument",
                BlockInfo::default(),
                true,
                None,
                platform_version,
            )
            .expect("should delete document");
    }

    #[test]
    fn test_modify_dashpay_contact_request() {
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
            .document_type_for_name("contactRequest")
            .expect("expected to get document type");

        let random_owner_id = rand::thread_rng().gen::<[u8; 32]>();

        let dashpay_cr_document = json_document_to_document(
            "tests/supporting_files/contract/dashpay/contact-request0.json",
            Some(random_owner_id.into()),
            document_type,
            platform_version,
        )
        .expect("expected to get document");

        drive
            .add_document_for_contract(
                DocumentAndContractInfo {
                    owned_document_info: OwnedDocumentInfo {
                        document_info: DocumentRefInfo((
                            &dashpay_cr_document,
                            StorageFlags::optional_default_as_cow(),
                        )),
                        owner_id: Some(random_owner_id),
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
            .update_document_for_contract(
                &dashpay_cr_document,
                &contract,
                document_type,
                Some(random_owner_id),
                BlockInfo::default(),
                true,
                StorageFlags::optional_default_as_cow(),
                Some(&db_transaction),
                platform_version,
            )
            .expect_err("expected not to be able to update a non mutable document");

        drive
            .add_document_for_contract(
                DocumentAndContractInfo {
                    owned_document_info: OwnedDocumentInfo {
                        document_info: DocumentRefInfo((
                            &dashpay_cr_document,
                            StorageFlags::optional_default_as_cow(),
                        )),
                        owner_id: Some(random_owner_id),
                    },
                    contract: &contract,
                    document_type,
                },
                true,
                BlockInfo::default(),
                true,
                Some(&db_transaction),
                platform_version,
            )
            .expect_err("expected not to be able to override a non mutable document");
    }

    #[test]
    fn test_update_dashpay_profile_with_history() {
        let drive = setup_drive_with_initial_state_structure();

        let db_transaction = drive.grove.start_transaction();

        let platform_version = PlatformVersion::latest();

        let contract = setup_contract(
            &drive,
            "tests/supporting_files/contract/dashpay/dashpay-contract-with-profile-history.json",
            None,
            Some(&db_transaction),
        );

        let document_type = contract
            .document_type_for_name("profile")
            .expect("expected to get document type");

        let random_owner_id = rand::thread_rng().gen::<[u8; 32]>();

        let dashpay_profile_document = json_document_to_document(
            "tests/supporting_files/contract/dashpay/profile0.json",
            Some(random_owner_id.into()),
            document_type,
            platform_version,
        )
        .expect("expected to get cbor document");

        let dashpay_profile_updated_public_message_document = json_document_to_document(
            "tests/supporting_files/contract/dashpay/profile0-updated-public-message.json",
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
            .update_document_for_contract(
                &dashpay_profile_updated_public_message_document,
                &contract,
                document_type,
                Some(random_owner_id),
                BlockInfo::default(),
                true,
                StorageFlags::optional_default_as_cow(),
                Some(&db_transaction),
                platform_version,
            )
            .expect("expected to update a document with history successfully");
    }

    fn test_fees_for_update_document(using_history: bool, using_transaction: bool) {
        let config = DriveConfig {
            batching_consistency_verification: true,
            has_raw_enabled: true,
            default_genesis_time: Some(0),
            ..Default::default()
        };

        let platform_version = PlatformVersion::latest();

        let drive: Drive = setup_drive(Some(config));

        let transaction = if using_transaction {
            Some(drive.grove.start_transaction())
        } else {
            None
        };

        drive
            .create_initial_state_structure(transaction.as_ref(), platform_version)
            .expect("expected to create root tree successfully");

        let path = if using_history {
            "tests/supporting_files/contract/family/family-contract-with-history-only-message-index.json"
        } else {
            "tests/supporting_files/contract/family/family-contract-only-message-index.json"
        };

        // setup code
        let contract = setup_contract(&drive, path, None, transaction.as_ref());

        let id = Identifier::from([1u8; 32]);
        let owner_id = Identifier::from([2u8; 32]);
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

        let document_type = contract
            .document_type_for_name("person")
            .expect("expected to get document type");

        let value = platform_value::to_value(&person_0_original).expect("person into value");

        let document =
            Document::from_platform_value(value, platform_version).expect("value to document");

        let document_serialized = document
            .serialize_consume(document_type, platform_version)
            .expect("expected to serialize document");

        assert_eq!(document_serialized.len(), 119);
        let original_fees = apply_person(
            &drive,
            &contract,
            BlockInfo::default(),
            &person_0_original,
            true,
            transaction.as_ref(),
            platform_version,
        );
        let original_bytes = original_fees.storage_fee
            / Epoch::new(0)
                .unwrap()
                .cost_for_known_cost_item(StorageDiskUsageCreditPerByte);
        let expected_added_bytes = if using_history {
            //Explanation for 1237

            //todo
            1237
        } else {
            //Explanation for 959

            // Document Storage

            //// Item
            // = 356 Bytes

            // Explanation for 354 storage_written_bytes

            // Key -> 65 bytes
            // 32 bytes for the key prefix
            // 32 bytes for the unique id
            // 1 byte for key_size (required space for 64)

            // Value -> 223
            //   1 for the flag option with flags
            //   1 for the flags size
            //   35 for flags 32 + 1 + 2
            //   1 for the enum type
            //   1 for item
            //   117 for item serialized bytes (verified above)
            //   1 for Basic Merk
            // 32 for node hash
            // 32 for value hash
            // 2 byte for the value_size (required space for above 128)

            // Parent Hook -> 68
            // Key Bytes 32
            // Hash Size 32
            // Key Length 1
            // Child Heights 2
            // Basic Merk 1

            // Total 65 + 224 + 68 = 357

            //// Tree 1 / <PersonDataContract> / 1 / person / message
            // Key: My apples are safe
            // = 179 Bytes

            // Explanation for 179 storage_written_bytes

            // Key -> 51 bytes
            // 32 bytes for the key prefix
            // 18 bytes for the key "My apples are safe" 18 characters
            // 1 byte for key_size (required space for 50)

            // Value -> 74
            //   1 for the flag option with flags
            //   1 for the flags size
            //   35 for flags
            //   1 for the enum type
            //   1 for empty tree value
            //   1 for Basic Merk
            // 32 for node hash
            // 0 for value hash
            // 2 byte for the value_size (required space for 73 + up to 256 for child key)

            // Parent Hook -> 54
            // Key Bytes 18
            // Hash Size 32
            // Key Length 1
            // Child Heights 2
            // Basic merk 1

            // Total 51 + 74 + 54 = 179

            //// Tree 1 / <PersonDataContract> / 1 / person / message / My apples are safe
            // Key: 0
            // = 145 Bytes

            // Explanation for 145 storage_written_bytes

            // Key -> 34 bytes
            // 32 bytes for the key prefix
            // 1 bytes for the key "My apples are safe" 18 characters
            // 1 byte for key_size (required space for 33)

            // Value -> 74
            //   1 for the flag option with flags
            //   1 for the flags size
            //   35 for flags
            //   1 for the enum type
            //   1 for empty tree value
            // 32 for node hash
            // 0 for value hash
            // 1 for Basic Merk
            // 2 byte for the value_size (required space for 73 + up to 256 for child key)

            // Parent Hook -> 37
            // Key Bytes 1
            // Hash Size 32
            // Key Length 1
            // Child Heights 2
            // Basic Merk 1

            // Total 34 + 74 + 37 = 145

            //// Ref 1 / <PersonDataContract> / 1 / person / message / My apples are safe
            // Reference to Serialized Item
            // = 276 Bytes

            // Explanation for 276 storage_written_bytes

            // Key -> 65 bytes
            // 32 bytes for the key prefix
            // 32 bytes for the unique id
            // 1 byte for key_size (required space for 64)

            // Value -> 145
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
            // 1 for Basic Merk
            // 2 byte for the value_size (required space for above 128)

            // Parent Hook -> 68
            // Key Bytes 32
            // Hash Size 32
            // Key Length 1
            // Child Heights 2
            // Basic Merk 1

            // Total 65 + 145 + 68 = 278

            //// 359 + 179 + 145 + 278

            961
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
                platform_version,
            );

            let removed_credits = deletion_fees
                .fee_refunds
                .get(owner_id.as_bytes())
                .unwrap()
                .get(&0)
                .unwrap();

            assert_eq!(*removed_credits, 25913567);
            let refund_equivalent_bytes = removed_credits.to_unsigned()
                / Epoch::new(0)
                    .unwrap()
                    .cost_for_known_cost_item(StorageDiskUsageCreditPerByte);

            assert!(expected_added_bytes > refund_equivalent_bytes);
            assert_eq!(refund_equivalent_bytes, 959); // we refunded 959 instead of 962

            // let's re-add it again
            let original_fees = apply_person(
                &drive,
                &contract,
                BlockInfo::default(),
                &person_0_original,
                true,
                transaction.as_ref(),
                platform_version,
            );

            let original_bytes = original_fees.storage_fee
                / Epoch::new(0)
                    .unwrap()
                    .cost_for_known_cost_item(StorageDiskUsageCreditPerByte);

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
            platform_version,
        );
        // we both add and remove bytes
        // this is because trees are added because of indexes, and also removed
        let added_bytes = update_fees.storage_fee
            / Epoch::new(0)
                .unwrap()
                .cost_for_known_cost_item(StorageDiskUsageCreditPerByte);

        let expected_added_bytes = if using_history { 312 } else { 1 };
        assert_eq!(added_bytes, expected_added_bytes);
    }

    fn test_fees_for_update_document_on_index(using_history: bool, using_transaction: bool) {
        let config = DriveConfig {
            batching_consistency_verification: true,
            has_raw_enabled: true,
            default_genesis_time: Some(0),
            ..Default::default()
        };

        let platform_version = PlatformVersion::latest();

        let drive: Drive = setup_drive(Some(config));

        let transaction = if using_transaction {
            Some(drive.grove.start_transaction())
        } else {
            None
        };

        drive
            .create_initial_state_structure(transaction.as_ref(), platform_version)
            .expect("expected to create root tree successfully");

        let path = if using_history {
            "tests/supporting_files/contract/family/family-contract-with-history-only-message-index.json"
        } else {
            "tests/supporting_files/contract/family/family-contract-only-message-index.json"
        };

        // setup code
        let contract = setup_contract(&drive, path, None, transaction.as_ref());

        let id = Identifier::from([1u8; 32]);
        let owner_id = Identifier::from([2u8; 32]);
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
            platform_version,
        );
        let original_bytes = original_fees.storage_fee
            / Epoch::new(0)
                .unwrap()
                .cost_for_known_cost_item(StorageDiskUsageCreditPerByte);
        let expected_added_bytes = if using_history { 1237 } else { 961 };
        assert_eq!(original_bytes, expected_added_bytes);
        if !using_history {
            // let's delete it, just to make sure everything is working.
            let deletion_fees = delete_person(
                &drive,
                &contract,
                BlockInfo::default(),
                &person_0_original,
                transaction.as_ref(),
                platform_version,
            );

            let removed_credits = deletion_fees
                .fee_refunds
                .get(owner_id.as_bytes())
                .unwrap()
                .get(&0)
                .unwrap();

            assert_eq!(*removed_credits, 25913567);
            let refund_equivalent_bytes = removed_credits.to_unsigned()
                / Epoch::new(0)
                    .unwrap()
                    .cost_for_known_cost_item(StorageDiskUsageCreditPerByte);

            assert!(expected_added_bytes > refund_equivalent_bytes);
            assert_eq!(refund_equivalent_bytes, 959); // we refunded 959 instead of 1011

            // let's re-add it again
            let original_fees = apply_person(
                &drive,
                &contract,
                BlockInfo::default(),
                &person_0_original,
                true,
                transaction.as_ref(),
                platform_version,
            );

            let original_bytes = original_fees.storage_fee
                / Epoch::new(0)
                    .unwrap()
                    .cost_for_known_cost_item(StorageDiskUsageCreditPerByte);

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
            platform_version,
        );
        // we both add and remove bytes
        // this is because trees are added because of indexes, and also removed
        let added_bytes = update_fees.storage_fee
            / Epoch::new(0)
                .unwrap()
                .cost_for_known_cost_item(StorageDiskUsageCreditPerByte);

        let removed_credits = update_fees
            .fee_refunds
            .get(owner_id.as_bytes())
            .unwrap()
            .get(&0)
            .unwrap();

        // We added one byte, and since it is an index, and keys are doubled it's 2 extra bytes
        let expected_added_bytes = if using_history { 607 } else { 605 };
        assert_eq!(added_bytes, expected_added_bytes);

        let expected_removed_credits = if using_history { 16286655 } else { 16232643 };
        assert_eq!(*removed_credits, expected_removed_credits);
        let refund_equivalent_bytes = removed_credits.to_unsigned()
            / Epoch::new(0)
                .unwrap()
                .cost_for_known_cost_item(StorageDiskUsageCreditPerByte);

        assert!(expected_added_bytes > refund_equivalent_bytes);
        let expected_remove_bytes = if using_history { 603 } else { 601 };
        assert_eq!(refund_equivalent_bytes, expected_remove_bytes); // we refunded 1011 instead of 1014
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

    fn test_estimated_fees_for_update_document(using_history: bool, using_transaction: bool) {
        let config = DriveConfig {
            batching_consistency_verification: true,
            has_raw_enabled: true,
            default_genesis_time: Some(0),
            ..Default::default()
        };

        let platform_version = PlatformVersion::latest();

        let drive: Drive = setup_drive(Some(config));

        let transaction = if using_transaction {
            Some(drive.grove.start_transaction())
        } else {
            None
        };

        drive
            .create_initial_state_structure(transaction.as_ref(), platform_version)
            .expect("expected to create root tree successfully");

        let path = if using_history {
            "tests/supporting_files/contract/family/family-contract-with-history-only-message-index.json"
        } else {
            "tests/supporting_files/contract/family/family-contract-only-message-index.json"
        };

        // setup code
        let contract = setup_contract(&drive, path, None, transaction.as_ref());

        let id = Identifier::from([1u8; 32]);
        let owner_id = Identifier::from([2u8; 32]);
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
            false,
            transaction.as_ref(),
            platform_version,
        );
        let original_bytes = original_fees.storage_fee
            / Epoch::new(0)
                .unwrap()
                .cost_for_known_cost_item(StorageDiskUsageCreditPerByte);
        let expected_added_bytes = if using_history {
            //Explanation for 1237

            //todo
            1237
        } else {
            //Explanation for 959

            // Document Storage

            //// Item
            // = 358 Bytes

            // Explanation for 358 storage_written_bytes

            // Key -> 65 bytes
            // 32 bytes for the key prefix
            // 32 bytes for the unique id
            // 1 byte for key_size (required space for 64)

            // Value -> 225
            //   1 for the flag option with flags
            //   1 for the flags size
            //   35 for flags 32 + 1 + 2
            //   1 for the enum type
            //   1 for item
            //   116 for item serialized bytes
            //   1 for Basic Merk
            // 32 for node hash
            // 32 for value hash
            // 2 byte for the value_size (required space for above 128)

            // Parent Hook -> 68
            // Key Bytes 32
            // Hash Size 32
            // Key Length 1
            // Child Heights 2
            // Feature Type Basic 1

            // Total 65 + 223 + 68 = 356

            //// Tree 1 / <PersonDataContract> / 1 / person / message
            // Key: My apples are safe
            // = 177 Bytes

            // Explanation for 177 storage_written_bytes

            // Key -> 51 bytes
            // 32 bytes for the key prefix
            // 18 bytes for the key "My apples are safe" 18 characters
            // 1 byte for key_size (required space for 50)

            // Value -> 74
            //   1 for the flag option with flags
            //   1 for the flags size
            //   35 for flags
            //   1 for the enum type
            //   1 for empty tree value
            //   1 for Basic Merk
            // 32 for node hash
            // 0 for value hash
            // 2 byte for the value_size (required space for 73 + up to 256 for child key)

            // Parent Hook -> 54
            // Key Bytes 18
            // Hash Size 32
            // Key Length 1
            // Child Heights 2
            // Basic Merk 1

            // Total 51 + 74 + 54 = 179

            //// Tree 1 / <PersonDataContract> / 1 / person / message / My apples are safe
            // Key: 0
            // = 143 Bytes

            // Explanation for 145 storage_written_bytes

            // Key -> 34 bytes
            // 32 bytes for the key prefix
            // 1 bytes for the key "My apples are safe" 18 characters
            // 1 byte for key_size (required space for 33)

            // Value -> 74
            //   1 for the flag option with flags
            //   1 for the flags size
            //   35 for flags
            //   1 for the enum type
            //   1 for empty tree value
            //   1 for Basic Merk
            // 32 for node hash
            // 0 for value hash
            // 2 byte for the value_size (required space for 73 + up to 256 for child key)

            // Parent Hook -> 37
            // Key Bytes 1
            // Hash Size 32
            // Key Length 1
            // Child Heights 2
            // Basic Merk 1

            // Total 34 + 74 + 37 = 145

            //// Ref 1 / <PersonDataContract> / 1 / person / message / My apples are safe
            // Reference to Serialized Item
            // = 319 Bytes

            // Explanation for 276 storage_written_bytes

            // Key -> 65 bytes
            // 32 bytes for the key prefix
            // 32 bytes for the unique id
            // 1 byte for key_size (required space for 64)

            // Value -> 145
            //   1 for the flag option with flags
            //   1 for the flags size
            //   35 for flags 32 + 1 + 2
            //   1 for the element type as reference
            //   1 for reference type as upstream root reference
            //   1 for reference root height
            //   36 for the reference path bytes ( 1 + 1 + 32 + 1 + 1)
            //   2 for the max reference hop
            //   1 for Basic Merk
            // 32 for node hash
            // 32 for value hash
            // 2 byte for the value_size (required space for above 128)

            // Parent Hook -> 68
            // Key Bytes 32
            // Hash Size 32
            // Key Length 1
            // Child Heights 2
            // No Sum Tree 1

            // Total 65 + 145 + 68 = 278

            // 360 + 179 + 145 + 278 = 960

            961
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
            platform_version,
        );
        // we both add and remove bytes
        // this is because trees are added because of indexes, and also removed
        let added_bytes = update_fees.storage_fee
            / Epoch::new(0)
                .unwrap()
                .cost_for_known_cost_item(StorageDiskUsageCreditPerByte);

        let expected_added_bytes = if using_history { 1238 } else { 962 };
        assert_eq!(added_bytes, expected_added_bytes);
    }

    #[test]
    fn test_estimated_fees_for_update_document_no_history_using_transaction() {
        test_estimated_fees_for_update_document(false, true)
    }

    #[test]
    fn test_estimated_fees_for_update_document_no_history_no_transaction() {
        test_estimated_fees_for_update_document(false, false)
    }

    #[test]
    fn test_estimated_fees_for_update_document_with_history_using_transaction() {
        test_estimated_fees_for_update_document(true, true)
    }

    #[test]
    fn test_estimated_fees_for_update_document_with_history_no_transaction() {
        test_estimated_fees_for_update_document(true, false)
    }

    #[derive(Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Person {
        #[serde(rename = "$id")]
        id: Identifier,
        #[serde(rename = "$ownerId")]
        owner_id: Identifier,
        first_name: String,
        middle_name: String,
        last_name: String,
        message: Option<String>,
        age: u8,
    }

    fn apply_person(
        drive: &Drive,
        contract: &DataContract,
        block_info: BlockInfo,
        person: &Person,
        apply: bool,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> FeeResult {
        let document_type = contract
            .document_type_for_name("person")
            .expect("expected to get document type");

        let value = platform_value::to_value(person).expect("person into value");

        let document =
            Document::from_platform_value(value, platform_version).expect("value to document");

        let storage_flags = Some(Cow::Owned(StorageFlags::SingleEpochOwned(
            0,
            person.owner_id.to_buffer(),
        )));

        drive
            .add_document_for_contract(
                DocumentAndContractInfo {
                    owned_document_info: OwnedDocumentInfo {
                        document_info: DocumentRefInfo((&document, storage_flags)),
                        owner_id: None,
                    },
                    contract,
                    document_type,
                },
                true,
                block_info,
                apply,
                transaction,
                platform_version,
            )
            .expect("expected to add document")
    }

    fn delete_person(
        drive: &Drive,
        contract: &DataContract,
        block_info: BlockInfo,
        person: &Person,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> FeeResult {
        drive
            .delete_document_for_contract(
                person.id.to_buffer(),
                contract,
                "person",
                block_info,
                true,
                transaction,
                platform_version,
            )
            .expect("expected to remove person")
    }

    fn test_update_complex_person(
        using_history: bool,
        using_transaction: bool,
        using_has_raw: bool,
    ) {
        let config = DriveConfig {
            batching_consistency_verification: true,
            has_raw_enabled: using_has_raw,
            default_genesis_time: Some(0),
            ..Default::default()
        };

        let platform_version = PlatformVersion::latest();

        let drive: Drive = setup_drive(Some(config));

        let transaction = if using_transaction {
            Some(drive.grove.start_transaction())
        } else {
            None
        };

        drive
            .create_initial_state_structure(transaction.as_ref(), platform_version)
            .expect("expected to create root tree successfully");

        let path = if using_history {
            "tests/supporting_files/contract/family/family-contract-with-history-only-message-index.json"
        } else {
            "tests/supporting_files/contract/family/family-contract-only-message-index.json"
        };

        // setup code
        let contract = setup_contract(&drive, path, None, transaction.as_ref());

        let person_0_original = Person {
            id: Identifier::from([0u8; 32]),
            owner_id: Identifier::from([0u8; 32]),
            first_name: "Samuel".to_string(),
            middle_name: "Abraham".to_string(),
            last_name: "Westrich".to_string(),
            message: Some("My apples are safe".to_string()),
            age: 33,
        };

        let person_0_updated = Person {
            id: Identifier::from([0u8; 32]),
            owner_id: Identifier::from([0u8; 32]),
            first_name: "Samuel".to_string(),
            middle_name: "Abraham".to_string(),
            last_name: "Westrich".to_string(),
            message: Some("Lemons are now my thing too".to_string()),
            age: 35,
        };

        let person_1_original = Person {
            id: Identifier::from([1u8; 32]),
            owner_id: Identifier::from([1u8; 32]),
            first_name: "Wisdom".to_string(),
            middle_name: "Madabuchukwu".to_string(),
            last_name: "Ogwu".to_string(),
            message: Some("Cantaloupe is the best fruit under the sun".to_string()),
            age: 20,
        };

        let person_1_updated = Person {
            id: Identifier::from([1u8; 32]),
            owner_id: Identifier::from([1u8; 32]),
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
            platform_version,
        );
        apply_person(
            &drive,
            &contract,
            BlockInfo::default(),
            &person_1_original,
            true,
            transaction.as_ref(),
            platform_version,
        );
        apply_person(
            &drive,
            &contract,
            BlockInfo::default_with_time(100),
            &person_0_updated,
            true,
            transaction.as_ref(),
            platform_version,
        );
        apply_person(
            &drive,
            &contract,
            BlockInfo::default_with_time(100),
            &person_1_updated,
            true,
            transaction.as_ref(),
            platform_version,
        );
    }

    #[test]
    fn test_update_complex_person_with_history_no_transaction_and_has_raw() {
        test_update_complex_person(true, false, true)
    }

    #[test]
    fn test_update_complex_person_with_history_no_transaction_and_get_raw() {
        test_update_complex_person(true, false, false)
    }

    #[test]
    fn test_update_complex_person_with_history_with_transaction_and_has_raw() {
        test_update_complex_person(true, true, true)
    }

    #[test]
    fn test_update_complex_person_with_history_with_transaction_and_get_raw() {
        test_update_complex_person(true, true, false)
    }

    #[test]
    fn test_update_complex_person_no_history_no_transaction_and_has_raw() {
        test_update_complex_person(false, false, true)
    }

    #[test]
    fn test_update_complex_person_no_history_no_transaction_and_get_raw() {
        test_update_complex_person(false, false, false)
    }

    #[test]
    fn test_update_complex_person_no_history_with_transaction_and_has_raw() {
        test_update_complex_person(false, true, true)
    }

    #[test]
    fn test_update_complex_person_no_history_with_transaction_and_get_raw() {
        test_update_complex_person(false, true, false)
    }

    #[test]
    fn test_update_document_without_apply_should_calculate_storage_fees() {
        let drive = setup_drive_with_initial_state_structure();

        let platform_version = PlatformVersion::latest();

        // Create a contract

        let block_info = BlockInfo::default();
        let owner_id = Identifier::new([2u8; 32]);

        let documents = platform_value!({
            "niceDocument": {
                "type": "object",
                "properties": {
                    "name": {
                        "type": "string",
                        "position": 0,
                    }
                },
                "required": [
                    "$createdAt"
                ],
                "additionalProperties": false
            }
        });

        let factory = DataContractFactory::new(1).expect("expected to create factory");

        let contract = factory
            .create_with_value_config(owner_id, 0, documents, None, None)
            .expect("data in fixture should be correct")
            .data_contract_owned();

        drive
            .apply_contract(
                &contract,
                block_info.clone(),
                true,
                StorageFlags::optional_default_as_cow(),
                None,
                platform_version,
            )
            .expect("should apply contract");

        // Create a document factory

        let document_factory = SpecializedDocumentFactory::new(1, contract)
            .expect("expected to create document factory");

        // Create a document

        let document_type_name = "niceDocument".to_string();

        let document_type = document_factory
            .data_contract()
            .document_type_for_name(document_type_name.as_str())
            .expect("expected document type");

        let mut document = document_factory
            .create_document(
                owner_id,
                document_type_name.clone(),
                json!({ "name": "Ivan" }).into(),
            )
            .expect("should create a document");

        let storage_flags = Some(Cow::Owned(StorageFlags::SingleEpochOwned(
            0,
            owner_id.to_buffer(),
        )));

        let document_info = DocumentRefInfo((&document, storage_flags.clone()));

        let create_fees = drive
            .add_document_for_contract(
                DocumentAndContractInfo {
                    owned_document_info: OwnedDocumentInfo {
                        document_info,
                        owner_id: Some(owner_id.to_buffer()),
                    },
                    contract: document_factory.data_contract(),
                    document_type,
                },
                false,
                block_info,
                true,
                None,
                platform_version,
            )
            .expect("should create document");

        assert_ne!(create_fees.storage_fee, 0);

        // Update the document in a second

        document.set("name", Value::Text("Ivaaaaaaaaaan!".to_string()));

        let block_info = BlockInfo::default_with_time(10000);

        let update_fees = drive
            .update_document_for_contract(
                &document,
                document_factory.data_contract(),
                document_type,
                Some(owner_id.to_buffer()),
                block_info,
                false,
                storage_flags,
                None,
                platform_version,
            )
            .expect("should update document");

        assert_ne!(update_fees.storage_fee, 0);
    }
}
