mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::verify::RootHash;
use platform_version::version::PlatformVersion;

impl Drive {
    /// Verifies the revision of an identity by their identity ID.
    ///
    /// # Parameters
    ///
    /// - `proof`: A byte slice representing the proof of authentication from the user.
    /// - `identity_id`: A 32-byte array representing the identity ID of the user.
    /// - `verify_subset_of_proof`: A boolean indicating whether we are verifying a subset of a larger proof.
    /// - `platform_version`: The version of the platform used to verify the identity revision.
    ///
    /// # Returns
    ///
    /// If the verification is successful, it returns a `Result` with a tuple of `RootHash` and
    /// an `Option<u64>`. The `RootHash` represents the root hash of GroveDB, and the
    /// `Option<u64>` represents the revision of the user's identity if it exists.
    ///
    /// # Errors
    ///
    /// Returns an `Error` if:
    ///
    /// - The proof of authentication is not valid.
    /// - The identity ID does not correspond to a valid revision.
    /// - The proof contains elements unrelated to the requested revision.
    ///
    pub fn verify_identity_revision_for_identity_id(
        proof: &[u8],
        identity_id: [u8; 32],
        verify_subset_of_proof: bool,
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, Option<u64>), Error> {
        match platform_version
            .drive
            .methods
            .verify
            .identity
            .verify_identity_revision_for_identity_id
        {
            0 => Self::verify_identity_revision_for_identity_id_v0(
                proof,
                identity_id,
                verify_subset_of_proof,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "verify_identity_revision_for_identity_id".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
