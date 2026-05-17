use crate::verify::RootHash;

use crate::error::proof::ProofError;
use crate::error::Error;
use crate::query::DriveDocumentQuery;
use dpp::document::serialization_traits::DocumentPlatformConversionMethodsV0;
use dpp::document::Document;
use dpp::version::PlatformVersion;
use grovedb::{GroveDb, PathQuery};

impl DriveDocumentQuery<'_> {
    /// Verifies if a document exists at the beginning of a proof,
    /// and returns the root hash and the optionally found document.
    ///
    /// # Arguments
    ///
    /// * `proof` - A byte slice containing the proof data.
    /// * `is_proof_subset` - A boolean indicating whether the proof is a subset query or not.
    /// * `document_id` - A byte_32 array, representing the ID of the document to start at.
    ///
    /// # Returns
    ///
    /// A `Result` with a tuple containing:
    /// * The root hash of the verified proof.
    /// * An `Option<Document>` containing the found document if available.
    ///
    /// # Errors
    ///
    /// This function returns an Error in the following cases:
    /// * If the proof is corrupted (wrong path, wrong key, etc.).
    /// * If the provided proof has an incorrect number of elements.
    #[inline(always)]
    pub(super) fn verify_start_at_document_in_proof_v0(
        &self,
        proof: &[u8],
        is_proof_subset: bool,
        document_id: [u8; 32],
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, Option<Document>), Error> {
        let (start_at_document_path, start_at_document_key) =
            self.start_at_document_path_and_key(&document_id);
        let path_query = PathQuery::new_single_key(
            start_at_document_path.clone(),
            start_at_document_key.clone(),
        );
        let (root_hash, mut proved_key_values) = if is_proof_subset {
            GroveDb::verify_subset_query(proof, &path_query, &platform_version.drive.grove_version)?
        } else {
            GroveDb::verify_query(proof, &path_query, &platform_version.drive.grove_version)?
        };
        match proved_key_values.len() {
            1 => {
                let (path, key, maybe_element) = proved_key_values.remove(0);
                if path != start_at_document_path {
                    return Err(Error::Proof(ProofError::CorruptedProof(
                        "we did not get back a document for the correct path".to_string(),
                    )));
                }
                if key != start_at_document_key {
                    return Err(Error::Proof(ProofError::CorruptedProof(
                        "we did not get back a document for the correct key".to_string(),
                    )));
                }
                let document = maybe_element
                    .map(|element| {
                        let document_bytes = element.into_item_bytes().map_err(Error::from)?;
                        Document::from_bytes(
                            document_bytes.as_slice(),
                            self.document_type,
                            platform_version,
                        )
                        .map_err(Error::from)
                    })
                    .transpose()?;
                Ok((root_hash, document))
            }
            0 => Err(Error::Proof(ProofError::WrongElementCount {
                expected: 1,
                got: 0,
            })),
            _ => Err(Error::Proof(ProofError::TooManyElements(
                "expected one document for start at",
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::query::DriveDocumentQuery;
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
    fn should_prove_and_verify_start_at_document_in_proof() {
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
            .random_document(Some(42), platform_version)
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

        // Build a query - we use any_item_query to get all, then use the proof
        // to verify a specific document via verify_start_at_document_in_proof
        let query = DriveDocumentQuery::any_item_query(&contract, document_type);

        // Get proof that covers all documents
        let (proof, _cost) = query
            .clone()
            .execute_with_proof(&drive, None, None, platform_version)
            .expect("expected to execute query with proof");

        // Verify that the specific document is found in the proof
        let (_root_hash, found_document) = query
            .verify_start_at_document_in_proof(
                proof.as_slice(),
                true, // is_proof_subset = true because the proof covers all documents
                doc_id.to_buffer(),
                platform_version,
            )
            .expect("expected proof verification to succeed");

        assert!(
            found_document.is_some(),
            "expected document to be found in proof"
        );
        let found_doc = found_document.unwrap();
        assert_eq!(found_doc.id(), doc_id);
    }
}
