use crate::drive::verify::RootHash;

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
                            .map_err(Error::Protocol)
                    })
                    .transpose()?;
                Ok((root_hash, document))
            })?
    }
}
