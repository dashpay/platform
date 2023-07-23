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
pub use add_document::*;

// Module: add_document_for_contract
// This module contains functionality for adding a document for a given contract
mod add_document_for_contract;
pub use add_document_for_contract::*;

// Module: add_document_for_contract_apply_and_add_to_operations
// This module contains functionality for applying and adding operations for a contract document
mod add_document_for_contract_apply_and_add_to_operations;
pub use add_document_for_contract_apply_and_add_to_operations::*;

// Module: add_document_for_contract_operations
// This module contains functionality for adding a document for contract operations
mod add_document_for_contract_operations;
pub use add_document_for_contract_operations::*;

// Module: add_document_to_primary_storage
// This module contains functionality for adding a document to primary storage
mod add_document_to_primary_storage;
pub use add_document_to_primary_storage::*;

// Module: add_indices_for_index_level_for_contract_operations
// This module contains functionality for adding indices for an index level for contract operations
mod add_indices_for_index_level_for_contract_operations;
pub use add_indices_for_index_level_for_contract_operations::*;

// Module: add_indices_for_top_index_level_for_contract_operations
// This module contains functionality for adding indices for the top index level for contract operations
mod add_indices_for_top_index_level_for_contract_operations;
pub use add_indices_for_top_index_level_for_contract_operations::*;

// Module: add_reference_for_index_level_for_contract_operations
// This module contains functionality for adding a reference for an index level for contract operations
mod add_reference_for_index_level_for_contract_operations;
pub use add_reference_for_index_level_for_contract_operations::*;

use grovedb::TransactionArg;
use std::borrow::Cow;
use std::option::Option::None;

use crate::drive::flags::StorageFlags;
use crate::drive::object_size_info::DocumentInfo::DocumentRefInfo;
use crate::drive::object_size_info::{DocumentAndContractInfo, OwnedDocumentInfo};
use crate::drive::Drive;
use crate::error::Error;
use crate::fee::op::LowLevelDriveOperation;
use dpp::data_contract::DataContract;

use dpp::block::block_info::BlockInfo;
use dpp::data_contract::base::DataContractBaseMethodsV0;
#[cfg(any(feature = "full", feature = "fixtures-and-mocks"))]
use dpp::data_contract::conversion::cbor_conversion::DataContractCborConversionMethodsV0;
use dpp::document::serialization_traits::DocumentCborMethodsV0;
use dpp::document::Document;
use dpp::fee::fee_result::FeeResult;
use dpp::version::PlatformVersion;

impl Drive {
    /// Deserializes a document and a contract and adds the document to the contract.
    #[cfg(any(feature = "full", feature = "fixtures-and-mocks"))]
    pub fn add_cbor_serialized_document_for_serialized_contract(
        &self,
        serialized_document: &[u8],
        serialized_contract: &[u8],
        document_type_name: &str,
        owner_id: Option<[u8; 32]>,
        override_document: bool,
        block_info: BlockInfo,
        apply: bool,
        storage_flags: Option<Cow<StorageFlags>>,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<FeeResult, Error> {
        let contract = DataContract::from_cbor(serialized_contract, platform_version)?;

        let document = Document::from_cbor(serialized_document, None, owner_id, platform_version)?;

        let document_info = DocumentRefInfo((&document, storage_flags));

        let document_type = contract.document_type_for_name(document_type_name)?;

        self.add_document_for_contract(
            DocumentAndContractInfo {
                owned_document_info: OwnedDocumentInfo {
                    document_info,
                    owner_id,
                },
                contract: &contract,
                document_type,
            },
            override_document,
            block_info,
            apply,
            transaction,
            platform_version,
        )
    }
}

#[cfg(test)]
mod tests {
    use std::option::Option::None;

    use super::*;
    use dpp::data_contract::extra::common::json_document_to_document;
    use rand::Rng;
    use tempfile::TempDir;

