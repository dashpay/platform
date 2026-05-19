use crate::verify::RootHash;

use crate::error::proof::ProofError;
use crate::error::Error;
use crate::query::DriveDocumentQuery;

use dpp::version::PlatformVersion;
use grovedb::GroveDb;

impl DriveDocumentQuery<'_> {
    /// Verifies the given proof and returns the root hash of the GroveDB tree and a vector
    /// of serialized documents if the verification is successful.
    ///
    /// # Arguments
    /// * `proof` - A byte slice representing the proof to be verified.
    ///
    /// # Returns
    /// * On success, returns a tuple containing the root hash of the GroveDB tree and a vector of serialized documents.
    /// * On failure, returns an Error.
    ///
    /// # Errors
    /// This function will return an Error if:
    /// * The start at document is not present in proof and it is expected to be.
    /// * The path query fails to verify against the given proof.
    /// * Converting the element into bytes fails.
    #[inline(always)]
    pub(super) fn verify_proof_keep_serialized_v0(
        &self,
        proof: &[u8],
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, Vec<Vec<u8>>), Error> {
        let path_query = if let Some(start_at) = &self.start_at {
            let (_, start_document) =
                self.verify_start_at_document_in_proof(proof, true, *start_at, platform_version)?;
            let document = start_document.ok_or(Error::Proof(ProofError::IncompleteProof(
                "expected start at document to be present in proof",
            )))?;
            self.construct_path_query(Some(document), platform_version)
        } else {
            self.construct_path_query(None, platform_version)
        }?;

        let (root_hash, proved_key_values) = if self.start_at.is_some() {
            GroveDb::verify_subset_query(proof, &path_query, &platform_version.drive.grove_version)?
        } else {
            GroveDb::verify_query(proof, &path_query, &platform_version.drive.grove_version)?
        };

        let documents = proved_key_values
            .into_iter()
            .filter_map(|(_path, _key, element)| element)
            .map(|element| element.into_item_bytes().map_err(Error::from))
            .collect::<Result<Vec<Vec<u8>>, Error>>()?;
        Ok((root_hash, documents))
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
    use dpp::system_data_contracts::load_system_data_contract;
    use dpp::version::PlatformVersion;

    #[test]
    fn should_prove_and_verify_keep_serialized_document_collection() {
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

        // Insert some documents
        for seed in 1u64..=2 {
            let document = document_type
                .random_document(Some(seed), platform_version)
                .expect("expected a random document");

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
        }

        // Build a query that fetches all documents (any_item_query has limit=1)
        let query = DriveDocumentQuery::all_items_query(&contract, document_type, None);

        // Get proof
        let (proof, _cost) = query
            .clone()
            .execute_with_proof(&drive, None, None, platform_version)
            .expect("expected to execute query with proof");

        // Verify proof keeping serialized
        let (_root_hash, serialized_docs) = query
            .verify_proof_keep_serialized(proof.as_slice(), platform_version)
            .expect("expected proof verification to succeed");

        assert_eq!(serialized_docs.len(), 2);

        // Each serialized doc should be non-empty
        for doc_bytes in &serialized_docs {
            assert!(
                !doc_bytes.is_empty(),
                "serialized document should not be empty"
            );
        }
    }

    #[test]
    fn should_prove_and_verify_keep_serialized_empty_result() {
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

        // No documents inserted
        let query = DriveDocumentQuery::any_item_query(&contract, document_type);

        let (proof, _cost) = query
            .clone()
            .execute_with_proof(&drive, None, None, platform_version)
            .expect("expected to execute query with proof");

        let (_root_hash, serialized_docs) = query
            .verify_proof_keep_serialized(proof.as_slice(), platform_version)
            .expect("expected proof verification to succeed");

        assert!(serialized_docs.is_empty());
    }
}
