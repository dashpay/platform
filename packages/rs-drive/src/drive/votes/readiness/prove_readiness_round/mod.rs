mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

impl Drive {
    /// Proves a contract's current compilation readiness round: the pointer and, when it
    /// names a round, that round's record and reports count tree element, in one merged
    /// proof `verify_readiness_round` decodes.
    ///
    /// # Parameters
    ///
    /// * `contract_id` - The contract.
    /// * `transaction` - The current transaction.
    /// * `platform_version` - The platform version to use.
    ///
    /// # Returns
    ///
    /// * The proof bytes.
    /// * `Err(DriveError::VersionNotActive)` on a platform version without readiness.
    pub fn prove_readiness_round(
        &self,
        contract_id: [u8; 32],
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, Error> {
        match platform_version.drive.methods.vote.readiness.fetch_round {
            Some(0) => self.prove_readiness_round_v0(contract_id, transaction, platform_version),
            None => Err(Error::Drive(DriveError::VersionNotActive {
                method: "prove_readiness_round".to_string(),
                known_versions: vec![0],
            })),
            Some(version) => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "prove_readiness_round".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }

    /// Proves one accepted report of a round.
    ///
    /// # Parameters
    ///
    /// * `contract_id` - The contract.
    /// * `round_id` - The round.
    /// * `pro_tx_hash` - The reporting evonode.
    /// * `transaction` - The current transaction.
    /// * `platform_version` - The platform version to use.
    ///
    /// # Returns
    ///
    /// * The proof bytes.
    /// * `Err(DriveError::VersionNotActive)` on a platform version without readiness.
    pub fn prove_readiness_report(
        &self,
        contract_id: [u8; 32],
        round_id: [u8; 32],
        pro_tx_hash: [u8; 32],
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, Error> {
        match platform_version
            .drive
            .methods
            .vote
            .readiness
            .fetch_reports_page
        {
            Some(0) => self.prove_readiness_report_v0(
                contract_id,
                round_id,
                pro_tx_hash,
                transaction,
                platform_version,
            ),
            None => Err(Error::Drive(DriveError::VersionNotActive {
                method: "prove_readiness_report".to_string(),
                known_versions: vec![0],
            })),
            Some(version) => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "prove_readiness_report".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
