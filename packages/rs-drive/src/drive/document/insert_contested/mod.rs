//! Insert Documents.
//!
//! This module implements functions in Drive relevant to inserting documents.
//!

// Module: add_contested_document
// This module contains functionality for adding a document
mod add_contested_document;

// Module: add_contested_document_for_contract
// This module contains functionality for adding a document for a given contract
mod add_contested_document_for_contract;

// Module: add_contested_document_for_contract_apply_and_add_to_operations
// This module contains functionality for applying and adding operations for a contract document
mod add_contested_document_for_contract_apply_and_add_to_operations;

// Module: add_contested_document_for_contract_operations
// This module contains functionality for adding a document for contract operations
mod add_contested_document_for_contract_operations;

// Module: add_contested_document_to_primary_storage
// This module contains functionality for adding a document to primary storage
mod add_contested_document_to_primary_storage;

// Module: add_contested_indices_for_index_level_for_contract_operations
// This module contains functionality for adding indices for an index level for contract operations
// mod add_contested_indices_for_index_level_for_contract_operations;

// Module: add_contested_indices_for_top_index_level_for_contract_operations
// This module contains functionality for adding indices for the top index level for contract operations
mod add_contested_indices_for_contract_operations;

// Module: add_contested_reference_and_vote_subtree_to_document_operations
// This module contains functionality for adding a reference for an index level for contract operations
mod add_contested_reference_and_vote_subtree_to_document_operations;
mod add_contested_vote_subtrees_for_non_identities_operations;

#[cfg(all(
    feature = "fixtures-and-mocks",
    feature = "data-contract-cbor-conversion"
))]
use dpp::data_contract::conversion::cbor::DataContractCborConversionMethodsV0;

#[cfg(test)]
mod tests {
    use std::option::Option::None;

    use dpp::block::block_info::BlockInfo;
    use rand::random;

    use crate::drive::document::tests::setup_dashpay;
    use crate::util::object_size_info::{DocumentAndContractInfo, OwnedDocumentInfo};
    use crate::util::storage_flags::StorageFlags;
    use dpp::data_contract::accessors::v0::DataContractV0Getters;

    use crate::util::object_size_info::DocumentInfo::DocumentRefInfo;
    use dpp::tests::json_document::json_document_to_document;
    use dpp::version::PlatformVersion;

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
