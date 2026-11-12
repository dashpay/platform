mod v0;
mod v1;

use crate::drive::Drive;
use dpp::document::Document;
use std::collections::BTreeMap;

use crate::error::drive::DriveError;
use crate::error::Error;
use crate::query::document_history_drive_query::{
    DocumentHistoryDriveQuery, DocumentHistoryDriveQueryExecutionResult,
};
use dpp::data_contract::document_type::DocumentTypeRef;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

impl Drive {
    /// Reads the per-document timestamp-keyed history used through protocol version 14.
    ///
    /// # Parameters
    /// `start_after_ms` excludes matching timestamps; `offset` optionally skips entries after
    /// that bound for unproved queries. The platform version must select the timestamp-keyed reader.
    ///
    /// # Returns
    /// The history result, or an error for an unsupported reader generation.
    #[allow(clippy::too_many_arguments)]
    pub fn fetch_document_history_after_time(
        &self,
        contract_id: [u8; 32],
        document_type: DocumentTypeRef,
        document_id: [u8; 32],
        transaction: TransactionArg,
        start_after_ms: u64,
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
                document_type,
                document_id,
                transaction,
                start_after_ms,
                limit,
                offset,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "fetch_document_history_after_time".to_owned(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }

    /// Fetches a page and lifecycle from the per-type composite-keyed history tree.
    pub fn fetch_document_history(
        &self,
        query: &DocumentHistoryDriveQuery,
        document_type: DocumentTypeRef,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<DocumentHistoryDriveQueryExecutionResult, Error> {
        match platform_version
            .drive
            .methods
            .document
            .query
            .fetch_document_history
        {
            1 => {
                self.fetch_document_history_v1(query, document_type, transaction, platform_version)
            }
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "fetch_document_history".to_string(),
                known_versions: vec![1],
                received: version,
            })),
        }
    }
}

#[cfg(test)]
mod after_time_tests {
    use super::*;
    use crate::query::document_history_drive_query::DocumentHistoryFilter;
    use crate::util::object_size_info::DocumentInfo::DocumentRefInfo;
    use crate::util::object_size_info::{DocumentAndContractInfo, OwnedDocumentInfo};
    use crate::util::storage_flags::StorageFlags;
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use dpp::block::block_info::BlockInfo;
    use dpp::data_contract::accessors::v0::DataContractV0Getters;
    use dpp::document::{DocumentV0Getters, DocumentV0Setters};
    use dpp::tests::json_document::{json_document_to_contract, json_document_to_document};
    use dpp::tests::utils::generate_random_identifier_struct;
    use dpp::version::FeatureVersion;

    const DOCUMENT_TYPE_NAME: &str = "profile";

