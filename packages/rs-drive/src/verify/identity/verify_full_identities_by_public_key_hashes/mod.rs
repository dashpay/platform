mod v0;

use crate::drive::Drive;

use crate::error::drive::DriveError;

use crate::error::Error;

use crate::verify::RootHash;

pub use dpp::prelude::Identity;

use dpp::version::PlatformVersion;

use std::iter::FromIterator;

impl Drive {
    /// Verifies the full identities of multiple users by their public key hashes.
    ///
    /// This function takes a byte slice representing the serialized proof and a list of public key hashes.
    /// It verifies the full identities and returns a collection of the public key hash and associated identity for each user.
    ///
    /// # Parameters
    ///
    /// - `proof`: A byte slice representing the proof of authentication from the users.
    /// - `public_key_hashes`: A reference to a slice of 20-byte arrays, each representing a hash of a public key of a user.
    /// - `platform_version`: The platform version against which to verify the identities.
    ///
    /// # Generic Parameters
    ///
    /// - `T`: The type of the collection to hold the results.
    ///
    /// # Returns
    ///
    /// If the verification is successful, it returns a `Result` with a tuple of `RootHash` and `T`.
    ///
    /// # Errors
    ///
    /// This function returns an `Error` variant if:
    /// - The proof of authentication is not valid.
    /// - Any of the public key hashes do not correspond to a valid identity ID.
    /// - Any of the identity IDs do not correspond to a valid full identity.
    /// - An unknown or unsupported platform version is provided.
    ///
    pub fn verify_full_identities_by_public_key_hashes<
        T: FromIterator<([u8; 20], Option<Identity>)>,
    >(
        proof: &[u8],
        public_key_hashes: &[[u8; 20]],
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, T), Error> {
        match platform_version
            .drive
            .methods
            .verify
            .identity
            .verify_full_identities_by_public_key_hashes
        {
            0 => Self::verify_full_identities_by_public_key_hashes_v0(
                proof,
                public_key_hashes,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "verify_full_identities_by_public_key_hashes".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
