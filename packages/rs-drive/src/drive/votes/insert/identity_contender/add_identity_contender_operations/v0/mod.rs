use crate::drive::constants::AVERAGE_IDENTITY_CONTENDER_INFO_SIZE;
use crate::drive::votes::paths::{IDENTITY_CONTENDER_INFO_KEY, VOTING_STORAGE_TREE_KEY};
use crate::drive::votes::resolved::vote_polls::identity_contender_vote_poll::IdentityContenderVotePollPaths;
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
use grovedb::EstimatedSumTrees::AllSumTrees;
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
        let choice = ResourceVoteChoice::TowardsIdentity(identity_id);
        let contender_path = vote_poll.choice_path_vec(&choice)?;
        let contender_votes_path = vote_poll.choice_votes_path_vec(&choice)?;

        if let Some(estimated_costs_only_with_layer_info) = estimated_costs_only_with_layer_info {
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

        let (poll_path, contender_key) = {
            let mut path = contender_path.clone();
            let key = path.pop().ok_or(Error::Protocol(Box::new(
                ProtocolError::CorruptedCodeExecution("a choice path has a key".to_string()),
            )))?;
            (path, key)
        };
        let inserted = self.batch_insert_empty_tree_if_not_exists(
            PathKeyInfo::PathKey::<0>((poll_path, contender_key)),
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
