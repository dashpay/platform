//! Update Documents.
//!
//! This modules implements functions in Drive relevant to updating Documents.
//!

// Module: add_update_multiple_documents_operations
// This module contains functionality for adding operations to update multiple documents
#[cfg(feature = "server")]
mod add_update_multiple_documents_operations;

// Module: update_document_for_contract
// This module contains functionality for updating a document for a given contract
#[cfg(feature = "server")]
mod update_document_for_contract;

// Module: update_document_for_contract_id
// This module contains functionality for updating a document associated with a given contract id
#[cfg(feature = "server")]
mod update_document_for_contract_id;

// Module: update_document_with_serialization_for_contract
// This module contains functionality for updating a document (with serialization) for a contract
mod internal;
mod update_document_with_serialization_for_contract;

#[cfg(test)]
mod tests {
    use dpp::data_contract::{DataContract, DataContractFactory};
    use grovedb::TransactionArg;
    use std::borrow::Cow;
    use std::collections::BTreeMap;
    use std::default::Default;
    use std::option::Option::None;

    use dpp::platform_value::{platform_value, Identifier, Value};

    use dpp::block::block_info::BlockInfo;

    use dpp::balances::credits::Creditable;
    use rand::{random, Rng};
    use serde::{Deserialize, Serialize};
    use serde_json::json;

    use crate::config::DriveConfig;
    use crate::drive::Drive;
    use crate::error::drive::DriveError;
    use crate::error::Error;
    use crate::util::object_size_info::DocumentInfo::{DocumentOwnedInfo, DocumentRefInfo};
    use crate::util::object_size_info::{DocumentAndContractInfo, OwnedDocumentInfo};
    use crate::util::storage_flags::StorageFlags;

    use crate::drive::document::query::QueryDocumentsOutcomeV0Methods;
    use crate::drive::document::tests::setup_dashpay;
    use crate::query::DriveDocumentQuery;
    use crate::util::test_helpers::setup::{setup_drive, setup_drive_with_initial_state_structure};
    use crate::util::test_helpers::setup_contract;
    use dpp::block::epoch::Epoch;
    use dpp::data_contract::accessors::v0::DataContractV0Getters;
    use dpp::data_contract::conversion::value::v0::DataContractValueConversionMethodsV0;
    use dpp::data_contract::document_type::methods::DocumentTypeV0Methods;
    use dpp::document::document_methods::DocumentMethodsV0;
    use dpp::document::serialization_traits::DocumentPlatformConversionMethodsV0;
    use dpp::document::specialized_document_factory::SpecializedDocumentFactory;
    use dpp::document::{Document, DocumentV0Getters, DocumentV0Setters};
    use dpp::fee::default_costs::KnownCostItem::StorageDiskUsageCreditPerByte;
    use dpp::fee::default_costs::{CachedEpochIndexFeeVersions, EpochCosts};
    use dpp::fee::fee_result::FeeResult;
    use dpp::platform_value;
    use dpp::serialization::ValueConvertible;
    use dpp::tests::json_document::json_document_to_document;
    use dpp::version::fee::FeeVersion;
    use once_cell::sync::Lazy;
    use platform_version::version::PlatformVersion;

    static EPOCH_CHANGE_FEE_VERSION_TEST: Lazy<CachedEpochIndexFeeVersions> =
        Lazy::new(|| BTreeMap::from([(0, FeeVersion::first())]));

