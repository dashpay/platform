use crate::drive::votes::paths::{
    vote_contested_resource_end_date_queries_at_time_tree_path_vec,
    vote_end_date_queries_tree_path_vec,
};
use crate::drive::Drive;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::common::encode::encode_u64;
use crate::util::grove_operations::pending_grove_operations::pending_grove_operations;
use crate::util::grove_operations::BatchInsertApplyType;
use crate::util::object_size_info::PathKeyElementInfo::PathKeyRefElement;
use crate::util::storage_flags::StorageFlags;
use dpp::block::block_info::BlockInfo;
use dpp::identity::TimestampMillis;
use dpp::serialization::PlatformSerializable;
use dpp::voting::vote_polls::VotePoll;
use grovedb::batch::key_info::KeyInfo;
use grovedb::batch::{GroveOp, KeyInfoPath};
use grovedb::{Element, EstimatedLayerInformation, TransactionArg};
use platform_version::version::PlatformVersion;
use std::collections::HashMap;

impl Drive {
    /// Version 1 (protocol version 15): when the previous operations already queue an insert of
    /// the end date's tree, whatever its storage flags, only the poll's entry is added and the
    /// tree keeps the flags it was queued with. Otherwise it is version 0, whose tree insert skips
    /// only an identical insert (flags included) and cancels a queued deletion of the tree. A
    /// contest and a yes/no poll opening that share an end date in one batch queue the tree with
    /// different flags, so version 0 queued it twice when the contest came second.
    #[allow(clippy::too_many_arguments)]
    pub(in crate::drive::votes::insert) fn add_vote_poll_end_date_query_operations_v1(
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
        let end_date_tree_insert_queued = estimated_costs_only_with_layer_info.is_none()
            && previous_batch_operations
                .as_deref()
                .is_some_and(|previous| end_date_tree_insert_queued(previous, end_date));
        if !end_date_tree_insert_queued {
            return self.add_vote_poll_end_date_query_operations_v0(
                creator_identity_id,
                vote_poll,
                end_date,
                block_info,
                estimated_costs_only_with_layer_info,
                previous_batch_operations,
                batch_operations,
                transaction,
                platform_version,
            );
        }

        // The entry version 0 writes after the tree.
        let storage_flags = creator_identity_id.map(|creator_identity_id| {
            StorageFlags::new_single_epoch(block_info.epoch.index, Some(creator_identity_id))
        });
        let unique_id = vote_poll.unique_id(platform_version)?;
        let item = Element::Item(
            vote_poll.serialize_to_bytes()?,
            StorageFlags::map_to_some_element_flags(storage_flags.as_ref()),
        );
        self.batch_insert_if_not_exists(
            PathKeyRefElement::<0>((
                vote_contested_resource_end_date_queries_at_time_tree_path_vec(end_date),
                unique_id.as_bytes(),
                item,
            )),
            BatchInsertApplyType::StatefulBatchInsert,
            transaction,
            batch_operations,
            &platform_version.drive,
        )?;
        Ok(())
    }
}

/// Whether `previous_batch_operations` queue an insert of the tree of `end_date` in the end date
/// index.
fn end_date_tree_insert_queued(
    previous_batch_operations: &[LowLevelDriveOperation],
    end_date: TimestampMillis,
) -> bool {
    let path = KeyInfoPath::from_known_owned_path(vote_end_date_queries_tree_path_vec());
    let key = KeyInfo::KnownKey(encode_u64(end_date));
    pending_grove_operations(previous_batch_operations).any(|operation| {
        operation.path == path
            && operation.key.as_ref() == Some(&key)
            && matches!(
                &operation.op,
                GroveOp::InsertOrReplace { element }
                    | GroveOp::InsertOrReplaceDontCheckForBackwardsReferences { element }
                    | GroveOp::InsertWithKnownToNotAlreadyExist { element }
                    | GroveOp::InsertIfNotExists { element, .. }
                    if element.is_any_tree()
            )
    })
}
