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

//! Insert Documents.
//!
//! This module implements functions in Drive relevant to inserting documents.
//!

// Module: add_document
// This module contains functionality for adding a document
mod add_document;

// Module: add_document_for_contract
// This module contains functionality for adding a document for a given contract
mod add_document_for_contract;

// Module: add_document_for_contract_apply_and_add_to_operations
// This module contains functionality for applying and adding operations for a contract document
mod add_document_for_contract_apply_and_add_to_operations;

// Module: add_document_for_contract_operations
// This module contains functionality for adding a document for contract operations
mod add_document_for_contract_operations;

// Module: add_document_to_primary_storage
// This module contains functionality for adding a document to primary storage
mod add_document_to_primary_storage;

// Module: add_indices_for_index_level_for_contract_operations
// This module contains functionality for adding indices for an index level for contract operations
mod add_indices_for_index_level_for_contract_operations;

// Module: add_indices_for_top_index_level_for_contract_operations
// This module contains functionality for adding indices for the top index level for contract operations
mod add_indices_for_top_index_level_for_contract_operations;

// Module: add_reference_for_index_level_for_contract_operations
// This module contains functionality for adding a reference for an index level for contract operations
mod add_reference_for_index_level_for_contract_operations;

#[cfg(all(
    feature = "fixtures-and-mocks",
    feature = "data-contract-cbor-conversion"
))]
use dpp::data_contract::conversion::cbor::DataContractCborConversionMethodsV0;

#[cfg(test)]
mod tests {
    use std::borrow::Cow;
    use std::option::Option::None;

    use dpp::block::block_info::BlockInfo;
    use rand::Rng;

    use crate::common::setup_contract;
    use crate::drive::document::tests::setup_dashpay;
    use crate::drive::flags::StorageFlags;
    use crate::drive::object_size_info::{DocumentAndContractInfo, OwnedDocumentInfo};
    use crate::fee::op::LowLevelDriveOperation;

    use dpp::block::epoch::Epoch;
    use dpp::data_contract::accessors::v0::DataContractV0Getters;
    use dpp::document::Document;

    use crate::drive::object_size_info::DocumentInfo::DocumentRefInfo;
    use crate::tests::helpers::setup::setup_drive_with_initial_state_structure;
    use dpp::document::serialization_traits::DocumentCborMethodsV0;
    use dpp::fee::default_costs::EpochCosts;
    use dpp::fee::default_costs::KnownCostItem::StorageDiskUsageCreditPerByte;
    use dpp::fee::fee_result::FeeResult;
    use dpp::tests::fixtures::get_dpns_data_contract_fixture;
    use dpp::tests::json_document::json_document_to_document;
    use dpp::version::PlatformVersion;

