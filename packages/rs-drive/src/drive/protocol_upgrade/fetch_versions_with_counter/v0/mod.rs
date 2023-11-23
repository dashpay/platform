use crate::drive::protocol_upgrade::versions_counter_path_vec;
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;

use crate::query::QueryItem;
use dpp::util::deserializer::ProtocolVersion;
use dpp::version::drive_versions::DriveVersion;
use grovedb::query_result_type::QueryResultType;
use grovedb::{PathQuery, Query, TransactionArg};
use integer_encoding::VarInt;
use nohash_hasher::IntMap;
use std::ops::RangeFull;

impl Drive {
    /// Fetch versions by count for the upgrade window
    pub(super) fn fetch_versions_with_counter_v0(
        &self,
        transaction: TransactionArg,
        drive_version: &DriveVersion,
    ) -> Result<IntMap<ProtocolVersion, u64>, Error> {
        let mut version_counter = IntMap::<ProtocolVersion, u64>::default();
        let path_query = PathQuery::new_unsized(
            versions_counter_path_vec(),
            Query::new_single_query_item(QueryItem::RangeFull(RangeFull)),
        );
        let (results, _) = self.grove_get_path_query(
            &path_query,
            transaction,
            QueryResultType::QueryKeyElementPairResultType,
            &mut vec![],
            drive_version,
        )?;
        for (version_bytes, count_element) in results.to_key_elements() {
            let version = ProtocolVersion::decode_var(version_bytes.as_slice())
                .ok_or(Error::Drive(DriveError::CorruptedElementType(
                    "encoded value could not be decoded",
                )))
                .map(|(value, _)| value)?;

            let count_bytes = count_element.as_item_bytes()?;
            let count = u64::decode_var(count_bytes)
                .ok_or(Error::Drive(DriveError::CorruptedElementType(
                    "encoded value could not be decoded",
                )))
                .map(|(value, _)| value)?;
            version_counter.insert(version, count);
        }
        Ok(version_counter)
    }
}
