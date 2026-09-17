mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

/// One entry of the activation deadline queue.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReadinessRoundDue {
    /// The deadline the entry sits under.
    pub deadline_ms: u64,
    /// The contract.
    pub contract_id: [u8; 32],
    /// The round the entry was queued for. The block event checks it is still the
    /// contract's current round before activating; a stale entry is dropped.
    pub round_id: [u8; 32],
}

impl Drive {
    /// Fetches up to `limit` deadline queue entries whose deadline is at or before `at_ms`,
    /// oldest first.
    ///
    /// # Parameters
    ///
    /// * `at_ms` - The committed block time.
    /// * `limit` - The maximum number of entries to return.
    /// * `transaction` - The current transaction.
    /// * `drive_operations` - The accumulator the read cost is appended to.
    /// * `platform_version` - The platform version to use.
    ///
    /// # Returns
    ///
    /// * The due entries, oldest first.
    /// * `Err(DriveError::VersionNotActive)` on a platform version without readiness.
    pub fn fetch_readiness_rounds_due_operations(
        &self,
        at_ms: u64,
        limit: u16,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<ReadinessRoundDue>, Error> {
        match platform_version.drive.methods.vote.readiness.fetch_rounds_due {
            Some(0) => self.fetch_readiness_rounds_due_operations_v0(
                at_ms,
                limit,
                transaction,
                drive_operations,
                platform_version,
            ),
            None => Err(Error::Drive(DriveError::VersionNotActive {
                method: "fetch_readiness_rounds_due_operations".to_string(),
                known_versions: vec![0],
            })),
            Some(version) => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "fetch_readiness_rounds_due_operations".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }

    /// Fetches up to `limit` deadline queue entries whose deadline is at or before `at_ms`.
    ///
    /// # Parameters
    ///
    /// * `at_ms` - The committed block time.
    /// * `limit` - The maximum number of entries to return.
    /// * `transaction` - The current transaction.
    /// * `platform_version` - The platform version to use.
    ///
    /// # Returns
    ///
    /// * The due entries, oldest first.
    pub fn fetch_readiness_rounds_due(
        &self,
        at_ms: u64,
        limit: u16,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<ReadinessRoundDue>, Error> {
        self.fetch_readiness_rounds_due_operations(
            at_ms,
            limit,
            transaction,
            &mut vec![],
            platform_version,
        )
    }
}
