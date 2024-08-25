mod v0;

use crate::error::drive::DriveError;
use crate::verify::RootHash;

use crate::error::Error;
use crate::query::SingleDocumentDriveQuery;

use dpp::version::PlatformVersion;

impl SingleDocumentDriveQuery {
    /// Verifies the proof of a document while keeping it serialized.
    ///
    /// `is_subset` indicates if the function should verify a subset of a larger proof.
    ///
    /// # Parameters
    ///
    /// - `is_subset`: A boolean indicating whether to verify a subset of a larger proof.
    /// - `proof`: A byte slice representing the proof to be verified.
    /// - `platform_version`: The platform version against which to verify the proof.
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
    /// - An unknown or unsupported platform version is provided.
    /// - Any other error as documented in the specific versioned function.
    pub fn verify_proof_keep_serialized(
        &self,
        is_subset: bool,
        proof: &[u8],
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, Option<Vec<u8>>), Error> {
        match platform_version
            .drive
            .methods
            .verify
            .single_document
            .verify_proof_keep_serialized
        {
            0 => self.verify_proof_keep_serialized_v0(is_subset, proof, platform_version),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "SingleDocumentDriveQuery::verify_proof_keep_serialized".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
