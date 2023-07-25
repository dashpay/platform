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
#[cfg(feature = "full")]
pub use add_update_multiple_documents_operations::*;

// Module: update_document_for_contract
// This module contains functionality for updating a document for a given contract
#[cfg(feature = "full")]
mod update_document_for_contract;
#[cfg(feature = "full")]
pub use update_document_for_contract::*;

// Module: update_document_for_contract_id
// This module contains functionality for updating a document associated with a given contract id
#[cfg(feature = "full")]
mod update_document_for_contract_id;
#[cfg(feature = "full")]
pub use update_document_for_contract_id::*;

// Module: update_document_with_serialization_for_contract
// This module contains functionality for updating a document (with serialization) for a contract
mod internal;
#[cfg(feature = "fixtures-and-mocks")]
mod test_helpers;
mod update_document_with_serialization_for_contract;

pub use update_document_with_serialization_for_contract::*;

use std::borrow::Cow;
use std::collections::{HashMap, HashSet};

use dpp::data_contract::document_type::DocumentTypeRef;

use grovedb::batch::key_info::KeyInfo;
use grovedb::batch::key_info::KeyInfo::KnownKey;
use grovedb::batch::KeyInfoPath;
use grovedb::{Element, EstimatedLayerInformation, TransactionArg};

use crate::drive::batch::drive_op_batch::{
    DocumentOperation, DocumentOperationsForContractDocumentType, UpdateOperationInfo,
};
use crate::drive::batch::{DocumentOperationType, DriveOperation};
use crate::drive::defaults::CONTRACT_DOCUMENTS_PATH_HEIGHT;
use crate::drive::document::{
    contract_document_type_path,
    contract_documents_keeping_history_primary_key_path_for_document_id,
    contract_documents_primary_key_path, make_document_reference,
};
use crate::drive::flags::StorageFlags;
use crate::drive::object_size_info::DocumentInfo::{
    DocumentOwnedInfo, DocumentRefAndSerialization, DocumentRefInfo,
};
use dpp::data_contract::DataContract;
use dpp::document::Document;

use crate::drive::object_size_info::PathKeyElementInfo::PathKeyRefElement;
use crate::drive::object_size_info::{
    DocumentAndContractInfo, DriveKeyInfo, OwnedDocumentInfo, PathKeyInfo,
};
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fee::op::LowLevelDriveOperation;

use crate::drive::object_size_info::DriveKeyInfo::{Key, KeyRef, KeySize};
use crate::error::document::DocumentError;
use dpp::block::block_info::BlockInfo;
use dpp::data_contract::base::DataContractBaseMethodsV0;
use dpp::fee::fee_result::FeeResult;
use dpp::version::PlatformVersion;

use crate::drive::grove_operations::{
    BatchDeleteUpTreeApplyType, BatchInsertApplyType, BatchInsertTreeApplyType, DirectQueryType,
    QueryType,
};

#[cfg(feature = "full")]
#[cfg(test)]
mod tests {
    use grovedb::TransactionArg;
    use std::default::Default;
    use std::option::Option::None;
    use std::sync::Arc;

    use dpp::data_contract::DataContractFactory;
    use dpp::document::document_factory::DocumentFactory;

    use dpp::platform_value::{platform_value, Identifier, Value};

    use dpp::block::block_info::BlockInfo;

    use dpp::balances::credits::Creditable;
    use rand::Rng;
    use serde::{Deserialize, Serialize};
    use serde_json::json;
    use tempfile::TempDir;

    use super::*;
    use crate::drive::config::DriveConfig;
    use crate::drive::flags::StorageFlags;
    use crate::drive::object_size_info::DocumentAndContractInfo;
    use crate::drive::object_size_info::DocumentInfo::DocumentRefInfo;
    use crate::drive::{defaults, Drive};

