mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::block::block_info::BlockInfo;
use dpp::fee::fee_result::FeeResult;
use dpp::identity::TimestampMillis;
use dpp::version::PlatformVersion;
use dpp::voting::vote_polls::identity_contender_vote_poll::IdentityContenderVotePoll;
use grovedb::batch::KeyInfoPath;
use grovedb::{EstimatedLayerInformation, TransactionArg};
use std::collections::HashMap;

impl Drive {
    /// Opens an identity contender vote poll in its join phase and applies the operations:
    /// its stored info, its abstain votes tree and its end date entry at the end of the join
    /// phase. Contenders are added separately, and the poll's prefunded specialized balance is
    /// funded separately. Opening a poll that already has a state refuses.
    pub fn open_identity_contender_vote_poll(
        &self,
        vote_poll: &IdentityContenderVotePoll,
        join_end_time_ms: TimestampMillis,
        vote_end_time_ms: TimestampMillis,
        block_info: &BlockInfo,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<FeeResult, Error> {
        match platform_version
            .drive
            .methods
            .vote
            .identity_contender
            .open_identity_contender_vote_poll_operations
        {
            0 => self.open_identity_contender_vote_poll_v0(
                vote_poll,
                join_end_time_ms,
                vote_end_time_ms,
                block_info,
                transaction,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "open_identity_contender_vote_poll".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }

    /// The operations opening an identity contender vote poll in its join phase: its stored
    /// info, its abstain votes tree and its end date entry at the end of the join phase, plus
    /// the trees of the branch when this is the first such poll.
    #[allow(clippy::too_many_arguments)]
    pub fn open_identity_contender_vote_poll_operations(
        &self,
        vote_poll: &IdentityContenderVotePoll,
        join_end_time_ms: TimestampMillis,
        vote_end_time_ms: TimestampMillis,
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
            .identity_contender
            .open_identity_contender_vote_poll_operations
        {
            0 => self.open_identity_contender_vote_poll_operations_v0(
                vote_poll,
                join_end_time_ms,
                vote_end_time_ms,
                block_info,
                estimated_costs_only_with_layer_info,
                previous_batch_operations,
                batch_operations,
                transaction,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "open_identity_contender_vote_poll_operations".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
