use crate::drive::Drive;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::batch::drive_op_batch::DriveLowLevelOperationConverter;
use dpp::block::block_info::BlockInfo;
use dpp::prelude::TimestampMillis;
use dpp::voting::vote_polls::yes_no_vote_poll::YesNoVotePoll;
use grovedb::batch::KeyInfoPath;
use grovedb::{EstimatedLayerInformation, TransactionArg};
use platform_version::version::PlatformVersion;
use std::collections::HashMap;

/// Operations on vote polls
#[derive(Clone, Debug)]
pub enum VotePollOperationType {
    /// Opens a yes/no vote poll that takes votes until `end_date` (protocol version 15), as
    /// [`Drive::open_yes_no_vote_poll_operations`] does. The poll's fund is credited by a
    /// prefunded balance operation of its own.
    OpenYesNoVotePoll {
        /// The poll to open
        vote_poll: YesNoVotePoll,
        /// When the poll closes, in milliseconds
        end_date: TimestampMillis,
    },
}

impl DriveLowLevelOperationConverter for VotePollOperationType {
    fn into_low_level_drive_operations(
        self,
        drive: &Drive,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        block_info: &BlockInfo,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        match self {
            VotePollOperationType::OpenYesNoVotePoll {
                vote_poll,
                end_date,
            } => {
                // Converted without the batch's earlier operations, which the opener would use
                // to share an end date's tree with another opening or a contest in the batch.
                // None can be there: a documents batch holds one transition
                // (`max_transitions_in_documents_batch` is 1 at every protocol version, pinned by
                // `documents_batch_is_capped_at_one_transition_at_every_protocol_version`), and
                // the create opening the poll writes no votes tree besides. Raising the cap needs `apply_drive_operations` to give each
                // conversion the batch's earlier operations first.
                let mut batch_operations = vec![];
                drive.open_yes_no_vote_poll_operations(
                    &vote_poll,
                    end_date,
                    block_info,
                    estimated_costs_only_with_layer_info,
                    &mut None,
                    &mut batch_operations,
                    transaction,
                    platform_version,
                )?;
                Ok(batch_operations)
            }
        }
    }
}