    use crate::common::setup_contract;
    use crate::drive::document::tests::setup_dashpay;
    use crate::drive::flags::StorageFlags;
    use crate::drive::object_size_info::DocumentAndContractInfo;
    use crate::drive::Drive;
    use crate::fee::op::LowLevelDriveOperation;
    use dpp::block::epoch::Epoch;
    use dpp::document::serialization_traits::DocumentCborMethodsV0;
    use dpp::fee::default_costs::EpochCosts;
    use dpp::fee::default_costs::KnownCostItem::StorageDiskUsageCreditPerByte;
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
                &platform_version,
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
                &platform_version,
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
                &platform_version,
            )
            .expect("expected to override a document successfully");
    }

    #[test]
    fn test_add_dashpay_documents() {
        let tmp_dir = TempDir::new().unwrap();
        let drive: Drive = Drive::open(tmp_dir, None).expect("expected to open Drive successfully");

        let db_transaction = drive.grove.start_transaction();

        let platform_version = PlatformVersion::latest();
        drive
            .create_initial_state_structure(Some(&db_transaction), &platform_version)
            .expect("expected to create root tree successfully");

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
                &platform_version,
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
                &platform_version,
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
                &platform_version,
            )
            .expect("expected to override a document successfully");
    }

    #[test]
    fn test_add_dashpay_contact_request_with_fee() {
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
                &platform_version,
            )
            .expect("expected to insert a document successfully");

        assert_eq!(
            fee_result,
            FeeResult {
                storage_fee: 3055
                    * Epoch::new(0)
                        .unwrap()
                        .cost_for_known_cost_item(StorageDiskUsageCreditPerByte),
                processing_fee: 2316070,
                ..Default::default()
            }
        );
    }

    #[test]
    fn test_add_dashpay_profile_with_fee() {
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
                &platform_version,
            )
            .expect("expected to insert a document successfully");

        assert_eq!(
            fee_result,
            FeeResult {
                storage_fee: 1302
                    * Epoch::new(0)
                        .unwrap()
                        .cost_for_known_cost_item(StorageDiskUsageCreditPerByte),
                processing_fee: 1496810,
                ..Default::default()
            }
        );
    }

    #[test]
    fn test_add_dashpay_profile_average_case_cost_fee() {
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
        assert_eq!(1302, added_bytes);
        assert_eq!(145124400, processing_fee);
    }

    #[test]
    fn test_unknown_state_cost_dashpay_fee_for_add_documents() {
        let tmp_dir = TempDir::new().unwrap();
        let drive: Drive = Drive::open(tmp_dir, None).expect("expected to open Drive successfully");

        let db_transaction = drive.grove.start_transaction();

        let platform_version = PlatformVersion::latest();
        drive
            .create_initial_state_structure(Some(&db_transaction), &platform_version)
            .expect("expected to create root tree successfully");

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
                &platform_version,
            )
            .expect("expected to insert a document successfully");

        assert_eq!(fees.storage_fee, actual_fees.storage_fee);
    }

    #[test]
    fn test_add_dashpay_fee_for_documents_detail() {
        let tmp_dir = TempDir::new().unwrap();
        let drive: Drive = Drive::open(tmp_dir, None).expect("expected to open Drive successfully");

        let db_transaction = drive.grove.start_transaction();

        let platform_version = PlatformVersion::latest();
        drive
            .create_initial_state_structure(Some(&db_transaction), &platform_version)
            .expect("expected to create root tree successfully");

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
        let tmp_dir = TempDir::new().unwrap();
        let drive: Drive = Drive::open(tmp_dir, None).expect("expected to open Drive successfully");

        let db_transaction = drive.grove.start_transaction();

        let platform_version = PlatformVersion::latest();
        drive
            .create_initial_state_structure(Some(&db_transaction), &platform_version)
            .expect("expected to create root tree successfully");

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
                &platform_version,
            )
            .expect("expected to insert a document successfully");

        assert_eq!(
            fee_result,
            FeeResult {
                storage_fee: 1748
                    * Epoch::new(0)
                        .unwrap()
                        .cost_for_known_cost_item(StorageDiskUsageCreditPerByte),
                processing_fee: 2083390,
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
                &platform_version,
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
                &platform_version,
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
                &platform_version,
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
                &platform_version,
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
                &platform_version,
            )
            .expect_err(
                "expected not to be able to insert document with already existing unique index",
            );
    }

    #[test]
    fn test_create_two_documents_with_the_same_index_in_different_transactions() {
        let tmp_dir = TempDir::new().unwrap();
        let drive: Drive = Drive::open(tmp_dir, None).expect("expected to open Drive successfully");

        let db_transaction = drive.grove.start_transaction();

        let platform_version = PlatformVersion::latest();
        drive
            .create_initial_state_structure(Some(&db_transaction), &platform_version)
            .expect("expected to create root tree successfully");

        let contract_cbor = hex::decode("01a5632469645820e668c659af66aee1e72c186dde7b5b7e0a1d712a09c40d5721f622bf53c531556724736368656d61783468747470733a2f2f736368656d612e646173682e6f72672f6470702d302d342d302f6d6574612f646174612d636f6e7472616374676f776e6572496458203012c19b98ec0033addb36cd64b7f510670f2a351a4304b5f6994144286efdac6776657273696f6e0169646f63756d656e7473a266646f6d61696ea66474797065666f626a65637467696e646963657383a3646e616d6572706172656e744e616d65416e644c6162656c66756e69717565f56a70726f7065727469657382a1781a6e6f726d616c697a6564506172656e74446f6d61696e4e616d6563617363a16f6e6f726d616c697a65644c6162656c63617363a3646e616d656e646173684964656e74697479496466756e69717565f56a70726f7065727469657381a1781c7265636f7264732e64617368556e697175654964656e74697479496463617363a2646e616d656964617368416c6961736a70726f7065727469657381a1781b7265636f7264732e64617368416c6961734964656e746974794964636173636824636f6d6d656e74790137496e206f7264657220746f207265676973746572206120646f6d61696e20796f75206e65656420746f206372656174652061207072656f726465722e20546865207072656f726465722073746570206973206e656564656420746f2070726576656e74206d616e2d696e2d7468652d6d6964646c652061747461636b732e206e6f726d616c697a65644c6162656c202b20272e27202b206e6f726d616c697a6564506172656e74446f6d61696e206d757374206e6f74206265206c6f6e676572207468616e20323533206368617273206c656e67746820617320646566696e65642062792052464320313033352e20446f6d61696e20646f63756d656e74732061726520696d6d757461626c653a206d6f64696669636174696f6e20616e642064656c6574696f6e20617265207265737472696374656468726571756972656486656c6162656c6f6e6f726d616c697a65644c6162656c781a6e6f726d616c697a6564506172656e74446f6d61696e4e616d656c7072656f7264657253616c74677265636f7264736e737562646f6d61696e52756c65736a70726f70657274696573a6656c6162656ca5647479706566737472696e67677061747465726e782a5e5b612d7a412d5a302d395d5b612d7a412d5a302d392d5d7b302c36317d5b612d7a412d5a302d395d24696d61784c656e677468183f696d696e4c656e677468036b6465736372697074696f6e7819446f6d61696e206c6162656c2e20652e672e2027426f62272e677265636f726473a66474797065666f626a6563746824636f6d6d656e747890436f6e73747261696e742077697468206d617820616e64206d696e2070726f7065727469657320656e737572652074686174206f6e6c79206f6e65206964656e74697479207265636f72642069732075736564202d206569746865722061206064617368556e697175654964656e74697479496460206f722061206064617368416c6961734964656e746974794964606a70726f70657274696573a27364617368416c6961734964656e746974794964a764747970656561727261796824636f6d6d656e7478234d75737420626520657175616c20746f2074686520646f63756d656e74206f776e6572686d61784974656d731820686d696e4974656d73182069627974654172726179f56b6465736372697074696f6e783d4964656e7469747920494420746f206265207573656420746f2063726561746520616c696173206e616d657320666f7220746865204964656e7469747970636f6e74656e744d656469615479706578216170706c69636174696f6e2f782e646173682e6470702e6964656e7469666965727464617368556e697175654964656e746974794964a764747970656561727261796824636f6d6d656e7478234d75737420626520657175616c20746f2074686520646f63756d656e74206f776e6572686d61784974656d731820686d696e4974656d73182069627974654172726179f56b6465736372697074696f6e783e4964656e7469747920494420746f206265207573656420746f2063726561746520746865207072696d617279206e616d6520746865204964656e7469747970636f6e74656e744d656469615479706578216170706c69636174696f6e2f782e646173682e6470702e6964656e7469666965726d6d617850726f70657274696573016d6d696e50726f7065727469657301746164646974696f6e616c50726f70657274696573f46c7072656f7264657253616c74a56474797065656172726179686d61784974656d731820686d696e4974656d73182069627974654172726179f56b6465736372697074696f6e782253616c74207573656420696e20746865207072656f7264657220646f63756d656e746e737562646f6d61696e52756c6573a56474797065666f626a656374687265717569726564816f616c6c6f77537562646f6d61696e736a70726f70657274696573a16f616c6c6f77537562646f6d61696e73a3647479706567626f6f6c65616e6824636f6d6d656e74784f4f6e6c792074686520646f6d61696e206f776e657220697320616c6c6f77656420746f2063726561746520737562646f6d61696e7320666f72206e6f6e20746f702d6c6576656c20646f6d61696e736b6465736372697074696f6e785b54686973206f7074696f6e20646566696e65732077686f2063616e2063726561746520737562646f6d61696e733a2074727565202d20616e796f6e653b2066616c7365202d206f6e6c792074686520646f6d61696e206f776e65726b6465736372697074696f6e7842537562646f6d61696e2072756c657320616c6c6f7720646f6d61696e206f776e65727320746f20646566696e652072756c657320666f7220737562646f6d61696e73746164646974696f6e616c50726f70657274696573f46f6e6f726d616c697a65644c6162656ca5647479706566737472696e67677061747465726e78215e5b612d7a302d395d5b612d7a302d392d5d7b302c36317d5b612d7a302d395d246824636f6d6d656e7478694d75737420626520657175616c20746f20746865206c6162656c20696e206c6f776572636173652e20546869732070726f70657274792077696c6c20626520646570726563617465642064756520746f206361736520696e73656e73697469766520696e6469636573696d61784c656e677468183f6b6465736372697074696f6e7850446f6d61696e206c6162656c20696e206c6f7765726361736520666f7220636173652d696e73656e73697469766520756e697175656e6573732076616c69646174696f6e2e20652e672e2027626f6227781a6e6f726d616c697a6564506172656e74446f6d61696e4e616d65a6647479706566737472696e67677061747465726e78285e247c5e5b5b612d7a302d395d5b612d7a302d392d5c2e5d7b302c3138387d5b612d7a302d395d246824636f6d6d656e74788c4d7573742065697468657220626520657175616c20746f20616e206578697374696e6720646f6d61696e206f7220656d70747920746f20637265617465206120746f70206c6576656c20646f6d61696e2e204f6e6c7920746865206461746120636f6e7472616374206f776e65722063616e2063726561746520746f70206c6576656c20646f6d61696e732e696d61784c656e67746818be696d696e4c656e677468006b6465736372697074696f6e785e412066756c6c20706172656e7420646f6d61696e206e616d6520696e206c6f7765726361736520666f7220636173652d696e73656e73697469766520756e697175656e6573732076616c69646174696f6e2e20652e672e20276461736827746164646974696f6e616c50726f70657274696573f4687072656f72646572a66474797065666f626a65637467696e646963657381a3646e616d656a73616c7465644861736866756e69717565f56a70726f7065727469657381a17073616c746564446f6d61696e48617368636173636824636f6d6d656e74784a5072656f7264657220646f63756d656e74732061726520696d6d757461626c653a206d6f64696669636174696f6e20616e642064656c6574696f6e206172652072657374726963746564687265717569726564817073616c746564446f6d61696e486173686a70726f70657274696573a17073616c746564446f6d61696e48617368a56474797065656172726179686d61784974656d731820686d696e4974656d73182069627974654172726179f56b6465736372697074696f6e7859446f75626c65207368612d323536206f662074686520636f6e636174656e6174696f6e206f66206120333220627974652072616e646f6d2073616c7420616e642061206e6f726d616c697a656420646f6d61696e206e616d65746164646974696f6e616c50726f70657274696573f4").unwrap();

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

        // Create dash TLD

        let dash_tld_cbor = hex::decode("01ac632469645820d7f2c53f46a917ab6e5b39a2d7bc260b649289453744d1e0d4f26a8d8eff37cf65247479706566646f6d61696e656c6162656c6464617368677265636f726473a17364617368416c6961734964656e74697479496458203012c19b98ec0033addb36cd64b7f510670f2a351a4304b5f6994144286efdac68246f776e6572496458203012c19b98ec0033addb36cd64b7f510670f2a351a4304b5f6994144286efdac69247265766973696f6e016a246372656174656441741b0000017f07c861586c7072656f7264657253616c745820e0b508c5a36825a206693a1f414aa13edbecf43c41e3c799ea9e737b4f9aa2266e737562646f6d61696e52756c6573a16f616c6c6f77537562646f6d61696e73f56f2464617461436f6e747261637449645820e668c659af66aee1e72c186dde7b5b7e0a1d712a09c40d5721f622bf53c531556f6e6f726d616c697a65644c6162656c6464617368781a6e6f726d616c697a6564506172656e74446f6d61696e4e616d6560").unwrap();

        drive
            .add_cbor_serialized_document_for_serialized_contract(
                dash_tld_cbor.as_slice(),
                contract_cbor.as_slice(),
                "domain",
                None,
                true,
                BlockInfo::default(),
                true,
                StorageFlags::optional_default_as_cow(),
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

        let random_tld_cbor = hex::decode("01ab632469645820655c9b5606f4ad53daea90de9c540aad656ed5fbe5fb14b40700f6f56dc793ac65247479706566646f6d61696e656c6162656c746433653966343532373963343865306261363561677265636f726473a17364617368416c6961734964656e74697479496458203012c19b98ec0033addb36cd64b7f510670f2a351a4304b5f6994144286efdac68246f776e6572496458203012c19b98ec0033addb36cd64b7f510670f2a351a4304b5f6994144286efdac69247265766973696f6e016c7072656f7264657253616c745820219353a923a29cd02c521b141f326ac0d12c362a84f1979a5de89b8dba12891b6e737562646f6d61696e52756c6573a16f616c6c6f77537562646f6d61696e73f56f2464617461436f6e747261637449645820e668c659af66aee1e72c186dde7b5b7e0a1d712a09c40d5721f622bf53c531556f6e6f726d616c697a65644c6162656c746433653966343532373963343865306261363561781a6e6f726d616c697a6564506172656e74446f6d61696e4e616d6560").unwrap();

        drive
            .add_cbor_serialized_document_for_serialized_contract(
                random_tld_cbor.as_slice(),
                contract_cbor.as_slice(),
                "domain",
                None,
                true,
                BlockInfo::default(),
                true,
                StorageFlags::optional_default_as_cow(),
                Some(&db_transaction),
                platform_version,
            )
            .expect("should add random tld");
    }
}
