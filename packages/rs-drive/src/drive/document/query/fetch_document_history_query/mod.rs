mod v0;
mod v1;

use crate::drive::document::history::{invalid, DocumentHistoryFilter, DocumentHistoryQueryV1};
use crate::drive::document::MAX_DOCUMENT_HISTORY_FETCH_LIMIT;
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use dpp::version::PlatformVersion;
use grovedb::PathQuery;

impl Drive {
    /// Creates a path query for historical entries of a specified document.
    ///
    /// Filtering by revision or by cursor is expressed through
    /// [`DocumentHistoryQueryV1::entries_query`].
    #[allow(clippy::too_many_arguments)]
    pub fn fetch_document_history_query(
        contract_id: [u8; 32],
        document_type_name: &str,
        document_id: [u8; 32],
        start_at_ms: u64,
        limit: Option<u16>,
        offset: Option<u16>,
        platform_version: &PlatformVersion,
    ) -> Result<PathQuery, Error> {
        match platform_version
            .drive
            .methods
            .document
            .query
            .fetch_document_history_query
        {
            0 => Self::fetch_document_history_query_v0(
                contract_id,
                document_type_name,
                document_id,
                start_at_ms,
                limit,
                offset,
            ),
            1 => {
                // The per-type history tree pages by a cursor, not by a
                // position within the time range.
                if offset.is_some() {
                    return Err(invalid(
                        "offset pagination is not supported by the protocol version 14 history layout",
                    ));
                }
                Self::fetch_document_history_query_v1(&DocumentHistoryQueryV1 {
                    contract_id,
                    document_type_name: document_type_name.to_string(),
                    document_id,
                    filter: DocumentHistoryFilter::StartAtTime(start_at_ms),
                    limit,
                })
            }
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "fetch_document_history_query".to_string(),
                known_versions: vec![0, 1],
                received: version,
            })),
        }
    }

    pub(crate) fn validate_document_history_limit(limit: Option<u16>) -> Result<u16, Error> {
        let limit = limit.unwrap_or(MAX_DOCUMENT_HISTORY_FETCH_LIMIT);
        if !(1..=MAX_DOCUMENT_HISTORY_FETCH_LIMIT).contains(&limit) {
            return Err(Error::Drive(DriveError::InvalidDocumentHistoryFetchLimit(
                limit,
            )));
        }

        Ok(limit)
    }
}
