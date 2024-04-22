use crate::drive::verify::RootHash;

use crate::error::Error;
use crate::query::DriveQuery;
use dpp::document::serialization_traits::DocumentPlatformConversionMethodsV0;
use dpp::document::Document;

use dpp::version::PlatformVersion;

impl<'a> DriveQuery<'a> {
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
                        .map_err(Error::Protocol)
                    })
                    .collect::<Result<Vec<Document>, Error>>()?;
                Ok((root_hash, documents))
            })?
    }
}
