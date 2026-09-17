mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

impl Drive {
    /// Fetches the raw distinct report count of a round: the count of its reports count tree.
    /// One key per pro tx hash, so a retransmission cannot inflate it. The count is not an
    /// eligibility proof; the block event validates the reporters against the membership
    /// view before a crossing.
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
    /// * The count, `0` when the round has no reports tree.
    /// * `Err(DriveError::VersionNotActive)` on a platform version without readiness.
    pub fn fetch_readiness_round_raw_count_operations(
        &self,
        contract_id: [u8; 32],
        round_id: [u8; 32],
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<u64, Error> {
        match platform_version.drive.methods.vote.readiness.fetch_raw_count {
            Some(0) => self.fetch_readiness_round_raw_count_operations_v0(
                contract_id,
                round_id,
                transaction,
                drive_operations,
                platform_version,
            ),
            None => Err(Error::Drive(DriveError::VersionNotActive {
                method: "fetch_readiness_round_raw_count_operations".to_string(),
                known_versions: vec![0],
            })),
            Some(version) => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "fetch_readiness_round_raw_count_operations".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }

    /// Fetches the raw distinct report count of a round.
    ///
    /// # Parameters
    ///
    /// * `contract_id` - The contract.
    /// * `round_id` - The round.
    /// * `transaction` - The current transaction.
    /// * `platform_version` - The platform version to use.
    ///
    /// # Returns
    ///
    /// * The count, `0` when the round has no reports tree.
    pub fn fetch_readiness_round_raw_count(
        &self,
        contract_id: [u8; 32],
        round_id: [u8; 32],
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<u64, Error> {
        self.fetch_readiness_round_raw_count_operations(
            contract_id,
            round_id,
            transaction,
            &mut vec![],
            platform_version,
        )
    }
}