    /// Build a `Document` from a legacy un-tagged `platform_value!` map by
    /// inserting `$formatVersion: "0"` and routing through canonical
    /// `ValueConvertible::from_object`. Replaces the deleted
    /// `Document::from_platform_value` ingest path.
    fn document_from_legacy_value(mut value: Value) -> Document {
        if let Value::Map(ref mut entries) = value {
            let has_tag = entries
                .iter()
                .any(|(k, _)| matches!(k, Value::Text(s) if s == "$formatVersion"));
            if !has_tag {
                entries.push((
                    Value::Text("$formatVersion".to_string()),
                    Value::Text("0".to_string()),
                ));
            }
        }
        Document::from_object(value).expect("expected to make document from legacy value")
    }

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
                Some(&EPOCH_CHANGE_FEE_VERSION_TEST),
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
                Some(&EPOCH_CHANGE_FEE_VERSION_TEST),
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
                Some(&EPOCH_CHANGE_FEE_VERSION_TEST),
            )
            .expect("should create alice profile");

        // Check Alice profile

        let sql_string = "select * from profile";
        let query = DriveDocumentQuery::from_sql_expr(
            sql_string,
            &contract,
            Some(&DriveConfig::default()),
            platform_version,
        )
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
                Some(&EPOCH_CHANGE_FEE_VERSION_TEST),
            )
            .expect("should update alice profile");

        let (results_no_transaction, _, _) = query
            .execute_raw_results_no_proof(&drive, None, None, platform_version)
            .expect("expected to execute query");

        assert_eq!(results_no_transaction.len(), 1);
    }

    #[test]
    fn test_update_nonexistent_document_returns_error() {
        let (drive, contract) = setup_dashpay("update-nonexistent", true);

        let platform_version = PlatformVersion::latest();

        let document_type = contract
            .document_type_for_name("contactRequest")
            .expect("contactRequest document exists");

        let owner_id = random::<[u8; 32]>();

        let document = json_document_to_document(
            "tests/supporting_files/contract/dashpay/contact-request0.json",
            Some(owner_id.into()),
            document_type,
            platform_version,
        )
        .expect("expected to get cbor document");

        let err = drive
            .update_document_for_contract(
                &document,
                &contract,
                document_type,
                Some(owner_id),
                BlockInfo::default(),
                true,
                StorageFlags::optional_default_as_cow(),
                None,
                platform_version,
                Some(&EPOCH_CHANGE_FEE_VERSION_TEST),
            )
            .expect_err("expected updating a nonexistent document to fail");

        assert!(
            matches!(
                err,
                Error::Drive(DriveError::UpdatingDocumentThatDoesNotExist(_))
            ) || matches!(
                err,
                Error::GroveDB(ref grovedb_error)
                    if matches!(grovedb_error.as_ref(), grovedb::Error::PathKeyNotFound(_))
            )
        );
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
                Some(&EPOCH_CHANGE_FEE_VERSION_TEST),
            )
            .expect("should create alice profile");

        drive
            .grove
            .commit_transaction(db_transaction)
            .unwrap()
            .expect("should commit transaction");

        // Check Alice profile

        let sql_string = "select * from profile";
        let query = DriveDocumentQuery::from_sql_expr(
            sql_string,
            &contract,
            Some(&DriveConfig::default()),
            platform_version,
        )
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
                Some(&EPOCH_CHANGE_FEE_VERSION_TEST),
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
                Some(&EPOCH_CHANGE_FEE_VERSION_TEST),
            )
            .expect("should create alice profile");

        drive
            .grove
            .commit_transaction(db_transaction)
            .unwrap()
            .expect("should commit transaction");

        // Check Alice profile

        let sql_string = "select * from profile";
        let query = DriveDocumentQuery::from_sql_expr(
            sql_string,
            &contract,
            Some(&DriveConfig::default()),
            platform_version,
        )
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
                document.id(),
                &contract,
                "profile",
                BlockInfo::default(),
                true,
                Some(&db_transaction),
                platform_version,
                Some(&EPOCH_CHANGE_FEE_VERSION_TEST),
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
                Some(&EPOCH_CHANGE_FEE_VERSION_TEST),
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
        let drive = setup_drive_with_initial_state_structure(None);

        let platform_version = PlatformVersion::latest();

        let contract = platform_value!({
            "$formatVersion": "0",
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

        let document = document_from_legacy_value(document_values);

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
                None,
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

        let document = document_from_legacy_value(document_values);

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
                Some(&EPOCH_CHANGE_FEE_VERSION_TEST),
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
                Some(&EPOCH_CHANGE_FEE_VERSION_TEST),
            )
            .expect("should delete document");
    }

    #[test]
    fn test_update_document_with_unique_index_when_some_indexed_fields_are_null() {
        // A unique index where SOME (not all) of the indexed properties are
        // null must use the non-unique storage layout (a `[0]` tree keyed by
        // document id) because uniqueness can't be enforced on null — that is
        // what the insert and delete walkers do. The update walker must agree
        // on the layout, both when deleting the entry under the old values
        // and when writing the entry under the new ones.
        let drive = setup_drive_with_initial_state_structure(None);

        let platform_version = PlatformVersion::latest();

        let contract = platform_value!({
            "$formatVersion": "0",
            "id": "BZUodcFoFL6KvnonehrnMVggTvCe8W5MiRnZuqLb6M54",
            "schema": "https://schema.dash.org/dpp-0-4-0/meta/data-contract",
            "version": 1,
            "ownerId": "GZVdTnLFAN2yE9rLeCHBDBCr7YQgmXJuoExkY347j7Z5",
            "documentSchemas": {
                "indexedDocument": {
                    "type": "object",
                    "indices": [
                        {"name":"uniqueFirstLast", "properties": [{"firstName":"asc"}, {"lastName":"asc"}], "unique":true},
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
                    "required": ["firstName"],
                    "additionalProperties": false,
                },
            },
        });

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

        // Create a document with a null lastName (one of two indexed fields)

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
        });

        let document = document_from_legacy_value(document_values);

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
                None,
            )
            .expect("should add document");

        // Update the non-null indexed property, lastName stays null

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
        });

        let document = document_from_legacy_value(document_values);

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
                Some(&EPOCH_CHANGE_FEE_VERSION_TEST),
            )
            .expect("should update document");

        // The document must be findable under the new index value…

        let query = DriveDocumentQuery::from_sql_expr(
            "select * from indexedDocument where firstName = 'updatedName'",
            &contract,
            Some(&DriveConfig::default()),
            platform_version,
        )
        .expect("should build query");

        let (results, _, _) = query
            .execute_raw_results_no_proof(&drive, None, None, platform_version)
            .expect("expected to execute query for the new index value");

        assert_eq!(
            results.len(),
            1,
            "updated document should be found under its new index value"
        );

        // …and no entry may remain under the old index value

        let query = DriveDocumentQuery::from_sql_expr(
            "select * from indexedDocument where firstName = 'myName'",
            &contract,
            Some(&DriveConfig::default()),
            platform_version,
        )
        .expect("should build query");

        let (results, _, _) = query
            .execute_raw_results_no_proof(&drive, None, None, platform_version)
            .expect("expected to execute query for the old index value");

        assert_eq!(
            results.len(),
            0,
            "no index entry should remain under the old index value"
        );
    }

    #[test]
    fn test_modify_dashpay_contact_request() {
        let drive = setup_drive_with_initial_state_structure(None);

        let db_transaction = drive.grove.start_transaction();

        let platform_version = PlatformVersion::latest();

        let contract = setup_contract(
            &drive,
            "tests/supporting_files/contract/dashpay/dashpay-contract.json",
            None,
            None,
            None::<fn(&mut DataContract)>,
            Some(&db_transaction),
            None,
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
                None,
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
                Some(&EPOCH_CHANGE_FEE_VERSION_TEST),
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
                Some(&EPOCH_CHANGE_FEE_VERSION_TEST),
            )
            .expect_err("expected not to be able to override a non mutable document");
    }

    #[test]
    fn test_update_dashpay_profile_with_history() {
        let drive = setup_drive_with_initial_state_structure(None);

        let db_transaction = drive.grove.start_transaction();

        let platform_version = PlatformVersion::latest();

        let contract = setup_contract(
            &drive,
            "tests/supporting_files/contract/dashpay/dashpay-contract-with-profile-history.json",
            None,
            None,
            None::<fn(&mut DataContract)>,
            Some(&db_transaction),
            None,
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
                None,
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
                Some(&EPOCH_CHANGE_FEE_VERSION_TEST),
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
        let contract = setup_contract(
            &drive,
            path,
            None,
            None,
            None::<fn(&mut DataContract)>,
            transaction.as_ref(),
            None,
        );

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

        let document = document_from_legacy_value(value);

        let document_serialized = DocumentPlatformConversionMethodsV0::serialize(
            &document,
            document_type,
            &contract,
            platform_version,
        )
        .expect("expected to serialize document");

        assert_eq!(document_serialized.len(), 121);
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
            / Epoch::new(0).unwrap().cost_for_known_cost_item(
                &EPOCH_CHANGE_FEE_VERSION_TEST,
                StorageDiskUsageCreditPerByte,
            );
        let expected_added_bytes = if using_history {
            //Explanation for 1237

            //todo
            1238
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

            962
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

            assert_eq!(*removed_credits, 25940733);
            let refund_equivalent_bytes = removed_credits.to_unsigned()
                / Epoch::new(0).unwrap().cost_for_known_cost_item(
                    &EPOCH_CHANGE_FEE_VERSION_TEST,
                    StorageDiskUsageCreditPerByte,
                );

            assert!(expected_added_bytes > refund_equivalent_bytes);
            assert_eq!(refund_equivalent_bytes, 960); // we refunded 960 instead of 963

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
                / Epoch::new(0).unwrap().cost_for_known_cost_item(
                    &EPOCH_CHANGE_FEE_VERSION_TEST,
                    StorageDiskUsageCreditPerByte,
                );

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
            / Epoch::new(0).unwrap().cost_for_known_cost_item(
                &EPOCH_CHANGE_FEE_VERSION_TEST,
                StorageDiskUsageCreditPerByte,
            );

        let expected_added_bytes = if using_history { 313 } else { 1 };
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
        let contract = setup_contract(
            &drive,
            path,
            None,
            None,
            None::<fn(&mut DataContract)>,
            transaction.as_ref(),
            None,
        );

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
            / Epoch::new(0).unwrap().cost_for_known_cost_item(
                &EPOCH_CHANGE_FEE_VERSION_TEST,
                StorageDiskUsageCreditPerByte,
            );
        let expected_added_bytes = if using_history { 1238 } else { 962 };
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

            assert_eq!(*removed_credits, 25940733);
            let refund_equivalent_bytes = removed_credits.to_unsigned()
                / Epoch::new(0).unwrap().cost_for_known_cost_item(
                    &EPOCH_CHANGE_FEE_VERSION_TEST,
                    StorageDiskUsageCreditPerByte,
                );

            assert!(expected_added_bytes > refund_equivalent_bytes);
            assert_eq!(refund_equivalent_bytes, 960); // we refunded 960 instead of 1012

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
                / Epoch::new(0).unwrap().cost_for_known_cost_item(
                    &EPOCH_CHANGE_FEE_VERSION_TEST,
                    StorageDiskUsageCreditPerByte,
                );

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
            / Epoch::new(0).unwrap().cost_for_known_cost_item(
                &EPOCH_CHANGE_FEE_VERSION_TEST,
                StorageDiskUsageCreditPerByte,
            );

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
            / Epoch::new(0).unwrap().cost_for_known_cost_item(
                &EPOCH_CHANGE_FEE_VERSION_TEST,
                StorageDiskUsageCreditPerByte,
            );

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
        let contract = setup_contract(
            &drive,
            path,
            None,
            None,
            None::<fn(&mut DataContract)>,
            transaction.as_ref(),
            None,
        );

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
            / Epoch::new(0).unwrap().cost_for_known_cost_item(
                &EPOCH_CHANGE_FEE_VERSION_TEST,
                StorageDiskUsageCreditPerByte,
            );
        let expected_added_bytes = if using_history {
            //Explanation for 1237

            //todo
            1238
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

            962
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
            / Epoch::new(0).unwrap().cost_for_known_cost_item(
                &EPOCH_CHANGE_FEE_VERSION_TEST,
                StorageDiskUsageCreditPerByte,
            );

        let expected_added_bytes = if using_history { 1239 } else { 963 };
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

        let document = document_from_legacy_value(value);

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
                Some(&EPOCH_CHANGE_FEE_VERSION_TEST),
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
                person.id,
                contract,
                "person",
                block_info,
                true,
                transaction,
                platform_version,
                Some(&EPOCH_CHANGE_FEE_VERSION_TEST),
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
        let contract = setup_contract(
            &drive,
            path,
            None,
            None,
            None::<fn(&mut DataContract)>,
            transaction.as_ref(),
            None,
        );

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
        let drive = setup_drive_with_initial_state_structure(None);

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
                block_info,
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
                None,
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
                None,
            )
            .expect("should update document");

        assert_ne!(update_fees.storage_fee, 0);
    }

    #[test]
    fn test_add_update_multiple_documents_operations() {
        let (drive, contract) = setup_dashpay("multi-update", true);

        let platform_version = PlatformVersion::latest();

        let document_type = contract
            .document_type_for_name("contactRequest")
            .expect("contactRequest document exists");

        let owner_id = random::<[u8; 32]>();

        let doc0 = json_document_to_document(
            "tests/supporting_files/contract/dashpay/contact-request0.json",
            Some(owner_id.into()),
            document_type,
            platform_version,
        )
        .expect("expected to get document");

        let doc1 = json_document_to_document(
            "tests/supporting_files/contract/dashpay/contact-request1.json",
            Some(owner_id.into()),
            document_type,
            platform_version,
        )
        .expect("expected to get document");

        // Insert documents first so update operations target existing documents
        drive
            .add_document_for_contract(
                DocumentAndContractInfo {
                    owned_document_info: OwnedDocumentInfo {
                        document_info: DocumentRefInfo((
                            &doc0,
                            StorageFlags::optional_default_as_cow(),
                        )),
                        owner_id: Some(owner_id),
                    },
                    contract: &contract,
                    document_type,
                },
                false,
                BlockInfo::default(),
                true,
                None,
                platform_version,
                None,
            )
            .expect("expected to insert first document");

        drive
            .add_document_for_contract(
                DocumentAndContractInfo {
                    owned_document_info: OwnedDocumentInfo {
                        document_info: DocumentRefInfo((
                            &doc1,
                            StorageFlags::optional_default_as_cow(),
                        )),
                        owner_id: Some(owner_id),
                    },
                    contract: &contract,
                    document_type,
                },
                false,
                BlockInfo::default(),
                true,
                None,
                platform_version,
                None,
            )
            .expect("expected to insert second document");

        let documents = vec![doc0, doc1];

        let mut drive_operations = vec![];

        drive
            .add_update_multiple_documents_operations(
                &documents,
                &contract,
                document_type,
                &mut drive_operations,
                &platform_version.drive,
            )
            .expect("expected to add update operations");

        // Should have created one operation containing both documents
        assert_eq!(drive_operations.len(), 1);
    }

    #[test]
    fn test_add_update_multiple_documents_operations_empty() {
        let (drive, contract) = setup_dashpay("multi-update-empty", true);

        let platform_version = PlatformVersion::latest();

        let document_type = contract
            .document_type_for_name("contactRequest")
            .expect("contactRequest document exists");

        let documents: Vec<dpp::document::Document> = vec![];

        let mut drive_operations = vec![];

        drive
            .add_update_multiple_documents_operations(
                &documents,
                &contract,
                document_type,
                &mut drive_operations,
                &platform_version.drive,
            )
            .expect("expected empty documents to succeed");

        // Empty documents should produce no operations
        assert_eq!(drive_operations.len(), 0);
    }

    #[test]
    fn test_update_document_for_contract_immutable_error() {
        let drive = setup_drive_with_initial_state_structure(None);

        let db_transaction = drive.grove.start_transaction();

        let platform_version = PlatformVersion::latest();

        // Use the non-mutable dashpay contract
        let contract = setup_contract(
            &drive,
            "tests/supporting_files/contract/dashpay/dashpay-contract.json",
            None,
            None,
            None::<fn(&mut DataContract)>,
            Some(&db_transaction),
            None,
        );

        // contactRequest is not mutable in this contract
        let document_type = contract
            .document_type_for_name("contactRequest")
            .expect("contactRequest document exists");

        let owner_id = random::<[u8; 32]>();

        let document = json_document_to_document(
            "tests/supporting_files/contract/dashpay/contact-request0.json",
            Some(owner_id.into()),
            document_type,
            platform_version,
        )
        .expect("expected to get cbor document");

        // Try to update an immutable document type
        let err = drive
            .update_document_for_contract(
                &document,
                &contract,
                document_type,
                Some(owner_id),
                BlockInfo::default(),
                true,
                StorageFlags::optional_default_as_cow(),
                Some(&db_transaction),
                platform_version,
                None,
            )
            .expect_err("expected updating an immutable document type to fail");

        assert!(matches!(
            err,
            Error::Drive(DriveError::UpdatingReadOnlyImmutableDocument(_))
        ));
    }

    #[test]
    fn test_update_document_with_serialization_for_contract() {
        let (drive, contract) = setup_dashpay("update-serialized", true);

        let platform_version = PlatformVersion::latest();

        let document_type = contract
            .document_type_for_name("profile")
            .expect("profile document exists");

        let owner_id: Identifier = random::<[u8; 32]>().into();

        let mut document = document_type
            .create_document_from_data(
                platform_value!({"displayName": "Alice"}),
                owner_id,
                random(),
                random(),
                random(),
                platform_version,
            )
            .expect("should create document");

        // First create the document
        let info = DocumentAndContractInfo {
            owned_document_info: OwnedDocumentInfo {
                document_info: DocumentRefInfo((
                    &document,
                    StorageFlags::optional_default_as_cow(),
                )),
                owner_id: Some(owner_id.to_buffer()),
            },
            contract: &contract,
            document_type,
        };

        drive
            .add_document_for_contract(
                info,
                true,
                BlockInfo::default(),
                true,
                None,
                platform_version,
                None,
            )
            .expect("should create profile");

        // Update document
        document.set("displayName", "Bob".into());
        document
            .increment_revision()
            .expect("document should have a revision to increment");

        let serialized = DocumentPlatformConversionMethodsV0::serialize(
            &document,
            document_type,
            &contract,
            platform_version,
        )
        .expect("expected to serialize");

        let fee_result = drive
            .update_document_with_serialization_for_contract(
                &document,
                &serialized,
                &contract,
                "profile",
                Some(owner_id.to_buffer()),
                BlockInfo::default(),
                true,
                StorageFlags::optional_default_as_cow(),
                None,
                platform_version,
                None,
            )
            .expect("expected to update document with serialization");

        assert!(fee_result.processing_fee > 0);

        // Fetch the document back and verify the update took effect
        let sql_string = "select * from profile";
        let query = DriveDocumentQuery::from_sql_expr(
            sql_string,
            &contract,
            Some(&DriveConfig::default()),
            platform_version,
        )
        .expect("should build query");

        let outcome = drive
            .query_documents(query, None, false, None, None)
            .expect("expected to query documents");

        assert_eq!(outcome.documents().len(), 1);
        let fetched_doc = &outcome.documents()[0];
        assert_eq!(
            fetched_doc
                .get("displayName")
                .expect("displayName should exist")
                .as_text()
                .expect("displayName should be text"),
            "Bob",
            "displayName should have been updated to Bob"
        );
    }

    #[test]
    fn test_update_document_for_contract_id() {
        let (drive, contract) = setup_dashpay("update-by-id", true);

        let platform_version = PlatformVersion::latest();

        let document_type = contract
            .document_type_for_name("profile")
            .expect("profile document exists");

        let owner_id: Identifier = random::<[u8; 32]>().into();

        let mut document = document_type
            .create_document_from_data(
                platform_value!({"displayName": "Alice"}),
                owner_id,
                random(),
                random(),
                random(),
                platform_version,
            )
            .expect("should create document");

        // First create the document
        let info = DocumentAndContractInfo {
            owned_document_info: OwnedDocumentInfo {
                document_info: DocumentRefInfo((
                    &document,
                    StorageFlags::optional_default_as_cow(),
                )),
                owner_id: Some(owner_id.to_buffer()),
            },
            contract: &contract,
            document_type,
        };

        drive
            .add_document_for_contract(
                info,
                true,
                BlockInfo::default(),
                true,
                None,
                platform_version,
                None,
            )
            .expect("should create profile");

        // Update document using contract_id API
        document.set("displayName", "Bob".into());
        document
            .increment_revision()
            .expect("document should have a revision to increment");

        let serialized = DocumentPlatformConversionMethodsV0::serialize(
            &document,
            document_type,
            &contract,
            platform_version,
        )
        .expect("expected to serialize");

        let fee_result = drive
            .update_document_for_contract_id(
                &serialized,
                contract.id().to_buffer(),
                "profile",
                Some(owner_id.to_buffer()),
                BlockInfo::default(),
                true,
                StorageFlags::optional_default_as_cow(),
                None,
                platform_version,
                None,
            )
            .expect("expected to update document for contract id");

        assert!(fee_result.processing_fee > 0);

        // Fetch the document back and verify the update took effect
        let sql_string = "select * from profile";
        let query = DriveDocumentQuery::from_sql_expr(
            sql_string,
            &contract,
            Some(&DriveConfig::default()),
            platform_version,
        )
        .expect("should build query");

        let outcome = drive
            .query_documents(query, None, false, None, None)
            .expect("expected to query documents");

        assert_eq!(outcome.documents().len(), 1);
        let fetched_doc = &outcome.documents()[0];
        assert_eq!(
            fetched_doc
                .get("displayName")
                .expect("displayName should exist")
                .as_text()
                .expect("displayName should be text"),
            "Bob",
            "displayName should have been updated to Bob"
        );
    }

    // ---------- Error-path tests (added for coverage) ----------

    #[test]
    fn test_update_document_for_contract_id_nonexistent_contract_returns_error() {
        // `update_document_for_contract_id` resolves the contract by id.
        // A nonexistent id must yield DocumentError::DataContractNotFound.
        use crate::error::document::DocumentError;

        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        // Arbitrary serialized bytes - we never reach the deserialize step
        // because the contract lookup fails first.
        let dummy_serialized = vec![0u8; 32];
        let nonexistent_contract_id: [u8; 32] = random();

        let err = drive
            .update_document_for_contract_id(
                &dummy_serialized,
                nonexistent_contract_id,
                "profile",
                None,
                BlockInfo::default(),
                true,
                StorageFlags::optional_default_as_cow(),
                None,
                platform_version,
                Some(&EPOCH_CHANGE_FEE_VERSION_TEST),
            )
            .expect_err("expected update_document_for_contract_id to fail");

        assert!(
            matches!(err, Error::Document(DocumentError::DataContractNotFound)),
            "expected DataContractNotFound, got {err:?}"
        );
    }

    #[test]
    fn test_update_document_for_contract_id_invalid_document_type_returns_error() {
        // The contract exists, but the document_type_name does not. This
        // exercises the `contract.document_type_for_name(...)?` error branch
        // in update_document_for_contract_id_v0.
        let (drive, contract) = setup_dashpay("update-by-id-bad-type", true);
        let platform_version = PlatformVersion::latest();

        let dummy_serialized = vec![0u8; 32];

        let err = drive
            .update_document_for_contract_id(
                &dummy_serialized,
                contract.id().to_buffer(),
                "not_a_real_document_type",
                None,
                BlockInfo::default(),
                true,
                StorageFlags::optional_default_as_cow(),
                None,
                platform_version,
                Some(&EPOCH_CHANGE_FEE_VERSION_TEST),
            )
            .expect_err("expected update_document_for_contract_id with bad type to fail");

        assert!(
            !matches!(
                err,
                Error::Document(crate::error::document::DocumentError::DataContractNotFound)
            ),
            "expected a document-type error, got {err:?}"
        );
    }

    #[test]
    fn test_update_document_for_contract_id_malformed_serialized_returns_error() {
        // Contract & document type exist, but the serialized bytes are not a
        // valid document. This exercises the `Document::from_bytes(...)?`
        // error branch in update_document_for_contract_id_v0.
        let (drive, contract) = setup_dashpay("update-by-id-bad-bytes", true);
        let platform_version = PlatformVersion::latest();

        // Definitively malformed: all zeros won't be a valid serialized doc.
        let malformed = vec![0xFFu8; 8];

        let err = drive
            .update_document_for_contract_id(
                &malformed,
                contract.id().to_buffer(),
                "profile",
                None,
                BlockInfo::default(),
                true,
                StorageFlags::optional_default_as_cow(),
                None,
                platform_version,
                Some(&EPOCH_CHANGE_FEE_VERSION_TEST),
            )
            .expect_err("expected update_document_for_contract_id with malformed bytes to fail");

        // Should NOT be a contract-not-found error - the contract is valid.
        assert!(
            !matches!(
                err,
                Error::Document(crate::error::document::DocumentError::DataContractNotFound)
            ),
            "expected a deserialization error, got {err:?}"
        );
    }

    #[test]
    fn test_update_document_with_serialization_for_contract_invalid_document_type() {
        // Exercises the `contract.document_type_for_name(...)?` error branch in
        // update_document_with_serialization_for_contract_v0.
        let (drive, contract) = setup_dashpay("update-ser-bad-type", true);
        let platform_version = PlatformVersion::latest();

        // Build a valid document & serialization against the real profile type
        // so that we reach `document_type_for_name` with a bad name, not fail
        // earlier elsewhere.
        let document_type = contract
            .document_type_for_name("profile")
            .expect("profile document exists");

        let owner_id: Identifier = random::<[u8; 32]>().into();

        let document = document_type
            .create_document_from_data(
                platform_value!({"displayName": "Alice"}),
                owner_id,
                random(),
                random(),
                random(),
                platform_version,
            )
            .expect("should create document");

        let serialized = DocumentPlatformConversionMethodsV0::serialize(
            &document,
            document_type,
            &contract,
            platform_version,
        )
        .expect("expected to serialize");

        let err = drive
            .update_document_with_serialization_for_contract(
                &document,
                &serialized,
                &contract,
                "not_a_real_document_type",
                Some(owner_id.to_buffer()),
                BlockInfo::default(),
                true,
                StorageFlags::optional_default_as_cow(),
                None,
                platform_version,
                None,
            )
            .expect_err(
                "expected update_document_with_serialization_for_contract with bad type to fail",
            );

        // Make sure this is not a DataContractNotFound - the contract is valid.
        assert!(
            !matches!(
                err,
                Error::Document(crate::error::document::DocumentError::DataContractNotFound)
            ),
            "expected a document-type error, got {err:?}"
        );
    }

    #[test]
    fn test_update_document_for_contract_id_v0_without_apply_dry_run_nonexistent_contract() {
        // Exercises the estimation-costs branch (apply=false) combined with a
        // nonexistent contract to verify the branch ordering: contract lookup
        // occurs before estimation path setup matters, so we still get
        // DataContractNotFound even with apply=false.
        use crate::error::document::DocumentError;

        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        let dummy_serialized = vec![0u8; 32];
        let nonexistent_contract_id: [u8; 32] = random();

        let err = drive
            .update_document_for_contract_id(
                &dummy_serialized,
                nonexistent_contract_id,
                "profile",
                None,
                BlockInfo::default(),
                false, // apply=false ⇒ estimated costs path
                StorageFlags::optional_default_as_cow(),
                None,
                platform_version,
                Some(&EPOCH_CHANGE_FEE_VERSION_TEST),
            )
            .expect_err("expected update_document_for_contract_id (dry run) to fail");

        assert!(
            matches!(err, Error::Document(DocumentError::DataContractNotFound)),
            "expected DataContractNotFound, got {err:?}"
        );
    }

    /// Regression test for the summable-index update bug at
    /// `update_document_for_contract_operations/v0`: when an index has
    /// `summable: "<prop>"` set and a document update changes ONLY the
    /// summed property (keeping every index key the same), the v0
    /// dispatcher hits the `change_occurred_on_index == false` branch
    /// and emits a `batch_refresh_reference` op.
    ///
    /// Before the fix, `batch_refresh_reference_v0` only accepted
    /// `Element::Reference` and returned `CorruptedCodeExecution` on
    /// the `Element::ReferenceWithSumItem` that the summable-aware
    /// per-index reference builder emits. The user-visible symptom
    /// was: a benign no-op `update_document_for_contract` call to
    /// rewrite the summed value would fail with a 500-equivalent
    /// server error, and the ancestor sum aggregates would never
    /// pick up the delta — silently wedging anything trying to
    /// keep a sum index in sync with a mutable document.
    ///
    /// Post-fix, `batch_refresh_reference_v0` dispatches on the
    /// element variant and emits a sum-item override
    /// `RefreshReference` op for `ReferenceWithSumItem` inputs, so
    /// ancestor sum trees propagate the delta automatically (grovedb
    /// `refresh_reference_with_sum_item_op` semantics).
    ///
    /// The test exercises the full update path end-to-end:
    ///   1. Build a v12 contract with a `byColor` index that's
    ///      `summable: "amount" + countable: "countable"` (no range
    ///      axes — this is the point-lookup SUM/AVG shape).
    ///   2. Insert a `widget` with `color="red", amount=5`.
    ///   3. Update the same document to `amount=42` (color unchanged
    ///      so every index key is identical — refresh path, not
    ///      insert-then-delete path).
    ///   4. The update MUST succeed (regression: previously failed
    ///      with `CorruptedCodeExecution`).
    #[test]
    fn summable_index_update_keeps_unchanged_keys_via_refresh_path() {
        use crate::util::object_size_info::DocumentAndContractInfo;
        use dpp::data_contract::DataContractFactory;
        use dpp::document::DocumentV0;
        use dpp::platform_value::{platform_value, Value};

        const PROTOCOL_VERSION_V12: u32 = 12;

        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        let factory =
            DataContractFactory::new(PROTOCOL_VERSION_V12).expect("expected to create factory");
        let document_schema = platform_value!({
            "type": "object",
            // `documentMutable: true` — without this the doctype is
            // immutable and `update_document_for_contract` rejects at
            // the head with `UpdatingReadOnlyImmutableDocument` (the
            // pre-refresh-path gate) before reaching the bug.
            "documentsMutable": true,
            "properties": {
                "color":  {"type": "string",  "position": 0, "maxLength": 32},
                "amount": {"type": "integer", "position": 1, "minimum": 0, "maximum": 1000},
            },
            "required": ["color", "amount"],
            "indices": [{
                "name": "byColor",
                "properties": [{"color": "asc"}],
                "summable":  "amount",
                "countable": "countable",
            }],
            "additionalProperties": false,
        });
        let schemas = platform_value!({ "widget": document_schema });
        let data_contract = factory
            .create_with_value_config(
                dpp::tests::utils::generate_random_identifier_struct(),
                0,
                schemas,
                None,
                None,
            )
            .expect("create data contract")
            .data_contract_owned();

        drive
            .apply_contract(
                &data_contract,
                BlockInfo::default(),
                true,
                StorageFlags::optional_default_as_cow(),
                None,
                platform_version,
            )
            .expect("apply contract");

        let document_type = data_contract
            .document_type_for_name("widget")
            .expect("widget type exists");

        // Insert: `color="red", amount=5`. `add_document_for_contract`
        // writes a `ReferenceWithSumItem(sum=5)` to the byColor index
        // and the doctype-level primary-key sum tree picks up +5.
        let doc_id = Identifier::from([7u8; 32]);
        let mut properties_initial = BTreeMap::new();
        properties_initial.insert("color".to_string(), Value::Text("red".to_string()));
        properties_initial.insert("amount".to_string(), Value::U64(5));
        let document_initial: dpp::document::Document = DocumentV0 {
            id: doc_id,
            owner_id: Identifier::from([0u8; 32]),
            properties: properties_initial,
            revision: Some(1),
            created_at: None,
            updated_at: None,
            transferred_at: None,
            created_at_block_height: None,
            updated_at_block_height: None,
            transferred_at_block_height: None,
            created_at_core_block_height: None,
            updated_at_core_block_height: None,
            transferred_at_core_block_height: None,
            creator_id: None,
        }
        .into();
        let storage_flags = Some(Cow::Owned(StorageFlags::SingleEpoch(0)));

        drive
            .add_document_for_contract(
                DocumentAndContractInfo {
                    owned_document_info: OwnedDocumentInfo {
                        document_info: DocumentRefInfo((&document_initial, storage_flags.clone())),
                        owner_id: None,
                    },
                    contract: &data_contract,
                    document_type,
                },
                false,
                BlockInfo::default(),
                true,
                None,
                platform_version,
                None,
            )
            .expect("insert widget");

        // Update: keep `color="red"` (index key unchanged, hits the
        // no-key-change refresh branch) but change `amount` 5 → 42.
        // This is the exact shape that previously errored:
        //   - `change_occurred_on_index == false`
        //   - `index.summable.is_some()` → builds `ReferenceWithSumItem`
        //   - `batch_refresh_reference_v0` rejects with
        //     `CorruptedCodeExecution` pre-fix.
        let mut properties_updated = BTreeMap::new();
        properties_updated.insert("color".to_string(), Value::Text("red".to_string()));
        properties_updated.insert("amount".to_string(), Value::U64(42));
        let document_updated: dpp::document::Document = DocumentV0 {
            id: doc_id,
            owner_id: Identifier::from([0u8; 32]),
            properties: properties_updated,
            revision: Some(2),
            created_at: None,
            updated_at: None,
            transferred_at: None,
            created_at_block_height: None,
            updated_at_block_height: None,
            transferred_at_block_height: None,
            created_at_core_block_height: None,
            updated_at_core_block_height: None,
            transferred_at_core_block_height: None,
            creator_id: None,
        }
        .into();

        drive
            .update_document_for_contract(
                &document_updated,
                &data_contract,
                document_type,
                None,
                BlockInfo::default(),
                true,
                storage_flags,
                None,
                platform_version,
                None,
            )
            .expect(
                "summable-index update with unchanged keys must succeed; pre-fix this returned \
                 CorruptedCodeExecution from batch_refresh_reference_v0 because the helper only \
                 accepted Element::Reference and the summable index builds an \
                 Element::ReferenceWithSumItem",
            );

        // Verify the byColor index aggregate picked up the delta:
        // pre-update SUM where color="red" should equal 5 (initial),
        // post-update SUM should equal 42 (rewritten via the
        // refresh-with-sum-item op the helper now emits). Anything
        // other than 42 here means the refresh op didn't carry the
        // new sum_value through to ancestor sum trees, which is the
        // *second* half of the regression: even if the call returned
        // Ok, the aggregate had to actually update.
        use crate::config::DriveConfig;
        use crate::query::drive_document_sum_query::{
            DocumentSumRequest, DocumentSumResponse, SumMode,
        };
        use crate::query::{WhereClause, WhereOperator};
        let drive_config = DriveConfig::default();
        let color_eq_red = WhereClause {
            field: "color".to_string(),
            operator: WhereOperator::Equal,
            value: Value::Text("red".to_string()),
        };
        let sum_request = DocumentSumRequest {
            contract: &data_contract,
            document_type,
            sum_property: "amount".to_string(),
            where_clauses: vec![color_eq_red],
            order_clauses: Vec::new(),
            mode: SumMode::Aggregate,
            limit: None,
            prove: false,
            drive_config: &drive_config,
        };
        let sum_response = drive
            .execute_document_sum_request(sum_request, None, platform_version)
            .expect("point-lookup SUM no-proof on byColor where color=red");
        // `SumMode::Aggregate` with a no-range Equal `where` resolves
        // to the point-lookup arm and collapses the per-key entries
        // into a single `Aggregate(i64)` response (the no-proof side
        // mirrors the prove side's verifier-folded shape). Anything
        // other than 42 means the refresh op didn't propagate the
        // new sum_value into the ancestor sum tree.
        match sum_response {
            DocumentSumResponse::Aggregate(total) => assert_eq!(
                total, 42,
                "expected the byColor `color=red` aggregate to reflect the updated \
                 `amount=42` after the in-place refresh; got {total}. A mismatch here means \
                 batch_refresh_reference_v0 emitted a refresh op that didn't propagate the \
                 new sum_value into the ancestor sum tree."
            ),
            DocumentSumResponse::Entries(entries) => {
                let total: i64 = entries.iter().filter_map(|e| e.sum).sum();
                assert_eq!(
                    total, 42,
                    "expected the byColor `color=red` aggregate to reflect the updated \
                     `amount=42` after the in-place refresh; got {total} from Entries shape"
                );
            }
            other => panic!(
                "expected Aggregate or Entries response from point-lookup SUM, got {other:?}"
            ),
        }
    }

    /// Regression test for the key-changing-update tree-type bug in
    /// `update_document_for_contract_operations/v0` — the
    /// inconsistency the reviewer caught at the four
    /// `batch_insert_empty_tree_if_not_exists` call sites that were
    /// hardcoded to `TreeType::NormalTree`.
    ///
    /// Pre-fix behavior: when an update moved a document into a
    /// previously-unseen branch under an aggregate (summable +
    /// countable) index, the update path materialized the new
    /// top-level value tree (and any inner branches) as
    /// `NormalTree`. The insert path, by contrast, would have
    /// materialized those same branches as
    /// `CountSumTree` / `ProvableCount*Tree` etc. via the v1
    /// dispatch in
    /// `add_indices_for_index_level_for_contract_operations_v1`.
    /// Two nodes whose pre-state had a doc in branch X and then
    /// inserted (or updated-into) branch Y would commit to different
    /// merk roots — consensus break.
    ///
    /// Setup: a v12 contract with a `byColor` index that's
    /// `summable: "amount" + countable: "countable"` (no range
    /// axes). Insert a `widget` at `color="red"` (creates the
    /// "red" branch's value tree as `CountSumTree` via the
    /// insert path), then update the same widget to
    /// `color="blue"` (moves into a previously-unseen "blue"
    /// branch — which the update path materializes via the
    /// fixed dispatch). Finally, SUM the index under `color="blue"`
    /// and assert the aggregate equals the doc's amount: a
    /// mismatch means either the new branch landed as the wrong
    /// `TreeType` (no count/sum carrier) OR the reference under
    /// it didn't propagate the sum_value through the right
    /// ancestor tree type.
    ///
    /// Companion to
    /// `summable_index_update_keeps_unchanged_keys_via_refresh_path`
    /// above, which covers the no-key-change refresh arm.
    #[test]
    fn summable_index_update_changes_key_into_new_branch_materializes_aggregate_tree_type() {
        use crate::config::DriveConfig;
        use crate::query::drive_document_sum_query::{
            DocumentSumRequest, DocumentSumResponse, SumMode,
        };
        use crate::query::{WhereClause, WhereOperator};
        use crate::util::object_size_info::DocumentAndContractInfo;
        use dpp::data_contract::DataContractFactory;
        use dpp::document::DocumentV0;
        use dpp::platform_value::{platform_value, Value};

        const PROTOCOL_VERSION_V12: u32 = 12;

        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        let factory =
            DataContractFactory::new(PROTOCOL_VERSION_V12).expect("expected to create factory");
        let document_schema = platform_value!({
            "type": "object",
            "documentsMutable": true,
            "properties": {
                "color":  {"type": "string",  "position": 0, "maxLength": 32},
                "amount": {"type": "integer", "position": 1, "minimum": 0, "maximum": 1000},
            },
            "required": ["color", "amount"],
            "indices": [{
                "name": "byColor",
                "properties": [{"color": "asc"}],
                "summable":  "amount",
                "countable": "countable",
            }],
            "additionalProperties": false,
        });
        let schemas = platform_value!({ "widget": document_schema });
        let data_contract = factory
            .create_with_value_config(
                dpp::tests::utils::generate_random_identifier_struct(),
                0,
                schemas,
                None,
                None,
            )
            .expect("create data contract")
            .data_contract_owned();

        drive
            .apply_contract(
                &data_contract,
                BlockInfo::default(),
                true,
                StorageFlags::optional_default_as_cow(),
                None,
                platform_version,
            )
            .expect("apply contract");

        let document_type = data_contract
            .document_type_for_name("widget")
            .expect("widget type exists");

        // Insert with color="red", amount=11. The insert path
        // creates the "red" value tree as CountSumTree.
        let doc_id = Identifier::from([9u8; 32]);
        let mut properties_initial = BTreeMap::new();
        properties_initial.insert("color".to_string(), Value::Text("red".to_string()));
        properties_initial.insert("amount".to_string(), Value::U64(11));
        let document_initial: dpp::document::Document = DocumentV0 {
            id: doc_id,
            owner_id: Identifier::from([0u8; 32]),
            properties: properties_initial,
            revision: Some(1),
            created_at: None,
            updated_at: None,
            transferred_at: None,
            created_at_block_height: None,
            updated_at_block_height: None,
            transferred_at_block_height: None,
            created_at_core_block_height: None,
            updated_at_core_block_height: None,
            transferred_at_core_block_height: None,
            creator_id: None,
        }
        .into();
        let storage_flags = Some(Cow::Owned(StorageFlags::SingleEpoch(0)));
        drive
            .add_document_for_contract(
                DocumentAndContractInfo {
                    owned_document_info: OwnedDocumentInfo {
                        document_info: DocumentRefInfo((&document_initial, storage_flags.clone())),
                        owner_id: None,
                    },
                    contract: &data_contract,
                    document_type,
                },
                false,
                BlockInfo::default(),
                true,
                None,
                platform_version,
                None,
            )
            .expect("insert widget");

        // Update: change color "red" → "blue", amount 11 → 17.
        // The "blue" branch doesn't exist yet — the update path
        // has to create it. Pre-fix, that branch landed as
        // NormalTree (no aggregate carrier); post-fix, it lands
        // as CountSumTree, matching the insert path.
        let mut properties_updated = BTreeMap::new();
        properties_updated.insert("color".to_string(), Value::Text("blue".to_string()));
        properties_updated.insert("amount".to_string(), Value::U64(17));
        let document_updated: dpp::document::Document = DocumentV0 {
            id: doc_id,
            owner_id: Identifier::from([0u8; 32]),
            properties: properties_updated,
            revision: Some(2),
            created_at: None,
            updated_at: None,
            transferred_at: None,
            created_at_block_height: None,
            updated_at_block_height: None,
            transferred_at_block_height: None,
            created_at_core_block_height: None,
            updated_at_core_block_height: None,
            transferred_at_core_block_height: None,
            creator_id: None,
        }
        .into();
        drive
            .update_document_for_contract(
                &document_updated,
                &data_contract,
                document_type,
                None,
                BlockInfo::default(),
                true,
                storage_flags,
                None,
                platform_version,
                None,
            )
            .expect("key-changing update on aggregate index must succeed");

        // Walk: SUM where color="blue" — drives the point-lookup
        // SUM arm under the byColor index. The "blue" value tree
        // is the one materialized by the UPDATE path. If it landed
        // as NormalTree (pre-fix), no sum_value carrier exists at
        // the parent; the SUM either errors or returns 0. Post-fix
        // it's CountSumTree and the aggregate equals 17.
        let drive_config = DriveConfig::default();
        let color_eq_blue = WhereClause {
            field: "color".to_string(),
            operator: WhereOperator::Equal,
            value: Value::Text("blue".to_string()),
        };
        let sum_request = DocumentSumRequest {
            contract: &data_contract,
            document_type,
            sum_property: "amount".to_string(),
            where_clauses: vec![color_eq_blue],
            order_clauses: Vec::new(),
            mode: SumMode::Aggregate,
            limit: None,
            prove: false,
            drive_config: &drive_config,
        };
        let sum_response = drive
            .execute_document_sum_request(sum_request, None, platform_version)
            .expect(
                "point-lookup SUM on update-materialized branch must succeed; pre-fix the \
                 branch landed as NormalTree and the dispatcher would fail (or silently \
                 return 0) because there's no count+sum aggregate at the parent",
            );
        match sum_response {
            DocumentSumResponse::Aggregate(total) => assert_eq!(
                total, 17,
                "SUM(amount) where color=blue must equal 17 — the updated value. Got {total}; \
                 pre-fix the update path created the 'blue' branch as NormalTree, dropping \
                 the per-doc sum contribution at the value-tree's parent."
            ),
            DocumentSumResponse::Entries(entries) => {
                let total: i64 = entries.iter().filter_map(|e| e.sum).sum();
                assert_eq!(total, 17, "expected 17, got {total} via Entries shape");
            }
            other => panic!("expected Aggregate or Entries, got {other:?}"),
        }
    }
}
