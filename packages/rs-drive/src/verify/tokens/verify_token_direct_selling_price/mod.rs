mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use dpp::tokens::token_pricing_schedule::TokenPricingSchedule;

use crate::verify::RootHash;

use dpp::version::PlatformVersion;

impl Drive {
    /// Verifies the status of a token using a provided cryptographic proof.
    ///
    /// This function takes a cryptographic proof, a token ID, and other parameters to verify
    /// the existence and status of a token in the data structure. It delegates the verification
    /// process to the appropriate versioned implementation based on the platform version.
    ///
    /// # Parameters
    ///
    /// - `proof`: A slice of bytes representing the cryptographic proof of the token's status.
    /// - `token_id`: A 32-byte identifier representing the unique ID of the token.
    /// - `verify_subset_of_proof`: A boolean indicating whether to verify a subset of the provided proof.
    /// - `platform_version`: A reference to the [PlatformVersion] object that specifies which version
    ///   of the function implementation to invoke.
    ///
    /// # Returns
    ///
    /// Returns a `Result` containing:
    /// - `Ok((RootHash, Option<TokenStatus>))`: A tuple where:
    ///   - `RootHash`: The root hash of the data structure at the time the proof was created.
    ///   - `Option<TokenStatus>`: The status of the token if it exists, or `None` if the token does not exist.
    /// - `Err(Error)`: An error if the verification fails due to an invalid proof, incorrect data, or version mismatch.
    ///
    /// # Errors
    ///
    /// This function can return an `Error` in the following cases:
    /// - The proof is invalid or corrupted.
    /// - The token's status data is missing or inconsistent.
    /// - The platform version does not match any of the known implementations.
    pub fn verify_token_direct_selling_price(
        proof: &[u8],
        token_id: [u8; 32],
        verify_subset_of_proof: bool,
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, Option<TokenPricingSchedule>), Error> {
        match platform_version
            .drive
            .methods
            .verify
            .token
            .verify_token_direct_selling_price
        {
            0 => Self::verify_token_direct_selling_price_v0(
                proof,
                token_id,
                verify_subset_of_proof,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "verify_token_direct_selling_price".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
