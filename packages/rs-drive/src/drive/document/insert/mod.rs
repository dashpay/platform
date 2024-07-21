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
    use rand::{random, Rng};

    use crate::drive::document::tests::setup_dashpay;
    use crate::fees::op::LowLevelDriveOperation;
    use crate::util::object_size_info::{DocumentAndContractInfo, OwnedDocumentInfo};
    use crate::util::storage_flags::StorageFlags;
    use crate::util::test_helpers::setup_contract;
    use once_cell::sync::Lazy;
    use std::collections::BTreeMap;

    use dpp::block::epoch::Epoch;
    use dpp::data_contract::accessors::v0::DataContractV0Getters;

    use crate::util::object_size_info::DocumentInfo::DocumentRefInfo;
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use dpp::fee::default_costs::KnownCostItem::StorageDiskUsageCreditPerByte;
    use dpp::fee::default_costs::{CachedEpochIndexFeeVersions, EpochCosts};
    use dpp::fee::fee_result::FeeResult;
    use dpp::tests::json_document::json_document_to_document;
    use dpp::version::PlatformVersion;

    static EPOCH_CHANGE_FEE_VERSION_TEST: Lazy<CachedEpochIndexFeeVersions> =
        Lazy::new(|| BTreeMap::from([(0, PlatformVersion::first().fee_version.clone())]));

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
                None,
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
                None,
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
                Some(&EPOCH_CHANGE_FEE_VERSION_TEST),
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

        let random_owner_id = random::<[u8; 32]>();

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
                None,
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
                None,
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
                Some(&EPOCH_CHANGE_FEE_VERSION_TEST),
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
                None,
            )
            .expect("expected to insert a document successfully");

        assert_eq!(
            fee_result,
            FeeResult {
                storage_fee: 3058
                    * Epoch::new(0).unwrap().cost_for_known_cost_item(
                        &EPOCH_CHANGE_FEE_VERSION_TEST,
                        StorageDiskUsageCreditPerByte,
                    ),
                processing_fee: 1695100,
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
                None,
            )
            .expect("expected to insert a document successfully");

        assert_eq!(
            fee_result,
            FeeResult {
                storage_fee: 1305
                    * Epoch::new(0).unwrap().cost_for_known_cost_item(
                        &EPOCH_CHANGE_FEE_VERSION_TEST,
                        StorageDiskUsageCreditPerByte,
                    ),
                processing_fee: 900400,
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
                None,
            )
            .expect("expected to insert a document successfully");

        let added_bytes = storage_fee
            / Epoch::new(0).unwrap().cost_for_known_cost_item(
                &EPOCH_CHANGE_FEE_VERSION_TEST,
                StorageDiskUsageCreditPerByte,
            );
        assert_eq!(1305, added_bytes);
        assert_eq!(73253660, processing_fee);
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

        let random_owner_id = random::<[u8; 32]>();

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
                None,
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
                None,
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

        let random_owner_id = random::<[u8; 32]>();

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
            .root_hash(Some(&db_transaction), &platform_version.drive.grove_version)
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
            .root_hash(Some(&db_transaction), &platform_version.drive.grove_version)
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
                None,
            )
            .expect("expected to insert a document successfully");

        assert_eq!(
            fee_result,
            FeeResult {
                storage_fee: 1761
                    * Epoch::new(0).unwrap().cost_for_known_cost_item(
                        &EPOCH_CHANGE_FEE_VERSION_TEST,
                        StorageDiskUsageCreditPerByte,
                    ),
                processing_fee: 1227900,
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

        let random_owner_id = random::<[u8; 32]>();

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
                None,
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
                None,
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
                None,
            )
            .expect("expected to insert a document successfully");
    }

    #[test]
    fn test_add_dashpay_conflicting_unique_index_documents() {
        let (drive, dashpay) = setup_dashpay("add_conflict", true);

        let document_type = dashpay
            .document_type_for_name("contactRequest")
            .expect("expected to get document type");

        let random_owner_id = random::<[u8; 32]>();

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
                None,
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
                None,
            )
            .expect_err(
                "expected not to be able to insert document with already existing unique index",
            );
    }
}
