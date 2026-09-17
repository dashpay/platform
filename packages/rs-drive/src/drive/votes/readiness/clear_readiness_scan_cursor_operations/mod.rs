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
    /// Removes the persisted position of a round's paged eligibility walk if one is open.
    ///
    /// # Parameters
    ///
    /// * `contract_id` - The contract.
    /// * `round_id` - The round.
    /// * `estimated_costs_only_with_layer_info` - `Some` to estimate instead of read state.
    /// * `transaction` - The current transaction.
    /// * `platform_version` - The platform version to use.
    ///
    /// # Returns
    ///
    /// * The low level operations that perform the delete (empty when no cursor is open).
    /// * `Err(DriveError::VersionNotActive)` on a platform version without readiness.
    pub fn clear_readiness_scan_cursor_operations(
        &self,
        contract_id: [u8; 32],
        round_id: [u8; 32],
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        match platform_version.drive.methods.vote.readiness.clear_scan_cursor {
            Some(0) => self.clear_readiness_scan_cursor_operations_v0(
                contract_id,
                round_id,
                estimated_costs_only_with_layer_info,
                transaction,
                platform_version,
            ),
            None => Err(Error::Drive(DriveError::VersionNotActive {
                method: "clear_readiness_scan_cursor_operations".to_string(),
                known_versions: vec![0],
            })),
            Some(version) => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "clear_readiness_scan_cursor_operations".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
