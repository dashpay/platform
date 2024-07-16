mod v0;

use crate::error::drive::DriveError;
use crate::verify::RootHash;

use crate::error::Error;
use crate::query::DriveDocumentQuery;

use dpp::version::PlatformVersion;

impl<'a> DriveDocumentQuery<'a> {
    /// Verifies the given proof and returns the root hash of the GroveDB tree and a vector
    /// of serialized documents if the verification is successful.
    ///
    /// # Arguments
    /// * `proof` - A byte slice representing the proof to be verified.
    /// * `platform_version` - The platform version against which to verify the proof.
    ///
    /// # Returns
    /// * On success, returns a tuple containing the root hash of the GroveDB tree and a vector of serialized documents.
    /// * On failure, returns an Error.
    ///
    /// # Errors
    /// This function will return an Error if:
    /// 1. The start at document is not present in proof and it is expected to be.
    /// 2. The path query fails to verify against the given proof.
    /// 3. Converting the element into bytes fails.
    pub fn verify_proof_keep_serialized(
        &self,
        proof: &[u8],
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, Vec<Vec<u8>>), Error> {
        match platform_version
            .drive
            .methods
            .verify
            .document
            .verify_proof_keep_serialized
        {
            0 => self.verify_proof_keep_serialized_v0(proof, platform_version),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "verify_proof_keep_serialized".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
