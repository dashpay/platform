use crate::drive::constants::AVERAGE_IDENTITY_CONTENDER_VOTE_POLL_STORED_INFO_SIZE;
use crate::drive::votes::insert::identity_contender::add_estimation_costs_for_identity_contender_polls_tree_levels;
use crate::drive::votes::paths::{
    vote_identity_contender_active_polls_tree_path_vec,
    vote_identity_contender_poll_choice_tree_path_vec,
    vote_identity_contender_poll_choice_votes_path_vec, vote_identity_contender_poll_tree_path_vec,
    vote_identity_contender_polls_tree_path_vec, vote_root_path_vec, ACTIVE_POLLS_TREE_KEY,
    IDENTITY_CONTENDER_POLLS_TREE_KEY, IDENTITY_VOTES_TREE_KEY,
    RESOURCE_ABSTAIN_VOTE_TREE_KEY_U8_32, RESOURCE_STORED_INFO_KEY_U8_32, VOTING_STORAGE_TREE_KEY,
};
use crate::drive::Drive;
use crate::error::Error;
use crate::fees::op::{LowLevelDriveOperation, LowLevelDriveOperationTreeTypeConverter};
use crate::util::grove_operations::{BatchInsertTreeApplyType, DirectQueryType};
use crate::util::object_size_info::PathKeyElementInfo::PathKeyElement;
use crate::util::object_size_info::PathKeyInfo;
use crate::util::type_constants::{DEFAULT_HASH_SIZE_U8, U8_SIZE_U32, U8_SIZE_U8};
use dpp::block::block_info::BlockInfo;
use dpp::fee::fee_result::FeeResult;
use dpp::identity::TimestampMillis;
use dpp::serialization::PlatformSerializable;
use dpp::version::PlatformVersion;
use dpp::voting::vote_choices::resource_vote_choice::ResourceVoteChoice;
use dpp::voting::vote_info_storage::identity_contender_vote_poll_stored_info::IdentityContenderVotePollStoredInfo;
use dpp::voting::vote_polls::identity_contender_vote_poll::IdentityContenderVotePoll;
use dpp::voting::vote_polls::VotePoll;
use dpp::ProtocolError;
use grovedb::batch::KeyInfoPath;
use grovedb::EstimatedLayerCount::{ApproximateElements, PotentiallyAtMaxElements};
use grovedb::EstimatedLayerSizes::{AllItems, AllSubtrees, Mix};
use grovedb::EstimatedSumTrees::{AllSumTrees, NoSumTrees};
use grovedb::{Element, EstimatedLayerInformation, TransactionArg, TreeType};
use std::collections::HashMap;