    #[test]
    fn test_add_dashpay_documents_no_transaction() {
        let (drive, dashpay) = setup_dashpay("add", true);

        let random_owner_id = rand::thread_rng().gen::<[u8; 32]>();

        let platform_version = PlatformVersion::first();

        let document_type = dashpay
            .document_type_for_name("contactRequest")
            .expect("expected to get document type");

        let dashpay_cr_document = json_document_to_document(
            "tests/supporting_files/contract/dashpay/contact-request0.json",
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
                            &dashpay_cr_document,
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
                    contract: &dashpay,
                    document_type,
                },
                false,
                BlockInfo::default(),
                true,
                None,
                platform_version,
            )
            .expect_err("expected not to be able to insert same document twice");

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
                    contract: &dashpay,
                    document_type,
                },
                true,
                BlockInfo::default(),
                true,
                None,
                platform_version,
            )
            .expect("expected to override a document successfully");
    }

    #[test]
    fn test_add_dashpay_documents() {
        let drive = setup_drive_with_initial_state_structure();

        let db_transaction = drive.grove.start_transaction();

        let platform_version = PlatformVersion::latest();

        let contract = setup_contract(
            &drive,
            "tests/supporting_files/contract/dashpay/dashpay-contract-all-mutable.json",
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
        .expect("expected to get cbor document");

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
            .expect_err("expected not to be able to insert same document twice");

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
            .expect("expected to override a document successfully");
    }

    #[test]
    fn test_add_dashpay_contact_request_with_fee() {
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
        .expect("expected to get cbor document");

        let fee_result = drive
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

        assert_eq!(
            fee_result,
            FeeResult {
                storage_fee: 3057
                    * Epoch::new(0)
                        .unwrap()
                        .cost_for_known_cost_item(StorageDiskUsageCreditPerByte),
                processing_fee: 2316870,
                ..Default::default()
            }
        );
    }

    #[test]
    fn test_add_dashpay_profile_with_fee() {
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
            .expect("expected to get document type");

        let random_owner_id = rand::thread_rng().gen::<[u8; 32]>();

        let dashpay_profile_document = json_document_to_document(
            "tests/supporting_files/contract/dashpay/profile0.json",
            Some(random_owner_id.into()),
            document_type,
            platform_version,
        )
        .expect("expected to get cbor document");

        let fee_result = drive
            .add_document_for_contract(
                DocumentAndContractInfo {
                    owned_document_info: OwnedDocumentInfo {
                        document_info: DocumentRefInfo((
                            &dashpay_profile_document,
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

        assert_eq!(
            fee_result,
            FeeResult {
                storage_fee: 1304
                    * Epoch::new(0)
                        .unwrap()
                        .cost_for_known_cost_item(StorageDiskUsageCreditPerByte),
                processing_fee: 1481610,
                ..Default::default()
            }
        );
    }

    #[test]
    fn test_add_dashpay_profile_average_case_cost_fee() {
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
            .expect("expected to get document type");

        let random_owner_id = rand::thread_rng().gen::<[u8; 32]>();

        let dashpay_profile_document = json_document_to_document(
            "tests/supporting_files/contract/dashpay/profile0.json",
            Some(random_owner_id.into()),
            document_type,
            platform_version,
        )
        .expect("expected to get cbor document");

        let FeeResult {
            storage_fee,
            processing_fee,
            fee_refunds: _,
            removed_bytes_from_system: _,
        } = drive
            .add_document_for_contract(
                DocumentAndContractInfo {
                    owned_document_info: OwnedDocumentInfo {
                        document_info: DocumentRefInfo((
                            &dashpay_profile_document,
                            StorageFlags::optional_default_as_cow(),
                        )),
                        owner_id: Some(random_owner_id),
                    },
                    contract: &contract,
                    document_type,
                },
                false,
                BlockInfo::default(),
                false,
                Some(&db_transaction),
                platform_version,
            )
            .expect("expected to insert a document successfully");

        let added_bytes = storage_fee
            / Epoch::new(0)
                .unwrap()
                .cost_for_known_cost_item(StorageDiskUsageCreditPerByte);
        assert_eq!(1304, added_bytes);
        assert_eq!(142936400, processing_fee);
    }

    #[test]
    fn test_unknown_state_cost_dashpay_fee_for_add_documents() {
        let drive = setup_drive_with_initial_state_structure();

        let db_transaction = drive.grove.start_transaction();

        let platform_version = PlatformVersion::latest();

        let contract = setup_contract(
            &drive,
            "tests/supporting_files/contract/dashpay/dashpay-contract-all-mutable.json",
            None,
            Some(&db_transaction),
        );

        let random_owner_id = rand::thread_rng().gen::<[u8; 32]>();

        let document_type = contract
            .document_type_for_name("contactRequest")
            .expect("expected to get document type");

        let dashpay_cr_document = json_document_to_document(
            "tests/supporting_files/contract/dashpay/contact-request0.json",
            Some(random_owner_id.into()),
            document_type,
            platform_version,
        )
        .expect("expected to get cbor document");

        let fees = drive
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
                false,
                Some(&db_transaction),
                platform_version,
            )
            .expect("expected to insert a document successfully");

        let actual_fees = drive
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

        assert_eq!(fees.storage_fee, actual_fees.storage_fee);
    }

    #[test]
    fn test_add_dashpay_fee_for_documents_detail() {
        let drive = setup_drive_with_initial_state_structure();

        let db_transaction = drive.grove.start_transaction();

        let platform_version = PlatformVersion::latest();

        let contract = setup_contract(
            &drive,
            "tests/supporting_files/contract/dashpay/dashpay-contract-all-mutable.json",
            None,
            Some(&db_transaction),
        );

        let random_owner_id = rand::thread_rng().gen::<[u8; 32]>();

        let document_type = contract
            .document_type_for_name("contactRequest")
            .expect("expected to get document type");

        let dashpay_cr_document = json_document_to_document(
            "tests/supporting_files/contract/dashpay/contact-request0.json",
            Some(random_owner_id.into()),
            document_type,
            platform_version,
        )
        .expect("expected to get cbor document");

        let storage_flags = Some(Cow::Owned(StorageFlags::SingleEpoch(0)));

        let document_info = DocumentRefInfo((&dashpay_cr_document, storage_flags));

        let mut fee_drive_operations: Vec<LowLevelDriveOperation> = vec![];
        let mut actual_drive_operations: Vec<LowLevelDriveOperation> = vec![];

        let root_hash = drive
            .grove
            .root_hash(Some(&db_transaction))
            .unwrap()
            .expect("expected a root hash calculation to succeed");

        drive
            .add_document_for_contract_apply_and_add_to_operations(
                DocumentAndContractInfo {
                    owned_document_info: OwnedDocumentInfo {
                        document_info: document_info.clone(),
                        owner_id: None,
                    },
                    contract: &contract,
                    document_type,
                },
                false,
                &BlockInfo::default(),
                true,
                false,
                Some(&db_transaction),
                &mut fee_drive_operations,
                platform_version,
            )
            .expect("expected to get back fee for document insertion successfully");

        let root_hash_after_fee = drive
            .grove
            .root_hash(Some(&db_transaction))
            .unwrap()
            .expect("expected a root hash calculation to succeed");

        assert_eq!(root_hash, root_hash_after_fee);

        drive
            .add_document_for_contract_apply_and_add_to_operations(
                DocumentAndContractInfo {
                    owned_document_info: OwnedDocumentInfo {
                        document_info,
                        owner_id: None,
                    },
                    contract: &contract,
                    document_type,
                },
                false,
                &BlockInfo::default(),
                true,
                true,
                Some(&db_transaction),
                &mut actual_drive_operations,
                platform_version,
            )
            .expect("expected to get back fee for document insertion successfully");

        assert_eq!(actual_drive_operations.len(), fee_drive_operations.len());
    }

    #[test]
    fn test_add_dpns_document_with_fee() {
        let drive = setup_drive_with_initial_state_structure();

        let db_transaction = drive.grove.start_transaction();

        let platform_version = PlatformVersion::latest();

        let contract = setup_contract(
            &drive,
            "tests/supporting_files/contract/dpns/dpns-contract.json",
            None,
            Some(&db_transaction),
        );

        let random_owner_id = rand::thread_rng().gen::<[u8; 32]>();

        let document_type = contract
            .document_type_for_name("domain")
            .expect("expected to get document type");

        let dpns_domain_document = json_document_to_document(
            "tests/supporting_files/contract/dpns/domain0.json",
            Some(random_owner_id.into()),
            document_type,
            platform_version,
        )
        .expect("expected to get document");

        let storage_flags = Some(Cow::Owned(StorageFlags::SingleEpoch(0)));

        let fee_result = drive
            .add_document_for_contract(
                DocumentAndContractInfo {
                    owned_document_info: OwnedDocumentInfo {
                        document_info: DocumentRefInfo((&dpns_domain_document, storage_flags)),
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

        assert_eq!(
            fee_result,
            FeeResult {
                storage_fee: 1760
                    * Epoch::new(0)
                        .unwrap()
                        .cost_for_known_cost_item(StorageDiskUsageCreditPerByte),
                processing_fee: 2068990,
                ..Default::default()
            }
        );

        drive
            .grove
            .commit_transaction(db_transaction)
            .unwrap()
            .expect("unable to commit transaction");
    }

    #[test]
    fn test_add_dashpay_many_non_conflicting_documents() {
        let (drive, dashpay) = setup_dashpay("add_no_conflict", true);

        let random_owner_id = rand::thread_rng().gen::<[u8; 32]>();

        let platform_version = PlatformVersion::first();

        let document_type = dashpay
            .document_type_for_name("contactRequest")
            .expect("expected to get document type");

        let dashpay_cr_document_0 = json_document_to_document(
            "tests/supporting_files/contract/dashpay/contact-request0.json",
            Some(random_owner_id.into()),
            document_type,
            platform_version,
        )
        .expect("expected to get cbor document");

        let dashpay_cr_document_1 = json_document_to_document(
            "tests/supporting_files/contract/dashpay/contact-request1.json",
            Some(random_owner_id.into()),
            document_type,
            platform_version,
        )
        .expect("expected to get cbor document");

        let dashpay_cr_document_2 = json_document_to_document(
            "tests/supporting_files/contract/dashpay/contact-request2.json",
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
                            &dashpay_cr_document_0,
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

        drive
            .add_document_for_contract(
                DocumentAndContractInfo {
                    owned_document_info: OwnedDocumentInfo {
                        document_info: DocumentRefInfo((
                            &dashpay_cr_document_1,
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

        drive
            .add_document_for_contract(
                DocumentAndContractInfo {
                    owned_document_info: OwnedDocumentInfo {
                        document_info: DocumentRefInfo((
                            &dashpay_cr_document_2,
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
    }

    #[test]
    fn test_add_dashpay_conflicting_unique_index_documents() {
        let (drive, dashpay) = setup_dashpay("add_conflict", true);

        let document_type = dashpay
            .document_type_for_name("contactRequest")
            .expect("expected to get document type");

        let random_owner_id = rand::thread_rng().gen::<[u8; 32]>();

        let platform_version = PlatformVersion::first();

        let dashpay_cr_document_0 = json_document_to_document(
            "tests/supporting_files/contract/dashpay/contact-request0.json",
            Some(random_owner_id.into()),
            document_type,
            platform_version,
        )
        .expect("expected to get cbor document");

        let dashpay_cr_document_0_dup = json_document_to_document(
            "tests/supporting_files/contract/dashpay/contact-request0-dup-unique-index.json",
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
                            &dashpay_cr_document_0,
                            StorageFlags::optional_default_as_cow(),
                        )),
                        owner_id: None,
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

        drive
            .add_document_for_contract(
                DocumentAndContractInfo {
                    owned_document_info: OwnedDocumentInfo {
                        document_info: DocumentRefInfo((
                            &dashpay_cr_document_0_dup,
                            StorageFlags::optional_default_as_cow(),
                        )),
                        owner_id: None,
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
            .expect_err(
                "expected not to be able to insert document with already existing unique index",
            );
    }

    #[test]
    fn test_create_two_documents_with_the_same_index_in_different_transactions() {
        let drive = setup_drive_with_initial_state_structure();

        let db_transaction = drive.grove.start_transaction();

        let platform_version = PlatformVersion::latest();

        let created_contract =
            get_dpns_data_contract_fixture(None, 0, platform_version.protocol_version);

        drive
            .apply_contract(
                created_contract.data_contract(),
                BlockInfo::default(),
                true,
                StorageFlags::optional_default_as_cow(),
                Some(&db_transaction),
                platform_version,
            )
            .expect("expected to apply contract successfully");

        // Create dash TLD

        let dash_tld_cbor = hex::decode("00ac632469645820d7f2c53f46a917ab6e5b39a2d7bc260b649289453744d1e0d4f26a8d8eff37cf65247479706566646f6d61696e656c6162656c6464617368677265636f726473a17364617368416c6961734964656e74697479496458203012c19b98ec0033addb36cd64b7f510670f2a351a4304b5f6994144286efdac68246f776e6572496458203012c19b98ec0033addb36cd64b7f510670f2a351a4304b5f6994144286efdac69247265766973696f6e016a246372656174656441741b0000017f07c861586c7072656f7264657253616c745820e0b508c5a36825a206693a1f414aa13edbecf43c41e3c799ea9e737b4f9aa2266e737562646f6d61696e52756c6573a16f616c6c6f77537562646f6d61696e73f56f2464617461436f6e747261637449645820e668c659af66aee1e72c186dde7b5b7e0a1d712a09c40d5721f622bf53c531556f6e6f726d616c697a65644c6162656c6464617368781a6e6f726d616c697a6564506172656e74446f6d61696e4e616d6560").unwrap();
        let dash_tld = Document::from_cbor(&dash_tld_cbor, None, None, platform_version)
            .expect("expected to get document");

        let info = DocumentAndContractInfo {
            owned_document_info: OwnedDocumentInfo {
                document_info: DocumentRefInfo((
                    &dash_tld,
                    StorageFlags::optional_default_as_cow(),
                )),
                owner_id: None,
            },
            contract: created_contract.data_contract(),
            document_type: created_contract
                .data_contract()
                .document_type_for_name("domain")
                .expect("expected to get document type"),
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
            .expect("should create dash tld");

        drive
            .grove
            .commit_transaction(db_transaction)
            .unwrap()
            .expect("should commit transaction");

        let db_transaction = drive.grove.start_transaction();

        // add random TLD

        let random_tld_cbor = hex::decode("00ab632469645820655c9b5606f4ad53daea90de9c540aad656ed5fbe5fb14b40700f6f56dc793ac65247479706566646f6d61696e656c6162656c746433653966343532373963343865306261363561677265636f726473a17364617368416c6961734964656e74697479496458203012c19b98ec0033addb36cd64b7f510670f2a351a4304b5f6994144286efdac68246f776e6572496458203012c19b98ec0033addb36cd64b7f510670f2a351a4304b5f6994144286efdac69247265766973696f6e016c7072656f7264657253616c745820219353a923a29cd02c521b141f326ac0d12c362a84f1979a5de89b8dba12891b6e737562646f6d61696e52756c6573a16f616c6c6f77537562646f6d61696e73f56f2464617461436f6e747261637449645820e668c659af66aee1e72c186dde7b5b7e0a1d712a09c40d5721f622bf53c531556f6e6f726d616c697a65644c6162656c746433653966343532373963343865306261363561781a6e6f726d616c697a6564506172656e74446f6d61696e4e616d6560").unwrap();
        let _random_tld = Document::from_cbor(&random_tld_cbor, None, None, platform_version)
            .expect("expected to get document");

        let info = DocumentAndContractInfo {
            owned_document_info: OwnedDocumentInfo {
                document_info: DocumentRefInfo((
                    &dash_tld,
                    StorageFlags::optional_default_as_cow(),
                )),
                owner_id: None,
            },
            contract: created_contract.data_contract(),
            document_type: created_contract
                .data_contract()
                .document_type_for_name("domain")
                .expect("expected to get document type"),
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
            .expect("should create random tld");
    }
}
