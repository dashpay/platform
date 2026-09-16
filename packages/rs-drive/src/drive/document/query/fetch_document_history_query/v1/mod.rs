use crate::drive::document::paths::document_history_path;
use crate::drive::document::MAX_DOCUMENT_HISTORY_FETCH_LIMIT;
use crate::drive::Drive;
use crate::error::Error;
use crate::query::document_history_drive_query::{
    DocumentHistoryDriveQuery, DocumentHistoryFilter,
};
use crate::util::common::encode::encode_u64;
use grovedb::{PathQuery, Query, SizedQuery};

impl Drive {
    /// The leaf-only query over the per-type history tree required for
    /// authenticated count-offset pagination.
    pub(crate) fn fetch_document_history_query_v1(
        query: &DocumentHistoryDriveQuery,
    ) -> Result<PathQuery, Error> {
        query.validate()?;
        let mut grove_query = Query::new();
        let mut limit = query.limit.unwrap_or(MAX_DOCUMENT_HISTORY_FETCH_LIMIT);
        let offset = match query.filter {
            DocumentHistoryFilter::StartAtTime(time) => {
                grove_query.insert_range_from(encode_u64(time)..);
                None
            }
            DocumentHistoryFilter::StartAfter { time_ms, revision } => {
                let mut key = encode_u64(time_ms);
                key.extend(encode_u64(revision));
                grove_query.insert_range_after(key..);
                None
            }
            DocumentHistoryFilter::StartAtRevision(revision)
            | DocumentHistoryFilter::Revision(revision) => {
                grove_query.insert_all();
                if matches!(query.filter, DocumentHistoryFilter::Revision(_)) {
                    limit = 1;
                }
                (revision > 1).then_some((revision - 1) as u16)
            }
        };
        Ok(PathQuery::new(
            document_history_path(
                &query.contract_id,
                &query.document_type_name,
                &query.document_id,
            ),
            SizedQuery::new(grove_query, Some(limit), offset),
        ))
    }
}
