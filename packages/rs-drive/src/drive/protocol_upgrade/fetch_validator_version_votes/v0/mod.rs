use crate::drive::protocol_upgrade::{
    desired_version_for_validators_path, desired_version_for_validators_path_vec,
    versions_counter_path_vec,
};
use crate::drive::Drive;
use crate::error::Error;
use std::collections::BTreeMap;
use std::ops::RangeFull;

use crate::error::drive::DriveError;
use crate::error::query::QuerySyntaxError;
use crate::query::QueryItem;
use dpp::util::deserializer::ProtocolVersion;
use dpp::version::drive_versions::DriveVersion;
use grovedb::query_result_type::QueryResultType;
use grovedb::{PathQuery, Query, SizedQuery, TransactionArg};
use integer_encoding::VarInt;

impl Drive {
    /// Fetch versions by validators for the upgrade window and return them as a proved item
    pub(super) fn fetch_validator_version_votes_v0(
        &self,
        start_protx_hash: Option<[u8; 32]>,
        count: u16,
        transaction: TransactionArg,
        drive_version: &DriveVersion,
    ) -> Result<BTreeMap<[u8; 32], ProtocolVersion>, Error> {
        if count == 0 {
            return Err(Error::Query(QuerySyntaxError::NoQueryItems(
                "We did not ask for the votes of any validators",
            )));
        }
        let path = desired_version_for_validators_path_vec();

        let query_item = if let Some(start_protx_hash) = start_protx_hash {
            QueryItem::RangeFrom(start_protx_hash.into())
        } else {
            QueryItem::RangeFull(RangeFull)
        };

        let path_query = PathQuery::new(
            path,
            SizedQuery::new(Query::new_single_query_item(query_item), Some(count), None),
        );

        let results = self
            .grove_get_path_query(
                &path_query,
                transaction,
                QueryResultType::QueryKeyElementPairResultType,
                &mut vec![],
                drive_version,
            )?
            .0
            .to_key_elements();

        results
            .into_iter()
            .map(|(key, value)| {
                let value = value.as_item_bytes()?;
                let version = ProtocolVersion::decode_var(value)
                    .ok_or(Error::Drive(DriveError::CorruptedDriveState(
                        "version in state not representative of a ProtocolVersion".to_string(),
                    )))?
                    .0;
                Ok((key.try_into()?, version))
            })
            .collect()
    }
}
