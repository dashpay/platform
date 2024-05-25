mod v0;

use crate::drive::Drive;
use grovedb::batch::KeyInfoPath;
use std::collections::HashMap;

use crate::error::drive::DriveError;
use crate::error::Error;

use crate::fee::op::LowLevelDriveOperation;
use dpp::block::block_info::BlockInfo;
use dpp::prelude::TimestampMillis;
use dpp::version::PlatformVersion;
use dpp::voting::vote_polls::VotePoll;
use grovedb::{EstimatedLayerInformation, TransactionArg};

impl Drive {
    /// We add votes poll references by end date in order to be able to check on every new block if
    /// any votes poll should be closed. This will remove them to recoup space
    pub fn remove_vote_poll_end_date_query_operations(
        &self,
        vote_polls: Vec<VotePoll>,
        end_date: TimestampMillis,
        batch_operations: &mut Vec<LowLevelDriveOperation>,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        match platform_version
            .drive
            .methods
            .vote
            .contested_resource_insert
            .remove_vote_poll_end_date_query_operations
        {
            0 => self.remove_vote_poll_end_date_query_operations_v0(
                vote_polls,
                end_date,
                batch_operations,
                transaction,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "remove_vote_poll_end_date_query".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
