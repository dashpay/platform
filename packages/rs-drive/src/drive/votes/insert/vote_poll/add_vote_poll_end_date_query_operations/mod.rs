mod v0;

use std::collections::HashMap;
use grovedb::batch::KeyInfoPath;
use crate::drive::Drive;

use crate::error::drive::DriveError;
use crate::error::Error;

use dpp::fee::fee_result::FeeResult;

use dpp::prelude::{Identifier, TimestampMillis};
use dpp::version::PlatformVersion;
use grovedb::{EstimatedLayerInformation, TransactionArg};
use dpp::voting::vote_polls::VotePoll;
use crate::fee::op::LowLevelDriveOperation;

impl Drive {
    /// We add votes poll references by end date in order to be able to check on every new block if
    /// any votes poll should be closed.
    pub fn add_vote_poll_end_date_query_operations(
        &self,
        vote_poll: VotePoll,
        end_date: TimestampMillis,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        previous_batch_operations: &mut Option<&mut Vec<LowLevelDriveOperation>>,
        batch_operations: &mut Vec<LowLevelDriveOperation>,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<FeeResult, Error> {
        match platform_version
            .drive
            .methods
            .vote
            .contested_resource_insert
            .add_vote_poll_end_date_query
        {
            0 => self.add_vote_poll_end_date_query_operations_v0(vote_poll, end_date, estimated_costs_only_with_layer_info, previous_batch_operations, batch_operations, transaction, platform_version),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "add_vote_poll_end_date_query_operations".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
