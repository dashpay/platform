mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use dpp::balances::credits::TokenAmount;
use dpp::identifier::Identifier;
use dpp::prelude::TimestampMillis;

use crate::error::Error;

use crate::verify::RootHash;

use crate::drive::tokens::distribution::queries::QueryPreProgrammedDistributionStartAt;
use dpp::version::PlatformVersion;

impl Drive {
    /// Verifies the pre-programmed token distributions using a cryptographic proof.
    ///
    /// This function checks the proof and reconstructs the token’s pre-programmed distributions
    /// for all timestamps and recipients. It uses generics to allow flexibility in return types.
    ///
    /// # Parameters
    /// - `proof`: The cryptographic proof.
    /// - `token_id`: The ID of the token (32-byte array).
    /// - `verify_subset_of_proof`: Whether to verify only a subset of the proof.
    /// - `platform_version`: The current platform version.
    ///
    /// # Returns
    /// - `Ok((RootHash, T))`:
    ///   - `RootHash`: The verified root hash of the database.
    ///   - `T`: A collection implementing `FromIterator<(TimestampMillis, D)>` where `D`
    ///     implements `FromIterator<(Identifier, TokenAmount)>`.
    ///
    /// # Errors
    /// - `Error::Drive(DriveError::UnknownVersionMismatch)` if the platform version is unsupported.
    /// - `Error::Proof(ProofError::WrongElementCount)` if the number of elements in the proof is incorrect.
    /// - `Error::Proof(ProofError::InvalidSumItemValue)` if the element does not represent a valid sum item.
    pub fn verify_token_pre_programmed_distributions<
        T: FromIterator<(TimestampMillis, D)>,
        D: FromIterator<(Identifier, TokenAmount)>,
    >(
        proof: &[u8],
        token_id: [u8; 32],
        start_at: Option<QueryPreProgrammedDistributionStartAt>,
        limit: Option<u16>,
        verify_subset_of_proof: bool,
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, T), Error> {
        match platform_version
            .drive
            .methods
            .verify
            .token
            .verify_token_pre_programmed_distributions
        {
            0 => Self::verify_token_pre_programmed_distributions_v0(
                proof,
                token_id,
                start_at,
                limit,
                verify_subset_of_proof,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "verify_token_pre_programmed_distributions".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
