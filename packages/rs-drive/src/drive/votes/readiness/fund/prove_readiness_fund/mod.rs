mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

impl Drive {
    /// Proves a readiness fund balance.
    ///
    /// # Parameters
    ///
    /// * `fund_id` - The fund, derived from the round id.
    /// * `transaction` - The current transaction.
    /// * `platform_version` - The platform version to use.
    ///
    /// # Returns
    ///
    /// * The proof bytes.
    /// * `Err(DriveError::VersionNotActive)` on a platform version without readiness.
    pub fn prove_readiness_fund(
        &self,
        fund_id: [u8; 32],
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, Error> {
        match platform_version.drive.methods.vote.readiness.prove_fund {
            Some(0) => self.prove_readiness_fund_v0(fund_id, transaction, platform_version),
            None => Err(Error::Drive(DriveError::VersionNotActive {
                method: "prove_readiness_fund".to_string(),
                known_versions: vec![0],
            })),
            Some(version) => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "prove_readiness_fund".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
