mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::version::PlatformVersion;
use dpp::voting::readiness::round::ReadinessRound;
use grovedb::batch::KeyInfoPath;
use grovedb::{EstimatedLayerInformation, TransactionArg};
use std::collections::HashMap;

impl Drive {
    /// Records a fully validated crossing: marks the round crossed at `crossing_ms` with a
    /// deadline of `crossing_ms + clamp(crossing_ms - accepted_at_ms, min_wait_ms,
    /// max_wait_ms)`, rewrites the record and queues the deadline. Any open scan cursor is
    /// removed. The caller has already done the eligibility walk; nothing here checks it.
    ///
    /// # Parameters
    ///
    /// * `round` - The pending round; updated to its crossed state only once every operation
    ///   is built, and left untouched by an estimate or a failure.
    /// * `crossing_ms` - The committed block time of the crossing.
    /// * `min_wait_ms` - The lower bound of the additional wait (from `SystemLimits`).
    /// * `max_wait_ms` - The upper bound of the additional wait (from `SystemLimits`).
    /// * `estimated_costs_only_with_layer_info` - `Some` to estimate instead of read state.
    /// * `transaction` - The current transaction.
    /// * `platform_version` - The platform version to use.
    ///
    /// # Returns
    ///
    /// * The deadline and the low level operations that perform the writes.
    /// * `Err(DriveError::VersionNotActive)` on a platform version without readiness.
    #[allow(clippy::too_many_arguments)]
    pub fn record_readiness_crossing_operations(
        &self,
        round: &mut ReadinessRound,
        crossing_ms: u64,
        min_wait_ms: u64,
        max_wait_ms: u64,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(u64, Vec<LowLevelDriveOperation>), Error> {
        match platform_version
            .drive
            .methods
            .vote
            .readiness
            .record_crossing
        {
            Some(0) => self.record_readiness_crossing_operations_v0(
                round,
                crossing_ms,
                min_wait_ms,
                max_wait_ms,
                estimated_costs_only_with_layer_info,
                transaction,
                platform_version,
            ),
            None => Err(Error::Drive(DriveError::VersionNotActive {
                method: "record_readiness_crossing_operations".to_string(),
                known_versions: vec![0],
            })),
            Some(version) => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "record_readiness_crossing_operations".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