impl Drive {
    pub(super) fn open_identity_contender_vote_poll_v0(
        &self,
        vote_poll: &IdentityContenderVotePoll,
        join_end_time_ms: TimestampMillis,
        vote_end_time_ms: TimestampMillis,
        block_info: &BlockInfo,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<FeeResult, Error> {
        let mut batch_operations = vec![];
        self.open_identity_contender_vote_poll_operations_v0(
            vote_poll,
            join_end_time_ms,
            vote_end_time_ms,
            block_info,
            &mut None,
            &mut None,
            &mut batch_operations,
            transaction,
            platform_version,
        )?;
        let mut drive_operations: Vec<LowLevelDriveOperation> = vec![];
        self.apply_batch_low_level_drive_operations(
            None,
            transaction,
            batch_operations,
            &mut drive_operations,
            &platform_version.drive,
        )?;
        Drive::calculate_fee(
            None,
            Some(drive_operations),
            &block_info.epoch,
            self.config.epochs_per_era,
            platform_version,
            None,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn open_identity_contender_vote_poll_operations_v0(
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
        // Strictly before: the vote phase's end date entry is keyed like the join phase's, so
        // the same time would make the clean-up of the join phase remove the vote phase's entry
        if join_end_time_ms >= vote_end_time_ms {
            return Err(Error::Protocol(Box::new(ProtocolError::VoteError(format!(
                "an identity contender vote poll's join phase must end strictly before its vote phase, got join end {} and vote end {}",
                join_end_time_ms, vote_end_time_ms
            )))));
        }
        let vote_poll_id = vote_poll.unique_id()?;
        let poll_path = vote_identity_contender_poll_tree_path_vec(vote_poll_id.as_slice());
        let abstain_path = vote_identity_contender_poll_choice_tree_path_vec(
            vote_poll_id.as_slice(),
            &ResourceVoteChoice::Abstain,
        );
        let abstain_votes_path = vote_identity_contender_poll_choice_votes_path_vec(
            vote_poll_id.as_slice(),
            &ResourceVoteChoice::Abstain,
        );
        let drive_version = &platform_version.drive;

        if let Some(estimated_costs_only_with_layer_info) = estimated_costs_only_with_layer_info {
            add_estimation_costs_for_identity_contender_polls_tree_levels(
                estimated_costs_only_with_layer_info,
            );
            // The poll: its stored info item next to the abstain tree and the contender trees
            estimated_costs_only_with_layer_info.insert(
                KeyInfoPath::from_known_owned_path(poll_path.clone()),
                EstimatedLayerInformation {
                    tree_type: TreeType::NormalTree,
                    estimated_layer_count: ApproximateElements(4),
                    estimated_layer_sizes: Mix {
                        subtrees_size: Some((DEFAULT_HASH_SIZE_U8, NoSumTrees, None, 3)),
                        items_size: Some((
                            DEFAULT_HASH_SIZE_U8,
                            AVERAGE_IDENTITY_CONTENDER_VOTE_POLL_STORED_INFO_SIZE,
                            None,
                            1,
                        )),
                        references_size: None,
                        items_with_sum_item_size: None,
                        references_with_sum_item_size: None,
                    },
                },
            );
            estimated_costs_only_with_layer_info.insert(
                KeyInfoPath::from_known_owned_path(abstain_path.clone()),
                EstimatedLayerInformation {
                    tree_type: TreeType::NormalTree,
                    estimated_layer_count: ApproximateElements(1),
                    estimated_layer_sizes: AllSubtrees(U8_SIZE_U8, AllSumTrees, None),
                },
            );
            estimated_costs_only_with_layer_info.insert(
                KeyInfoPath::from_known_owned_path(abstain_votes_path),
                EstimatedLayerInformation {
                    tree_type: TreeType::SumTree,
                    estimated_layer_count: PotentiallyAtMaxElements,
                    estimated_layer_sizes: AllItems(DEFAULT_HASH_SIZE_U8, U8_SIZE_U32, None),
                },
            );
        }

        let tree_apply_type = if estimated_costs_only_with_layer_info.is_none() {
            BatchInsertTreeApplyType::StatefulBatchInsertTree
        } else {
            BatchInsertTreeApplyType::StatelessBatchInsertTree {
                in_tree_type: TreeType::NormalTree,
                tree_type: TreeType::NormalTree,
                flags_len: 0,
            }
        };

        // The branch is created by the first poll rather than at genesis, so chains that
        // upgrade need no migration.
        for (path, key) in [
            (
                vote_root_path_vec(),
                vec![IDENTITY_CONTENDER_POLLS_TREE_KEY as u8],
            ),
            (
                vote_identity_contender_polls_tree_path_vec(),
                vec![ACTIVE_POLLS_TREE_KEY as u8],
            ),
            (
                vote_identity_contender_polls_tree_path_vec(),
                vec![IDENTITY_VOTES_TREE_KEY as u8],
            ),
        ] {
            self.batch_insert_empty_tree_if_not_exists(
                PathKeyInfo::PathKey::<0>((path, key)),
                TreeType::NormalTree,
                None,
                tree_apply_type,
                transaction,
                previous_batch_operations,
                batch_operations,
                drive_version,
            )?;
        }

        // A poll over a resource path that already has a state, running or resolved, is not
        // reopened: the opener tells its polls apart through the resource path.
        let inserted_poll_tree = self.batch_insert_empty_tree_if_not_exists(
            PathKeyInfo::PathKey::<0>((
                vote_identity_contender_active_polls_tree_path_vec(),
                vote_poll_id.to_vec(),
            )),
            TreeType::NormalTree,
            None,
            tree_apply_type,
            transaction,
            previous_batch_operations,
            batch_operations,
            drive_version,
        )?;
        if !inserted_poll_tree && estimated_costs_only_with_layer_info.is_none() {
            let existing_stored_info = self.grove_get_raw_optional(
                poll_path.as_slice().into(),
                RESOURCE_STORED_INFO_KEY_U8_32.as_slice(),
                DirectQueryType::StatefulDirectQuery,
                transaction,
                &mut vec![],
                drive_version,
            )?;
            if existing_stored_info.is_some() {
                return Err(Error::Protocol(Box::new(ProtocolError::VoteError(format!(
                    "identity contender vote poll {} already has a state and can not be opened again",
                    vote_poll
                )))));
            }
        }

        let stored_info = IdentityContenderVotePollStoredInfo::new(
            *block_info,
            join_end_time_ms,
            vote_end_time_ms,
            platform_version,
        )?;
        self.batch_insert::<0>(
            PathKeyElement((
                poll_path.clone(),
                RESOURCE_STORED_INFO_KEY_U8_32.to_vec(),
                Element::new_item(stored_info.serialize_consume_to_bytes()?),
            )),
            batch_operations,
            drive_version,
        )?;

        // The abstain choice and its votes
        self.batch_insert_empty_tree_if_not_exists(
            PathKeyInfo::PathKey::<0>((
                poll_path.clone(),
                RESOURCE_ABSTAIN_VOTE_TREE_KEY_U8_32.to_vec(),
            )),
            TreeType::NormalTree,
            None,
            tree_apply_type,
            transaction,
            previous_batch_operations,
            batch_operations,
            drive_version,
        )?;
        batch_operations.push(TreeType::SumTree.empty_tree_operation_for_known_path_key(
            abstain_path,
            vec![VOTING_STORAGE_TREE_KEY],
            None,
        )?);

        // The poll ends its join phase at the join end time
        self.add_vote_poll_end_date_query_operations(
            None,
            VotePoll::IdentityContenderVotePoll(vote_poll.clone()),
            join_end_time_ms,
            block_info,
            estimated_costs_only_with_layer_info,
            previous_batch_operations,
            batch_operations,
            transaction,
            platform_version,
        )
    }
}

#[cfg(test)]
mod tests {
    use crate::drive::Drive;
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use dpp::block::block_info::BlockInfo;
    use dpp::voting::vote_polls::identity_contender_vote_poll::IdentityContenderVotePoll;
    use platform_version::version::PlatformVersion;
    use std::collections::HashMap;

    fn poll() -> IdentityContenderVotePoll {
        IdentityContenderVotePoll::new(vec![vec![7; 32], b"election".to_vec()])
    }

    #[test]
    fn should_refuse_a_join_phase_that_does_not_end_strictly_before_the_vote_phase() {
        let platform_version = PlatformVersion::latest();
        let drive = setup_drive_with_initial_state_structure(Some(platform_version));
        for (join_end, vote_end) in [(1_000, 1_000), (1_000, 999)] {
            let error = drive
                .open_identity_contender_vote_poll(
                    &poll(),
                    join_end,
                    vote_end,
                    &BlockInfo::default(),
                    None,
                    platform_version,
                )
                .expect_err("expected the poll to be refused");
            assert!(
                error.to_string().contains("strictly before"),
                "unexpected error {error}"
            );
        }
        drive
            .open_identity_contender_vote_poll(
                &poll(),
                1_000,
                1_001,
                &BlockInfo::default(),
                None,
                platform_version,
            )
            .expect("expected the poll to open");
    }

    /// Opening is what a document create transition does for its applicant, whose fee is
    /// estimated before anything is written: the estimate must come out without state.
    #[test]
    fn should_estimate_the_costs_of_opening_a_poll() {
        let platform_version = PlatformVersion::latest();
        let drive = setup_drive_with_initial_state_structure(Some(platform_version));
        let mut estimated_costs_only_with_layer_info = Some(HashMap::new());
        let mut batch_operations = vec![];
        drive
            .open_identity_contender_vote_poll_operations(
                &poll(),
                1_000,
                2_000,
                &BlockInfo::default(),
                &mut estimated_costs_only_with_layer_info,
                &mut None,
                &mut batch_operations,
                None,
                platform_version,
            )
            .expect("expected the operations");
        assert!(!batch_operations.is_empty());
        let mut drive_operations = vec![];
        drive
            .apply_batch_low_level_drive_operations(
                estimated_costs_only_with_layer_info,
                None,
                batch_operations,
                &mut drive_operations,
                &platform_version.drive,
            )
            .expect("expected the estimated operations to apply");
        let fee = Drive::calculate_fee(
            None,
            Some(drive_operations),
            &Default::default(),
            drive.config.epochs_per_era,
            platform_version,
            None,
        )
        .expect("expected a fee");
        assert!(fee.storage_fee > 0);
        assert!(fee.processing_fee > 0);
    }
}
