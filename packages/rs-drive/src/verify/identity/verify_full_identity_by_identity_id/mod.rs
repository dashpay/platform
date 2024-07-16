mod v0;

use crate::drive::Drive;

use crate::error::drive::DriveError;

use crate::error::Error;

use crate::verify::RootHash;

pub use dpp::prelude::Identity;

use dpp::version::PlatformVersion;

impl Drive {
    /// Verifies the full identity of a user by their identity ID.
    ///
    /// # Parameters
    ///
    /// - `proof`: A byte slice representing the proof of authentication from the user.
    /// - `is_proof_subset`: A boolean indicating whether the proof is a subset.
    /// - `identity_id`: A 32-byte array representing the identity ID of the user.
    /// - `platform_version`: The platform version against which to verify the identity.
    ///
    /// # Returns
    ///
    /// If the verification is successful, it returns a `Result` with a tuple of `RootHash` and
    /// an `Option` of `Identity`. The `RootHash` represents the root hash of GroveDB, and the
    /// `Option<Identity>` represents the full identity of the user if it exists.
    ///
    /// # Errors
    ///
    /// Returns an `Error` if:
    ///
    /// - The proof of authentication is not valid.
    /// - The identity ID does not correspond to a valid full identity.
    /// - The balance, revision, or keys information is missing or incorrect.
    /// - An unknown or unsupported platform version is provided.
    ///
    pub fn verify_full_identity_by_identity_id(
        proof: &[u8],
        is_proof_subset: bool,
        identity_id: [u8; 32],
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, Option<Identity>), Error> {
        match platform_version
            .drive
            .methods
            .verify
            .identity
            .verify_full_identity_by_identity_id
        {
            0 => Self::verify_full_identity_by_identity_id_v0(
                proof,
                is_proof_subset,
                identity_id,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "verify_full_identity_by_identity_id".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
