mod v0;

use crate::drive::Drive;
use dpp::balances::credits::TokenAmount;

use crate::error::drive::DriveError;

use crate::error::Error;

use crate::verify::RootHash;

use dpp::version::PlatformVersion;

impl Drive {
    /// Verifies the balance of a token held by a specific identity using a cryptographic proof.
    ///
    /// This method checks the cryptographic proof to verify the balances of a list of tokens
    /// associated with the given identity ID. It dispatches to version-specific implementations
    /// based on the platform version.
    ///
    /// # Parameters
    /// - `proof`: The cryptographic proof to verify.
    /// - `token_id`: A token ID to verify.
    /// - `identity_id`: The unique identifier of the identity (32-byte array).
    /// - `verify_subset_of_proof`: Whether to verify only a subset of the proof.
    /// - `platform_version`: The current platform version.
    ///
    /// # Returns
    /// - `Ok((RootHash, T))`:
    ///   - `RootHash`: The verified root hash of the database.
    ///   - `T`: A collection of `(token ID, token balance)` pairs.
    ///
    /// # Errors
    /// - `Error::Drive(DriveError::UnknownVersionMismatch)`:
    ///   - Occurs when the platform version does not match any known version for this method.
    /// - `Error::Proof(ProofError::WrongElementCount)`:
    ///   - If the number of elements in the proof does not match the number of token IDs.
    /// - `Error::Proof(ProofError::IncorrectValueSize)`:
    ///   - If the token ID size or proof value size is invalid.
    /// - `Error::Proof(ProofError::InvalidSumItemValue)`:
    ///   - If the proof element does not represent a valid sum item.
    /// - `Error::Proof(ProofError::InvalidItemType)`:
    ///   - If the proof element is not a sum item as expected for balances.
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