    fn setup_history_document() -> (Drive, dpp::prelude::DataContract, dpp::document::Document) {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::get(14).expect("protocol 14");
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
        let platform_version = PlatformVersion::get(14).expect("protocol 14");
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
            )
            .expect("put document");
    }

    #[test]
    fn should_fetch_document_history_after_an_exclusive_time_with_pagination() {
        let (drive, contract, mut document) = setup_history_document();
        let version = PlatformVersion::get(14).expect("protocol 14");
        let document_type = contract.document_type_for_name(DOCUMENT_TYPE_NAME).unwrap();
        for time in [1000, 2000, 3000] {
            document.set("displayName", format!("Alice {time}").into());
            put_document(&drive, &contract, &document, time);
        }
        for (start_after_ms, limit, offset, expected) in [
            (0, None, None, vec![1000, 2000, 3000]),
            (1000, Some(1), None, vec![2000]),
            (2000, Some(10), None, vec![3000]),
            (3000, Some(10), None, vec![]),
            (1000, Some(10), Some(1), vec![3000]),
        ] {
            let history = drive
                .fetch_document_history_after_time(
                    contract.id().to_buffer(),
                    document_type,
                    document.id().to_buffer(),
                    None,
                    start_after_ms,
                    limit,
                    offset,
                    version,
                )
                .expect("fetch timestamp-keyed history");
            assert_eq!(history.keys().copied().collect::<Vec<_>>(), expected);
        }
    }

    #[test]
    fn should_prove_and_verify_document_history_after_an_exclusive_time() {
        let (drive, contract, mut document) = setup_history_document();
        let version = PlatformVersion::get(14).expect("protocol 14");
        let document_type = contract.document_type_for_name(DOCUMENT_TYPE_NAME).unwrap();
        put_document(&drive, &contract, &document, 1000);
        document.set("displayName", "Alice 2".into());
        put_document(&drive, &contract, &document, 2000);
        for (start_after_ms, expected) in
            [(0, vec![1000, 2000]), (1000, vec![2000]), (2000, vec![])]
        {
            let proof = drive
                .prove_document_history_after_time(
                    contract.id().to_buffer(),
                    document_type,
                    document.id().to_buffer(),
                    None,
                    start_after_ms,
                    Some(10),
                    None,
                    version,
                )
                .expect("prove timestamp-keyed history");
            let (_, history) = Drive::verify_document_history_after_time(
                &proof,
                contract.id().to_buffer(),
                DOCUMENT_TYPE_NAME,
                document_type,
                document.id().to_buffer(),
                start_after_ms,
                Some(10),
                None,
                version,
            )
            .expect("verify timestamp-keyed history");
            assert_eq!(
                history.unwrap().keys().copied().collect::<Vec<_>>(),
                expected
            );
        }
    }

    fn assert_version_mismatch(
        error: Error,
        expected_method: &str,
        expected_version: FeatureVersion,
        received_version: FeatureVersion,
    ) {
        assert!(
            matches!(error, Error::Drive(DriveError::UnknownVersionMismatch {
            method, known_versions, received,
        }) if method == expected_method && known_versions == vec![expected_version] && received == received_version)
        );
    }

    #[test]
    fn should_refuse_composite_history_queries_with_timestamp_method_versions() {
        let (drive, contract, document) = setup_history_document();
        let version = PlatformVersion::get(14).expect("protocol 14");
        let document_type = contract.document_type_for_name(DOCUMENT_TYPE_NAME).unwrap();
        for filter in [
            DocumentHistoryFilter::StartAtTime(0),
            DocumentHistoryFilter::StartAtRevision(1),
        ] {
            let query = DocumentHistoryDriveQuery {
                contract_id: contract.id().to_buffer(),
                document_type_name: DOCUMENT_TYPE_NAME.into(),
                document_id: document.id().to_buffer(),
                filter,
                limit: None,
            };
            assert_version_mismatch(
                drive
                    .fetch_document_history(&query, document_type, None, version)
                    .unwrap_err(),
                "fetch_document_history",
                1,
                0,
            );
            assert_version_mismatch(
                drive
                    .prove_document_history(&query, document_type, None, version)
                    .unwrap_err(),
                "prove_document_history",
                1,
                0,
            );
            assert_version_mismatch(
                Drive::verify_document_history(&query, &[], document_type, version).unwrap_err(),
                "verify_document_history",
                1,
                0,
            );
        }
    }

    #[test]
    fn should_refuse_timestamp_history_reads_with_composite_method_versions() {
        let (drive, contract, document) = setup_history_document();
        let version = PlatformVersion::latest();
        let document_type = contract.document_type_for_name(DOCUMENT_TYPE_NAME).unwrap();
        assert_version_mismatch(
            drive
                .fetch_document_history_after_time(
                    contract.id().to_buffer(),
                    document_type,
                    document.id().to_buffer(),
                    None,
                    0,
                    None,
                    None,
                    version,
                )
                .unwrap_err(),
            "fetch_document_history_after_time",
            0,
            1,
        );
        assert_version_mismatch(
            drive
                .prove_document_history_after_time(
                    contract.id().to_buffer(),
                    document_type,
                    document.id().to_buffer(),
                    None,
                    0,
                    None,
                    None,
                    version,
                )
                .unwrap_err(),
            "prove_document_history_after_time",
            0,
            1,
        );
        assert_version_mismatch(
            Drive::verify_document_history_after_time(
                &[1],
                contract.id().to_buffer(),
                DOCUMENT_TYPE_NAME,
                document_type,
                document.id().to_buffer(),
                0,
                None,
                None,
                version,
            )
            .unwrap_err(),
            "verify_document_history_after_time",
            0,
            1,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::drive::document::paths::DOCUMENT_HISTORY_TREE_KEY;
    use crate::util::object_size_info::DocumentInfo::DocumentRefInfo;
    use crate::util::object_size_info::{DocumentAndContractInfo, OwnedDocumentInfo};
    use crate::util::storage_flags::StorageFlags;
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use dpp::block::block_info::BlockInfo;
    use dpp::data_contract::accessors::v0::DataContractV0Getters;
    use dpp::document::Document;
    use dpp::document::{DocumentV0Getters, DocumentV0Setters};
    use dpp::tests::json_document::{json_document_to_contract, json_document_to_document};
    use dpp::tests::utils::generate_random_identifier_struct;

    const DOCUMENT_TYPE_NAME: &str = "profile";

    fn setup_history_document(
        platform_version: &PlatformVersion,
    ) -> (Drive, dpp::prelude::DataContract, dpp::document::Document) {
        let drive = setup_drive_with_initial_state_structure(None);
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
        platform_version: &PlatformVersion,
    ) {
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
            )
            .expect("put document");
    }

    #[test]
    fn should_retain_both_replacements_in_the_same_block() {
        use crate::drive::document::paths::contract_document_type_path_vec;
        use crate::util::common::encode::encode_u64;
        use dpp::document::serialization_traits::DocumentPlatformConversionMethodsV0;
        use grovedb::query_result_type::{QueryResultElement, QueryResultType};
        use grovedb::{Element, PathQuery, Query, SizedQuery};

        let platform_version = PlatformVersion::latest();
        let (drive, contract, mut document) = setup_history_document(platform_version);
        let document_type = contract
            .document_type_for_name(DOCUMENT_TYPE_NAME)
            .expect("profile document type");
        document.set_revision(Some(1));
        put_document(&drive, &contract, &document, 1000, platform_version);

        for revision in [2, 3] {
            document.set_revision(Some(revision));
            document.set("displayName", format!("Revision {revision}").into());
            drive
                .update_document_for_contract(
                    &document,
                    &contract,
                    document_type,
                    Some(document.owner_id().to_buffer()),
                    BlockInfo::default_with_time(2000),
                    true,
                    StorageFlags::optional_default_as_cow(),
                    None,
                    platform_version,
                )
                .expect("replace document in the same block");
        }

        let mut history_path =
            contract_document_type_path_vec(contract.id().as_slice(), DOCUMENT_TYPE_NAME);
        history_path.extend([vec![DOCUMENT_HISTORY_TREE_KEY], document.id().to_vec()]);
        let mut query = Query::new();
        query.insert_range_from(encode_u64(0)..);
        let (results, _) = drive
            .grove_get_path_query(
                &PathQuery::new(history_path, SizedQuery::new(query, None, None)),
                None,
                QueryResultType::QueryKeyElementPairResultType,
                &mut Vec::new(),
                &platform_version.drive,
            )
            .expect("read retained revision bodies");
        let revisions = results
            .elements
            .into_iter()
            .map(|entry| match entry {
                QueryResultElement::KeyElementPairResultItem((_, Element::Item(bytes, _))) => {
                    Document::from_bytes(&bytes, document_type, platform_version)
                        .expect("deserialize retained revision")
                        .revision()
                        .expect("mutable document has a revision")
                }
                _ => panic!("history must contain document items"),
            })
            .collect::<Vec<_>>();
        assert_eq!(
            revisions,
            vec![1, 2, 3],
            "every accepted edit must remain readable"
        );
    }
}
