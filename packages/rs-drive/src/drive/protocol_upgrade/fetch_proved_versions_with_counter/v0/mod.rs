use crate::drive::protocol_upgrade::versions_counter_path_vec;
use crate::drive::Drive;
use crate::error::Error;

use crate::query::QueryItem;
use dpp::version::drive_versions::DriveVersion;
use grovedb::{PathQuery, Query, TransactionArg};
use std::ops::RangeFull;

impl Drive {
    /// Fetch versions by count for the upgrade window and return them as a proved item
    pub(super) fn fetch_proved_versions_with_counter_v0(
        &self,
        transaction: TransactionArg,
        drive_version: &DriveVersion,
    ) -> Result<Vec<u8>, Error> {
        let path_query = PathQuery::new_unsized(
            versions_counter_path_vec(),
            Query::new_single_query_item(QueryItem::RangeFull(RangeFull)),
        );
        self.grove_get_proved_path_query(&path_query, transaction, &mut vec![], drive_version)
    }
}
