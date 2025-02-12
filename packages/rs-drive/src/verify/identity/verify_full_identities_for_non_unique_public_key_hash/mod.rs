mod v0;

use crate::drive::Drive;

use crate::error::drive::DriveError;

use crate::error::Error;

use crate::verify::RootHash;

pub use dpp::prelude::Identity;

use dpp::version::PlatformVersion;

use std::iter::FromIterator;

impl Drive {
    /// Verifies and retrieves the full identities associated with a given non-unique public key hash.
    ///
    /// This function validates the provided proof to confirm identity IDs for a specified public key hash,
    /// and then retrieves the full identity data for each verified identity ID. It uses the versioning
    /// specified in `PlatformVersion` to select the correct method implementation.
    ///
    /// # Type Parameters
    ///
    /// - `T`: The output collection type for identities, which must implement `FromIterator<Identity>`.
    ///
    /// # Parameters
    ///
    /// - `proof`: A byte slice containing the proof data to verify the identities.
    /// - `public_key_hash`: A 20-byte array representing the non-unique public key hash.
    /// - `limit`: An optional limit for the number of identities to retrieve.
    /// - `platform_version`: A reference to the `PlatformVersion`, used to determine the verification method version.
    ///
    /// # Returns
    ///
    /// Returns a `Result` containing:
    /// - `RootHash`: The root hash from GroveDB after proof verification.
    /// - `T`: A collection of verified `Identity` instances corresponding to the public key hash.
    ///
    /// # Errors
    ///
    /// Returns an `Error` if:
    /// - The provided proof is invalid or incomplete.
    /// - No full identity data is available for any of the retrieved identity IDs.
    /// - The method version specified in `PlatformVersion` is not supported.
    ///
    pub fn verify_full_identities_for_non_unique_public_key_hash<T: FromIterator<Identity>>(
        proof: &[u8],
        public_key_hash: [u8; 20],
        limit: Option<u16>,
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, T), Error> {
        match platform_version
            .drive
            .methods
            .verify
            .identity
            .verify_full_identities_for_non_unique_public_key_hash
        {
            0 => Self::verify_full_identities_for_non_unique_public_key_hash_v0(
                proof,
                public_key_hash,
                limit,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "verify_full_identities_for_non_unique_public_key_hash".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
