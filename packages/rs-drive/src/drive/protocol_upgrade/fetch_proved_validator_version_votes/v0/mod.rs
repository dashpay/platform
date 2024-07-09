use crate::drive::protocol_upgrade::desired_version_for_validators_path_vec;
use crate::drive::Drive;
use crate::error::Error;
use std::ops::RangeFull;

use crate::error::query::QuerySyntaxError;
use crate::query::QueryItem;
use dpp::version::drive_versions::DriveVersion;
use grovedb::{PathQuery, Query, SizedQuery, TransactionArg};

impl Drive {
    /// Fetch versions by validators for the upgrade window and return them as a proved item
    pub(super) fn fetch_proved_validator_version_votes_v0(
        &self,
        start_protx_hash: Option<[u8; 32]>,
        count: u16,
        transaction: TransactionArg,
        drive_version: &DriveVersion,
    ) -> Result<Vec<u8>, Error> {
        if count == 0 {
            return Err(Error::Query(QuerySyntaxError::NoQueryItems(
                "We did not ask for the votes of any validators",
            )));
        }
        let path = desired_version_for_validators_path_vec();

        let query_item = if let Some(start_protx_hash) = start_protx_hash {
            QueryItem::RangeFrom(start_protx_hash.to_vec()..)
        } else {
            QueryItem::RangeFull(RangeFull)
        };

        let path_query = PathQuery::new(
            path,
            SizedQuery::new(Query::new_single_query_item(query_item), Some(count), None),
        );

        self.grove_get_proved_path_query(&path_query, transaction, &mut vec![], drive_version)
    }
}