    use crate::query::DriveQuery;
    use crate::{common::setup_contract, drive::test_utils::TestEntropyGenerator};
    use dpp::block::epoch::Epoch;
    use dpp::data_contract::conversion::cbor_conversion::DataContractCborConversionMethodsV0;
    use dpp::data_contract::extra::common::json_document_to_document;
    use dpp::document::serialization_traits::{
        DocumentCborMethodsV0, DocumentPlatformConversionMethodsV0, DocumentPlatformValueMethodsV0,
    };
    use dpp::document::{DocumentV0Getters, DocumentV0Setters};
    use dpp::fee::default_costs::EpochCosts;
    use dpp::fee::default_costs::KnownCostItem::StorageDiskUsageCreditPerByte;
    use dpp::platform_value;
    use dpp::serialization::serialization_traits::PlatformSerializable;
    use dpp::util::cbor_serializer;
    use dpp::version::drive_versions::DriveVersion;

    #[test]
    fn test_create_and_update_document_same_transaction() {
        let tmp_dir = TempDir::new().unwrap();
        let drive: Drive = Drive::open(tmp_dir, None).expect("expected to open Drive successfully");

        let db_transaction = drive.grove.start_transaction();
        let platform_version = PlatformVersion::latest();
        drive
            .create_initial_state_structure(Some(&db_transaction), &platform_version)
            .expect("expected to create root tree successfully");

        let contract_cbor = hex::decode("01a5632469645820b0248cd9a27f86d05badf475dd9ff574d63219cd60c52e2be1e540c2fdd713336724736368656d61783468747470733a2f2f736368656d612e646173682e6f72672f6470702d302d342d302f6d6574612f646174612d636f6e7472616374676f776e6572496458204c9bf0db6ae315c85465e9ef26e6a006de9673731d08d14881945ddef1b5c5f26776657273696f6e0169646f63756d656e7473a267636f6e74616374a56474797065666f626a65637467696e646963657381a3646e616d656f6f6e7765724964546f55736572496466756e69717565f56a70726f7065727469657382a168246f776e6572496463617363a168746f557365724964636173636872657175697265648268746f557365724964697075626c69634b65796a70726f70657274696573a268746f557365724964a56474797065656172726179686d61784974656d731820686d696e4974656d73182069627974654172726179f570636f6e74656e744d656469615479706578216170706c69636174696f6e2f782e646173682e6470702e6964656e746966696572697075626c69634b6579a36474797065656172726179686d61784974656d73182169627974654172726179f5746164646974696f6e616c50726f70657274696573f46770726f66696c65a56474797065666f626a65637467696e646963657381a3646e616d65676f776e6572496466756e69717565f56a70726f7065727469657381a168246f776e6572496463617363687265717569726564826961766174617255726c6561626f75746a70726f70657274696573a26561626f7574a2647479706566737472696e67696d61784c656e67746818ff6961766174617255726ca3647479706566737472696e6766666f726d61746375726c696d61784c656e67746818ff746164646974696f6e616c50726f70657274696573f4").unwrap();

        drive
            .apply_contract_cbor(
                contract_cbor.clone(),
                None,
                BlockInfo::default(),
                true,
                StorageFlags::optional_default_as_cow(),
                Some(&db_transaction),
                platform_version,
            )
            .expect("expected to apply contract successfully");

        // Create Alice profile

        let alice_profile_cbor = hex::decode("01a763246964582035edfec54aea574df968990abb47b39c206abe5c43a6157885f62958a1f1230c6524747970656770726f66696c656561626f75746a4920616d20416c69636568246f776e65724964582041d52f93f6f7c5af79ce994381c90df73cce2863d3850b9c05ef586ff0fe795f69247265766973696f6e016961766174617255726c7819687474703a2f2f746573742e636f6d2f616c6963652e6a70676f2464617461436f6e747261637449645820b0248cd9a27f86d05badf475dd9ff574d63219cd60c52e2be1e540c2fdd71333").unwrap();

        drive
            .add_cbor_serialized_document_for_serialized_contract(
                alice_profile_cbor.as_slice(),
                contract_cbor.as_slice(),
                "profile",
                None,
                true,
                BlockInfo::default(),
                true,
                StorageFlags::optional_default_as_cow(),
                Some(&db_transaction),
                platform_version,
            )
            .expect("should create alice profile");

        // Update Alice profile

        let updated_alice_profile_cbor = hex::decode("01a763246964582035edfec54aea574df968990abb47b39c206abe5c43a6157885f62958a1f1230c6524747970656770726f66696c656561626f75746a4920616d20416c69636568246f776e65724964582041d52f93f6f7c5af79ce994381c90df73cce2863d3850b9c05ef586ff0fe795f69247265766973696f6e026961766174617255726c781a687474703a2f2f746573742e636f6d2f616c696365322e6a70676f2464617461436f6e747261637449645820b0248cd9a27f86d05badf475dd9ff574d63219cd60c52e2be1e540c2fdd71333").unwrap();

        drive
            .update_document_for_contract_cbor(
                updated_alice_profile_cbor.as_slice(),
                contract_cbor.as_slice(),
                "profile",
                None,
                BlockInfo::default(),
                true,
                StorageFlags::optional_default_as_cow(),
                Some(&db_transaction),
                platform_version,
            )
            .expect("should update alice profile");
    }

