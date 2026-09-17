mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::verify::RootHash;
use dpp::version::PlatformVersion;
use dpp::voting::readiness::report_record::ReadinessReportRecord;

impl Drive {
    /// Verifies one accepted compilation readiness report of a round.
    ///
    /// # Parameters
    ///
    /// - `proof`: A byte slice representing the proof.
    /// - `contract_id`: The contract.
    /// - `round_id`: The round.
    /// - `pro_tx_hash`: The reporting evonode.
    /// - `verify_subset_of_proof`: Whether we are verifying a subset of a larger proof.
    /// - `platform_version`: The platform version against which to verify.
    ///
    /// # Returns
    ///
    /// The root hash and the report record if the report exists.
    ///
    /// # Errors
    ///
    /// Returns an `Error` if the proof is not valid, the proved key value is not for the
    /// correct path or key, more than one element is found, or the platform version is
    /// unknown.
    pub fn verify_readiness_report(
        proof: &[u8],
        contract_id: [u8; 32],
        round_id: [u8; 32],
        pro_tx_hash: [u8; 32],
        verify_subset_of_proof: bool,
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, Option<ReadinessReportRecord>), Error> {
        match platform_version
            .drive
            .methods
            .verify
            .voting
            .verify_readiness_report
        {
            0 => Self::verify_readiness_report_v0(
                proof,
                contract_id,
                round_id,
                pro_tx_hash,
                verify_subset_of_proof,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "verify_readiness_report".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
