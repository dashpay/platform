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
    /// Stores the block event's fairness cursor: the last contract id it visited, or clears
    /// it when the sweep wrapped.
    ///
    /// # Parameters
    ///
    /// * `last_contract_id` - The contract to resume after; `None` clears the cursor.
    /// * `estimated_costs_only_with_layer_info` - `Some` to estimate instead of read state.
    /// * `transaction` - The current transaction.
    /// * `platform_version` - The platform version to use.
    ///
    /// # Returns
    ///
    /// * The low level operations that perform the write.
    /// * `Err(DriveError::VersionNotActive)` on a platform version without readiness.
    pub fn store_readiness_evaluation_cursor_operations(
        &self,
        last_contract_id: Option<[u8; 32]>,
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
            .store_evaluation_cursor
        {
            Some(0) => self.store_readiness_evaluation_cursor_operations_v0(
                last_contract_id,
                estimated_costs_only_with_layer_info,
                transaction,
                platform_version,
            ),
            None => Err(Error::Drive(DriveError::VersionNotActive {
                method: "store_readiness_evaluation_cursor_operations".to_string(),
                known_versions: vec![0],
            })),
            Some(version) => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "store_readiness_evaluation_cursor_operations".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
