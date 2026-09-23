use crate::drive::votes::paths::{vote_decisions_active_polls_tree_path_vec, YesNoVotePollPaths};
use crate::drive::votes::YesNoAbstainVoteChoiceToKeyTrait;
use crate::drive::Drive;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::grove_operations::BatchInsertTreeApplyType;
use crate::util::object_size_info::{DriveKeyInfo, PathKeyInfo};
use dpp::block::block_info::BlockInfo;
use dpp::prelude::TimestampMillis;
use dpp::version::PlatformVersion;
use dpp::voting::vote_choices::yes_no_abstain_vote_choice::YesNoAbstainVoteChoice;
use dpp::voting::vote_info_storage::yes_no_vote_poll_stored_info::YesNoVotePollStoredInfo;
use dpp::voting::vote_polls::yes_no_vote_poll::YesNoVotePoll;
use dpp::voting::vote_polls::VotePoll;
use dpp::ProtocolError;
use grovedb::{TransactionArg, TreeType};

impl Drive {
    pub(super) fn open_yes_no_vote_poll_v0(
        &self,
        vote_poll: &YesNoVotePoll,
        end_date: TimestampMillis,
        block_info: &BlockInfo,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        let mut batch_operations = vec![];
        self.open_yes_no_vote_poll_operations_v0(
            vote_poll,
            end_date,
            block_info,
            &mut None,
            &mut batch_operations,
            transaction,
            platform_version,
        )?;
        self.apply_batch_low_level_drive_operations(
            None,
            transaction,
            batch_operations,
            &mut vec![],
            &platform_version.drive,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn open_yes_no_vote_poll_operations_v0(
        &self,
        vote_poll: &YesNoVotePoll,
        end_date: TimestampMillis,
        block_info: &BlockInfo,
        previous_batch_operations: &mut Option<&mut Vec<LowLevelDriveOperation>>,
        batch_operations: &mut Vec<LowLevelDriveOperation>,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        vote_poll.validate_parameters(platform_version)?;
        let poll_id = vote_poll.unique_id()?;

        // The poll's own tree. A poll that already has one was opened before.
        let inserted = self.batch_insert_empty_tree_if_not_exists(
            PathKeyInfo::PathKey::<0>((
                vote_decisions_active_polls_tree_path_vec(),
                poll_id.to_vec(),
            )),
            TreeType::NormalTree,
            None,
            BatchInsertTreeApplyType::StatefulBatchInsertTree,
            transaction,
            previous_batch_operations,
            batch_operations,
            &platform_version.drive,
        )?;
        if !inserted {
            return Err(Error::Protocol(Box::new(ProtocolError::VoteError(
                format!(
                    "yes/no vote poll {} is already open or was already decided",
                    vote_poll
                ),
            ))));
        }

        // Its stored info: started in this block.
        let stored_info = YesNoVotePollStoredInfo::new(*block_info, platform_version)?;
        let mut stored_info_operations = self.insert_stored_info_for_yes_no_vote_poll_operations(
            vote_poll,
            stored_info,
            platform_version,
        )?;
        batch_operations.append(&mut stored_info_operations);

        // One sum tree per choice; the sums are the tallies.
        let poll_path = vote_poll.poll_path_vec()?;
        for vote_choice in YesNoAbstainVoteChoice::ALL {
            self.batch_insert_empty_sum_tree(
                poll_path.iter().map(|segment| segment.as_slice()),
                DriveKeyInfo::Key(vec![vote_choice.to_tree_key()]),
                None,
                batch_operations,
                &platform_version.drive,
            )?;
        }

        // The end date index closes the poll when its time comes. Nobody is refunded for the
        // entry: the poll is a system action, so it carries no storage flags.
        self.add_vote_poll_end_date_query_operations(
            None,
            VotePoll::YesNoVotePoll(vote_poll.clone()),
            end_date,
            block_info,
            &mut None,
            previous_batch_operations,
            batch_operations,
            transaction,
            platform_version,
        )
    }
}
