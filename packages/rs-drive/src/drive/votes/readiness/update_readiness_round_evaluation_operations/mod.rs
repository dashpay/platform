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
    /// Rewrites a round's record. The caller has already changed the record in memory (its
    /// evaluation mark, its funding flag or its status); this writes it back under its own
    /// key.
    ///
    /// # Parameters
    ///
    /// * `round` - The record to write.
    /// * `estimated_costs_only_with_layer_info` - `Some` to estimate instead of read state.
    /// * `transaction` - The current transaction.
    /// * `platform_version` - The platform version to use.
    ///
    /// # Returns
    ///
    /// * The low level operations that perform the write.
    /// * `Err(DriveError::VersionNotActive)` on a platform version without readiness.
    pub fn update_readiness_round_evaluation_operations(
        &self,
        round: &ReadinessRound,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        match platform_version
            .drive
            .methods
            .vote
            .readiness
            .update_round_evaluation
        {
            Some(0) => self.update_readiness_round_evaluation_operations_v0(
                round,
                estimated_costs_only_with_layer_info,
                transaction,
                platform_version,
            ),
            None => Err(Error::Drive(DriveError::VersionNotActive {
                method: "update_readiness_round_evaluation_operations".to_string(),
                known_versions: vec![0],
            })),
            Some(version) => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "update_readiness_round_evaluation_operations".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
