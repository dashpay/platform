mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use dpp::tokens::token_pricing_schedule::TokenPricingSchedule;

use crate::verify::RootHash;

use dpp::version::PlatformVersion;

impl Drive {
    /// Verifies the direct_selling_prices of multiple tokens using a cryptographic proof.
    ///
    /// This method validates the cryptographic proof to retrieve the direct_selling_prices of the specified token IDs.
    /// It dispatches to version-specific implementations based on the provided platform version.
    ///
    /// # Parameters
    /// - `proof`: The cryptographic proof to verify.
    /// - `token_ids`: A list of token IDs to verify (each a 32-byte array).
    /// - `verify_subset_of_proof`: Whether to verify only a subset of the proof.
    /// - `platform_version`: The current platform version.
    ///
    /// # Returns
    /// - `Ok((RootHash, T))`:
    ///   - `RootHash`: The verified root hash of the database.
    ///   - `T`: A collection of `(token ID, token pricing schedule)` pairs.
    ///
    /// # Errors
    /// - `Error::Drive(DriveError::UnknownVersionMismatch)`:
    ///   - Occurs when the platform version does not match any known version for this method.
    /// - `Error::Proof(ProofError::WrongElementCount)`:
    ///   - If the number of elements in the proof does not match the number of token IDs.
    /// - `Error::Proof(ProofError::IncorrectValueSize)`:
    ///   - If the token ID size or proof value size is invalid.
    /// - `Error::Proof(ProofError::DeserializationFailed)`:
    ///   - If the token status cannot be deserialized from the proof.
    /// - `Error::Proof(ProofError::InvalidItemType)`:
    ///   - If the proof element is not an expected item type (e.g., `Item`).
    pub fn verify_token_direct_selling_prices<
        T: FromIterator<(I, Option<TokenPricingSchedule>)>,
        I: From<[u8; 32]>,
    >(
        proof: &[u8],
        token_ids: &[[u8; 32]],
        verify_subset_of_proof: bool,
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, T), Error> {
        match platform_version
            .drive
            .methods
            .verify
            .token
            .verify_token_direct_selling_prices
        {
            0 => Self::verify_token_direct_selling_prices_v0(
                proof,
                token_ids,
                verify_subset_of_proof,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "verify_token_direct_selling_prices".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
