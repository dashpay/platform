use crate::drive::votes::paths::{
    vote_contested_resource_end_date_queries_at_time_tree_path_vec,
    vote_end_date_queries_tree_path_vec,
};
use crate::drive::Drive;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::query::VotePollsByEndDateDriveQuery;
use crate::util::common::encode::encode_u64;
use crate::util::grove_operations::BatchDeleteApplyType;
use dpp::identifier::Identifier;
use dpp::identity::TimestampMillis;
use grovedb::{MaybeTree, TransactionArg};
use platform_version::version::PlatformVersion;
use std::collections::BTreeMap;

impl Drive {
    pub(super) fn remove_vote_poll_end_date_query_operations_v0(
        &self,
        vote_polls: &[(Identifier, TimestampMillis)],
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
        let delete_apply_type = BatchDeleteApplyType::StatefulBatchDelete {
            is_known_to_be_subtree_with_sum: Some(MaybeTree::NotTree),
        };

        let mut by_end_date: BTreeMap<TimestampMillis, Vec<Identifier>> = BTreeMap::new();
        for (unique_id, end_date) in vote_polls {
            by_end_date.entry(*end_date).or_default().push(*unique_id);
        }

        for (end_date, unique_ids) in by_end_date {
            let time_path =
                vote_contested_resource_end_date_queries_at_time_tree_path_vec(end_date);
            let count = unique_ids.len();
            for unique_id in unique_ids {
                self.batch_delete(
                    time_path.as_slice().into(),
                    unique_id.as_bytes(),
                    delete_apply_type,
                    transaction,
                    batch_operations,
                    &platform_version.drive,
                )?;
            }

            // The block closes at most `maximum_vote_polls_to_process` polls. Fewer than that
            // at one end date means every poll ending then was fetched, so the tree empties;
            // otherwise look whether more are queued behind the ones processed.
            let maximum_vote_polls_to_process = platform_version
                .drive_abci
                .validation_and_processing
                .event_constants
                .maximum_vote_polls_to_process;
            let should_delete_parent_time_tree = if count < maximum_vote_polls_to_process as usize {
                true
            } else {
                let total_count =
                        VotePollsByEndDateDriveQuery::execute_no_proof_for_specialized_end_time_query_only_check_end_time(
                            end_date,
                            maximum_vote_polls_to_process + 1,
                            self,
                            transaction,
                            &mut vec![],
                            platform_version,
                        )?
                        .len();
                total_count <= count
            };

            if should_delete_parent_time_tree {
                self.batch_delete(
                    vote_end_date_queries_tree_path_vec().as_slice().into(),
                    encode_u64(end_date).as_slice(),
                    delete_apply_type,
                    transaction,
                    batch_operations,
                    &platform_version.drive,
                )?;
            }
        }

        Ok(())
    }
}
