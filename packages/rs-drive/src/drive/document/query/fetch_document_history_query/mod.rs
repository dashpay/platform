use crate::drive::document::history::{invalid, DocumentHistoryQuery};
mod v0;

use crate::drive::document::MAX_DOCUMENT_HISTORY_FETCH_LIMIT;
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use dpp::version::PlatformVersion;
use grovedb::PathQuery;

impl Drive {
    /// Dispatches historical document queries using the selected protocol layout.
    pub fn fetch_document_history_query(
        query: &DocumentHistoryQuery,
        platform_version: &PlatformVersion,
    ) -> Result<PathQuery, Error> {
        match (
            platform_version
                .drive
                .methods
                .document
                .query
                .fetch_document_history_query,
            query,
        ) {
            (
                0,
                DocumentHistoryQuery::V0 {
                    contract_id,
                    document_type_name,
                    document_id,
                    start_at_ms,
                    limit,
                    offset,
                },
            ) => Self::fetch_document_history_query_v0(
                *contract_id,
                document_type_name,
                *document_id,
                *start_at_ms,
                *limit,
                *offset,
            ),
            (1, DocumentHistoryQuery::V1(query)) => query.entries_query_v1(),
            (0 | 1, _) => Err(invalid(
                "document history request shape is unsupported at this protocol version",
            )),
            (version, _) => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "fetch_document_history_query".to_owned(),
                known_versions: vec![0, 1],
                received: version,
            })),
        }
    }

    /// Creates a path query for historical entries of a specified document.
    #[allow(clippy::too_many_arguments)]
    pub fn fetch_document_history_query_legacy(
        contract_id: [u8; 32],
        document_type_name: &str,
        document_id: [u8; 32],
        start_at_ms: u64,
        limit: Option<u16>,
        offset: Option<u16>,
        platform_version: &PlatformVersion,
    ) -> Result<PathQuery, Error> {
        Self::fetch_document_history_query(
            &DocumentHistoryQuery::V0 {
                contract_id,
                document_type_name: document_type_name.to_owned(),
                document_id,
                start_at_ms,
                limit,
                offset,
            },
            platform_version,
        )
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
