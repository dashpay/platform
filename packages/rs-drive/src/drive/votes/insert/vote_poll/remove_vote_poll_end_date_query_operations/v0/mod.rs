use crate::drive::votes::paths::{vote_contested_resource_end_date_queries_at_time_tree_path_vec, vote_contested_resource_end_date_queries_tree_path_vec};
use crate::drive::Drive;
use crate::error::Error;
use crate::fee::op::LowLevelDriveOperation;
use dpp::identity::TimestampMillis;
use dpp::serialization::PlatformSerializable;
use dpp::voting::vote_polls::VotePoll;
use grovedb::TransactionArg;
use platform_version::version::PlatformVersion;
use crate::common::encode::encode_u64;
use crate::drive::grove_operations::BatchDeleteApplyType::StatefulBatchDelete;

impl Drive {
    /// We add votes poll references by end date in order to be able to check on every new block if
    /// any vote polls should be closed.
    pub(in crate::drive::votes::insert) fn remove_vote_poll_end_date_query_operations_v0(
        &self,
        vote_polls: Vec<VotePoll>,
        end_date: TimestampMillis,
        batch_operations: &mut Vec<LowLevelDriveOperation>,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        // This is a GroveDB Tree (Not Sub Tree Merk representation)
        //                         End Date queries
        //              /                                  \
        //       15/08/2025 5PM                                   15/08/2025 6PM
        //          /              \                                    |
        //     VotePoll Info 1   VotePoll Info 2                 VotePoll Info 3

        // Let's start by inserting a tree for the end date


        let time_path = vote_contested_resource_end_date_queries_at_time_tree_path_vec(end_date);

        let delete_apply_type = StatefulBatchDelete {
            is_known_to_be_subtree_with_sum: Some((false, false)),
        };
        
        for vote_poll in vote_polls {
            let unique_id = vote_poll.unique_id()?;

            self.batch_delete(
                time_path.as_ref().into(),
                unique_id.as_bytes(),
                delete_apply_type,
                transaction,
                batch_operations,
                &platform_version.drive,
            )?;
        }



        let end_date_query_path = vote_contested_resource_end_date_queries_tree_path_vec();

        let end_date_key = encode_u64(end_date);

        let delete_apply_type = StatefulBatchDelete {
            is_known_to_be_subtree_with_sum: Some((true, false)),
        };

        self.batch_delete(
            end_date_query_path.into(),
            end_date_key.as_bytes(),
            delete_apply_type,
            transaction,
            batch_operations,
            &platform_version.drive,
        )?;

        Ok(())
    }
}
