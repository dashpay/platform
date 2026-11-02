use crate::drive::Drive;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::identifier::Identifier;
use dpp::identity::TimestampMillis;
use grovedb::TransactionArg;
use platform_version::version::PlatformVersion;
use std::collections::{BTreeMap, BTreeSet};

impl Drive {
    /// Removes the given vote polls, of any kind, from the end date index with the rule of
    /// `remove_contested_resource_vote_poll_end_date_query_operations` v2, whose body it reuses:
    /// an end date goes once none of its vote polls remain.
    pub(super) fn remove_vote_poll_end_date_query_operations_v0(
        &self,
        vote_polls: &[(Identifier, TimestampMillis)],
        batch_operations: &mut Vec<LowLevelDriveOperation>,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        let mut by_end_date: BTreeMap<TimestampMillis, BTreeSet<Identifier>> = BTreeMap::new();
        for (unique_id, end_date) in vote_polls {
            by_end_date.entry(*end_date).or_default().insert(*unique_id);
        }

        self.remove_vote_poll_ids_from_end_dates_operations_v2(
            by_end_date,
            batch_operations,
            transaction,
            platform_version,
        )
    }
}
