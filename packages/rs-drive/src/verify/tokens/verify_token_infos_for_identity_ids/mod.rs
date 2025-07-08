mod v0;

use crate::drive::Drive;
use dpp::tokens::info::IdentityTokenInfo;

use crate::error::drive::DriveError;

use crate::error::Error;

use crate::verify::RootHash;

use dpp::version::PlatformVersion;

impl Drive {
    /// Verifies the token infos for a set of identity IDs.
    ///
    /// This function checks the token infos of multiple identities by verifying the provided
    /// proof against the specified token ID and identity IDs. It also supports verifying a subset
    /// of a larger proof if necessary.
    ///
    /// # Parameters
    ///
    /// - `proof`: A byte slice representing the proof of authentication from the user. This is used
    ///   to verify the validity of the identity and its associated info.
    /// - `token_id`: A 32-byte array representing the unique identifier for the token whose info
    ///   is being verified.
    /// - `identity_ids`: A slice of 32-byte arrays, each representing a unique identity ID. These
    ///   are the identities whose token infos are being verified.
    /// - `verify_subset_of_proof`: A boolean flag indicating whether the proof being verified is a
    ///   subset of a larger proof. If `true`, the verification will consider only a part of the proof.
    /// - `platform_version`: The version of the platform against which the identity token infos are
    ///   being verified. This ensures compatibility with the correct API version.
    ///
    /// # Returns
    ///
    /// - `Result<(RootHash, BTreeMap<[u8; 32], Option<IdentityTokenInfo>>), Error>`: If the verification is successful:
    ///   - `RootHash`: The root hash of the GroveDB, representing the state of the database.
    ///   - `BTreeMap<[u8; 32], Option<IdentityTokenInfo>>`: A map of identity IDs to their associated token infos.
    ///
    /// # Errors
    ///
    /// The function will return an `Error` if any of the following occur:
    ///
    /// - The provided authentication proof is invalid.
    /// - The provided identity ID does not correspond to a valid info.
    /// - The provided platform version is unknown or unsupported.
    ///
    pub fn verify_token_infos_for_identity_ids<
        T: FromIterator<(I, Option<IdentityTokenInfo>)>,
        I: From<[u8; 32]>,
    >(
        proof: &[u8],
        token_id: [u8; 32],
        identity_ids: &[[u8; 32]],
        verify_subset_of_proof: bool,
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, T), Error> {
        match platform_version
            .drive
            .methods
            .verify
            .token
            .verify_token_infos_for_identity_ids
        {
            0 => Self::verify_token_infos_for_identity_ids_v0(
                proof,
                token_id,
                identity_ids,
                verify_subset_of_proof,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "verify_token_infos_for_identity_ids".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
