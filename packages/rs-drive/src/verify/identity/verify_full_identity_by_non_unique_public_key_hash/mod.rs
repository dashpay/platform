mod v0;

use crate::drive::Drive;

use crate::error::drive::DriveError;

use crate::error::Error;

use crate::verify::RootHash;

pub use dpp::prelude::Identity;

use crate::drive::identity::identity_and_non_unique_public_key_hash_double_proof::IdentityAndNonUniquePublicKeyHashDoubleProof;
use dpp::version::PlatformVersion;

impl Drive {
    /// Verifies the full identity of a user using their non-unique public key hash.
    ///
    /// This function acts as a dispatcher that selects the appropriate version-specific
    /// verification method based on the provided platform version.
    ///
    /// # Parameters
    ///
    /// - `proof`: A proof containing both the identity proof (if applicable) and the
    ///   proof linking the public key hash to an identity ID.
    /// - `public_key_hash`: A 20-byte array representing the hash of the user's public key.
    /// - `after`: An optional 32-byte array specifying an identity after which
    ///   the search should begin when retrieving the identity.
    /// - `platform_version`: A reference to the platform version, ensuring that
    ///   the correct verification method is used.
    ///
    /// # Returns
    ///
    /// Returns a `Result` containing:
    /// - `RootHash`: The root hash of GroveDB after verification.
    /// - `Option<Identity>`: The full identity of the user, if it exists.
    ///
    /// If no identity is found, the returned `Option<Identity>` will be `None`.
    ///
    /// # Errors
    ///
    /// This function returns an `Error` if:
    /// - The provided proof is invalid.
    /// - The public key hash does not correspond to a valid identity ID.
    /// - The identity ID exists but does not correspond to a valid full identity.
    /// - The provided platform version is unknown or unsupported.
    ///
    /// # Versioning
    ///
    /// - Currently, only version `0` of `verify_full_identity_by_non_unique_public_key_hash`
    ///   is implemented. If an unsupported version is provided, an `UnknownVersionMismatch`
    ///   error is returned.
    pub fn verify_full_identity_by_non_unique_public_key_hash(
        proof: &IdentityAndNonUniquePublicKeyHashDoubleProof,
        public_key_hash: [u8; 20],
        after: Option<[u8; 32]>,
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, Option<Identity>), Error> {
        match platform_version
            .drive
            .methods
            .verify
            .identity
            .verify_full_identity_by_non_unique_public_key_hash
        {
            0 => Self::verify_full_identity_by_non_unique_public_key_hash_v0(
                proof,
                public_key_hash,
                after,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "verify_full_identity_by_non_unique_public_key_hash".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
