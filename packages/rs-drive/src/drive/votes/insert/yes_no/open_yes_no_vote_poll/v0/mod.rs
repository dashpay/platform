use crate::drive::votes::paths::{
    vote_decisions_active_polls_tree_path_vec, vote_decisions_tree_path_vec,
    vote_end_date_queries_tree_path_vec, YesNoVotePollPaths, VOTE_DECISIONS_SUBTREE_KEYS,
};
use crate::drive::votes::YesNoAbstainVoteChoiceToKeyTrait;
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::fees::op::LowLevelDriveOperation::{EphemeralGroveOperation, GroveOperation};
use crate::util::common::encode::encode_u64;
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
use grovedb::batch::key_info::KeyInfo;
use grovedb::batch::KeyInfoPath;
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
        // Yes/no polls exist from protocol version 15. The end-of-block generations that
        // protocol version 14 also selects (`check_for_ended_vote_polls` v1) take their yes/no
        // handling in place on the ground that no yes/no poll is in the end date index before
        // 15, and this refusal is what keeps that true.
        if platform_version.protocol_version < 15 {
            return Err(Error::Drive(DriveError::NotSupported(
                "yes/no vote polls can only be opened from protocol version 15",
            )));
        }
        vote_poll.validate_parameters(platform_version)?;
        // A poll that ends in the block that opens it, or earlier, would close at the end of
        // this block with no vote cast.
        if end_date <= block_info.time_ms {
            return Err(Error::Protocol(Box::new(ProtocolError::VoteError(
                format!(
                    "yes/no vote poll {} must end after the block that opens it (at {}), not at {}",
                    vote_poll, block_info.time_ms, end_date
                ),
            ))));
        }
        let poll_id = vote_poll.unique_id()?;

        // The decisions trees, when the chain lacks them: genesis and the upgrade to protocol
        // version 15 create them, but a chain that reached 15 before they existed did not. An
        // earlier opening in this batch may already queue them.
        for subtree_key in VOTE_DECISIONS_SUBTREE_KEYS {
            let key = vec![subtree_key as u8];
            if !tree_queued_in(batch_operations, vote_decisions_tree_path_vec(), &key) {
                self.batch_insert_empty_tree_if_not_exists(
                    PathKeyInfo::PathKey::<0>((vote_decisions_tree_path_vec(), key)),
                    TreeType::NormalTree,
                    None,
                    BatchInsertTreeApplyType::StatefulBatchInsertTree,
                    transaction,
                    previous_batch_operations,
                    batch_operations,
                    &platform_version.drive,
                )?;
            }
        }

        // The poll's own tree. A poll that already has one was opened before, and so was one
        // whose tree an earlier opening queued in this batch: opened twice, it would be listed
        // under two end dates.
        let active_polls_path = vote_decisions_active_polls_tree_path_vec();
        let queued_in_this_batch = tree_queued_in(
            batch_operations,
            active_polls_path.clone(),
            poll_id.as_slice(),
        );
        let inserted = !queued_in_this_batch
            && self.batch_insert_empty_tree_if_not_exists(
                PathKeyInfo::PathKey::<0>((active_polls_path, poll_id.to_vec())),
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
        // entry: the poll is a system action, so it carries no storage flags. When an earlier
        // opening in this batch already queues the end date's tree, this batch serves as the
        // operations to check, so the tree is queued once.
        let end_date_tree_queued = tree_queued_in(
            batch_operations,
            vote_end_date_queries_tree_path_vec(),
            &encode_u64(end_date),
        );
        if end_date_tree_queued {
            let mut end_date_operations = vec![];
            self.add_vote_poll_end_date_query_operations(
                None,
                VotePoll::YesNoVotePoll(vote_poll.clone()),
                end_date,
                block_info,
                &mut None,
                &mut Some(batch_operations),
                &mut end_date_operations,
                transaction,
                platform_version,
            )?;
            batch_operations.append(&mut end_date_operations);
            Ok(())
        } else {
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
}

/// Whether `batch_operations` already queues a grove operation on `key` under `path`, which
/// `batch_insert_empty_tree_if_not_exists` only looks for in the previous operations.
fn tree_queued_in(
    batch_operations: &[LowLevelDriveOperation],
    path: Vec<Vec<u8>>,
    key: &[u8],
) -> bool {
    let path = KeyInfoPath::from_known_owned_path(path);
    let key = KeyInfo::KnownKey(key.to_vec());
    batch_operations.iter().any(|operation| {
        matches!(
            operation,
            GroveOperation(queued) | EphemeralGroveOperation(queued, _)
                if queued.path == path && queued.key.as_ref() == Some(&key)
        )
    })
}
