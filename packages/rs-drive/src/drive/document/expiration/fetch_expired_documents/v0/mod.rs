use crate::drive::document::expiration::paths::{
    decode_expiration_time, documents_expirations_at_time_path_vec, documents_expirations_path_vec,
    encode_expiration_time,
};
use crate::drive::document::expiration::{
    DocumentExpirationEntry, ExpiredDocument, ExpiredDocuments,
};
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::query::GroveError;
use dpp::identifier::Identifier;
use dpp::prelude::TimestampMillis;
use dpp::version::PlatformVersion;
use grovedb::query_result_type::{QueryResultElements, QueryResultType};
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
    ) -> Result<ExpiredDocuments, Error> {
        let mut expired = ExpiredDocuments::default();
        if limit == 0 {
            return Ok(expired);
        }

        // The expiry times that have passed, oldest first. Read as keys of their own, rather
        // than through a subquery, so a tree left empty by documents deleted some other way
        // is seen too and can be dropped.
        let mut times_query = Query::new();
        times_query.insert_item(QueryItem::RangeToInclusive(
            ..=encode_expiration_time(block_time_ms).to_vec(),
        ));
        let times = self.query_raw_or_nothing(
            &PathQuery::new(
                documents_expirations_path_vec(),
                SizedQuery::new(times_query, Some(limit), None),
            ),
            transaction,
            drive_operations,
            platform_version,
        )?;

        for (time_key, _) in times.to_key_elements() {
            let remaining = usize::from(limit).saturating_sub(expired.documents.len());
            if remaining == 0 {
                break;
            }
            let expires_at_ms = decode_expiration_time(&time_key).ok_or_else(|| {
                Error::Drive(DriveError::CorruptedDriveState(
                    "a documents expirations tree key must be an 8 byte time".to_string(),
                ))
            })?;
            expired.expiry_times.push(expires_at_ms);

            let mut documents_query = Query::new();
            documents_query.insert_all();
            let documents = self.query_raw_or_nothing(
                &PathQuery::new(
                    documents_expirations_at_time_path_vec(expires_at_ms),
                    SizedQuery::new(documents_query, Some(remaining as u16), None),
                ),
                transaction,
                drive_operations,
                platform_version,
            )?;
            for (document_id, element) in documents.to_key_elements() {
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
                expired.documents.push(ExpiredDocument {
                    expires_at_ms,
                    document_id,
                    contract_id: entry.contract_id,
                    document_type_name: entry.document_type_name,
                });
            }
        }
        Ok(expired)
    }

    /// Runs a raw path query, reading a missing tree as holding nothing.
    fn query_raw_or_nothing(
        &self,
        path_query: &PathQuery,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<QueryResultElements, Error> {
        match self.grove_get_raw_path_query(
            path_query,
            transaction,
            QueryResultType::QueryKeyElementPairResultType,
            drive_operations,
            &platform_version.drive,
        ) {
            Err(Error::GroveDB(e))
                if matches!(
                    e.as_ref(),
                    GroveError::PathKeyNotFound(_)
                        | GroveError::PathNotFound(_)
                        | GroveError::PathParentLayerNotFound(_)
                ) =>
            {
                Ok(QueryResultElements::new())
            }
            Err(e) => Err(e),
            Ok((elements, _)) => Ok(elements),
        }
    }
}
