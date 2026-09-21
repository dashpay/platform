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
use dpp::voting::vote_polls::identity_contender_vote_poll::IdentityContenderVotePoll;
use grovedb::{MaybeTree, TransactionArg};
use platform_version::version::PlatformVersion;
use std::collections::{BTreeMap, BTreeSet};

impl Drive {
    /// A time's tree goes with its last entries, but only when every entry it holds is one of
    /// these polls: the entries of contested resource polls ending at the same time are removed
    /// by their own clean-up, which then decides about the tree itself.
    pub(super) fn remove_identity_contender_vote_poll_end_date_query_operations_v0(
        &self,
        vote_polls: &[(&IdentityContenderVotePoll, TimestampMillis)],
        batch_operations: &mut Vec<LowLevelDriveOperation>,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        let delete_item = BatchDeleteApplyType::StatefulBatchDelete {
            is_known_to_be_subtree_with_sum: Some(MaybeTree::NotTree),
        };
        let mut by_end_date: BTreeMap<TimestampMillis, BTreeSet<Identifier>> = BTreeMap::new();
        for (vote_poll, end_date) in vote_polls {
            by_end_date
                .entry(*end_date)
                .or_default()
                .insert(vote_poll.unique_id()?);
        }
        for (end_date, unique_ids) in by_end_date {
            let time_path =
                vote_contested_resource_end_date_queries_at_time_tree_path_vec(end_date);
            for unique_id in &unique_ids {
                self.batch_delete(
                    time_path.as_slice().into(),
                    unique_id.as_bytes(),
                    delete_item,
                    transaction,
                    batch_operations,
                    &platform_version.drive,
                )?;
            }
            let limit = u16::try_from(unique_ids.len().saturating_add(1)).unwrap_or(u16::MAX);
            let entries_at_time =
                VotePollsByEndDateDriveQuery::execute_no_proof_for_specialized_end_time_query_only_check_end_time(
                    end_date,
                    limit,
                    self,
                    transaction,
                    &mut vec![],
                    platform_version,
                )?;
            let only_these_polls = entries_at_time.len() <= unique_ids.len()
                && entries_at_time
                    .iter()
                    .map(|vote_poll| vote_poll.unique_id())
                    .collect::<Result<Vec<_>, _>>()?
                    .iter()
                    .all(|unique_id| unique_ids.contains(unique_id));
            if only_these_polls {
                self.batch_delete(
                    vote_end_date_queries_tree_path_vec().as_slice().into(),
                    encode_u64(end_date).as_slice(),
                    delete_item,
                    transaction,
                    batch_operations,
                    &platform_version.drive,
                )?;
            }
        }
        Ok(())
    }
}
