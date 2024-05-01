use crate::drive::verify::RootHash;

use crate::error::proof::ProofError;
use crate::error::Error;
use crate::query::DriveQuery;
use dpp::document::serialization_traits::DocumentPlatformConversionMethodsV0;
use dpp::document::Document;
use dpp::version::PlatformVersion;
use grovedb::{GroveDb, PathQuery};

impl<'a> DriveQuery<'a> {
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
            GroveDb::verify_subset_query(proof, &path_query)?
        } else {
            GroveDb::verify_query(proof, &path_query)?
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
                        let document_bytes = element.into_item_bytes().map_err(Error::GroveDB)?;
                        Document::from_bytes(
                            document_bytes.as_slice(),
                            self.document_type,
                            platform_version,
                        )
                        .map_err(Error::Protocol)
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
