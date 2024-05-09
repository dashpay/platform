use std::collections::HashMap;
use grovedb::batch::KeyInfoPath;
use crate::drive::Drive;
use crate::error::Error;
use dpp::identity::TimestampMillis;
use grovedb::{Element, EstimatedLayerInformation, TransactionArg};
use grovedb::batch::key_info::KeyInfo;
use dpp::block::block_info::BlockInfo;
use dpp::identifier::Identifier;
use dpp::serialization::PlatformSerializable;
use dpp::voting::vote_polls::VotePoll;
use platform_version::version::PlatformVersion;
use crate::drive::flags::StorageFlags;
use crate::drive::grove_operations::BatchInsertTreeApplyType;
use crate::drive::object_size_info::{DriveKeyInfo, PathInfo, PathKeyElementInfo};
use crate::drive::object_size_info::PathKeyElementInfo::{PathKeyElementSize, PathKeyRefElement};
use crate::drive::votes::paths::{vote_contested_resource_end_date_queries_at_time_tree_path_vec, vote_contested_resource_end_date_queries_tree_path_vec};
use crate::fee::op::LowLevelDriveOperation;

impl Drive {
    /// We add votes poll references by end date in order to be able to check on every new block if
    /// any vote polls should be closed.
    pub(in crate::drive::votes::insert) fn add_vote_poll_end_date_query_operations_v0(
        &self,
        creator_identity_id: Option<Identifier>,
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
            StorageFlags::new_single_epoch(
                block_info.epoch.index,
                Some(creator_identity_id.to_buffer()),
            )
        });
        
        // This is a GroveDB Tree (Not Sub Tree Merk representation)
        //                         End Date queries
        //              /                                  \
        //       15/08/2025 5PM                                   15/08/2025 6PM
        //          /              \                                    |
        //     VotePoll Info 1   VotePoll Info 2                 VotePoll Info 3


        // Let's start by inserting a tree for the end date
        
        let end_date_query_path = vote_contested_resource_end_date_queries_tree_path_vec();
        
        let drive_key = DriveKeyInfo::Key(end_date.to_be_bytes().to_vec());
        
        let path_key_info = drive_key.add_path_info::<0>(PathInfo::PathAsVec(end_date_query_path));

        let apply_type = if estimated_costs_only_with_layer_info.is_none() {
            BatchInsertTreeApplyType::StatefulBatchInsertTree
        } else {
            BatchInsertTreeApplyType::StatelessBatchInsertTree {
                in_tree_using_sums: false,
                is_sum_tree: false,
                flags_len: storage_flags.as_ref()
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
        
        let item = Element::Item(vote_poll.serialize_to_bytes()?, StorageFlags::map_to_some_element_flags(storage_flags.as_ref()));

        let path_key_element_info : PathKeyElementInfo<'_, 0> = if estimated_costs_only_with_layer_info.is_none() {
            PathKeyRefElement((
                time_path,
                &[0],
                item,
            ))
        } else {
            PathKeyElementSize((
                KeyInfoPath::from_known_owned_path(time_path),
                KeyInfo::KnownKey(vec![0u8]),
                item,
            ))
        };
            
        self.batch_insert(path_key_element_info, batch_operations, &platform_version.drive)?;
        
        Ok(())
        
        //todo
        // if let Some(estimated_costs_only_with_layer_info) = estimated_costs_only_with_layer_info
        // {
        //     Self::add_estimation_costs_for_levels_up_to_contract_document_type_excluded(
        //         contract,
        //         estimated_costs_only_with_layer_info,
        //         drive_version,
        //     )?;
        // }

    }
}
