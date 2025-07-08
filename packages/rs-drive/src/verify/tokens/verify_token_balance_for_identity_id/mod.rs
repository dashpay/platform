mod v0;

use crate::drive::Drive;
use dpp::balances::credits::TokenAmount;

use crate::error::drive::DriveError;

use crate::error::Error;

use crate::verify::RootHash;

use dpp::version::PlatformVersion;

impl Drive {
    /// Verifies the token balance of a specific identity using a cryptographic proof.
    ///
    /// This function validates the token balance associated with an identity by verifying
    /// the provided cryptographic proof. It ensures the correctness of the balance stored
    /// for the given identity and token combination.
    ///
    /// # Parameters
    ///
    /// - `proof`: A slice of bytes containing the cryptographic proof of the token balance.
    /// - `token_id`: A 32-byte identifier representing the unique ID of the token to verify.
    /// - `identity_id`: A 32-byte identifier representing the identity whose token balance
    ///   is to be verified.
    /// - `verify_subset_of_proof`: A boolean indicating whether to verify only a subset of
    ///   the provided proof.
    /// - `platform_version`: A reference to the [PlatformVersion] object specifying which
    ///   implementation version of the function to invoke.
    ///
    /// # Returns
    ///
    /// Returns a `Result` containing:
    /// - `Ok((RootHash, Option<TokenAmount>))`: A tuple where:
    ///   - `RootHash`: The root hash of the data structure at the time the proof was generated.
    ///   - `Option<TokenAmount>`: The token balance if it exists, or `None` if the balance is absent.
    /// - `Err(Error)`: An error if the verification fails due to an invalid proof, incorrect data,
    ///   or version mismatch.
    ///
    /// # Errors
    ///
    /// This function may return an `Error` in the following cases:
    /// - The provided proof is invalid or corrupted.
    /// - The token balance data is missing, inconsistent, or does not match the proof.
    /// - The specified platform version does not match any known or supported implementations.
    pub fn verify_token_balance_for_identity_id(
        proof: &[u8],
        token_id: [u8; 32],
        identity_id: [u8; 32],
        verify_subset_of_proof: bool,
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, Option<TokenAmount>), Error> {
        match platform_version
            .drive
            .methods
            .verify
            .token
            .verify_token_balance_for_identity_id
        {
            0 => Self::verify_token_balance_for_identity_id_v0(
                proof,
                token_id,
                identity_id,
                verify_subset_of_proof,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "verify_token_balance_for_identity_id".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
