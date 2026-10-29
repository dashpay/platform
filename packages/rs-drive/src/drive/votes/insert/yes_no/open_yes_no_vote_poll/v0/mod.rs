use crate::drive::votes::paths::{
    vote_decisions_active_polls_tree_path_vec, vote_decisions_tree_path_vec, YesNoVotePollPaths,
    VOTE_DECISIONS_SUBTREE_KEYS,
};
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
        // A poll that ends in the block that opens it, or earlier, would close at the end of
        // this block with no vote cast. One that runs longer than the system limit would hold
        // its fund and trees for as long. The limit also keeps the end date below `i64::MAX`,
        // past which the end date index (which flips its top bit, `encode_u64`) would sort it
        // before every block time.
        let max_duration = platform_version
            .system_limits
            .max_yes_no_vote_poll_duration_ms;
        if end_date <= block_info.time_ms
            || end_date - block_info.time_ms > max_duration
            || end_date > i64::MAX as TimestampMillis
        {
            return Err(Error::Protocol(Box::new(ProtocolError::VoteError(
                format!(
                    "yes/no vote poll {} must end after the block that opens it (at {}), at \
                     most {} ms later and at most at {}, not at {}",
                    vote_poll,
                    block_info.time_ms,
                    max_duration,
                    i64::MAX,
                    end_date
                ),
            ))));
        }
        let poll_id = vote_poll.unique_id(platform_version)?;

        // The decisions trees, when the chain lacks them: genesis and the upgrade to protocol
        // version 15 create them, but a chain that reached 15 before they existed did not. An
        // earlier opening in the batch, among the previous operations, may already queue them.
        for subtree_key in VOTE_DECISIONS_SUBTREE_KEYS {
            self.batch_insert_empty_tree_if_not_exists(
                PathKeyInfo::PathKey::<0>((
                    vote_decisions_tree_path_vec(),
                    vec![subtree_key as u8],
                )),
                TreeType::NormalTree,
                None,
                BatchInsertTreeApplyType::StatefulBatchInsertTree,
                transaction,
                previous_batch_operations,
                batch_operations,
                &platform_version.drive,
            )?;
        }

        // The poll's own tree. A poll that already has one, in state or among the previous
        // operations, was opened before: opened twice, it would be listed under two end dates.
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
        let poll_path = vote_poll.poll_path_vec(platform_version)?;
        for vote_choice in YesNoAbstainVoteChoice::ALL {
            self.batch_insert_empty_sum_tree(
                poll_path.iter().map(|segment| segment.as_slice()),
                DriveKeyInfo::Key(vec![vote_choice.to_tree_key()]),
                None,
                batch_operations,
                &platform_version.drive,
            )?;
        }

        // The end date index closes the poll when its time comes: the entry a contest gets, under
        // the end date's tree. Nobody is refunded for what an opening adds: the poll is a system
        // action, so it carries no storage flags. A contest among the previous operations may
        // already queue the end date's tree with its contender's flags, which the tree keeps, or
        // queue its deletion (a contest leaving its join-window end date), which the insert
        // cancels.
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
        )?;
        Ok(())
    }
}
