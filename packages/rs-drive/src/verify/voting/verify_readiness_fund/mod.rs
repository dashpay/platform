mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::verify::RootHash;
use dpp::version::PlatformVersion;

impl Drive {
    /// Verifies the balance of a compilation readiness fund.
    ///
    /// # Parameters
    ///
    /// - `proof`: A byte slice representing the proof.
    /// - `fund_id`: The fund id a round derives from its round id.
    /// - `verify_subset_of_proof`: Whether we are verifying a subset of a larger proof.
    /// - `platform_version`: The platform version against which to verify.
    ///
    /// # Returns
    ///
    /// The root hash and the balance if the fund exists.
    ///
    /// # Errors
    ///
    /// Returns an `Error` if the proof is not valid, the proved key value is not for the
    /// correct path or key, more than one element is found, or the platform version is
    /// unknown.
    pub fn verify_readiness_fund(
        proof: &[u8],
        fund_id: [u8; 32],
        verify_subset_of_proof: bool,
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, Option<u64>), Error> {
        match platform_version
            .drive
            .methods
            .verify
            .voting
            .verify_readiness_fund
        {
            0 => Self::verify_readiness_fund_v0(
                proof,
                fund_id,
                verify_subset_of_proof,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "verify_readiness_fund".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
