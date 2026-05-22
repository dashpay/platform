mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use dpp::data_contract::document_type::DocumentTypeRef;
use dpp::document::Document;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;
use std::collections::BTreeMap;

impl Drive {
    /// Fetches the historical revisions of a document that keeps history.
    #[allow(clippy::too_many_arguments)]
    pub fn fetch_document_history(
        &self,
        contract_id: [u8; 32],
        document_type_name: &str,
        document_type: DocumentTypeRef,
        document_id: [u8; 32],
        transaction: TransactionArg,
        start_at_ms: u64,
        limit: Option<u16>,
        offset: Option<u16>,
        platform_version: &PlatformVersion,
    ) -> Result<BTreeMap<u64, Document>, Error> {
        match platform_version
            .drive
            .methods
            .document
            .query
            .fetch_document_history
        {
            0 => self.fetch_document_history_v0(
                contract_id,
                document_type_name,
                document_type,
                document_id,
                transaction,
                start_at_ms,
                limit,
                offset,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "fetch_document_history".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::object_size_info::DocumentInfo::DocumentRefInfo;
    use crate::util::object_size_info::{DocumentAndContractInfo, OwnedDocumentInfo};
    use crate::util::storage_flags::StorageFlags;
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use dpp::block::block_info::BlockInfo;
    use dpp::data_contract::accessors::v0::DataContractV0Getters;
    use dpp::document::{DocumentV0Getters, DocumentV0Setters};
    use dpp::tests::json_document::{json_document_to_contract, json_document_to_document};
    use dpp::tests::utils::generate_random_identifier_struct;

    const DOCUMENT_TYPE_NAME: &str = "profile";

    fn setup_history_document() -> (Drive, dpp::prelude::DataContract, dpp::document::Document) {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();
        let contract = json_document_to_contract(
            "tests/supporting_files/contract/dashpay/dashpay-contract-with-profile-history.json",
            false,
            platform_version,
        )
        .expect("expected contract");
        drive
            .apply_contract(
                &contract,
                BlockInfo::default(),
                true,
                StorageFlags::optional_default_as_cow(),
                None,
                platform_version,
            )
            .expect("apply contract");

        let document_type = contract
            .document_type_for_name(DOCUMENT_TYPE_NAME)
            .expect("profile document type");
        let document = json_document_to_document(
            "tests/supporting_files/contract/dashpay/profile0.json",
            Some(generate_random_identifier_struct()),
            document_type,
            platform_version,
        )
        .expect("expected document");

        (drive, contract, document)
    }

    fn put_document(
        drive: &Drive,
        contract: &dpp::prelude::DataContract,
        document: &dpp::document::Document,
        time_ms: u64,
    ) {
        let platform_version = PlatformVersion::latest();
        let document_type = contract
            .document_type_for_name(DOCUMENT_TYPE_NAME)
            .expect("profile document type");
        drive
            .add_document_for_contract(
                DocumentAndContractInfo {
                    owned_document_info: OwnedDocumentInfo {
                        document_info: DocumentRefInfo((
                            document,
                            StorageFlags::optional_default_as_cow(),
                        )),
                        owner_id: None,
                    },
                    contract,
                    document_type,
                },
                true,
                BlockInfo::default_with_time(time_ms),
                true,
                None,
                platform_version,
                None,
            )
            .expect("put document");
    }

    #[test]
    fn should_fetch_document_history_in_time_order_with_pagination() {
        let (drive, contract, mut document) = setup_history_document();
        let platform_version = PlatformVersion::latest();
        let contract_id = contract.id().to_buffer();
        let document_id = document.id().to_buffer();
        let document_type = contract
            .document_type_for_name(DOCUMENT_TYPE_NAME)
            .expect("profile document type");

        put_document(&drive, &contract, &document, 1000);
        document.set("displayName", "Alice 2".into());
        put_document(&drive, &contract, &document, 2000);
        document.set("displayName", "Alice 3".into());
        put_document(&drive, &contract, &document, 3000);

        let history = drive
            .fetch_document_history(
                contract_id,
                DOCUMENT_TYPE_NAME,
                document_type,
                document_id,
                None,
                0,
                None,
                None,
                platform_version,
            )
            .expect("fetch history");
        assert_eq!(
            history.keys().copied().collect::<Vec<_>>(),
            vec![1000, 2000, 3000]
        );

        let page = drive
            .fetch_document_history(
                contract_id,
                DOCUMENT_TYPE_NAME,
                document_type,
                document_id,
                None,
                1000,
                Some(1),
                None,
                platform_version,
            )
            .expect("fetch page");
        assert_eq!(page.keys().copied().collect::<Vec<_>>(), vec![2000]);

        let empty_page = drive
            .fetch_document_history(
                contract_id,
                DOCUMENT_TYPE_NAME,
                document_type,
                document_id,
                None,
                3000,
                Some(10),
                None,
                platform_version,
            )
            .expect("fetch empty page");
        assert!(empty_page.is_empty());
    }

    #[test]
    fn should_prove_and_verify_document_history() {
        let (drive, contract, mut document) = setup_history_document();
        let platform_version = PlatformVersion::latest();
        let contract_id = contract.id().to_buffer();
        let document_id = document.id().to_buffer();
        let document_type = contract
            .document_type_for_name(DOCUMENT_TYPE_NAME)
            .expect("profile document type");

        put_document(&drive, &contract, &document, 1000);
        document.set("displayName", "Alice 2".into());
        put_document(&drive, &contract, &document, 2000);

        let proof = drive
            .prove_document_history(
                contract_id,
                DOCUMENT_TYPE_NAME,
                document_id,
                None,
                0,
                Some(10),
                None,
                platform_version,
            )
            .expect("prove history");
        let (_root_hash, history) = Drive::verify_document_history(
            &proof,
            contract_id,
            DOCUMENT_TYPE_NAME,
            document_type,
            document_id,
            0,
            Some(10),
            None,
            platform_version,
        )
        .expect("verify history");

        let history = history.expect("history exists");
        assert_eq!(
            history.keys().copied().collect::<Vec<_>>(),
            vec![1000, 2000]
        );

        let empty_page_proof = drive
            .prove_document_history(
                contract_id,
                DOCUMENT_TYPE_NAME,
                document_id,
                None,
                2000,
                Some(10),
                None,
                platform_version,
            )
            .expect("prove empty page");
        let (_root_hash, empty_history) = Drive::verify_document_history(
            &empty_page_proof,
            contract_id,
            DOCUMENT_TYPE_NAME,
            document_type,
            document_id,
            2000,
            Some(10),
            None,
            platform_version,
        )
        .expect("verify empty history page");

        assert!(empty_history.expect("empty history page exists").is_empty());
    }
}
