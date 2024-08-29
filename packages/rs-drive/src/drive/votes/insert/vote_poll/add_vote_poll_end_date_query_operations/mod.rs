mod v0;

use crate::drive::Drive;
use grovedb::batch::KeyInfoPath;
use std::collections::HashMap;

use crate::error::drive::DriveError;
use crate::error::Error;

use crate::fees::op::LowLevelDriveOperation;
use dpp::block::block_info::BlockInfo;
use dpp::prelude::TimestampMillis;
use dpp::version::PlatformVersion;
use dpp::voting::vote_polls::VotePoll;
use grovedb::{EstimatedLayerInformation, TransactionArg};

impl Drive {
    /// We add votes poll references by end date in order to be able to check on every new block if
    /// any votes poll should be closed.
    pub fn add_vote_poll_end_date_query_operations(
        &self,
        creator_identity_id: Option<[u8; 32]>,
        vote_poll: VotePoll,
        end_date: TimestampMillis,
        block_info: &BlockInfo,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        previous_batch_operations: &mut Option<&mut Vec<LowLevelDriveOperation>>,
        batch_operations: &mut Vec<LowLevelDriveOperation>,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        match platform_version
            .drive
            .methods
            .vote
            .contested_resource_insert
            .add_vote_poll_end_date_query_operations
        {
            0 => self.add_vote_poll_end_date_query_operations_v0(
                creator_identity_id,
                vote_poll,
                end_date,
                block_info,
                estimated_costs_only_with_layer_info,
                previous_batch_operations,
                batch_operations,
                transaction,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "add_vote_poll_end_date_query_operations".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
