mod v0;

use crate::error::drive::DriveError;
use crate::verify::RootHash;

use crate::error::Error;
use crate::query::DriveDocumentQuery;
use dpp::document::Document;

use dpp::version::PlatformVersion;

impl<'a> DriveDocumentQuery<'a> {
    /// Verifies a proof for a collection of documents.
    ///
    /// This function takes a byte slice representing the serialized proof, verifies it, and returns a tuple consisting of the root hash
    /// and a vector of deserialized documents.
    ///
    /// # Arguments
    ///
    /// * `proof` - A byte slice representing the proof to be verified.
    /// * `platform_version` - The platform version against which to verify the proof.
    ///
    /// # Returns
    ///
    /// A `Result` containing:
    /// * A tuple with the root hash and a vector of deserialized `Document`s if the proof is valid.
    /// * An `Error` variant, in case the proof verification fails or a deserialization error occurs.
    ///
    /// # Errors
    ///
    /// This function will return an `Error` variant if:
    /// 1. The proof verification fails.
    /// 2. A deserialization error occurs when parsing the serialized document(s).
    pub fn verify_proof(
        &self,
        proof: &[u8],
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, Vec<Document>), Error> {
        match platform_version.drive.methods.verify.document.verify_proof {
            0 => self.verify_proof_v0(proof, platform_version),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "verify_proof".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
