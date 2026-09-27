mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::block::block_info::BlockInfo;
use dpp::prelude::TimestampMillis;
use dpp::version::PlatformVersion;
use grovedb::batch::KeyInfoPath;
use grovedb::EstimatedLayerInformation;
use std::collections::HashMap;

impl Drive {
    /// Records that `recipient_id` claimed the once-per-identity distribution of a token at
    /// `claimed_at_ms`, so every later claim by that identity is rejected.
    pub fn mark_once_per_identity_release_as_distributed_operations(
        &self,
        token_id: [u8; 32],
        recipient_id: [u8; 32],
        claimed_at_ms: TimestampMillis,
        block_info: &BlockInfo,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        match platform_version
            .drive
            .methods
            .token
            .distribution
            .mark_once_per_identity_release_as_distributed
        {
            0 => self.mark_once_per_identity_release_as_distributed_operations_v0(
                token_id,
                recipient_id,
                claimed_at_ms,
                block_info,
                estimated_costs_only_with_layer_info,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "mark_once_per_identity_release_as_distributed".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