    #[test]
    fn test_create_and_update_document_no_transactions() {
        let tmp_dir = TempDir::new().unwrap();
        let drive: Drive = Drive::open(tmp_dir, None).expect("expected to open Drive successfully");

        let platform_version = PlatformVersion::latest();
        let db_transaction = drive.grove.start_transaction();
        drive
            .create_initial_state_structure(Some(&db_transaction), &platform_version)
            .expect("expected to create root tree successfully");

        let contract_cbor = hex::decode("01a5632469645820b0248cd9a27f86d05badf475dd9ff574d63219cd60c52e2be1e540c2fdd713336724736368656d61783468747470733a2f2f736368656d612e646173682e6f72672f6470702d302d342d302f6d6574612f646174612d636f6e7472616374676f776e6572496458204c9bf0db6ae315c85465e9ef26e6a006de9673731d08d14881945ddef1b5c5f26776657273696f6e0169646f63756d656e7473a267636f6e74616374a56474797065666f626a65637467696e646963657381a3646e616d656f6f6e7765724964546f55736572496466756e69717565f56a70726f7065727469657382a168246f776e6572496463617363a168746f557365724964636173636872657175697265648268746f557365724964697075626c69634b65796a70726f70657274696573a268746f557365724964a56474797065656172726179686d61784974656d731820686d696e4974656d73182069627974654172726179f570636f6e74656e744d656469615479706578216170706c69636174696f6e2f782e646173682e6470702e6964656e746966696572697075626c69634b6579a36474797065656172726179686d61784974656d73182169627974654172726179f5746164646974696f6e616c50726f70657274696573f46770726f66696c65a56474797065666f626a65637467696e646963657381a3646e616d65676f776e6572496466756e69717565f56a70726f7065727469657381a168246f776e6572496463617363687265717569726564826961766174617255726c6561626f75746a70726f70657274696573a26561626f7574a2647479706566737472696e67696d61784c656e67746818ff6961766174617255726ca3647479706566737472696e6766666f726d61746375726c696d61784c656e67746818ff746164646974696f6e616c50726f70657274696573f4").unwrap();

        let contract = DataContract::from_cbor(contract_cbor.as_slice(), platform_version)
            .expect("expected to create contract");
        drive
            .apply_contract_cbor(
                contract_cbor.clone(),
                None,
                BlockInfo::default(),
                true,
                StorageFlags::optional_default_as_cow(),
                None,
                platform_version,
            )
            .expect("expected to apply contract successfully");

        // Create Alice profile

        let alice_profile_cbor = hex::decode("01a763246964582035edfec54aea574df968990abb47b39c206abe5c43a6157885f62958a1f1230c6524747970656770726f66696c656561626f75746a4920616d20416c69636568246f776e65724964582041d52f93f6f7c5af79ce994381c90df73cce2863d3850b9c05ef586ff0fe795f69247265766973696f6e016961766174617255726c7819687474703a2f2f746573742e636f6d2f616c6963652e6a70676f2464617461436f6e747261637449645820b0248cd9a27f86d05badf475dd9ff574d63219cd60c52e2be1e540c2fdd71333").unwrap();

        let alice_profile =
            Document::from_cbor(alice_profile_cbor.as_slice(), None, None, platform_version)
                .expect("expected to get a document");

        let document_type = contract
            .document_type_for_name("profile")
            .expect("expected to get a document type");

        let storage_flags = Some(Cow::Owned(StorageFlags::SingleEpoch(0)));

        drive
            .add_document_for_contract(
                DocumentAndContractInfo {
                    owned_document_info: OwnedDocumentInfo {
                        document_info: DocumentRefInfo((&alice_profile, storage_flags)),
                        owner_id: None,
                    },
                    contract: &contract,
                    document_type,
                },
                true,
                BlockInfo::default(),
                true,
                None,
                &platform_version,
            )
            .expect("should create alice profile");

        let sql_string = "select * from profile";
        let query = DriveQuery::from_sql_expr(sql_string, &contract, &DriveConfig::default())
            .expect("should build query");

        let (results_no_transaction, _, _) = query
            .execute_raw_results_no_proof(&drive, None, None, platform_version)
            .expect("expected to execute query");

        assert_eq!(results_no_transaction.len(), 1);

        // Update Alice profile

        let updated_alice_profile_cbor = hex::decode("01a763246964582035edfec54aea574df968990abb47b39c206abe5c43a6157885f62958a1f1230c6524747970656770726f66696c656561626f75746a4920616d20416c69636568246f776e65724964582041d52f93f6f7c5af79ce994381c90df73cce2863d3850b9c05ef586ff0fe795f69247265766973696f6e026961766174617255726c781a687474703a2f2f746573742e636f6d2f616c696365322e6a70676f2464617461436f6e747261637449645820b0248cd9a27f86d05badf475dd9ff574d63219cd60c52e2be1e540c2fdd71333").unwrap();

        drive
            .update_document_for_contract_cbor(
                updated_alice_profile_cbor.as_slice(),
                contract_cbor.as_slice(),
                "profile",
                None,
                BlockInfo::default(),
                true,
                StorageFlags::optional_default_as_cow(),
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
        let tmp_dir = TempDir::new().unwrap();
        let drive: Drive = Drive::open(tmp_dir, None).expect("expected to open Drive successfully");

        let db_transaction = drive.grove.start_transaction();

        let platform_version = PlatformVersion::latest();
        drive
            .create_initial_state_structure(Some(&db_transaction), &platform_version)
            .expect("expected to create root tree successfully");

        let contract_cbor = hex::decode("01a5632469645820b0248cd9a27f86d05badf475dd9ff574d63219cd60c52e2be1e540c2fdd713336724736368656d61783468747470733a2f2f736368656d612e646173682e6f72672f6470702d302d342d302f6d6574612f646174612d636f6e7472616374676f776e6572496458204c9bf0db6ae315c85465e9ef26e6a006de9673731d08d14881945ddef1b5c5f26776657273696f6e0169646f63756d656e7473a267636f6e74616374a56474797065666f626a65637467696e646963657381a3646e616d656f6f6e7765724964546f55736572496466756e69717565f56a70726f7065727469657382a168246f776e6572496463617363a168746f557365724964636173636872657175697265648268746f557365724964697075626c69634b65796a70726f70657274696573a268746f557365724964a56474797065656172726179686d61784974656d731820686d696e4974656d73182069627974654172726179f570636f6e74656e744d656469615479706578216170706c69636174696f6e2f782e646173682e6470702e6964656e746966696572697075626c69634b6579a36474797065656172726179686d61784974656d73182169627974654172726179f5746164646974696f6e616c50726f70657274696573f46770726f66696c65a56474797065666f626a65637467696e646963657381a3646e616d65676f776e6572496466756e69717565f56a70726f7065727469657381a168246f776e6572496463617363687265717569726564826961766174617255726c6561626f75746a70726f70657274696573a26561626f7574a2647479706566737472696e67696d61784c656e67746818ff6961766174617255726ca3647479706566737472696e6766666f726d61746375726c696d61784c656e67746818ff746164646974696f6e616c50726f70657274696573f4").unwrap();

        let contract = DataContract::from_cbor(contract_cbor.as_slice(), platform_version)
            .expect("expected to create contract");
        drive
            .apply_contract_cbor(
                contract_cbor.clone(),
                None,
                BlockInfo::default(),
                true,
                StorageFlags::optional_default_as_cow(),
                Some(&db_transaction),
                platform_version,
            )
            .expect("expected to apply contract successfully");

        // Create Alice profile

        let alice_profile_cbor = hex::decode("01a763246964582035edfec54aea574df968990abb47b39c206abe5c43a6157885f62958a1f1230c6524747970656770726f66696c656561626f75746a4920616d20416c69636568246f776e65724964582041d52f93f6f7c5af79ce994381c90df73cce2863d3850b9c05ef586ff0fe795f69247265766973696f6e016961766174617255726c7819687474703a2f2f746573742e636f6d2f616c6963652e6a70676f2464617461436f6e747261637449645820b0248cd9a27f86d05badf475dd9ff574d63219cd60c52e2be1e540c2fdd71333").unwrap();

        let alice_profile =
            Document::from_cbor(alice_profile_cbor.as_slice(), None, None, platform_version)
                .expect("expected to get a document");

        let document_type = contract
            .document_type_for_name("profile")
            .expect("expected to get a document type");

        let storage_flags = Some(Cow::Owned(StorageFlags::SingleEpoch(0)));

        drive
            .add_document_for_contract(
                DocumentAndContractInfo {
                    owned_document_info: OwnedDocumentInfo {
                        document_info: DocumentRefInfo((&alice_profile, storage_flags)),
                        owner_id: None,
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
            .expect("should create alice profile");

        drive
            .grove
            .commit_transaction(db_transaction)
            .unwrap()
            .expect("should commit transaction");

        let sql_string = "select * from profile";
        let query = DriveQuery::from_sql_expr(sql_string, &contract, &DriveConfig::default())
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

        // Update Alice profile

        let updated_alice_profile_cbor = hex::decode("01a763246964582035edfec54aea574df968990abb47b39c206abe5c43a6157885f62958a1f1230c6524747970656770726f66696c656561626f75746a4920616d20416c69636568246f776e65724964582041d52f93f6f7c5af79ce994381c90df73cce2863d3850b9c05ef586ff0fe795f69247265766973696f6e026961766174617255726c781a687474703a2f2f746573742e636f6d2f616c696365322e6a70676f2464617461436f6e747261637449645820b0248cd9a27f86d05badf475dd9ff574d63219cd60c52e2be1e540c2fdd71333").unwrap();

        drive
            .update_document_for_contract_cbor(
                updated_alice_profile_cbor.as_slice(),
                contract_cbor.as_slice(),
                "profile",
                None,
                BlockInfo::default(),
                true,
                StorageFlags::optional_default_as_cow(),
                Some(&db_transaction),
                platform_version,
            )
            .expect("should update alice profile");

        let (results_on_transaction, _, _) = query
            .execute_raw_results_no_proof(&drive, None, Some(&db_transaction), platform_version)
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

        let platform_version = PlatformVersion::latest();
        drive
            .create_initial_state_structure(Some(&db_transaction), &platform_version)
            .expect("expected to create root tree successfully");

        let contract_cbor = hex::decode("01a5632469645820b0248cd9a27f86d05badf475dd9ff574d63219cd60c52e2be1e540c2fdd713336724736368656d61783468747470733a2f2f736368656d612e646173682e6f72672f6470702d302d342d302f6d6574612f646174612d636f6e7472616374676f776e6572496458204c9bf0db6ae315c85465e9ef26e6a006de9673731d08d14881945ddef1b5c5f26776657273696f6e0169646f63756d656e7473a267636f6e74616374a56474797065666f626a65637467696e646963657381a3646e616d656f6f6e7765724964546f55736572496466756e69717565f56a70726f7065727469657382a168246f776e6572496463617363a168746f557365724964636173636872657175697265648268746f557365724964697075626c69634b65796a70726f70657274696573a268746f557365724964a56474797065656172726179686d61784974656d731820686d696e4974656d73182069627974654172726179f570636f6e74656e744d656469615479706578216170706c69636174696f6e2f782e646173682e6470702e6964656e746966696572697075626c69634b6579a36474797065656172726179686d61784974656d73182169627974654172726179f5746164646974696f6e616c50726f70657274696573f46770726f66696c65a56474797065666f626a65637467696e646963657381a3646e616d65676f776e6572496466756e69717565f56a70726f7065727469657381a168246f776e6572496463617363687265717569726564826961766174617255726c6561626f75746a70726f70657274696573a26561626f7574a2647479706566737472696e67696d61784c656e67746818ff6961766174617255726ca3647479706566737472696e6766666f726d61746375726c696d61784c656e67746818ff746164646974696f6e616c50726f70657274696573f4").unwrap();

        let contract = DataContract::from_cbor(contract_cbor.as_slice(), platform_version)
            .expect("expected to create contract");
        drive
            .apply_contract_cbor(
                contract_cbor.clone(),
                None,
                BlockInfo::default(),
                true,
                StorageFlags::optional_default_as_cow(),
                Some(&db_transaction),
                platform_version,
            )
            .expect("expected to apply contract successfully");

        // Create Alice profile

        let alice_profile_cbor = hex::decode("01a763246964582035edfec54aea574df968990abb47b39c206abe5c43a6157885f62958a1f1230c6524747970656770726f66696c656561626f75746a4920616d20416c69636568246f776e65724964582041d52f93f6f7c5af79ce994381c90df73cce2863d3850b9c05ef586ff0fe795f69247265766973696f6e016961766174617255726c7819687474703a2f2f746573742e636f6d2f616c6963652e6a70676f2464617461436f6e747261637449645820b0248cd9a27f86d05badf475dd9ff574d63219cd60c52e2be1e540c2fdd71333").unwrap();

        let alice_profile =
            Document::from_cbor(alice_profile_cbor.as_slice(), None, None, platform_version)
                .expect("expected to get a document");

        let document_type = contract
            .document_type_for_name("profile")
            .expect("expected to get a document type");

        let storage_flags = Some(Cow::Owned(StorageFlags::SingleEpoch(0)));

        drive
            .add_document_for_contract(
                DocumentAndContractInfo {
                    owned_document_info: OwnedDocumentInfo {
                        document_info: DocumentRefInfo((&alice_profile, storage_flags)),
                        owner_id: None,
                    },
                    contract: &contract,
                    document_type,
                },
                true,
                BlockInfo::default(),
                true,
                Some(&db_transaction),
                &platform_version,
            )
            .expect("should create alice profile");

        drive
            .grove
            .commit_transaction(db_transaction)
            .unwrap()
            .expect("should commit transaction");

        let sql_string = "select * from profile";
        let query = DriveQuery::from_sql_expr(sql_string, &contract, &DriveConfig::default())
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

        drive
            .delete_document_for_contract(
                alice_profile.id().to_buffer(),
                &contract,
                "profile",
                BlockInfo::default(),
                true,
                Some(&db_transaction),
                &platform_version,
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

        let updated_alice_profile_cbor = hex::decode("01a763246964582035edfec54aea574df968990abb47b39c206abe5c43a6157885f62958a1f1230c6524747970656770726f66696c656561626f75746a4920616d20416c69636568246f776e65724964582041d52f93f6f7c5af79ce994381c90df73cce2863d3850b9c05ef586ff0fe795f69247265766973696f6e026961766174617255726c781a687474703a2f2f746573742e636f6d2f616c696365322e6a70676f2464617461436f6e747261637449645820b0248cd9a27f86d05badf475dd9ff574d63219cd60c52e2be1e540c2fdd71333").unwrap();

        drive
            .update_document_for_contract_cbor(
                updated_alice_profile_cbor.as_slice(),
                contract_cbor.as_slice(),
                "profile",
                None,
                BlockInfo::default(),
                true,
                StorageFlags::optional_default_as_cow(),
                Some(&db_transaction),
                platform_version,
            )
            .expect("should update alice profile");
    }

    #[test]
    fn test_create_update_and_delete_document() {
        let tmp_dir = TempDir::new().unwrap();
        let drive: Drive = Drive::open(tmp_dir, None).expect("expected to open Drive successfully");
        let db_transaction = drive.grove.start_transaction();
        let platform_version = PlatformVersion::latest();
        drive
            .create_initial_state_structure(Some(&db_transaction), &platform_version)
            .expect("should create root tree");

        let contract = platform_value!({
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

        // first we need to deserialize the contract
        let contract =
            DataContract::from_object(contract, platform_version).expect("expected data contract");

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
           "$id": "DLRWw2eRbLAW5zDU2c7wwsSFQypTSZPhFYzpY48tnaXN",
           "$type": "indexedDocument",
           "$dataContractId": "BZUodcFoFL6KvnonehrnMVggTvCe8W5MiRnZuqLb6M54",
           "$ownerId": "GZVdTnLFAN2yE9rLeCHBDBCr7YQgmXJuoExkY347j7Z5",
           "$revision": 1,
           "firstName": "myName",
           "lastName": "lastName",
           "$createdAt":1647535750329_u64,
           "$updatedAt":1647535750329_u64,
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
           "$id": "DLRWw2eRbLAW5zDU2c7wwsSFQypTSZPhFYzpY48tnaXN",
           "$type": "indexedDocument",
           "$dataContractId": "BZUodcFoFL6KvnonehrnMVggTvCe8W5MiRnZuqLb6M54",
           "$ownerId": "GZVdTnLFAN2yE9rLeCHBDBCr7YQgmXJuoExkY347j7Z5",
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
                &platform_version,
            )
            .expect("should delete document");
    }

    #[test]
    fn test_modify_dashpay_contact_request() {
        let tmp_dir = TempDir::new().unwrap();
        let drive: Drive = Drive::open(tmp_dir, None).expect("expected to open Drive successfully");

        let db_transaction = drive.grove.start_transaction();

        let platform_version = PlatformVersion::latest();
        drive
            .create_initial_state_structure(Some(&db_transaction), &platform_version)
            .expect("expected to create root tree successfully");

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
                &platform_version,
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
                &platform_version,
            )
            .expect_err("expected not to be able to override a non mutable document");
    }

    #[test]
    fn test_update_dashpay_profile_with_history() {
        let tmp_dir = TempDir::new().unwrap();
        let drive: Drive = Drive::open(tmp_dir, None).expect("expected to open Drive successfully");

        let db_transaction = drive.grove.start_transaction();

        let platform_version = PlatformVersion::latest();
        drive
            .create_initial_state_structure(Some(&db_transaction), &platform_version)
            .expect("expected to create root tree successfully");

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
                &platform_version,
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
        let tmp_dir = TempDir::new().unwrap();
        let platform_version = PlatformVersion::latest();

        let drive: Drive =
            Drive::open(&tmp_dir, Some(config)).expect("expected to open Drive successfully");

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

        let document: Document = platform_value::from_value(value).expect("value to document");

        let document_serialized = document
            .serialize_consume(document_type, platform_version)
            .expect("expected to serialize document");

        assert_eq!(document_serialized.len(), 115);
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
            //Explanation for 1235

            //todo
            1235
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

            // Value -> 221
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

            // Total 65 + 221 + 68 = 354

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

            //// 356 + 179 + 145 + 278

            959
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

            assert_eq!(*removed_credits, 25827688);
            let refund_equivalent_bytes = removed_credits.to_unsigned()
                / Epoch::new(0)
                    .unwrap()
                    .cost_for_known_cost_item(StorageDiskUsageCreditPerByte);

            assert!(expected_added_bytes > refund_equivalent_bytes);
            assert_eq!(refund_equivalent_bytes, 956); // we refunded 956 instead of 959

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

        let expected_added_bytes = if using_history { 310 } else { 1 };
        assert_eq!(added_bytes, expected_added_bytes);
    }

    fn test_fees_for_update_document_on_index(using_history: bool, using_transaction: bool) {
        let config = DriveConfig {
            batching_consistency_verification: true,
            has_raw_enabled: true,
            default_genesis_time: Some(0),
            ..Default::default()
        };
        let tmp_dir = TempDir::new().unwrap();

        let platform_version = PlatformVersion::latest();

        let drive: Drive =
            Drive::open(&tmp_dir, Some(config)).expect("expected to open Drive successfully");

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
        let expected_added_bytes = if using_history { 1235 } else { 959 };
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

            assert_eq!(*removed_credits, 25827688);
            let refund_equivalent_bytes = removed_credits.to_unsigned()
                / Epoch::new(0)
                    .unwrap()
                    .cost_for_known_cost_item(StorageDiskUsageCreditPerByte);

            assert!(expected_added_bytes > refund_equivalent_bytes);
            assert_eq!(refund_equivalent_bytes, 956); // we refunded 1008 instead of 1011

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

        let expected_removed_credits = if using_history { 16266750 } else { 16212825 };
        assert_eq!(*removed_credits, expected_removed_credits);
        let refund_equivalent_bytes = removed_credits.to_unsigned()
            / Epoch::new(0)
                .unwrap()
                .cost_for_known_cost_item(StorageDiskUsageCreditPerByte);

        assert!(expected_added_bytes > refund_equivalent_bytes);
        let expected_remove_bytes = if using_history { 602 } else { 600 };
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
        let tmp_dir = TempDir::new().unwrap();

        let platform_version = PlatformVersion::latest();

        let drive: Drive =
            Drive::open(&tmp_dir, Some(config)).expect("expected to open Drive successfully");

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
            //Explanation for 1235

            //todo
            1235
        } else {
            //Explanation for 959

            // Document Storage

            //// Item
            // = 355 Bytes

            // Explanation for 355 storage_written_bytes

            // Key -> 65 bytes
            // 32 bytes for the key prefix
            // 32 bytes for the unique id
            // 1 byte for key_size (required space for 64)

            // Value -> 222
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

            // Total 65 + 222 + 68 = 355

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

            // 357 + 179 + 145 + 278 = 959

            959
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

        let expected_added_bytes = if using_history { 1236 } else { 960 };
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

        let document = platform_value::from_value(value).expect("value to document");

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
        let tmp_dir = TempDir::new().unwrap();

        let platform_version = PlatformVersion::latest();

        let drive: Drive =
            Drive::open(&tmp_dir, Some(config)).expect("expected to open Drive successfully");

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
        let tmp_dir = TempDir::new().unwrap();

        let drive: Drive =
            Drive::open(&tmp_dir, None).expect("expected to open Drive successfully");

        let platform_version = PlatformVersion::latest();
        drive
            .create_initial_state_structure(None, &platform_version)
            .expect("expected to create root tree successfully");

        // Create a contract

        let block_info = BlockInfo::default();
        let owner_id = Identifier::new([2u8; 32]);

        let documents = platform_value!({
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

        let factory = DataContractFactory::new(1, Some(Box::new(TestEntropyGenerator::new())))
            .expect("expected to create factory");

        let contract = factory
            .create(owner_id, documents, None, None)
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

        let document_factory =
            DocumentFactory::new(1, contract).expect("expected to create document factory");

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
