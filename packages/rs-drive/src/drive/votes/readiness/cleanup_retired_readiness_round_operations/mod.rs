mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::version::PlatformVersion;
use grovedb::batch::KeyInfoPath;
use grovedb::{EstimatedLayerInformation, TransactionArg};
use std::collections::HashMap;

/// What one cleanup step did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReadinessCleanupOutcome {
    /// Reports deleted by this step.
    pub reports_deleted: u64,
    /// Whether the round is gone: its record, cursor, count tree, round tree and queue entry
    /// were removed (and its contract tree when no live round remains).
    pub finished: bool,
}

impl Drive {
    /// Runs one bounded cleanup step over a retired round: deletes up to `max_deletes` of
    /// its reports and, once none remain, its record, cursor and empty count tree, then the
    /// round tree and the contract tree while empty, and finally its queue entry. The
    /// round's subtree has been unreachable through the pointer since it retired, so this
    /// only reclaims storage; the work is paid by the cleanup reserve charged at retirement.
    ///
    /// # Parameters
    ///
    /// * `round_id` - The retired round.
    /// * `contract_id` - Its contract.
    /// * `max_deletes` - The most reports this step may delete.
    /// * `estimated_costs_only_with_layer_info` - `Some` to estimate instead of read state; an
    ///   estimate prices `max_deletes` deletes plus the fixed tail.
    /// * `transaction` - The current transaction.
    /// * `platform_version` - The platform version to use.
    ///
    /// # Returns
    ///
    /// * What the step did and the low level operations that perform it.
    /// * `Err(DriveError::VersionNotActive)` on a platform version without readiness.
    pub fn cleanup_retired_readiness_round_operations(
        &self,
        round_id: [u8; 32],
        contract_id: [u8; 32],
        max_deletes: u16,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(ReadinessCleanupOutcome, Vec<LowLevelDriveOperation>), Error> {
        match platform_version
            .drive
            .methods
            .vote
            .readiness
            .cleanup_retired_round
        {
            Some(0) => self.cleanup_retired_readiness_round_operations_v0(
                round_id,
                contract_id,
                max_deletes,
                estimated_costs_only_with_layer_info,
                transaction,
                platform_version,
            ),
            None => Err(Error::Drive(DriveError::VersionNotActive {
                method: "cleanup_retired_readiness_round_operations".to_string(),
                known_versions: vec![0],
            })),
            Some(version) => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "cleanup_retired_readiness_round_operations".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
