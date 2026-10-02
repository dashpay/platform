mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::version::PlatformVersion;
use dpp::voting::readiness::round::ReadinessRound;
use grovedb::TransactionArg;

impl Drive {
    /// Fetches a contract's current compilation readiness round: the pointer, then the record
    /// under the round it names.
    ///
    /// # Parameters
    ///
    /// * `contract_id` - The contract.
    /// * `transaction` - The current transaction.
    /// * `platform_version` - The platform version to use.
    ///
    /// # Returns
    ///
    /// * `Ok(None)` when the contract has no round.
    /// * `Err(DriveError::VersionNotActive)` on a platform version without readiness.
    pub fn fetch_readiness_round(
        &self,
        contract_id: [u8; 32],
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Option<ReadinessRound>, Error> {
        let mut drive_operations = vec![];
        self.fetch_readiness_round_operations(
            contract_id,
            transaction,
            &mut drive_operations,
            platform_version,
        )
    }

    /// Fetches a contract's current round, accumulating the cost of the reads.
    ///
    /// # Parameters
    ///
    /// * `contract_id` - The contract.
    /// * `transaction` - The current transaction.
    /// * `drive_operations` - The accumulator the read costs are appended to.
    /// * `platform_version` - The platform version to use.
    ///
    /// # Returns
    ///
    /// * `Ok(None)` when the contract has no round.
    /// * `Err(DriveError::VersionNotActive)` on a platform version without readiness.
    pub fn fetch_readiness_round_operations(
        &self,
        contract_id: [u8; 32],
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<Option<ReadinessRound>, Error> {
        match platform_version.drive.methods.vote.readiness.fetch_round {
            Some(0) => self.fetch_readiness_round_operations_v0(
                contract_id,
                transaction,
                drive_operations,
                platform_version,
            ),
            None => Err(Error::Drive(DriveError::VersionNotActive {
                method: "fetch_readiness_round_operations".to_string(),
                known_versions: vec![0],
            })),
            Some(version) => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "fetch_readiness_round_operations".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }

    /// Fetches the round id a contract's pointer names, accumulating the cost of the read.
    ///
    /// # Parameters
    ///
    /// * `contract_id` - The contract.
    /// * `transaction` - The current transaction.
    /// * `drive_operations` - The accumulator the read cost is appended to.
    /// * `platform_version` - The platform version to use.
    ///
    /// # Returns
    ///
    /// * `Ok(None)` when the contract has no round.
    pub fn fetch_readiness_current_round_id_operations(
        &self,
        contract_id: [u8; 32],
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<Option<[u8; 32]>, Error> {
        match platform_version.drive.methods.vote.readiness.fetch_round {
            Some(0) => self.fetch_readiness_current_round_id_operations_v0(
                contract_id,
                transaction,
                drive_operations,
                platform_version,
            ),
            None => Err(Error::Drive(DriveError::VersionNotActive {
                method: "fetch_readiness_current_round_id_operations".to_string(),
                known_versions: vec![0],
            })),
            Some(version) => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "fetch_readiness_current_round_id_operations".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }

    /// Fetches the record of a round by contract id and round id, accumulating the cost of
    /// the read. Used for retired rounds that the pointer no longer names.
    ///
    /// # Parameters
    ///
    /// * `contract_id` - The contract.
    /// * `round_id` - The round.
    /// * `transaction` - The current transaction.
    /// * `drive_operations` - The accumulator the read cost is appended to.
    /// * `platform_version` - The platform version to use.
    ///
    /// # Returns
    ///
    /// * `Ok(None)` when no such round tree or record exists.
    pub fn fetch_readiness_round_record_operations(
        &self,
        contract_id: [u8; 32],
        round_id: [u8; 32],
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<Option<ReadinessRound>, Error> {
        match platform_version.drive.methods.vote.readiness.fetch_round {
            Some(0) => self.fetch_readiness_round_record_operations_v0(
                contract_id,
                round_id,
                transaction,
                drive_operations,
                platform_version,
            ),
            None => Err(Error::Drive(DriveError::VersionNotActive {
                method: "fetch_readiness_round_record_operations".to_string(),
                known_versions: vec![0],
            })),
            Some(version) => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "fetch_readiness_round_record_operations".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
