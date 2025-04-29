mod v0;

use dpp::data_contract::associated_token::token_perpetual_distribution::reward_distribution_moment::RewardDistributionMoment;
use dpp::data_contract::associated_token::token_perpetual_distribution::reward_distribution_type::RewardDistributionType;
use crate::drive::Drive;

use crate::error::drive::DriveError;

use crate::error::Error;

use crate::verify::RootHash;

use dpp::version::PlatformVersion;

impl Drive {
    /// Verifies the token information for a specific identity using a cryptographic proof.
    ///
    /// This function verifies the association between a token and an identity by processing the provided
    /// cryptographic proof. It checks the existence and correctness of the token's information in the
    /// context of the specified identity.
    ///
    /// # Parameters
    ///
    /// - `proof`: A slice of bytes containing the cryptographic proof of the token's information.
    /// - `token_id`: A 32-byte identifier representing the unique ID of the token to verify.
    /// - `identity_id`: A 32-byte identifier representing the identity associated with the token.
    /// - `verify_subset_of_proof`: A boolean indicating whether to verify only a subset of the provided proof.
    /// - `platform_version`: A reference to the [PlatformVersion] object specifying which implementation
    ///   version of the function to invoke.
    ///
    /// # Returns
    ///
    /// Returns a `Result` containing:
    /// - `Ok((RootHash, Option<IdentityTokenInfo>))`: A tuple where:
    ///   - `RootHash`: The root hash of the data structure at the time the proof was generated.
    ///   - `Option<IdentityTokenInfo>`: The token information if it exists, or `None` if the token information
    ///     is absent.
    /// - `Err(Error)`: An error if the verification fails due to an invalid proof, incorrect data, or version mismatch.
    ///
    /// # Errors
    ///
    /// This function may return an `Error` in the following cases:
    /// - The provided proof is invalid or corrupted.
    /// - The token's information is missing, inconsistent, or does not match the proof.
    /// - The specified platform version does not match any known or supported implementations.
    pub fn verify_token_perpetual_distribution_last_paid_time(
        proof: &[u8],
        token_id: [u8; 32],
        identity_id: [u8; 32],
        distribution_type: &RewardDistributionType,
        verify_subset_of_proof: bool,
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, Option<RewardDistributionMoment>), Error> {
        match platform_version
            .drive
            .methods
            .verify
            .token
            .verify_token_perpetual_distribution_last_paid_time
        {
            0 => Self::verify_token_perpetual_distribution_last_paid_time_v0(
                proof,
                token_id,
                identity_id,
                distribution_type,
                verify_subset_of_proof,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "verify_token_perpetual_distribution_last_paid_time".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
