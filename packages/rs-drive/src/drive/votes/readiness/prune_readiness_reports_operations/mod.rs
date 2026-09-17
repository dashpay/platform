mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::version::PlatformVersion;
use grovedb::batch::KeyInfoPath;
use grovedb::{EstimatedLayerInformation, TransactionArg};
use std::collections::HashMap;

impl Drive {
    /// Deletes the named reports from a round's count tree: the reporters the block event
    /// found ineligible under the block's membership view. The count tree's count drops by
    /// the number deleted.
    ///
    /// # Parameters
    ///
    /// * `contract_id` - The contract.
    /// * `round_id` - The round.
    /// * `pro_tx_hashes` - The reports to delete; each must exist.
    /// * `estimated_costs_only_with_layer_info` - `Some` to estimate instead of read state.
    /// * `transaction` - The current transaction.
    /// * `platform_version` - The platform version to use.
    ///
    /// # Returns
    ///
    /// * The low level operations that perform the deletes.
    /// * `Err(DriveError::VersionNotActive)` on a platform version without readiness.
    pub fn prune_readiness_reports_operations(
        &self,
        contract_id: [u8; 32],
        round_id: [u8; 32],
        pro_tx_hashes: &[[u8; 32]],
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        match platform_version.drive.methods.vote.readiness.prune_reports {
            Some(0) => self.prune_readiness_reports_operations_v0(
                contract_id,
                round_id,
                pro_tx_hashes,
                estimated_costs_only_with_layer_info,
                transaction,
                platform_version,
            ),
            None => Err(Error::Drive(DriveError::VersionNotActive {
                method: "prune_readiness_reports_operations".to_string(),
                known_versions: vec![0],
            })),
            Some(version) => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "prune_readiness_reports_operations".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
