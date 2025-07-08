mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::verify::RootHash;
use dpp::balances::total_single_token_balance::TotalSingleTokenBalance;
use dpp::version::PlatformVersion;

impl Drive {
    /// Verifies the total token supply and aggregated identity balances for a given token.
    ///
    /// This method checks the cryptographic proof to verify the total supply of a token and the
    /// aggregated balances of identities associated with that token. It dispatches to version-specific
    /// implementations based on the provided platform version.
    ///
    /// # Parameters
    /// - `proof`: The cryptographic proof to verify.
    /// - `token_id`: The unique identifier of the token (32-byte array).
    /// - `verify_subset_of_proof`: Whether to verify only a subset of the proof.
    /// - `platform_version`: The current platform version.
    ///
    /// # Returns
    /// - `Ok((RootHash, TotalSingleTokenBalance))`:
    ///   - `RootHash`: The verified root hash of the database.
    ///   - `TotalSingleTokenBalance`: The total supply and aggregated identity balances of the token.
    ///
    /// # Errors
    /// - `Error::Drive(DriveError::UnknownVersionMismatch)`:
    ///   - Occurs when the platform version does not match any known version for this method.
    /// - `Error::Proof(ProofError::UnexpectedResultProof)`:
    ///   - If the token does not exist in the proof.
    ///   - If the token's supply is not found in the proof.
    /// - `Error::Proof(ProofError::WrongElementCount)`:
    ///   - If the proof does not contain exactly two expected elements (total supply and aggregated balances).
    /// - `Error::Proof(ProofError::InvalidSumItemValue)`:
    ///   - If the retrieved proof element is not a valid sum item.
    pub fn verify_token_total_supply_and_aggregated_identity_balance(
        proof: &[u8],
        token_id: [u8; 32],
        verify_subset_of_proof: bool,
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, TotalSingleTokenBalance), Error> {
        match platform_version
            .drive
            .methods
            .verify
            .token
            .verify_token_total_supply_and_aggregated_identity_balance
        {
            0 => Self::verify_token_total_supply_and_aggregated_identity_balance_v0(
                proof,
                token_id,
                verify_subset_of_proof,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "verify_token_total_supply_and_aggregated_identity_balance".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
