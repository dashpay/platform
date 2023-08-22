use crate::drive::verify::RootHash;

use crate::error::proof::ProofError;
use crate::error::Error;
use crate::query::DriveQuery;
use dpp::document::Document;
use grovedb::{GroveDb, PathQuery};

impl<'a> DriveQuery<'a> {
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
    pub fn verify_proof_keep_serialized(
        &self,
        proof: &[u8],
    ) -> Result<(RootHash, Vec<Vec<u8>>), Error> {
        let path_query = if let Some(start_at) = &self.start_at {
            let (_, start_document) =
                self.verify_start_at_document_in_proof(proof, true, *start_at)?;
            let document = start_document.ok_or(Error::Proof(ProofError::IncompleteProof(
                "expected start at document to be present in proof",
            )))?;
            self.construct_path_query(Some(document))
        } else {
            self.construct_path_query(None)
        }?;
        let (root_hash, proved_key_values) = if self.start_at.is_some() {
            GroveDb::verify_subset_query(proof, &path_query)?
        } else {
            GroveDb::verify_query(proof, &path_query)?
        };

        let documents = proved_key_values
            .into_iter()
            .filter_map(|(_path, _key, element)| element)
            .map(|element| element.into_item_bytes().map_err(Error::GroveDB))
            .collect::<Result<Vec<Vec<u8>>, Error>>()?;
        Ok((root_hash, documents))
    }

    /// Verifies a proof for a collection of documents.
    ///
    /// This function takes a slice of bytes `proof` containing a serialized proof,
    /// verifies it, and returns a tuple consisting of the root hash and a vector of deserialized documents.
    ///
    /// # Arguments
    ///
    /// * `proof` - A byte slice representing the proof to be verified.
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
    pub fn verify_proof(&self, proof: &[u8]) -> Result<(RootHash, Vec<Document>), Error> {
        self.verify_proof_keep_serialized(proof)
            .map(|(root_hash, documents)| {
                let documents = documents
                    .into_iter()
                    .map(|serialized| {
                        Document::from_bytes(serialized.as_slice(), self.document_type)
                            .map_err(Error::Protocol)
                    })
                    .collect::<Result<Vec<Document>, Error>>()?;
                Ok((root_hash, documents))
            })?
    }

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
    pub fn verify_start_at_document_in_proof(
        &self,
        proof: &[u8],
        is_proof_subset: bool,
        document_id: [u8; 32],
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
                        Document::from_bytes(document_bytes.as_slice(), self.document_type)
                            .map_err(Error::Protocol)
                    })
                    .transpose()?;
                Ok((root_hash, document))
            }
            0 => Err(Error::Proof(ProofError::WrongElementCount(
                "expected one document for start at, got none",
            ))),
            _ => Err(Error::Proof(ProofError::TooManyElements(
                "expected one document for start at",
            ))),
        }
    }
}
