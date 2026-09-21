use crate::drive::constants::{
    AVERAGE_IDENTITY_CONTENDER_INFO_SIZE, AVERAGE_IDENTITY_CONTENDER_VOTE_POLL_STORED_INFO_SIZE,
};
use crate::drive::votes::insert::identity_contender::add_estimation_costs_for_identity_contender_polls_tree_levels;
use crate::drive::votes::paths::{
    vote_identity_contender_poll_choice_tree_path_vec,
    vote_identity_contender_poll_choice_votes_path_vec, vote_identity_contender_poll_tree_path_vec,
    IDENTITY_CONTENDER_INFO_KEY, VOTING_STORAGE_TREE_KEY,
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
use dpp::identifier::Identifier;
use dpp::serialization::PlatformSerializable;
use dpp::version::PlatformVersion;
use dpp::voting::contender_structs::IdentityContenderInfo;
use dpp::voting::vote_choices::resource_vote_choice::ResourceVoteChoice;
use dpp::voting::vote_polls::identity_contender_vote_poll::IdentityContenderVotePoll;
use dpp::ProtocolError;
use grovedb::batch::KeyInfoPath;
use grovedb::EstimatedLayerCount::{ApproximateElements, PotentiallyAtMaxElements};
use grovedb::EstimatedLayerSizes::{AllItems, Mix};
use grovedb::EstimatedSumTrees::{AllSumTrees, NoSumTrees};
use grovedb::{Element, EstimatedLayerInformation, TransactionArg, TreeType};
use std::collections::HashMap;

impl Drive {
    pub(super) fn add_identity_contender_v0(
        &self,
        vote_poll: &IdentityContenderVotePoll,
        identity_id: Identifier,
        contender_info: IdentityContenderInfo,
        block_info: &BlockInfo,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<FeeResult, Error> {
        let mut batch_operations = vec![];
        self.add_identity_contender_operations_v0(
            vote_poll,
            identity_id,
            contender_info,
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
    pub(super) fn add_identity_contender_operations_v0(
        &self,
        vote_poll: &IdentityContenderVotePoll,
        identity_id: Identifier,
        contender_info: IdentityContenderInfo,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        previous_batch_operations: &mut Option<&mut Vec<LowLevelDriveOperation>>,
        batch_operations: &mut Vec<LowLevelDriveOperation>,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        let drive_version = &platform_version.drive;
        let vote_poll_id = vote_poll.unique_id()?;
        let choice = ResourceVoteChoice::TowardsIdentity(identity_id);
        let poll_path = vote_identity_contender_poll_tree_path_vec(vote_poll_id.as_slice());
        let contender_path =
            vote_identity_contender_poll_choice_tree_path_vec(vote_poll_id.as_slice(), &choice);
        let contender_votes_path =
            vote_identity_contender_poll_choice_votes_path_vec(vote_poll_id.as_slice(), &choice);

        if let Some(estimated_costs_only_with_layer_info) = estimated_costs_only_with_layer_info {
            add_estimation_costs_for_identity_contender_polls_tree_levels(
                estimated_costs_only_with_layer_info,
            );
            // The poll's tree, which the contender's key goes into: the same estimate as at the
            // poll's opening, since a contender may join on its own long after
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
                KeyInfoPath::from_known_owned_path(contender_path.clone()),
                EstimatedLayerInformation {
                    tree_type: TreeType::NormalTree,
                    estimated_layer_count: ApproximateElements(2),
                    estimated_layer_sizes: Mix {
                        subtrees_size: Some((U8_SIZE_U8, AllSumTrees, None, 1)),
                        items_size: Some((
                            U8_SIZE_U8,
                            AVERAGE_IDENTITY_CONTENDER_INFO_SIZE,
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
                KeyInfoPath::from_known_owned_path(contender_votes_path.clone()),
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

        let inserted = self.batch_insert_empty_tree_if_not_exists(
            PathKeyInfo::PathKey::<0>((poll_path, identity_id.to_vec())),
            TreeType::NormalTree,
            None,
            tree_apply_type,
            transaction,
            previous_batch_operations,
            batch_operations,
            drive_version,
        )?;
        if !inserted && estimated_costs_only_with_layer_info.is_none() {
            let existing = self.grove_get_raw_optional(
                contender_path.as_slice().into(),
                &[IDENTITY_CONTENDER_INFO_KEY],
                DirectQueryType::StatefulDirectQuery,
                transaction,
                &mut vec![],
                drive_version,
            )?;
            if existing.is_some() {
                return Err(Error::Protocol(Box::new(ProtocolError::VoteError(
                    format!(
                        "identity {} is already a contender of identity contender vote poll {}",
                        identity_id, vote_poll
                    ),
                ))));
            }
        }

        self.batch_insert::<0>(
            PathKeyElement((
                contender_path.clone(),
                vec![IDENTITY_CONTENDER_INFO_KEY],
                Element::new_item(contender_info.serialize_consume_to_bytes()?),
            )),
            batch_operations,
            drive_version,
        )?;
        batch_operations.push(TreeType::SumTree.empty_tree_operation_for_known_path_key(
            contender_path,
            vec![VOTING_STORAGE_TREE_KEY],
            None,
        )?);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::drive::Drive;
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use dpp::block::block_info::BlockInfo;
    use dpp::identifier::Identifier;
    use dpp::voting::contender_structs::IdentityContenderInfo;
    use dpp::voting::vote_polls::identity_contender_vote_poll::IdentityContenderVotePoll;
    use platform_version::version::PlatformVersion;
    use std::collections::HashMap;

    /// A later applicant joins a poll that opened blocks ago: its fee is estimated from the
    /// contender's operations alone, so they must carry every layer they touch.
    #[test]
    fn should_estimate_the_costs_of_adding_a_contender_on_its_own() {
        let platform_version = PlatformVersion::latest();
        let drive = setup_drive_with_initial_state_structure(Some(platform_version));
        let vote_poll = IdentityContenderVotePoll::new(vec![vec![7; 32], b"election".to_vec()]);
        drive
            .open_identity_contender_vote_poll(
                &vote_poll,
                1_000,
                2_000,
                &BlockInfo::default(),
                None,
                platform_version,
            )
            .expect("expected the poll to open");
        let mut estimated_costs_only_with_layer_info = Some(HashMap::new());
        let mut batch_operations = vec![];
        drive
            .add_identity_contender_operations(
                &vote_poll,
                Identifier::new([0xa1; 32]),
                IdentityContenderInfo::new(
                    BlockInfo::default(),
                    Identifier::new([1; 32]),
                    platform_version,
                )
                .expect("expected the contender info"),
                &mut estimated_costs_only_with_layer_info,
                &mut None,
                &mut batch_operations,
                None,
                platform_version,
            )
            .expect("expected the operations");
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
    }
}
