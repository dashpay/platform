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
use std::collections::{BTreeMap, BTreeSet};

impl Drive {
    /// The kind-agnostic counterpart of `remove_contested_resource_vote_poll_end_date_query_operations`
    /// v2, with the same rule for an end date: it goes once none of its vote polls remain.
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

        let mut by_end_date: BTreeMap<TimestampMillis, BTreeSet<Identifier>> = BTreeMap::new();
        for (unique_id, end_date) in vote_polls {
            by_end_date.entry(*end_date).or_default().insert(*unique_id);
        }

        for (end_date, unique_ids) in by_end_date {
            let time_path =
                vote_contested_resource_end_date_queries_at_time_tree_path_vec(end_date);
            for unique_id in &unique_ids {
                self.batch_delete(
                    time_path.as_slice().into(),
                    unique_id.as_bytes(),
                    delete_apply_type,
                    transaction,
                    batch_operations,
                    &platform_version.drive,
                )?;
            }

            // The block's fetch shares one limit across every due end date, so the last end date
            // it reached can be cut short, and an end date can also hold a poll of the other
            // kind. Read one entry more than this batch removes: the end date is left empty only
            // when every entry read is removed here, and then the read found all of them. A
            // count too large for a query limit reads without one. An end date that lists
            // nothing (it has no tree) is not removed.
            let limit = u16::try_from(unique_ids.len().saturating_add(1)).ok();
            let entries = VotePollsByEndDateDriveQuery::execute_no_proof_keys_for_single_end_time(
                end_date,
                limit,
                self,
                transaction,
                &mut vec![],
                platform_version,
            )?;
            let none_remain = !entries.is_empty()
                && entries.iter().all(|key| {
                    Identifier::from_bytes(key).is_ok_and(|entry_id| unique_ids.contains(&entry_id))
                });

            if none_remain {
                // The end date holds nothing once this batch applies, so it goes in the same
                // batch. The `NotTree` hint is deliberate although the end date is a tree: the
                // read above is the emptiness check.
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
