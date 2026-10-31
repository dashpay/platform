use crate::drive::votes::paths::{
    vote_decisions_active_polls_tree_path, vote_decisions_active_polls_tree_path_vec,
    vote_decisions_tree_path, vote_decisions_tree_path_vec, vote_root_path, YesNoVotePollPaths,
    VOTE_DECISIONS_SUBTREE_KEYS,
};
use crate::drive::votes::YesNoAbstainVoteChoiceToKeyTrait;
use crate::drive::Drive;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::grove_operations::BatchInsertTreeApplyType;
use crate::util::object_size_info::{DriveKeyInfo, PathKeyInfo};
use crate::util::type_constants::DEFAULT_HASH_SIZE_U8;
use dpp::block::block_info::BlockInfo;
use dpp::prelude::TimestampMillis;
use dpp::version::PlatformVersion;
use dpp::voting::vote_choices::yes_no_abstain_vote_choice::YesNoAbstainVoteChoice;
use dpp::voting::vote_info_storage::yes_no_vote_poll_stored_info::YesNoVotePollStoredInfo;
use dpp::voting::vote_polls::yes_no_vote_poll::YesNoVotePoll;
use dpp::voting::vote_polls::VotePoll;
use dpp::ProtocolError;
use grovedb::batch::KeyInfoPath;
use grovedb::EstimatedLayerCount::ApproximateElements;
use grovedb::EstimatedLayerSizes::{AllSubtrees, Mix};
use grovedb::EstimatedSumTrees::{AllSumTrees, NoSumTrees};
use grovedb::{EstimatedLayerInformation, TransactionArg, TreeType};
use std::collections::HashMap;

/// What a yes/no poll's serialized stored info takes, about: the poll with its rule, and its
/// status once it is finished, with two blocks.
const ESTIMATED_YES_NO_VOTE_POLL_STORED_INFO_SIZE: u32 = 160;

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
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        previous_batch_operations: &mut Option<&mut Vec<LowLevelDriveOperation>>,
        batch_operations: &mut Vec<LowLevelDriveOperation>,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        vote_poll.validate_parameters(platform_version)?;
        // The end date must be in the window the opening block allows; validation of what opens
        // the poll refused any other with a consensus error before.
        let end_date_window = YesNoVotePoll::end_date_window(block_info.time_ms, platform_version)?;
        if !end_date_window.contains(&end_date) {
            return Err(Error::Protocol(Box::new(ProtocolError::VoteError(
                format!(
                    "yes/no vote poll {} opened at {} must end from {} to {}, not at {}",
                    vote_poll,
                    block_info.time_ms,
                    end_date_window.start(),
                    end_date_window.end(),
                    end_date
                ),
            ))));
        }
        let poll_id = vote_poll.unique_id(platform_version)?;
        let poll_path = vote_poll.poll_path_vec(platform_version)?;

        // Estimating, nothing is read: every tree is inserted as new.
        let tree_apply_type = if let Some(estimated_costs_only_with_layer_info) =
            estimated_costs_only_with_layer_info
        {
            // The votes tree holds the contested resources, the decisions and the end dates.
            estimated_costs_only_with_layer_info.insert(
                KeyInfoPath::from_known_path(vote_root_path()),
                EstimatedLayerInformation {
                    tree_type: TreeType::NormalTree,
                    estimated_layer_count: ApproximateElements(3),
                    estimated_layer_sizes: AllSubtrees(1, NoSumTrees, None),
                },
            );
            // The decisions tree holds the active polls and the identity votes.
            estimated_costs_only_with_layer_info.insert(
                KeyInfoPath::from_known_path(vote_decisions_tree_path()),
                EstimatedLayerInformation {
                    tree_type: TreeType::NormalTree,
                    estimated_layer_count: ApproximateElements(2),
                    estimated_layer_sizes: AllSubtrees(1, NoSumTrees, None),
                },
            );
            // The active polls, a tree per poll under its id.
            estimated_costs_only_with_layer_info.insert(
                KeyInfoPath::from_known_path(vote_decisions_active_polls_tree_path()),
                EstimatedLayerInformation {
                    tree_type: TreeType::NormalTree,
                    estimated_layer_count: ApproximateElements(100),
                    estimated_layer_sizes: AllSubtrees(DEFAULT_HASH_SIZE_U8, NoSumTrees, None),
                },
            );
            // The poll's tree: its stored info and its three vote sum trees.
            estimated_costs_only_with_layer_info.insert(
                KeyInfoPath::from_known_owned_path(poll_path.clone()),
                EstimatedLayerInformation {
                    tree_type: TreeType::NormalTree,
                    estimated_layer_count: ApproximateElements(4),
                    estimated_layer_sizes: Mix {
                        subtrees_size: Some((1, AllSumTrees, None, 3)),
                        items_size: Some((1, ESTIMATED_YES_NO_VOTE_POLL_STORED_INFO_SIZE, None, 1)),
                        items_with_sum_item_size: None,
                        references_size: None,
                        references_with_sum_item_size: None,
                    },
                },
            );
            BatchInsertTreeApplyType::StatelessBatchInsertTree {
                in_tree_type: TreeType::NormalTree,
                tree_type: TreeType::NormalTree,
                flags_len: 0,
            }
        } else {
            BatchInsertTreeApplyType::StatefulBatchInsertTree
        };

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
                tree_apply_type,
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
            tree_apply_type,
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

        // Its stored info: the poll with its rule, which votes naming the poll by id are judged
        // by, started in this block.
        let stored_info =
            YesNoVotePollStoredInfo::new(vote_poll.clone(), *block_info, platform_version)?;
        let mut stored_info_operations =
            self.insert_stored_info_for_yes_no_vote_poll_operations(stored_info, platform_version)?;
        batch_operations.append(&mut stored_info_operations);

        // One sum tree per choice; the sums are the tallies.
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
            estimated_costs_only_with_layer_info,
            previous_batch_operations,
            batch_operations,
            transaction,
            platform_version,
        )?;
        Ok(())
    }
}
