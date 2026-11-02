use crate::drive::document::expiration::paths::{
    decode_expiration_time, documents_expirations_path_vec, encode_expiration_time,
};
use crate::drive::document::expiration::{DocumentExpirationEntry, ExpiredDocument};
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::query::GroveError;
use dpp::identifier::Identifier;
use dpp::prelude::TimestampMillis;
use dpp::version::PlatformVersion;
use grovedb::query_result_type::QueryResultType;
use grovedb::{Element, PathQuery, Query, QueryItem, SizedQuery, TransactionArg};

impl Drive {
    #[inline(always)]
    pub(super) fn fetch_expired_documents_v0(
        &self,
        block_time_ms: TimestampMillis,
        limit: u16,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<ExpiredDocument>, Error> {
        if limit == 0 {
            return Ok(vec![]);
        }

        // The expiry times that have passed, oldest first, and the entries under each.
        let mut times_query = Query::new_single_query_item(QueryItem::RangeToInclusive(
            ..=encode_expiration_time(block_time_ms).to_vec(),
        ));
        times_query.set_subquery(Query::new_range_full());
        let path_query = PathQuery::new(
            documents_expirations_path_vec(),
            SizedQuery::new(times_query, Some(limit), None),
        );
        let entries = match self.grove_get_raw_path_query(
            &path_query,
            transaction,
            QueryResultType::QueryPathKeyElementTrioResultType,
            drive_operations,
            &platform_version.drive,
        ) {
            Ok((entries, _)) => entries,
            // The tree is created with the initial state and on the first block of protocol
            // version 14; without it nothing has expired.
            Err(Error::GroveDB(e))
                if matches!(
                    e.as_ref(),
                    GroveError::PathKeyNotFound(_)
                        | GroveError::PathNotFound(_)
                        | GroveError::PathParentLayerNotFound(_)
                ) =>
            {
                return Ok(vec![]);
            }
            Err(e) => return Err(e),
        };

        entries
            .to_path_key_elements()
            .into_iter()
            .map(|(path, document_id, element)| {
                let expires_at_ms = path
                    .last()
                    .and_then(|time_key| decode_expiration_time(time_key))
                    .ok_or_else(|| {
                        Error::Drive(DriveError::CorruptedDriveState(
                            "a documents expirations tree key must be an 8 byte time".to_string(),
                        ))
                    })?;
                let Element::Item(entry_bytes, _) = element else {
                    return Err(Error::Drive(DriveError::CorruptedDriveState(
                        "a document expiration entry must be an item".to_string(),
                    )));
                };
                let entry = DocumentExpirationEntry::from_bytes(&entry_bytes)?;
                let document_id = Identifier::from_bytes(&document_id).map_err(|_| {
                    Error::Drive(DriveError::CorruptedDriveState(
                        "a document expiration entry must be keyed by a 32 byte id".to_string(),
                    ))
                })?;
                Ok(ExpiredDocument {
                    expires_at_ms,
                    document_id,
                    contract_id: entry.contract_id,
                    document_type_name: entry.document_type_name,
                })
            })
            .collect()
    }
}
