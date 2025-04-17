mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::block::block_info::BlockInfo;
use dpp::prelude::TimestampMillis;
use dpp::version::PlatformVersion;
use grovedb::batch::KeyInfoPath;
use grovedb::{EstimatedLayerInformation, TransactionArg};
use std::collections::HashMap;

impl Drive {
    /// Marks a pre‑programmed token release as distributed in the state tree.
    ///
    /// This function removes the scheduled pre‑programmed release (i.e. its reference) from the
    /// distribution queue. In particular, it deletes the reference entry from the millisecond‑timed
    /// distributions tree for the given token, release time, and identity.
    ///
    /// # Parameters
    /// - `token_id`: The unique 32‑byte identifier of the token.
    /// - `owner_id`: The unique 32‑byte identifier of the owner initiating the distribution.
    /// - `identity_id`: The 32‑byte identity identifier for which the pre‑programmed release was scheduled.
    /// - `release_time`: The scheduled release time (in milliseconds).
    /// - `block_info`: Metadata about the current block, including epoch details.
    /// - `estimated_costs_only_with_layer_info`: Optional storage layer information for cost estimation.
    /// - `transaction`: The transaction context.
    /// - `platform_version`: The current platform version.
    ///
    /// # Returns
    /// - `Ok(operations)` if the operation succeeds.
    /// - `Err(Error::Drive(DriveError::UnknownVersionMismatch))` if an unsupported version is encountered.
    ///
    /// # Versioning
    /// - Uses version 0 of `mark_pre_programmed_release_as_distributed_operations_v0` if supported.
    /// - Returns an error if an unknown version is received.
    #[allow(clippy::too_many_arguments)]
    pub fn mark_pre_programmed_release_as_distributed_operations(
        &self,
        token_id: [u8; 32],
        recipient_id: [u8; 32],
        release_time: TimestampMillis,
        block_info: &BlockInfo,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        match platform_version
            .drive
            .methods
            .token
            .distribution
            .mark_pre_programmed_release_as_distributed
        {
            0 => self.mark_pre_programmed_release_as_distributed_operations_v0(
                token_id,
                recipient_id,
                release_time,
                block_info,
                estimated_costs_only_with_layer_info,
                transaction,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "mark_pre_programmed_release_as_distributed".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
