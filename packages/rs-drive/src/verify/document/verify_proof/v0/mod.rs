use crate::verify::RootHash;

use crate::error::Error;
use crate::query::DriveDocumentQuery;
use dpp::document::serialization_traits::DocumentPlatformConversionMethodsV0;
use dpp::document::Document;

use dpp::version::PlatformVersion;

impl DriveDocumentQuery<'_> {
    /// Verifies a proof for a collection of documents.
    ///
    /// This function takes a slice of bytes `proof` containing a serialized proof,
    /// verifies it, and returns a tuple consisting of the root hash and a vector of deserialized documents.
    ///
    /// # Arguments
    ///
    /// * `proof` - A byte slice representing the proof to be verified.
    /// * `drive_version` - The current active drive version
    ///
    /// # Returns
    ///
    /// A `Result` containing:
    /// * A tuple with the root hash and a vector of deserialized `Document`s, if the proof is valid.
    /// * An `Error` variant, in case the proof verification fails or deserialization error occurs.
    ///
    /// # Errors
    ///
    /// This function will return an `Error` variant if:
    /// 1. The proof verification fails.
    /// 2. There is a deserialization error when parsing the serialized document(s) into `Document` struct(s).
    #[inline(always)]
    pub(super) fn verify_proof_v0(
        &self,
        proof: &[u8],
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, Vec<Document>), Error> {
        self.verify_proof_keep_serialized(proof, platform_version)
            .map(|(root_hash, documents)| {
                let documents = documents
                    .into_iter()
                    .map(|serialized| {
                        Document::from_bytes(
                            serialized.as_slice(),
                            self.document_type,
                            platform_version,
                        )
                        .map_err(Error::from)
                    })
                    .collect::<Result<Vec<Document>, Error>>()?;
                Ok((root_hash, documents))
            })?
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
    fn should_prove_and_verify_document_collection() {
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

        // Insert a few documents
        let mut inserted_doc_ids = vec![];
        for seed in 1u64..=3 {
            let document = document_type
                .random_document(Some(seed), platform_version)
                .expect("expected a random document");
            let doc_id = document.id();
            inserted_doc_ids.push(doc_id);

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

        // Verify proof via the v0 method (called through the public verify_proof)
        let (_root_hash, verified_docs) = query
            .verify_proof(proof.as_slice(), platform_version)
            .expect("expected proof verification to succeed");

        assert_eq!(verified_docs.len(), 3);

        // Verify all inserted doc IDs are present
        for doc_id in &inserted_doc_ids {
            assert!(
                verified_docs.iter().any(|d| d.id() == *doc_id),
                "expected document {:?} to be present in verified results",
                doc_id
            );
        }
    }
}
