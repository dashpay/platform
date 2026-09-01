//! Insert Documents.
//!
//! This module implements functions in Drive relevant to inserting documents.
//!

// Module: add_document
// This module contains functionality for adding a document
mod add_document;

mod add_history_operations;

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

// Module: add_preallocated_index_tree_operations
// This module contains functionality for preallocating refersTo-determined
// indexOnly index trees when the referenced document is inserted
mod add_preallocated_index_tree_operations;

// Module: add_reference_for_index_level_for_contract_operations
// This module contains functionality for adding a reference for an index level for contract operations
mod add_reference_for_index_level_for_contract_operations;

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

    use crate::config::DriveConfig;
    use crate::error::drive::DriveError;
    use crate::error::Error;
    use crate::query::DriveDocumentQuery;
    use crate::util::object_size_info::DocumentInfo::DocumentRefInfo;
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use dpp::block::epoch::Epoch;
    use dpp::data_contract::accessors::v0::DataContractV0Getters;
    use dpp::data_contract::DataContract;
    use dpp::document::serialization_traits::DocumentPlatformConversionMethodsV0;
    use dpp::document::{Document, DocumentV0Getters};
    use dpp::fee::default_costs::KnownCostItem::StorageDiskUsageCreditPerByte;
    use dpp::fee::default_costs::{CachedEpochIndexFeeVersions, EpochCosts};
    use dpp::fee::fee_result::FeeResult;
    use dpp::tests::json_document::json_document_to_document;
    use dpp::version::fee::FeeVersion;
    use dpp::version::PlatformVersion;

    static EPOCH_CHANGE_FEE_VERSION_TEST: Lazy<CachedEpochIndexFeeVersions> =
        Lazy::new(|| BTreeMap::from([(0, FeeVersion::first())]));

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
    fn test_add_dashpay_document_duplicate_insert_returns_error() {
        let (drive, dashpay) = setup_dashpay("add-duplicate-error", true);

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

        let err = drive
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
            .expect_err("expected duplicate insert to return an error");

        assert!(matches!(
            err,
            Error::Drive(DriveError::CorruptedDocumentAlreadyExists(_))
        ));
    }

    #[test]
    fn test_add_dashpay_documents() {
        let drive = setup_drive_with_initial_state_structure(None);

        let db_transaction = drive.grove.start_transaction();

        let platform_version = PlatformVersion::latest();

        let contract = setup_contract(
            &drive,
            "tests/supporting_files/contract/dashpay/dashpay-contract-all-mutable.json",
            None,
            None,
            None::<fn(&mut DataContract)>,
            Some(&db_transaction),
            None,
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
    fn test_add_dashpay_profile_with_fee_first_version_apply() {
        let platform_version = PlatformVersion::first();
        let expected_fee_result = FeeResult {
            storage_fee: 1305
                * Epoch::new(0).unwrap().cost_for_known_cost_item(
                    &EPOCH_CHANGE_FEE_VERSION_TEST,
                    StorageDiskUsageCreditPerByte,
                ),
            processing_fee: 900400,
            ..Default::default()
        };

        do_test_add_dashpay_profile_with_fee(true, platform_version, expected_fee_result);
    }

    #[test]
    fn test_add_dashpay_profile_with_fee_first_version_estimated() {
        let platform_version = PlatformVersion::first();
        let expected_fee_result = FeeResult {
            storage_fee: 1305
                * Epoch::new(0).unwrap().cost_for_known_cost_item(
                    &EPOCH_CHANGE_FEE_VERSION_TEST,
                    StorageDiskUsageCreditPerByte,
                ),
            processing_fee: 73253660,
            ..Default::default()
        };

        do_test_add_dashpay_profile_with_fee(false, platform_version, expected_fee_result);
    }

    #[test]
    fn test_add_dashpay_profile_with_fee_latest_version_apply() {
        let platform_version = PlatformVersion::latest();
        let expected_fee_result = FeeResult {
            storage_fee: 1305
                * Epoch::new(0).unwrap().cost_for_known_cost_item(
                    &EPOCH_CHANGE_FEE_VERSION_TEST,
                    StorageDiskUsageCreditPerByte,
                ),
            processing_fee: 900400,
            ..Default::default()
        };

        do_test_add_dashpay_profile_with_fee(true, platform_version, expected_fee_result);
    }

    #[test]
    fn test_add_dashpay_profile_with_fee_latest_version_estimated() {
        let platform_version = PlatformVersion::latest();
        let expected_fee_result = FeeResult {
            storage_fee: 1305
                * Epoch::new(0).unwrap().cost_for_known_cost_item(
                    &EPOCH_CHANGE_FEE_VERSION_TEST,
                    StorageDiskUsageCreditPerByte,
                ),
            // estimated_size v1 adds the contract-version stamp varint to
            // the worst-case document size
            processing_fee: 73323060,
            ..Default::default()
        };

        do_test_add_dashpay_profile_with_fee(false, platform_version, expected_fee_result);
    }

    /// This helper sets up the environment, adds a dashpay profile document,
    /// and either applies or just estimates the cost.
    ///
    /// `apply`: if true, we commit the transaction (applying the changes).
    ///          if false, we do not commit, so changes are only estimated.
    /// `platform_version`: which PlatformVersion to use.
    /// `expected_fee_result`: the FeeResult we expect in the test assertion.
    fn do_test_add_dashpay_profile_with_fee(
        apply: bool,
        platform_version: &PlatformVersion,
        expected_fee_result: FeeResult,
    ) {
        let drive = setup_drive_with_initial_state_structure(None);

        let db_transaction = drive.grove.start_transaction();

        // Setup contract
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
            .document_type_for_name("profile")
            .expect("expected to get document type");

        let random_owner_id = random::<[u8; 32]>();

        // Build dashpay profile doc
        let dashpay_profile_document = json_document_to_document(
            "tests/supporting_files/contract/dashpay/profile0.json",
            Some(random_owner_id.into()),
            document_type,
            platform_version,
        )
        .expect("expected to get cbor document");

        // Perform the add operation, with either an apply or a dry run
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
                false, // override
                BlockInfo::default(),
                apply,
                Some(&db_transaction),
                platform_version,
                None,
            )
            .expect("expected to insert a document successfully");
        assert_eq!(fee_result, expected_fee_result);
    }

    #[test]
    fn test_unknown_state_cost_dashpay_fee_for_add_documents() {
        let drive = setup_drive_with_initial_state_structure(None);

        let db_transaction = drive.grove.start_transaction();

        let platform_version = PlatformVersion::latest();

        let contract = setup_contract(
            &drive,
            "tests/supporting_files/contract/dashpay/dashpay-contract-all-mutable.json",
            None,
            None,
            None::<fn(&mut DataContract)>,
            Some(&db_transaction),
            None,
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
        let drive = setup_drive_with_initial_state_structure(None);

        let db_transaction = drive.grove.start_transaction();

        let platform_version = PlatformVersion::latest();

        let contract = setup_contract(
            &drive,
            "tests/supporting_files/contract/dashpay/dashpay-contract-all-mutable.json",
            None,
            None,
            None::<fn(&mut DataContract)>,
            Some(&db_transaction),
            None,
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
        let drive = setup_drive_with_initial_state_structure(None);

        let db_transaction = drive.grove.start_transaction();

        let platform_version = PlatformVersion::latest();

        let contract = setup_contract(
            &drive,
            "tests/supporting_files/contract/dpns/dpns-contract.json",
            None,
            None,
            None::<fn(&mut DataContract)>,
            Some(&db_transaction),
            None,
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
                storage_fee: 1840
                    * Epoch::new(0).unwrap().cost_for_known_cost_item(
                        &EPOCH_CHANGE_FEE_VERSION_TEST,
                        StorageDiskUsageCreditPerByte,
                    ),
                processing_fee: 1264300,
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

    #[test]
    fn test_add_document_by_contract_id() {
        let drive = setup_drive_with_initial_state_structure(None);

        let db_transaction = drive.grove.start_transaction();

        let platform_version = PlatformVersion::latest();

        let contract = setup_contract(
            &drive,
            "tests/supporting_files/contract/dashpay/dashpay-contract-all-mutable.json",
            None,
            None,
            None::<fn(&mut DataContract)>,
            Some(&db_transaction),
            None,
        );

        let document_type = contract
            .document_type_for_name("profile")
            .expect("expected to get document type");

        let random_owner_id = random::<[u8; 32]>();

        let dashpay_profile_document = json_document_to_document(
            "tests/supporting_files/contract/dashpay/profile0.json",
            Some(random_owner_id.into()),
            document_type,
            platform_version,
        )
        .expect("expected to get document");

        let owned_document_info = OwnedDocumentInfo {
            document_info: DocumentRefInfo((
                &dashpay_profile_document,
                StorageFlags::optional_default_as_cow(),
            )),
            owner_id: Some(random_owner_id),
        };

        // Use the add_document API which takes contract_id instead of contract ref
        let fee_result = drive
            .add_document(
                owned_document_info,
                contract.id(),
                "profile",
                false,
                &BlockInfo::default(),
                true,
                Some(&db_transaction),
                platform_version,
            )
            .expect("expected to insert a document via add_document successfully");

        assert!(fee_result.storage_fee > 0);
        assert!(fee_result.processing_fee > 0);

        // Fetch the document back and verify content matches
        let sql_string = "select * from profile";
        let query = DriveDocumentQuery::from_sql_expr(
            sql_string,
            &contract,
            Some(&DriveConfig::default()),
            platform_version,
        )
        .expect("should build query");

        let (results, _, _) = query
            .execute_raw_results_no_proof(&drive, None, Some(&db_transaction), platform_version)
            .expect("expected to execute query");

        assert_eq!(results.len(), 1);

        let document_type = contract
            .document_type_for_name("profile")
            .expect("expected profile document type");
        let fetched_doc = Document::from_bytes(&results[0], document_type, platform_version)
            .expect("expected to deserialize document");
        assert_eq!(
            fetched_doc
                .get("displayName")
                .expect("displayName should exist")
                .as_text()
                .expect("displayName should be text"),
            "sam",
            "displayName should match the original value from profile0.json"
        );
    }

    #[test]
    fn test_add_document_with_history() {
        let drive = setup_drive_with_initial_state_structure(None);

        let db_transaction = drive.grove.start_transaction();

        let platform_version = PlatformVersion::latest();

        let contract = setup_contract(
            &drive,
            "tests/supporting_files/contract/family/family-contract-with-history.json",
            None,
            None,
            None::<fn(&mut DataContract)>,
            Some(&db_transaction),
            None,
        );

        let document_type = contract
            .document_type_for_name("person")
            .expect("expected to get document type");

        let random_owner_id = random::<[u8; 32]>();

        let person_document = json_document_to_document(
            "tests/supporting_files/contract/family/person0.json",
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
                        document_info: DocumentRefInfo((&person_document, storage_flags)),
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
            .expect("expected to insert a history-keeping document successfully");

        assert!(fee_result.storage_fee > 0);

        // Now add a second document with history
        let person_document1 = json_document_to_document(
            "tests/supporting_files/contract/family/person1.json",
            Some(random_owner_id.into()),
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
            .expect("expected to insert a second history-keeping document successfully");

        drive
            .grove
            .commit_transaction(db_transaction)
            .unwrap()
            .expect("unable to commit transaction");

        // Fetch both documents back and verify they exist with correct content
        let sql_string = "select * from person order by firstName asc limit 100";
        let query = DriveDocumentQuery::from_sql_expr(
            sql_string,
            &contract,
            Some(&DriveConfig::default()),
            platform_version,
        )
        .expect("should build query");

        let (results, _, _) = query
            .execute_raw_results_no_proof(&drive, None, None, platform_version)
            .expect("expected to execute query");

        assert_eq!(
            results.len(),
            2,
            "expected both history-keeping documents to be present"
        );

        let doc0 = Document::from_bytes(&results[0], document_type, platform_version)
            .expect("expected to deserialize first document");
        let doc1 = Document::from_bytes(&results[1], document_type, platform_version)
            .expect("expected to deserialize second document");

        // Results are ordered by firstName ascending: Samuel, Tom
        assert_eq!(
            doc0.get("firstName")
                .expect("firstName should exist")
                .as_text()
                .expect("firstName should be text"),
            "Samuel"
        );
        assert_eq!(
            doc1.get("firstName")
                .expect("firstName should exist")
                .as_text()
                .expect("firstName should be text"),
            "Tom"
        );
    }

    #[test]
    fn test_add_document_with_history_estimated_costs() {
        let drive = setup_drive_with_initial_state_structure(None);

        let db_transaction = drive.grove.start_transaction();

        let platform_version = PlatformVersion::latest();

        let contract = setup_contract(
            &drive,
            "tests/supporting_files/contract/family/family-contract-with-history.json",
            None,
            None,
            None::<fn(&mut DataContract)>,
            Some(&db_transaction),
            None,
        );

        let document_type = contract
            .document_type_for_name("person")
            .expect("expected to get document type");

        let random_owner_id = random::<[u8; 32]>();

        let person_document = json_document_to_document(
            "tests/supporting_files/contract/family/person0.json",
            Some(random_owner_id.into()),
            document_type,
            platform_version,
        )
        .expect("expected to get document");

        let storage_flags = Some(Cow::Owned(StorageFlags::SingleEpoch(0)));

        // apply=false for estimation path of history-keeping documents
        let fee_result = drive
            .add_document_for_contract(
                DocumentAndContractInfo {
                    owned_document_info: OwnedDocumentInfo {
                        document_info: DocumentRefInfo((&person_document, storage_flags)),
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
            .expect("expected to estimate insert of a history-keeping document");

        // Estimation should have storage fee and processing fee
        assert!(fee_result.storage_fee > 0);
        assert!(fee_result.processing_fee > 0);
    }

    #[test]
    fn test_add_document_for_contract_with_non_unique_batch() {
        let drive = setup_drive_with_initial_state_structure(None);

        let db_transaction = drive.grove.start_transaction();

        let platform_version = PlatformVersion::latest();

        let contract = setup_contract(
            &drive,
            "tests/supporting_files/contract/dashpay/dashpay-contract-all-mutable.json",
            None,
            None,
            None::<fn(&mut DataContract)>,
            Some(&db_transaction),
            None,
        );

        let document_type = contract
            .document_type_for_name("contactRequest")
            .expect("expected to get document type");

        let random_owner_id = random::<[u8; 32]>();

        let document0 = json_document_to_document(
            "tests/supporting_files/contract/dashpay/contact-request0.json",
            Some(random_owner_id.into()),
            document_type,
            platform_version,
        )
        .expect("expected to get document");

        let document1 = json_document_to_document(
            "tests/supporting_files/contract/dashpay/contact-request1.json",
            Some(random_owner_id.into()),
            document_type,
            platform_version,
        )
        .expect("expected to get document");

        // Insert first document setting document_is_unique_for_document_type_in_batch=true
        let mut drive_operations = vec![];
        drive
            .add_document_for_contract_apply_and_add_to_operations(
                DocumentAndContractInfo {
                    owned_document_info: OwnedDocumentInfo {
                        document_info: DocumentRefInfo((
                            &document0,
                            StorageFlags::optional_default_as_cow(),
                        )),
                        owner_id: Some(random_owner_id),
                    },
                    contract: &contract,
                    document_type,
                },
                false,
                &BlockInfo::default(),
                true, // unique in batch
                true,
                Some(&db_transaction),
                &mut drive_operations,
                platform_version,
            )
            .expect("expected first document insertion to succeed");

        // Insert second document with document_is_unique_for_document_type_in_batch=false
        // This exercises the else branch in apply_and_add_to_operations_v0
        drive
            .add_document_for_contract_apply_and_add_to_operations(
                DocumentAndContractInfo {
                    owned_document_info: OwnedDocumentInfo {
                        document_info: DocumentRefInfo((
                            &document1,
                            StorageFlags::optional_default_as_cow(),
                        )),
                        owner_id: Some(random_owner_id),
                    },
                    contract: &contract,
                    document_type,
                },
                false,
                &BlockInfo::default(),
                false, // not unique in batch - exercises else branch
                true,
                Some(&db_transaction),
                &mut drive_operations,
                platform_version,
            )
            .expect("expected second document insertion to succeed");

        // Verify both documents were inserted by fetching them
        let sql_string = "select * from contactRequest";
        let query = DriveDocumentQuery::from_sql_expr(
            sql_string,
            &contract,
            Some(&DriveConfig::default()),
            platform_version,
        )
        .expect("should build query");

        let (results, _, _) = query
            .execute_raw_results_no_proof(&drive, None, Some(&db_transaction), platform_version)
            .expect("expected to execute query");

        assert_eq!(
            results.len(),
            2,
            "expected both documents to be present after batch insertion"
        );
    }

    // ---------- Error-path tests (added for coverage) ----------

    #[test]
    fn test_add_document_with_nonexistent_contract_id_returns_data_contract_not_found() {
        // `add_document` takes a contract_id and must resolve it internally.
        // Supplying a random id that does not exist should fail with
        // DocumentError::DataContractNotFound (instead of, e.g., a panic or
        // lower-level grovedb error).
        use crate::error::document::DocumentError;

        let (drive, contract) = setup_dashpay("add-doc-nonexistent-contract", true);

        let platform_version = PlatformVersion::latest();

        let document_type = contract
            .document_type_for_name("profile")
            .expect("profile document exists");

        let random_owner_id = random::<[u8; 32]>();

        let document = json_document_to_document(
            "tests/supporting_files/contract/dashpay/profile0.json",
            Some(random_owner_id.into()),
            document_type,
            platform_version,
        )
        .expect("expected to get document");

        let nonexistent_contract_id: [u8; 32] = random();

        let err = drive
            .add_document(
                OwnedDocumentInfo {
                    document_info: DocumentRefInfo((
                        &document,
                        StorageFlags::optional_default_as_cow(),
                    )),
                    owner_id: Some(random_owner_id),
                },
                nonexistent_contract_id.into(),
                "profile",
                false,
                &BlockInfo::default(),
                true,
                None,
                platform_version,
            )
            .expect_err("expected add_document with nonexistent contract id to fail");

        assert!(
            matches!(err, Error::Document(DocumentError::DataContractNotFound)),
            "expected DataContractNotFound, got {err:?}"
        );
    }

    #[test]
    fn test_add_document_with_invalid_document_type_name_returns_error() {
        // `add_document` should fail when given a document type name that does
        // not exist in the contract (contract.document_type_for_name(...) ?).
        let (drive, contract) = setup_dashpay("add-doc-bad-type", true);

        let platform_version = PlatformVersion::latest();

        let document_type = contract
            .document_type_for_name("profile")
            .expect("profile document exists");

        let random_owner_id = random::<[u8; 32]>();

        let document = json_document_to_document(
            "tests/supporting_files/contract/dashpay/profile0.json",
            Some(random_owner_id.into()),
            document_type,
            platform_version,
        )
        .expect("expected to get document");

        let err = drive
            .add_document(
                OwnedDocumentInfo {
                    document_info: DocumentRefInfo((
                        &document,
                        StorageFlags::optional_default_as_cow(),
                    )),
                    owner_id: Some(random_owner_id),
                },
                contract.id(),
                "not_a_real_document_type",
                false,
                &BlockInfo::default(),
                true,
                None,
                platform_version,
            )
            .expect_err("expected add_document with bad document type to fail");

        // The error originates from DataContract::document_type_for_name, which
        // maps through the `?` into an Error. We don't match a specific variant
        // because that depends on dpp's mapping, but we verify it's not a
        // contract-not-found error (which would be the other likely branch).
        assert!(
            !matches!(
                err,
                Error::Document(crate::error::document::DocumentError::DataContractNotFound)
            ),
            "expected a document-type error, not DataContractNotFound: {err:?}"
        );
    }
}
