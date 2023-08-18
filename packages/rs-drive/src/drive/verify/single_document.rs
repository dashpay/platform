use crate::drive::verify::RootHash;

use crate::error::proof::ProofError;
use crate::error::Error;
use crate::query::SingleDocumentDriveQuery;
use dpp::data_contract::document_type::DocumentType;
use dpp::document::Document;

use grovedb::GroveDb;

impl SingleDocumentDriveQuery {
    /// Verifies the proof of a document while keeping it serialized.
    ///
    /// `is_subset` indicates if the function should verify a subset of a larger proof.
    ///
    /// # Parameters
    ///
    /// - `is_subset`: A boolean indicating whether to verify a subset of a larger proof.
    /// - `proof`: A byte slice representing the proof to be verified.
    ///
    /// # Returns
    ///
    /// Returns a `Result` with a tuple of `RootHash` and `Option<Vec<u8>>`. The `Option<Vec<u8>>`
    /// represents the serialized document if it exists.
    ///
    /// # Errors
    ///
    /// Returns an `Error` if:
    ///
    /// - The proof is corrupted.
    /// - The GroveDb verification fails.
    /// - The elements returned are not items, the proof is incorrect.
    pub fn verify_proof_keep_serialized(
        &self,
        is_subset: bool,
        proof: &[u8],
    ) -> Result<(RootHash, Option<Vec<u8>>), Error> {
        let path_query = self.construct_path_query();
        let (root_hash, mut proved_key_values) = if is_subset {
            GroveDb::verify_subset_query_with_absence_proof(proof, &path_query)?
        } else {
            GroveDb::verify_query_with_absence_proof(proof, &path_query)?
        };

        if proved_key_values.len() != 1 {
            return Err(Error::Proof(ProofError::CorruptedProof(
                "we should always get back one element".to_string(),
            )));
        }

        let element = proved_key_values.remove(0).2;

        let serialized = element
            .map(|element| element.into_item_bytes().map_err(Error::GroveDB))
            .transpose()?;

        Ok((root_hash, serialized))
    }

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
    pub fn verify_proof(
        &self,
        is_subset: bool,
        proof: &[u8],
        document_type: &DocumentType,
    ) -> Result<(RootHash, Option<Document>), Error> {
        self.verify_proof_keep_serialized(is_subset, proof)
            .map(|(root_hash, serialized)| {
                let document = serialized
                    .map(|serialized| {
                        Document::from_bytes(serialized.as_slice(), document_type)
                            .map_err(Error::Protocol)
                    })
                    .transpose()?;
                Ok((root_hash, document))
            })?
    }
}
