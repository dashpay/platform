mod v0;

use crate::drive::{identity::key::fetch::IdentityKeysRequest, Drive};

use crate::error::drive::DriveError;

use crate::error::Error;

use crate::drive::verify::RootHash;

use dpp::identity::PartialIdentity;
pub use dpp::prelude::{Identity, Revision};

use dpp::version::PlatformVersion;

impl Drive {
    /// Verifies the identity keys of a user by their identity ID.
    ///
    /// # Parameters
    ///
    /// - `proof`: A byte slice representing the proof of authentication from the user.
    /// - `is_proof_subset`: A boolean indicating whether the proof is a subset.
    /// - `identity_id`: A 32-byte array representing the identity ID of the user.
    /// - `platform_version`: The platform version against which to verify the identity keys.
    ///
    /// # Returns
    ///
    /// If the verification is successful, it returns a `Result` with a tuple of `RootHash` and
    /// an `Option` of `PartialIdentity`. The `RootHash` represents the root hash of GroveDB,
    /// and the `Option<PartialIdentity>` represents the partial identity of the user if it exists.
    ///
    /// # Errors
    ///
    /// Returns an `Error` if:
    ///
    /// - An unknown or unsupported platform version is provided.
    /// - Any other error as documented in the specific versioned function.
    ///
    pub fn verify_identity_keys_by_identity_id(
        proof: &[u8],
        key_request: IdentityKeysRequest,
        with_revision: bool,
        with_balance: bool,
        is_proof_subset: bool,
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, Option<PartialIdentity>), Error> {
        match platform_version
            .drive
            .methods
            .verify
            .identity
            .verify_identity_keys_by_identity_id
        {
            0 => Self::verify_identity_keys_by_identity_id_v0(
                proof,
                key_request,
                with_revision,
                with_balance,
                is_proof_subset,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "verify_identity_keys_by_identity_id".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
