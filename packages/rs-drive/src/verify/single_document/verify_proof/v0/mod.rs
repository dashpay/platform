use crate::verify::RootHash;

use crate::error::Error;
use crate::query::SingleDocumentDriveQuery;
use dpp::data_contract::document_type::DocumentTypeRef;
use dpp::document::Document;

use dpp::document::serialization_traits::DocumentPlatformConversionMethodsV0;
use dpp::version::PlatformVersion;

impl SingleDocumentDriveQuery {
    /// Verifies the proof of a single document query.
    ///
    /// `is_subset` indicates if the function should verify a subset of a larger proof.
    ///
    /// # Parameters
    ///
    /// - `is_subset`: A boolean indicating whether to verify a subset of a larger proof.
    /// - `proof`: A byte slice representing the proof to be verified.
    /// - `document_type`: The type of the document being verified.
    ///
    /// # Returns
    ///
    /// Returns a `Result` with a tuple of `RootHash` and `Option<Document>`. The `Option<Document>`
    /// represents the deserialized document if it exists.
    ///
    /// # Errors
    ///
    /// Returns an `Error` if:
    ///
    /// - The proof is corrupted.
    /// - The GroveDb query fails.
    /// - The document serialization fails.
    #[inline(always)]
    pub(super) fn verify_proof_v0(
        &self,
        is_subset: bool,
        proof: &[u8],
        document_type: DocumentTypeRef,
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, Option<Document>), Error> {
        self.verify_proof_keep_serialized(is_subset, proof, platform_version)
            .map(|(root_hash, serialized)| {
                let document = serialized
                    .map(|serialized| {
                        Document::from_bytes(serialized.as_slice(), document_type, platform_version)
                            .map_err(Error::from)
                    })
                    .transpose()?;
                Ok((root_hash, document))
            })?
    }
}

#[cfg(test)]
mod tests {
    use crate::query::{SingleDocumentDriveQuery, SingleDocumentDriveQueryContestedStatus};
    use crate::util::object_size_info::DocumentInfo::DocumentRefInfo;
    use crate::util::object_size_info::{DocumentAndContractInfo, OwnedDocumentInfo};
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use dpp::block::block_info::BlockInfo;
    use dpp::data_contract::accessors::v0::DataContractV0Getters;
    use dpp::data_contract::document_type::random_document::CreateRandomDocument;
    use dpp::data_contracts::SystemDataContract;
    use dpp::document::DocumentV0Getters;
    use dpp::system_data_contracts::load_system_data_contract;
    use dpp::version::PlatformVersion;

    #[test]
    fn should_prove_and_verify_single_document() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        let contract = load_system_data_contract(SystemDataContract::DPNS, platform_version)
            .expect("expected to load DPNS contract");

        drive
            .apply_contract(
                &contract,
                BlockInfo::default(),
                true,
                None,
                None,
                platform_version,
            )
            .expect("expected to apply contract");

        let document_type = contract
            .document_type_for_name("preorder")
            .expect("expected preorder document type");

        // Insert a document
        let document = document_type
            .random_document(Some(99), platform_version)
            .expect("expected a random document");

        let doc_id = document.id();

        drive
            .add_document_for_contract(
                DocumentAndContractInfo {
                    owned_document_info: OwnedDocumentInfo {
                        document_info: DocumentRefInfo((&document, None)),
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
            .expect("expected to insert document");

        // Build single document query
        let single_query = SingleDocumentDriveQuery {
            contract_id: contract.id().to_buffer(),
            document_type_name: "preorder".to_string(),
            document_type_keeps_history: false,
            document_id: doc_id.to_buffer(),
            block_time_ms: None,
            contested_status: SingleDocumentDriveQueryContestedStatus::NotContested,
        };

        // Generate proof
        let path_query = single_query
            .construct_path_query(platform_version)
            .expect("expected to construct path query");
        let proof = drive
            .grove_get_proved_path_query(&path_query, None, &mut vec![], &platform_version.drive)
            .expect("expected to get proof");

        // Verify proof
        let (_root_hash, verified_doc) = single_query
            .verify_proof(false, proof.as_slice(), document_type, platform_version)
            .expect("expected proof verification to succeed");

        assert!(verified_doc.is_some(), "expected document to be found");
        assert_eq!(verified_doc.unwrap().id(), doc_id);
    }

    #[test]
    fn should_prove_and_verify_absent_single_document() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        let contract = load_system_data_contract(SystemDataContract::DPNS, platform_version)
            .expect("expected to load DPNS contract");

        drive
            .apply_contract(
                &contract,
                BlockInfo::default(),
                true,
                None,
                None,
                platform_version,
            )
            .expect("expected to apply contract");

        let document_type = contract
            .document_type_for_name("preorder")
            .expect("expected preorder document type");

        // No document inserted - query for a non-existent doc
        let single_query = SingleDocumentDriveQuery {
            contract_id: contract.id().to_buffer(),
            document_type_name: "preorder".to_string(),
            document_type_keeps_history: false,
            document_id: [1u8; 32],
            block_time_ms: None,
            contested_status: SingleDocumentDriveQueryContestedStatus::NotContested,
        };

        let path_query = single_query
            .construct_path_query(platform_version)
            .expect("expected to construct path query");
        let proof = drive
            .grove_get_proved_path_query(&path_query, None, &mut vec![], &platform_version.drive)
            .expect("expected to get proof");

        let (_root_hash, verified_doc) = single_query
            .verify_proof(false, proof.as_slice(), document_type, platform_version)
            .expect("expected proof verification to succeed");

        assert!(verified_doc.is_none(), "expected document to be absent");
    }
}
