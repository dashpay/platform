use crate::drive::constants::AVERAGE_CONTESTED_RESOURCE_ITEM_REFERENCE_SIZE;
use crate::drive::votes::paths::{
    vote_contested_resource_end_date_queries_at_time_tree_path_vec,
    vote_end_date_queries_tree_path, vote_end_date_queries_tree_path_vec,
};
use crate::drive::Drive;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::common::encode::encode_u64;
use crate::util::grove_operations::QueryTarget::QueryTargetValue;
use crate::util::grove_operations::{BatchInsertApplyType, BatchInsertTreeApplyType};
use crate::util::object_size_info::PathKeyElementInfo::{PathKeyElementSize, PathKeyRefElement};
use crate::util::object_size_info::{DriveKeyInfo, PathInfo, PathKeyElementInfo};
use crate::util::storage_flags::StorageFlags;
use crate::util::type_constants::{DEFAULT_HASH_SIZE_U8, U64_SIZE_U8};
use dpp::block::block_info::BlockInfo;
use dpp::identity::TimestampMillis;
use dpp::serialization::PlatformSerializable;
use dpp::voting::vote_polls::VotePoll;
use grovedb::batch::key_info::KeyInfo;
use grovedb::batch::KeyInfoPath;
use grovedb::EstimatedLayerCount::ApproximateElements;
use grovedb::EstimatedLayerSizes::{AllItems, AllSubtrees};
use grovedb::EstimatedSumTrees::NoSumTrees;
use grovedb::{Element, EstimatedLayerInformation, TransactionArg};
use platform_version::version::PlatformVersion;
use std::collections::HashMap;

impl Drive {
    /// We add votes poll references by end date in order to be able to check on every new block if
    /// any vote polls should be closed.
    pub(in crate::drive::votes::insert) fn add_vote_poll_end_date_query_operations_v0(
        &self,
        creator_identity_id: Option<[u8; 32]>,
        vote_poll: VotePoll,
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
        let storage_flags = creator_identity_id.map(|creator_identity_id| {
            StorageFlags::new_single_epoch(block_info.epoch.index, Some(creator_identity_id))
        });

        if let Some(estimated_costs_only_with_layer_info) = estimated_costs_only_with_layer_info {
            estimated_costs_only_with_layer_info.insert(
                KeyInfoPath::from_known_path(vote_end_date_queries_tree_path()),
                EstimatedLayerInformation {
                    is_sum_tree: false,
                    // We can estimate that there is at least a vote concluding every block, and we put blocks at 6 seconds.
                    estimated_layer_count: ApproximateElements(201_600),
                    estimated_layer_sizes: AllSubtrees(
                        U64_SIZE_U8,
                        NoSumTrees,
                        Some(StorageFlags::approximate_size(true, None)),
                    ),
                },
            );

            estimated_costs_only_with_layer_info.insert(
                KeyInfoPath::from_known_owned_path(
                    vote_contested_resource_end_date_queries_at_time_tree_path_vec(end_date),
                ),
                EstimatedLayerInformation {
                    is_sum_tree: false,
                    // We can estimate that there is 2 votes ending per block.
                    estimated_layer_count: ApproximateElements(2),
                    estimated_layer_sizes: AllItems(
                        DEFAULT_HASH_SIZE_U8,
                        AVERAGE_CONTESTED_RESOURCE_ITEM_REFERENCE_SIZE,
                        Some(StorageFlags::approximate_size(true, None)),
                    ),
                },
            );
        }

        // This is a GroveDB Tree (Not Sub Tree Merk representation)
        //                         End Date queries
        //              /                                  \
        //       15/08/2025 5PM                                   15/08/2025 6PM
        //          /              \                                    |
        //     VotePoll Info 1   VotePoll Info 2                 VotePoll Info 3

        // Let's start by inserting a tree for the end date

        let end_date_query_path = vote_end_date_queries_tree_path_vec();

        let drive_key = DriveKeyInfo::Key(encode_u64(end_date));

        // println!("adding end vote date at end_date {} ({})", end_date, if estimated_costs_only_with_layer_info.is_some() { "costs"} else {"apply"});

        let path_key_info = drive_key.add_path_info::<0>(PathInfo::PathAsVec(end_date_query_path));

        let apply_type = if estimated_costs_only_with_layer_info.is_none() {
            BatchInsertTreeApplyType::StatefulBatchInsertTree
        } else {
            BatchInsertTreeApplyType::StatelessBatchInsertTree {
                in_tree_using_sums: false,
                is_sum_tree: false,
                flags_len: storage_flags
                    .as_ref()
                    .map(|s| s.serialized_size())
                    .unwrap_or_default(),
            }
        };

        // We check existing operations just because it is possible that we have already inserted the same
        // end data in the documents batch transition
        self.batch_insert_empty_tree_if_not_exists(
            path_key_info.clone(),
            false,
            storage_flags.as_ref(),
            apply_type,
            transaction,
            previous_batch_operations,
            batch_operations,
            &platform_version.drive,
        )?;

        let time_path = vote_contested_resource_end_date_queries_at_time_tree_path_vec(end_date);

        let item = Element::Item(
            vote_poll.serialize_to_bytes()?,
            StorageFlags::map_to_some_element_flags(storage_flags.as_ref()),
        );

        let apply_type = if estimated_costs_only_with_layer_info.is_none() {
            BatchInsertApplyType::StatefulBatchInsert
        } else {
            BatchInsertApplyType::StatelessBatchInsert {
                in_tree_using_sums: false,
                // todo: figure out a default serialized size to make this faster
                target: QueryTargetValue(
                    item.serialized_size(&platform_version.drive.grove_version)? as u32,
                ),
            }
        };

        let unique_id = vote_poll.unique_id()?;

        let path_key_element_info: PathKeyElementInfo<'_, 0> =
            if estimated_costs_only_with_layer_info.is_none() {
                PathKeyRefElement((time_path, unique_id.as_bytes(), item))
            } else {
                PathKeyElementSize((
                    KeyInfoPath::from_known_owned_path(time_path),
                    KeyInfo::KnownKey(unique_id.to_vec()),
                    item,
                ))
            };

        self.batch_insert_if_not_exists(
            path_key_element_info,
            apply_type,
            transaction,
            batch_operations,
            &platform_version.drive,
        )?;

        Ok(())
    }
}
