use crate::drive::document::history::{invalid, DocumentHistoryQueryV1, DocumentHistoryV1};
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use dpp::data_contract::document_type::DocumentTypeRef;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

impl Drive {
    /// Fetches a page of a historical document's retained revisions with its
    /// lifecycle, through the method version the protocol selects.
    ///
    /// The timestamp-keyed layout that earlier protocols wrote has no reader
    /// any more: no released network stores such history, and the protocol
    /// 14 migration leaves nothing for one to read.
    pub fn fetch_document_history(
        &self,
        query: &DocumentHistoryQueryV1,
        document_type: DocumentTypeRef,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<DocumentHistoryV1, Error> {
        match platform_version
            .drive
            .methods
            .document
            .query
            .fetch_document_history
        {
            1 => self.fetch_document_history_v1_impl(
                query,
                document_type,
                transaction,
                platform_version,
            ),
            0 => Err(invalid(
                "document history is served from protocol version 14",
            )),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "fetch_document_history".to_owned(),
                known_versions: vec![1],
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
                None,
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
                    None,
                )
                .expect("replace document in the same block");
        }

        let mut history_path =
            contract_document_type_path_vec(contract.id().as_slice(), DOCUMENT_TYPE_NAME);
        let history_key = if platform_version
            .drive
            .methods
            .document
            .insert
            .add_document_to_primary_storage
            == 0
        {
            0
        } else {
            2
        };
        history_path.extend([vec![history_key], document.id().to_vec()]);
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
