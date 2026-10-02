mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::verify::RootHash;
use dpp::version::PlatformVersion;
use dpp::voting::readiness::round::ReadinessRound;

/// A contract's compilation readiness round as a proof shows it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifiedReadinessRound {
    /// The round record.
    pub round: ReadinessRound,
    /// The raw distinct report count: the count of the reports count tree, not an
    /// eligibility judgement.
    pub raw_count: u64,
}

impl Drive {
    /// Verifies a contract's current compilation readiness round: the pointer, the record it
    /// names and the raw distinct report count.
    ///
    /// The proof is the merge of the pointer query and the round query, which the prover
    /// builds from the same helpers in `drive::votes::readiness::queries`.
    ///
    /// # Parameters
    ///
    /// - `proof`: A byte slice representing the proof.
    /// - `contract_id`: The contract.
    /// - `verify_subset_of_proof`: Whether we are verifying a subset of a larger proof.
    /// - `platform_version`: The platform version against which to verify.
    ///
    /// # Returns
    ///
    /// The root hash and the round if the contract has one.
    ///
    /// # Errors
    ///
    /// Returns an `Error` if the proof is not valid, an element is not for the expected path
    /// or key, the pointer names a round the proof does not carry, or the platform version is
    /// unknown.
    pub fn verify_readiness_round(
        proof: &[u8],
        contract_id: [u8; 32],
        verify_subset_of_proof: bool,
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, Option<VerifiedReadinessRound>), Error> {
        match platform_version
            .drive
            .methods
            .verify
            .voting
            .verify_readiness_round
        {
            0 => Self::verify_readiness_round_v0(
                proof,
                contract_id,
                verify_subset_of_proof,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "verify_readiness_round".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
